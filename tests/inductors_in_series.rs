// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Inductors in series keep their current when a switch changes the circuit.

use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;
use std::time::Duration;

fn current(s: &Session, i: usize) -> f64 {
    s.sim.elements[i].base().current
}

#[test]
fn series_inductors_keep_their_current_when_the_switch_is_flipped() {
    let text = std::fs::read_to_string("assets/circuits/indseries.txt").unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    for _ in 0..30 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let (single, l1, l2) = (current(&s, 14), current(&s, 15), current(&s, 16));
    assert!(l1 > 0.01 && (l1 - l2).abs() < 1e-9, "the series inductors share one current");
    // the left switch, then the circuit forms a loop with the right-most wire
    assert!(s.press_at(144.0, 100.0));
    assert!((current(&s, 15) - l1).abs() < 1e-6, "L1 keeps its current across the analysis");
    assert!((current(&s, 16) - l2).abs() < 1e-6, "L2 keeps its current across the analysis");
    for _ in 0..3 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    // the loop discharges through the wire and the right resistor, like the single inductor
    let wire = current(&s, 2).abs();
    assert!(wire > 0.5 * l1, "the right-most wire carries the loop current, got {wire}");
    let ratio = current(&s, 15) / l1;
    let single_ratio = current(&s, 14) / single;
    assert!(
        (ratio - single_ratio).abs() < 0.05,
        "decay like the 1 H inductor: {ratio} {single_ratio}"
    );
}
