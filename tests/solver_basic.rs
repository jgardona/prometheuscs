// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! End-to-end checks of analysis and stepping against closed-form results.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

fn simulate(text: &str) -> Simulation {
    let r = parse(text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let settings = r.circuit.options.sim_settings();
    let mut sim = Simulation::new(r.circuit.elements, settings);
    sim.analyze().expect("analysis");
    sim
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn resistor_divider_settles_to_the_ratio() {
    // 10 V source, 1k + 3k divider; the middle node is at 7.5 V
    let mut sim = simulate(
        "$ 1 0.000005 10 50 5 50 5e-11\nv 0 128 0 0 0 0 40 10 0 0 0.5\nr 0 0 64 0 0 1000\nr 64 0 64 128 0 3000\nw 64 128 0 128 0\n",
    );
    sim.run_steps(3).unwrap();
    let mid = sim.elements[1].get_post_voltage(1);
    assert!((mid - 7.5).abs() < 1e-9, "mid = {mid}");
    assert!((sim.elements[2].get_current() - 10.0 / 4000.0).abs() < 1e-9);
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn rc_charging_follows_the_exponential() {
    // 5 V, 1k, 1 uF: tau = 1 ms
    let mut sim = simulate(
        "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n",
    );
    sim.run_until(2e-3).unwrap();
    let vc = sim.elements[2].get_voltage_diff();
    let expected = 5.0 * (1.0 - (-sim.time() / 1e-3).exp());
    assert!((vc - expected).abs() < 5e-3, "vc = {vc}, expected {expected}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn lr_current_rises_with_time_constant_l_over_r() {
    // 10 V, 100 ohm, 0.1 H: tau = 1 ms, final current 0.1 A
    let mut sim = simulate(
        "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 10 0 0 0.5\nr 0 0 64 0 0 100\nl 64 0 64 64 0 0.1 0 0 0\nw 64 64 0 64 0\n",
    );
    sim.run_until(3e-3).unwrap();
    let i = sim.elements[2].get_current();
    let expected = 0.1 * (1.0 - (-sim.time() / 1e-3).exp());
    assert!((i - expected).abs() < 1e-3, "i = {i}, expected {expected}");
}

#[test]
fn analysis_reports_circuits_with_no_elements_as_ready() {
    let mut sim = Simulation::new(Vec::new(), Default::default());
    assert!(sim.analyze().is_ok());
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn lrc_example_loads_and_rings_down() {
    // Bundled default example: with the switch open the L, C and 10 ohm series resistor ring
    // down from the capacitor's initial voltage.
    let text = std::fs::read_to_string("assets/circuits/lrc.txt").expect("lrc.txt");
    let r = parse(&text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.circuit.elements.len(), 7);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    let mut peak: f64 = 0.0;
    while sim.time() < 20e-3 {
        sim.run_steps(20).unwrap();
        peak = peak.max(sim.elements[3].get_voltage_diff().abs());
    }
    let last = sim.elements[3].get_voltage_diff().abs();
    assert!(peak > 1.0, "capacitor never charged: {peak}");
    assert!(last <= peak, "ringing must not grow: {last} > {peak}");
}
