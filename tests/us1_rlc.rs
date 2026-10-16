// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The RLC circuits of the original's auto-tests, run here and compared with the scope data the
//! original recorded (task T035, the first golden comparison).
//!
//! Each recorded sample must equal the value our solver has at some time step close to where
//! the original took it. The original takes a sample about every 64 steps but not at exactly
//! regular steps (it samples once per display frame), so the match is searched in a window and
//! must advance monotonically. Within that window the values agree to the last printed digits.

mod common;

use common::auto_test::{self, AutoTest};
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

/// Largest difference, relative to the recorded peak, accepted between us and the original.
const TOLERANCE: f64 = 1e-9;
/// Steps between recorded samples.
const STEPS_PER_SAMPLE: usize = 64;
/// How far past its nominal step a sample may have been taken.
const JITTER: usize = 70;

/// Value of every plot at every time step of a run started from a reset.
fn fine_series(t: &AutoTest) -> (Vec<Vec<f64>>, f64) {
    let parsed = parse(&t.text, &Registry::standard());
    assert!(parsed.errors.is_empty(), "{}: {:?}", t.name, parsed.errors);
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    let dt = sim.time_step();
    let mut events = t.switch_events.clone();
    events.sort_by(|a, b| a.time.total_cmp(&b.time));
    let mut next = 0;
    let steps = (t.length / dt) as usize + STEPS_PER_SAMPLE * 3;
    let mut series: Vec<Vec<f64>> = vec![Vec::new(); t.scope_data.len()];
    for _ in 0..steps {
        while next < events.len() && sim.time() >= events[next].time - dt * 1e-6 {
            sim.elements[events[next].element].simple_toggle();
            sim.analyze().expect("analysis after switch");
            next += 1;
        }
        for (values, s) in series.iter_mut().zip(&t.scope_data) {
            values.push(sim.elements[s.element].scope_value(s.value));
        }
        sim.step().expect("step");
    }
    (series, dt)
}

/// Checks one plot: every recorded sample must be found, in order, in our series.
fn compare_plot(recorded: &[f64], ours: &[f64]) -> Result<(), String> {
    let peak = recorded.iter().fold(1e-12_f64, |m, v| m.max(v.abs()));
    let close = |k: usize, j: usize| {
        ours.get(j).is_some_and(|v| (v - recorded[k]).abs() / peak <= TOLERANCE)
    };
    let mut floor = 0;
    // the first sample is taken before the first solution and holds no information
    for k in 1..recorded.len() {
        let nominal = k * STEPS_PER_SAMPLE;
        let hi = (floor + 3 * STEPS_PER_SAMPLE).min(nominal + 2 * STEPS_PER_SAMPLE + JITTER);
        match (floor..=hi).find(|&j| close(k, j)) {
            Some(j) => floor = j,
            None => {
                let nearest = (floor..=hi)
                    .filter_map(|j| ours.get(j).map(|v| (v - recorded[k]).abs() / peak))
                    .fold(f64::MAX, f64::min);
                return Err(format!(
                    "sample {k}: nearest step in {floor}..={hi} differs by {nearest:e} of the peak"
                ));
            }
        }
    }
    Ok(())
}

fn check(name: &str) {
    let t = auto_test::load(name);
    let (series, _) = fine_series(&t);
    for (i, s) in t.scope_data.iter().enumerate() {
        if let Err(e) = compare_plot(&s.values, &series[i]) {
            panic!(
                "{name} scope {} plot {} (element {}, value {}): {e}",
                s.scope, s.plot, s.element, s.value
            );
        }
    }
}

#[test]
fn lrc_matches_the_recorded_run() {
    check("lrc");
}

#[test]
fn lrcswitch_matches_the_recorded_run_through_a_switch_event() {
    check("lrcswitch");
}

#[test]
fn capac_with_an_ac_source_matches_the_recorded_run() {
    check("capac");
}
