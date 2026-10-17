// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The controlled sources in small circuits with known answers.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

const HEAD: &str = "$ 1 0.000005 10 50 5 50 5e-11\n";

/// Runs the circuit and returns |voltage| across the element of class `class`.
fn load_voltage(body: &str, class: &str) -> f64 {
    let parsed = parse(&format!("{HEAD}{body}"), &Registry::standard());
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    for _ in 0..400 {
        sim.step().expect("step");
    }
    let e = sim.elements.iter().find(|e| e.class_name() == class).expect("load resistor");
    e.get_voltage_diff().abs()
}

const GROUNDED_OUTPUT: &str = "w 96 32 96 96 0\ng 96 96 96 128 0\n";

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn vcvs_multiplies_the_input_voltage() {
    // 2 V in, gain 3, 1 kohm load: 6 V
    let body = format!(
        "v 0 64 0 0 0 0 40 2 0 0 0.5\ng 0 64 0 96 0\n212 0 0 32 0 0 1 a*3\nr 96 0 160 0 0 1000\nw 160 0 160 96 0\ng 160 96 160 128 0\n{GROUNDED_OUTPUT}"
    );
    let v = load_voltage(&body, "ResistorElm");
    assert!((v - 6.0).abs() < 1e-3, "{v}");
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn vccs_turns_voltage_into_current() {
    // 2 V in, 1 mA/V, 1 kohm between the outputs with C- grounded: 2 V
    let body = "v 0 64 0 0 0 0 40 2 0 0 0.5\ng 0 64 0 96 0\n213 0 0 32 0 0 1 a*0.001\nr 96 0 96 32 0 1000\nw 96 32 96 96 0\ng 96 96 96 128 0\n";
    let v = load_voltage(body, "ResistorElm");
    assert!((v - 2.0).abs() < 1e-3, "{v}");
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn ccvs_turns_current_into_voltage() {
    // 2 mA through the input pair and 2000 V/A: 4 V
    let body = format!(
        "v -64 64 -64 0 0 0 40 2 0 0 0.5\nw -64 0 0 0 0\nw -64 64 -64 96 0\ng -64 96 -64 128 0\n214 0 0 32 0 0 2 2000*a\nr 0 32 0 96 0 1000\ng 0 96 0 128 0\nr 96 0 160 0 0 1000\nw 160 0 160 96 0\ng 160 96 160 128 0\n{GROUNDED_OUTPUT}"
    );
    let parsed = parse(&format!("{HEAD}{body}"), &Registry::standard());
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    for _ in 0..400 {
        sim.step().expect("step");
    }
    // the output load is the last resistor
    let load = sim.elements.iter().rev().find(|e| e.class_name() == "ResistorElm").unwrap();
    assert!((load.get_voltage_diff().abs() - 4.0).abs() < 1e-2, "{}", load.get_voltage_diff());
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn cccs_mirrors_the_sensed_current_with_gain() {
    // 2 mA sensed, gain 2: 4 mA into 1 kohm = 4 V
    let body = "v -64 64 -64 0 0 0 40 2 0 0 0.5\nw -64 0 0 0 0\nw -64 64 -64 96 0\ng -64 96 -64 128 0\n215 0 0 32 0 0 2 2*a\nr 0 32 0 96 0 1000\ng 0 96 0 128 0\nr 96 0 96 32 0 1000\nw 96 32 96 96 0\ng 96 96 96 128 0\n";
    let parsed = parse(&format!("{HEAD}{body}"), &Registry::standard());
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    for _ in 0..400 {
        sim.step().expect("step");
    }
    let load = sim.elements.iter().rev().find(|e| e.class_name() == "ResistorElm").unwrap();
    assert!((load.get_voltage_diff().abs() - 4.0).abs() < 1e-2, "{}", load.get_voltage_diff());
}
