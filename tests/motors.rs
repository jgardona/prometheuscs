// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The motors and the elements that were the last to be ported.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

fn load(name: &str) -> Simulation {
    let text = std::fs::read_to_string(format!("assets/circuits/{name}.txt")).unwrap();
    let r = parse(&text, &Registry::standard());
    assert!(r.errors.is_empty(), "{name}: {:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().unwrap_or_else(|e| panic!("{name}: {e:?}"));
    sim.reset();
    sim
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn the_three_phase_motor_spins_up_towards_the_synchronous_speed() {
    let mut sim = load("3motor");
    let i = sim.elements.iter().position(|e| e.class_name() == "ThreePhaseMotorElm").unwrap();
    for _ in 0..50000 {
        sim.step().unwrap();
    }
    // state: angle, filtered speed, speed; two pole pairs at 50 Hz turn at about 157 rad/s
    let speed = sim.elements[i].visual_state()[2];
    assert!(speed > 100.0 && speed < 220.0, "speed {speed} rad/s");
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn the_protection_switch_blows_when_its_i2t_is_used_up() {
    let mut sim = load("motorprotect");
    let i = sim.elements.iter().position(|e| e.class_name() == "MotorProtectionSwitchElm").unwrap();
    assert_eq!(sim.elements[i].visual_state()[3], 0.0, "starts closed");
    for _ in 0..60000 {
        sim.step().unwrap();
    }
    assert_eq!(sim.elements[i].visual_state()[3], 1.0, "blown after the overload");
}

#[cfg(feature = "digital")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "transistors")]
#[test]
fn custom_logic_follows_its_rules_table() {
    let mut sim = load("ledarray");
    let i = sim.elements.iter().position(|e| e.class_name() == "CustomLogicElm").unwrap();
    // the counter walks through the table; whatever the input, the rows match it
    let table = [
        "00111100", "01000010", "10101001", "10000101", "10000101", "10101001", "01000010",
        "00111100",
    ];
    let mut seen = std::collections::HashSet::new();
    let (mut last, mut stable) = (usize::MAX, 0);
    for _ in 0..7000 {
        sim.step().unwrap();
        let e = &sim.elements[i];
        let input = (0..3).fold(0, |a, k| (a << 1) | usize::from(e.get_post_voltage(k) > 2.5));
        let out: String =
            (3..11).map(|k| if e.get_post_voltage(k) > 2.5 { '1' } else { '0' }).collect();
        stable = if input == last { stable + 1 } else { 0 };
        last = input;
        // the outputs follow the inputs one step later, so judge a settled input only
        if sim.time() > 0.01 && stable > 5 {
            assert_eq!(out, table[input], "input {input}");
            seen.insert(input);
        }
    }
    assert!(seen.len() >= 4, "the counter moved through {} states", seen.len());
}
