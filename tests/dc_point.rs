// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The DC operating point: capacitors are open and the circuit is settled at once. As in the
//! original, only capacitors, batteries and alternating sources are treated specially (an
//! inductor keeps its companion model), so only capacitors are checked here.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

const HEAD: &str = "$ 1 0.000005 10 50 5 50 5e-11\n";

fn simulation(body: &str) -> Simulation {
    let r = parse(&format!("{HEAD}{body}"), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim
}

fn voltage_of(sim: &Simulation, class: &str) -> f64 {
    sim.elements.iter().find(|e| e.class_name() == class).unwrap().get_voltage_diff()
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn a_capacitor_is_open_at_the_operating_point() {
    // 10 V, 1k, 1k divider with a capacitor across the lower resistor: 5 V across it at DC,
    // where an uncharged capacitor would have held it at 0 V
    let mut sim = simulation(
        "v 0 128 0 0 0 0 40 10 0 0 0.5\nr 0 0 64 0 0 1000\nr 64 0 64 128 0 1000\nc 128 0 128 128 4 0.000001 0 0 0\nw 64 0 128 0 0\nw 64 128 128 128 0\nw 0 128 64 128 0\n",
    );
    sim.find_dc_operating_point().unwrap();
    let cap = voltage_of(&sim, "CapacitorElm");
    assert!((cap - 5.0).abs() < 1e-3, "{cap}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn the_operating_point_leaves_normal_analysis_in_place() {
    let mut sim = simulation(
        "v 0 128 0 0 0 0 40 10 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 128 4 0.000001 0 0 0\nw 64 128 0 128 0\n",
    );
    sim.find_dc_operating_point().unwrap();
    // the capacitor is a capacitor again: the run goes on, charging it over time
    sim.reset();
    sim.run_until(1e-3).unwrap();
    let cap = voltage_of(&sim, "CapacitorElm");
    assert!(cap > 5.0 && cap < 8.0, "charging after the operating point: {cap}");
}
