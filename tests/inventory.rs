// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Every instantiable element class of the original has a registered Rust element (SC-010).
//!
//! Ignored until the last family is ported (task T073); `cargo test -- --ignored inventory`
//! lists what is still missing.

use prometheuscs::elements::registry::Registry;
use serde_json::Value;

/// Families that exist as Cargo features today, with the classes each one registers.
const FEATURE_FAMILIES: &[(&str, &[&str])] = &[
    ("passive", &["WireElm", "ResistorElm", "GroundElm"]),
    ("reactive", &["CapacitorElm", "PolarCapacitorElm", "InductorElm"]),
    (
        "sources",
        &[
            "VoltageElm",
            "DCVoltageElm",
            "ACVoltageElm",
            "RailElm",
            "ACRailElm",
            "SquareRailElm",
            "ClockElm",
            "VarRailElm",
            "CurrentElm",
        ],
    ),
    ("switches", &["SwitchElm", "PushSwitchElm", "Switch2Elm"]),
];

fn family_enabled(name: &str) -> bool {
    match name {
        "passive" => cfg!(feature = "passive"),
        "reactive" => cfg!(feature = "reactive"),
        "sources" => cfg!(feature = "sources"),
        "switches" => cfg!(feature = "switches"),
        _ => true,
    }
}

/// True when the class belongs to a family whose feature is switched off in this build.
fn skipped(class: &str) -> bool {
    FEATURE_FAMILIES
        .iter()
        .any(|(family, classes)| classes.contains(&class) && !family_enabled(family))
}

fn inventory() -> Vec<Value> {
    let text = std::fs::read_to_string("assets/inventory.json").expect("inventory.json");
    serde_json::from_str(&text).expect("valid JSON")
}

#[test]
fn inventory_lists_the_expected_number_of_classes() {
    let inv = inventory();
    assert_eq!(inv.len(), 153);
    let instantiable = inv.iter().filter(|e| e["instantiable"] == true).count();
    assert_eq!(instantiable, 149);
}

#[test]
fn ported_classes_are_registered() {
    let registry = Registry::standard();
    for (family, classes) in FEATURE_FAMILIES {
        if !family_enabled(family) {
            continue;
        }
        for class in *classes {
            assert!(registry.contains(class), "{class} of family {family} is not registered");
        }
    }
}

#[test]
#[ignore = "enabled by the inventory-completion task"]
fn every_inventory_entry_has_a_registered_element() {
    let registry = Registry::standard();
    let missing: Vec<String> = inventory()
        .iter()
        .filter(|e| e["instantiable"] == true)
        .filter_map(|e| e["class"].as_str())
        .filter(|class| !skipped(class) && !registry.contains(class))
        .map(str::to_string)
        .collect();
    assert!(missing.is_empty(), "{} classes not ported: {missing:?}", missing.len());
}
