// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Operating a switch keeps the running state of the circuit: in the "Light Switch" example,
//! opening the switch while current flows into the load sends a negative step down the second
//! transmission line, as in the original, instead of starting both lines again from zero.

use prometheuscs::app::session::Session;
use prometheuscs::app::settings::AppSettings;
use prometheuscs::app::worker::{Command, Worker};
use prometheuscs::elements::registry::Registry;
use prometheuscs::solver::snapshot::FrameSnapshot;
use std::time::Duration;

fn light_switch() -> String {
    std::fs::read_to_string("assets/circuits/tllight.txt").unwrap()
}

/// Indices of the elements with this dump type, in the order of the file.
fn of_type(s: &Session, t: i32) -> Vec<usize> {
    s.sim.elements.iter().enumerate().filter(|(_, e)| e.dump_type() == t).map(|(i, _)| i).collect()
}

/// The voltages along a transmission line in a snapshot (after its two currents).
fn segments(snap: &FrameSnapshot, line: usize) -> &[f64] {
    &snap.element_states[line][2..]
}

fn lowest(v: &[f64]) -> f64 {
    v.iter().copied().fold(f64::MAX, f64::min)
}

/// Clicks element `i` where it takes clicks, as the pointer does: finds a point of the surface
/// that lands in the middle of its click area.
fn click(s: &mut Session, i: usize) -> bool {
    let r = s.sim.elements[i].click_rect().expect("a clickable element");
    let (cx, cy) = (r.x + r.width / 2, r.y + r.height / 2);
    for py in 0..1500 {
        for px in 0..2000 {
            let (gx, gy) = s.view.to_circuit(f64::from(px), f64::from(py));
            if (gx - cx).abs() <= 1 && (gy - cy).abs() <= 1 {
                return s.press_at(f64::from(px), f64::from(py));
            }
        }
    }
    panic!("no point of the surface reaches the switch");
}

fn run(w: &Worker, steps: usize) -> FrameSnapshot {
    w.send(Command::RunSteps(steps));
    assert!(w.sync());
    w.latest().expect("a snapshot")
}

#[test]
fn opening_the_switch_in_the_worker_sends_a_negative_wave_down_the_second_line() {
    let mut s = Session::load(&light_switch(), &Registry::standard()).unwrap();
    let (lines, switch) = (of_type(&s, 171), of_type(&s, 115)[0]);
    let second = lines[1];
    let w = Worker::spawn(Registry::standard(), AppSettings::default());
    w.send(Command::Load { xml: s.circuit_xml(), keep_time: false });
    w.send(Command::SetRunning(false));
    // 0.3 us: the waves have reached the load, which draws current through the closed switch
    let before = run(&w, 30_000);
    assert!(lowest(segments(&before, second)) > 0.0, "{:?}", segments(&before, second));
    assert!(segments(&before, lines[0]).iter().all(|v| *v > 1.0), "the first line is charged");
    w.send(Command::Toggle(vec![switch]));
    let mut low = f64::MAX;
    let mut first_low = f64::MAX;
    for _ in 0..40 {
        let snap = run(&w, 50);
        low = low.min(lowest(segments(&snap, second)));
        first_low = first_low.min(lowest(segments(&snap, lines[0])));
    }
    assert!(low < -100.0, "the interrupted current gives a negative step: lowest {low}");
    // the first line keeps its charge: it does not start again from zero
    assert!(first_low > 1.0, "the first line kept its waves: lowest {first_low}");
}

#[test]
fn a_click_on_the_switch_with_a_worker_keeps_the_waves_on_the_lines() {
    let mut s = Session::load(&light_switch(), &Registry::standard()).unwrap();
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    // time moves only by the steps asked for below
    s.set_running(false);
    let (lines, switch) = (of_type(&s, 171), of_type(&s, 115)[0]);
    let before = run(s.worker_handle().unwrap(), 30_000);
    assert!(segments(&before, lines[0]).iter().all(|v| *v > 1.0), "the first line is charged");
    // the click is what the pointer does: it goes through the session to the worker
    assert!(click(&mut s, switch), "the switch is operated");
    let mut low = f64::MAX;
    let mut first_low = f64::MAX;
    for _ in 0..40 {
        let snap = run(s.worker_handle().unwrap(), 50);
        low = low.min(lowest(segments(&snap, lines[1])));
        first_low = first_low.min(lowest(segments(&snap, lines[0])));
    }
    assert!(low < -100.0, "the click kept the state of the lines: lowest {low}");
    assert!(first_low > 1.0, "the first line was not emptied by the click: lowest {first_low}");
}

#[test]
fn without_a_worker_the_click_also_keeps_the_waves() {
    let mut s = Session::load(&light_switch(), &Registry::standard()).unwrap();
    let (lines, switch) = (of_type(&s, 171), of_type(&s, 115)[0]);
    for _ in 0..400 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    assert!(click(&mut s, switch));
    let mut low = f64::MAX;
    for _ in 0..200 {
        s.update(Duration::from_millis(16)).unwrap();
        low = low.min(lowest(&s.sim.elements[lines[1]].visual_state()[2..]));
    }
    assert!(low < -100.0, "lowest {low}");
}

#[test]
fn a_momentary_switch_pressed_and_released_through_the_worker_ends_where_it_started() {
    // the same circuit with a push button, open at rest
    let text = light_switch().replace("s 384 176 432 176 0 0 false", "s 384 176 432 176 0 1 true");
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.set_running(false);
    let switch = of_type(&s, 115)[0];
    let _ = run(s.worker_handle().unwrap(), 1_000);
    // the dump of the switch as the worker has it (reading the circuit fetches the live state)
    let position = |s: &mut Session| {
        let _ = s.circuit_xml();
        s.sim.elements[switch].dump()
    };
    assert!(position(&mut s).ends_with(" 1 true"), "open at rest");
    assert!(click(&mut s, switch));
    assert!(position(&mut s).ends_with(" 0 true"), "closed while held, in the worker too");
    let _ = run(s.worker_handle().unwrap(), 100);
    s.mouse_up();
    assert!(position(&mut s).ends_with(" 1 true"), "open again once released");
}
