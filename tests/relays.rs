// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Relays closing a contact when their coil carries enough current.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

const HEAD: &str = "$ 1 0.000005 10 50 5 50 5e-11\n";

fn load_voltage_after(body: &str, seconds: f64) -> f64 {
    let parsed = parse(&format!("{HEAD}{body}"), &Registry::standard());
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    let steps = (seconds / sim.time_step()) as usize;
    for _ in 0..steps {
        sim.step().expect("step");
    }
    let e = sim.elements.iter().rev().find(|e| e.class_name() == "ResistorElm").expect("load");
    e.get_voltage_diff().abs()
}

/// A coil driven by 5 V (250 mA through 20 ohms) with a separate contact in series with a
/// second 5 V source and a 1 kohm load.
fn coil_and_contact(contact_flags: i32, coil_voltage: &str) -> String {
    format!(
        "425 0 0 0 64 0 Q1 0.2 0 0.02 20 0.015 0.005 0 0 0\nv 64 0 64 64 0 0 40 {coil_voltage} 0 0 0.5\nw 0 0 64 0 0\nw 0 64 64 64 0\ng 64 64 64 96 0\n426 128 0 128 64 {contact_flags} Q1 0.05 1000000 0\nv 192 0 192 64 0 0 40 5 0 0 0.5\nw 128 0 192 0 0\ng 192 64 192 96 0\nr 128 64 128 128 0 1000\ng 128 128 128 160 0\n"
    )
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn contact_closes_after_the_switching_time() {
    let closed = load_voltage_after(&coil_and_contact(0, "5"), 0.05);
    assert!(closed > 4.9, "{closed}");
    let early = load_voltage_after(&coil_and_contact(0, "5"), 0.001);
    assert!(early < 0.1, "{early}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn an_unpowered_coil_leaves_the_contact_open() {
    let v = load_voltage_after(&coil_and_contact(0, "0"), 0.05);
    assert!(v < 0.01, "{v}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn a_normally_closed_contact_opens_when_the_coil_operates() {
    let idle = load_voltage_after(&coil_and_contact(2, "0"), 0.05);
    assert!(idle > 4.9, "{idle}");
    let operated = load_voltage_after(&coil_and_contact(2, "5"), 0.05);
    assert!(operated < 0.01, "{operated}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn bundled_relay_example_operates_and_conducts() {
    let text = std::fs::read_to_string("assets/circuits/relay.txt").unwrap();
    let parsed = parse(&text, &Registry::standard());
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    let relay = sim.elements.iter().position(|e| e.class_name() == "RelayElm").unwrap();
    assert_eq!(sim.elements[relay].visual_state()[1], 0.0, "starts released");
    for _ in 0..6000 {
        sim.step().expect("step");
    }
    let state = sim.elements[relay].visual_state();
    assert_eq!(state[1], 1.0, "operated");
    assert!((state[3] - 0.05).abs() < 1e-3, "5 V through 100 ohms: {}", state[3]);
}
