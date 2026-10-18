// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Saving (and the undo stack, and the automatic save) copies the running state from the
//! simulation's thread into the window's elements. That must not change where an element's
//! terminals are: chips used to lose their pins the first time the circuit was saved.

use prometheuscs::app::session::Session;
use prometheuscs::app::settings::AppSettings;
use prometheuscs::app::worker::Worker;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::writer::write_elements_xml;
use std::time::Duration;

fn with_worker(text: &str) -> Session {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    let _ = s.open_text(text, "test", None);
    for _ in 0..10 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(8));
    }
    s
}

fn posts(s: &Session) -> Vec<Vec<(i32, i32)>> {
    s.sim
        .elements
        .iter()
        .map(|e| (0..e.post_count()).filter_map(|n| e.get_post(n)).map(|p| (p.x, p.y)).collect())
        .collect()
}

#[cfg(feature = "digital")]
#[test]
fn a_rom_keeps_its_pins_when_the_circuit_is_saved() {
    let mut s = with_worker("$ 1 0.000005 10 50 5 50 5e-11\n436 256 128 288 128 0\n");
    let before = posts(&s);
    assert_eq!(before[0].len(), 9, "the ROM has its pins");
    assert!(before[0].iter().all(|p| *p != (0, 0)), "{before:?}");
    // what the automatic save, Save and the undo stack all do first
    let _ = s.circuit_xml();
    assert_eq!(posts(&s), before);
    // and the same through the undo stack and the automatic save
    s.push_undo();
    assert_eq!(posts(&s), before);
}

#[test]
fn no_element_moves_its_terminals_when_the_circuit_is_saved() {
    let registry = Registry::standard();
    let mut changed = Vec::new();
    for id in registry.ids() {
        let Some(mut e) = registry.create(id, 256, 128) else { continue };
        e.drag(320, 128);
        e.set_points();
        let text = write_elements_xml(std::iter::once(e.as_ref()));
        let mut s = with_worker(&text);
        let before = posts(&s);
        let _ = s.circuit_xml();
        if posts(&s) != before {
            changed.push(id);
        }
    }
    assert!(changed.is_empty(), "these elements move their terminals when saved: {changed:?}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn the_running_state_still_reaches_the_saved_text() {
    // 5 V into 1 kohm and 1 uF: after a while the capacitor holds charge, and the text
    // written while it runs has it
    let text = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n";
    let mut s = with_worker(text);
    for _ in 0..60 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let xml = s.circuit_xml();
    let line = xml.lines().find(|l| l.trim_start().starts_with("<c ")).expect("the capacitor");
    let vd: f64 =
        line.split("vd=\"").nth(1).and_then(|r| r.split('"').next()).unwrap().parse().unwrap();
    assert!(vd > 1.0, "the charge was saved: {line}");
}
