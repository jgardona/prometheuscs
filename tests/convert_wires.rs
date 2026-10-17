// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "passive")]
//! Tools > Convert Wires to Routed Wires.

use prometheuscs::app::commands::{Cmd, dispatch};
use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;

const HEAD: &str = "$ 1 0.000005 10 50 5 50 5e-11\n";
// a source, three wires in a row (two corners), then a resistor; a fourth wire branches off
// the middle of the row, which makes that point a junction
const CIRCUIT: &str = "v 0 64 0 0 0 0 40 5 0 0 0.5
w 0 0 32 0 0
w 32 0 32 32 0
w 32 32 64 32 0
r 64 32 64 96 0 1000
w 64 96 0 96 0
w 0 96 0 64 0
";

fn load() -> Session {
    Session::load(&format!("{HEAD}{CIRCUIT}"), &Registry::standard()).unwrap()
}

fn classes(s: &Session) -> Vec<&'static str> {
    s.sim.elements.iter().map(|e| e.class_name()).collect()
}

fn run_to_voltage(s: &mut Session) -> f64 {
    s.sim.run_steps(50).unwrap();
    s.sim.elements.iter().find(|e| e.class_name() == "ResistorElm").unwrap().get_voltage_diff()
}

#[test]
fn chains_of_wires_become_routed_wires_and_the_circuit_still_works() {
    let mut s = load();
    let before = run_to_voltage(&mut s);
    let mut s = load();
    let made = s.convert_wires();
    assert!(made >= 1);
    let c = classes(&s);
    assert!(c.contains(&"RoutedWireElm"), "{c:?}");
    assert!(c.len() < 7, "wires were joined: {c:?}");
    assert!(!c.contains(&"WireElm"), "every wire here is in a chain: {c:?}");
    // the same circuit: the resistor sees the same voltage
    assert!((run_to_voltage(&mut s) - before).abs() < 1e-9);
    // and it survives saving and loading
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    assert_eq!(classes(&again).iter().filter(|c| **c == "RoutedWireElm").count(), made);
}

#[test]
fn it_can_be_undone_and_does_nothing_twice() {
    let mut s = load();
    let before = s.circuit_xml();
    assert!(s.convert_wires() >= 1);
    assert_eq!(s.convert_wires(), 0, "routed wires are not converted again");
    assert!(s.undo());
    assert_eq!(s.circuit_xml(), before);
}

#[test]
fn only_the_selected_wires_are_converted_when_some_are_selected() {
    let mut s = load();
    let first = s.sim.elements.iter().position(|e| e.class_name() == "WireElm").unwrap();
    s.deselect_all();
    s.sim.elements[first].base_mut().selected = true;
    // one wire alone: it becomes a routed wire of two points, the others stay
    assert_eq!(s.convert_wires(), 1);
    let c = classes(&s);
    assert_eq!(c.iter().filter(|c| **c == "RoutedWireElm").count(), 1);
    assert_eq!(c.iter().filter(|c| **c == "WireElm").count(), 4);
}

#[test]
fn the_menu_entry_works_through_the_command() {
    let mut s = load();
    assert!(Cmd::ConvertWires.enabled(&s));
    dispatch(&mut s, &Cmd::ConvertWires);
    assert!(classes(&s).contains(&"RoutedWireElm"));
    let mut empty = Session::load(HEAD, &Registry::standard()).unwrap();
    assert!(!Cmd::ConvertWires.enabled(&empty));
    assert_eq!(empty.convert_wires(), 0);
}
