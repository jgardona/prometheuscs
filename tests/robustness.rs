// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The program must not panic on any bundled circuit or any sequence of user actions: a panic
//! inside a GTK callback aborts the whole application.

use prometheuscs::app::commands::{Cmd, dispatch};
use prometheuscs::app::edit::{Mods, Toggle, Tool, Transform};
use prometheuscs::app::session::Session;
use prometheuscs::app::settings::AppSettings;
use prometheuscs::app::worker::Worker;
use prometheuscs::elements::registry::Registry;
use prometheuscs::render::scene::{View, draw_scene, post_draw_list};
use prometheuscs::render::svg::SvgPainter;
use std::time::Duration;

fn examples() -> Vec<(String, String)> {
    let mut v: Vec<_> = std::fs::read_dir("assets/circuits")
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap_or_default(),
            )
        })
        .collect();
    v.sort();
    v
}

fn draw(s: &mut Session) {
    let posts = post_draw_list(&s.sim.elements);
    let mut g = SvgPainter::new(800.0, 600.0);
    draw_scene(&mut g, &mut s.sim.elements, &posts, &s.draw_ctx, View::default(), (800.0, 600.0));
}

#[test]
fn every_bundled_example_loads_runs_draws_and_survives_editing() {
    for (name, text) in examples() {
        let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
        let _ = s.open_text(&text, &name, None);
        for _ in 0..5 {
            s.update(Duration::from_millis(16)).ok();
        }
        draw(&mut s);
        s.select_all();
        s.copy_selected();
        s.paste();
        s.undo();
        s.transform(Transform::RotateCw);
        s.transform(Transform::MirrorX);
        s.delete_selected();
        s.undo();
        draw(&mut s);
        assert!(!name.is_empty());
    }
}

#[test]
fn the_worker_survives_bundled_examples() {
    let all = examples();
    let worker = Worker::spawn(Registry::standard(), AppSettings::default());
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(worker);
    for (name, text) in all.iter().step_by(4) {
        let _ = s.open_text(text, name, None);
        for _ in 0..4 {
            s.update(Duration::from_millis(16)).ok();
            std::thread::sleep(Duration::from_millis(3));
        }
        draw(&mut s);
        // still answering after this circuit means the thread did not die
        assert!(s.worker_handle().unwrap().sync(), "worker stopped on {name}");
    }
}

/// A small deterministic generator, so a failure can be replayed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn coord(&mut self) -> f64 {
        self.below(500) as f64
    }
}

#[test]
fn random_user_actions_never_panic_with_a_worker() {
    let text = std::fs::read_to_string("assets/circuits/lrc.txt").unwrap();
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(&text, "lrc", None).unwrap();
    let ids: Vec<&'static str> = s.registry.ids().collect();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for step in 0..600 {
        let (x, y) = (rng.coord(), rng.coord());
        match rng.below(14) {
            0 => {
                let id = ids[rng.below(ids.len() as u64) as usize];
                dispatch(&mut s, &Cmd::Place(id.to_string()));
            }
            1 => {
                dispatch(&mut s, &Cmd::SetTool(Tool::Select));
            }
            2..=4 => {
                s.mouse_down(x, y, Mods::default(), 1);
                for _ in 0..rng.below(4) {
                    s.mouse_drag(rng.coord(), rng.coord());
                }
                s.mouse_up();
            }
            5 => {
                dispatch(&mut s, &Cmd::SelectAll);
            }
            6 => {
                dispatch(&mut s, &Cmd::Duplicate);
            }
            7 => {
                dispatch(&mut s, &Cmd::Delete);
            }
            8 => {
                dispatch(&mut s, &Cmd::Undo);
            }
            9 => {
                dispatch(&mut s, &Cmd::Redo);
            }
            10 => {
                dispatch(&mut s, &Cmd::RotateCcw);
            }
            11 => {
                dispatch(&mut s, &Cmd::MirrorY);
            }
            12 => {
                s.press_at(x, y);
                s.mouse_up();
                dispatch(&mut s, &Cmd::Toggle(Toggle::SmallGrid));
            }
            _ => {
                s.set_slider(0, rng.below(101) as f64);
            }
        }
        s.update(Duration::from_millis(16)).ok();
        if step % 50 == 0 {
            draw(&mut s);
            assert!(s.worker_handle().unwrap().sync(), "worker stopped at step {step}");
        }
    }
    draw(&mut s);
}
