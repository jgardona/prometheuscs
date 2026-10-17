// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "subcircuits")]
//! "Edit Model": the circuit is set aside, the model's circuit is edited, and "Save" puts the
//! circuit back with the model replaced.
//!
//! Models are kept for the whole process, so every test uses names of its own.

use prometheuscs::app::session::Session;
use prometheuscs::app::subcircuits::ModelScope;
use prometheuscs::elements::registry::Registry;
use prometheuscs::elements::subcircuit;

const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11
207 64 96 32 96 0 in
r 64 96 128 96 0 1000
c 128 96 128 160 0 1e-6 0
w 128 96 192 96 0
207 192 96 224 96 0 out
g 128 160 128 176 0
";

/// A circuit holding one instance of a model called `name`, made from the RC circuit.
fn outer(name: &str) -> Session {
    let mut maker = Session::load(RC, &Registry::standard()).unwrap();
    let mut m = maker.start_subcircuit().unwrap();
    m.name = name.into();
    maker.finish_subcircuit(m, ModelScope::Session).unwrap();
    Session::load(
        &format!("$ 1 0.000005 10 50 5 50 5e-11\n410 200 100 232 100 0 {name}\n"),
        &Registry::standard(),
    )
    .unwrap()
}

fn resistor(s: &Session) -> usize {
    s.sim.elements.iter().position(|e| e.class_name() == "ResistorElm").unwrap()
}

fn set_resistance(s: &mut Session, ohms: f64) {
    let i = resistor(s);
    let mut fields = s.edit_fields(i);
    fields[0].value = ohms;
    s.apply_edit_fields(i, fields);
}

fn model_text(name: &str) -> String {
    subcircuit::lookup(name).unwrap().to_xml().to_pretty_string()
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[test]
fn the_model_circuit_replaces_the_circuit_while_it_is_edited() {
    let mut s = outer("ctx_open");
    assert!(s.can_edit_model(0));
    assert_eq!(s.editing_model(), None);
    s.edit_model(0).unwrap();
    assert_eq!(s.editing_model(), Some("ctx_open"));
    // the canvas holds the parts of the model, not the instance
    let classes: Vec<&str> = s.sim.elements.iter().map(|e| e.class_name()).collect();
    assert!(classes.contains(&"ResistorElm") && classes.contains(&"CapacitorElm"), "{classes:?}");
    assert!(!classes.contains(&"CustomCompositeElm"));
    assert!(s.title.contains("ctx_open"));
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[test]
fn save_keeps_the_changes_the_pin_layout_and_brings_the_circuit_back() {
    let mut s = outer("ctx_save");
    let pins = || subcircuit::lookup("ctx_save").unwrap().ext.clone();
    let before = pins();
    s.edit_model(0).unwrap();
    set_resistance(&mut s, 2200.0);
    s.save_context(None).unwrap();
    assert_eq!(s.editing_model(), None);
    // the first circuit is back: one instance
    assert_eq!(s.sim.elements.len(), 1);
    assert_eq!(s.sim.elements[0].class_name(), "CustomCompositeElm");
    // and the model has the new resistor and the pins where they were
    assert!(model_text("ctx_save").contains("2200"), "{}", model_text("ctx_save"));
    assert_eq!(pins(), before);
    assert!(s.dirty, "the circuit has changed");
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[test]
fn leaving_without_saving_discards_the_changes() {
    let mut s = outer("ctx_discard");
    s.edit_model(0).unwrap();
    set_resistance(&mut s, 4700.0);
    s.leave_context();
    assert_eq!(s.editing_model(), None);
    assert_eq!(s.sim.elements.len(), 1);
    assert!(!model_text("ctx_discard").contains("4700"));
    assert!(!s.dirty, "nothing changed in the circuit that was set aside");
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[test]
fn save_copy_makes_a_new_model_and_leaves_the_old_one() {
    let mut s = outer("ctx_orig");
    s.edit_model(0).unwrap();
    set_resistance(&mut s, 3300.0);
    s.save_context(Some("ctx_copy")).unwrap();
    assert!(model_text("ctx_copy").contains("3300"));
    assert!(!model_text("ctx_orig").contains("3300"));
    assert!(s.subcircuit_names().contains(&"ctx_copy".to_string()));
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[test]
fn a_model_that_no_longer_works_keeps_the_editor_open() {
    let mut s = outer("ctx_broken");
    s.edit_model(0).unwrap();
    // without its labeled nodes the circuit has no pins
    let labels: Vec<usize> = s
        .sim
        .elements
        .iter()
        .enumerate()
        .filter(|(_, e)| e.class_name() == "LabeledNodeElm")
        .map(|(i, _)| i)
        .collect();
    for i in labels.into_iter().rev() {
        s.deselect_all();
        s.sim.elements[i].base_mut().selected = true;
        s.delete_selected();
    }
    let err = s.save_context(None).unwrap_err();
    assert!(err.contains("no external"), "{err}");
    assert_eq!(s.editing_model(), Some("ctx_broken"), "still editing");
    // a copy needs a name
    s.leave_context();
    s.edit_model(0).unwrap();
    assert!(s.save_context(Some("  ")).is_err());
    assert_eq!(s.editing_model(), Some("ctx_broken"));
}

#[test]
fn models_that_cannot_be_edited_say_so() {
    // the default model has nothing to edit
    let mut s = Session::load(
        "$ 1 0.000005 10 50 5 50 5e-11\n410 200 100 232 100 0 default\n",
        &Registry::standard(),
    )
    .unwrap();
    assert!(!s.can_edit_model(0));
    assert!(s.edit_model(0).is_err());
    // an element that is not an instance neither
    let mut r =
        Session::load("$ 1 0.000005 10 50 5 50 5e-11\nr 0 0 64 0 0 1000\n", &Registry::standard())
            .unwrap();
    assert!(r.edit_model(0).is_err());
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[test]
fn opening_another_circuit_forgets_the_one_set_aside() {
    let mut s = outer("ctx_forget");
    s.edit_model(0).unwrap();
    s.new_circuit();
    assert_eq!(s.editing_model(), None);
    s.leave_context(); // nothing to go back to: no effect
    assert!(s.sim.elements.is_empty());
}

/// A circuit with an instance of `name` driven by 5 V at its `in` pin, with the load a
/// megohm at `out`.
fn driven(name: &str) -> String {
    let alone = format!("$ 1 0.000005 10 50 5 50 5e-11\n410 200 100 232 100 0 {name}\n");
    let r = prometheuscs::format::parser::parse(&alone, &Registry::standard());
    let (pin_in, pin_out) = {
        let e = &r.circuit.elements[0];
        (e.get_post(0).unwrap(), e.get_post(1).unwrap())
    };
    format!(
        "{alone}v {x} {y0} {x} {y} 0 0 40 5 0 0 0.5\ng {x} {y0} {x} {gy} 0\nr {ox} {oy} {ox} {oy2} 0 1000000\ng {ox} {oy2} {ox} {oy3} 0\n",
        x = pin_in.x,
        y = pin_in.y,
        y0 = pin_in.y - 64,
        gy = pin_in.y - 48,
        ox = pin_out.x,
        oy = pin_out.y,
        oy2 = pin_out.y + 32,
        oy3 = pin_out.y + 48,
    )
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[test]
fn the_parts_of_an_instance_can_be_shown_and_nothing_can_be_edited_there() {
    let mut s = outer("ctx_view");
    let circuit_len = s.sim.elements.len();
    assert!(s.can_view_components(0));
    assert!(!s.inspecting());
    let view = s.view;
    s.view_components(0).unwrap();
    assert!(s.inspecting());
    assert_eq!(s.inspect_names(), ["ctx_view"]);
    // the canvas shows the parts of the model, and the circuit is as it was
    let parts: Vec<&str> = s.display_elements().iter().map(|e| e.class_name()).collect();
    assert!(parts.contains(&"ResistorElm") && parts.contains(&"CapacitorElm"), "{parts:?}");
    assert_eq!(s.sim.elements.len(), circuit_len);
    assert!(!s.editing_allowed());
    // a click does nothing there, and nothing there can be a context menu
    assert!(!s.mouse_down(10.0, 10.0, prometheuscs::app::edit::Mods::default(), 1));
    assert!(!s.select_for_context(10.0, 10.0));
    s.leave_components();
    assert!(!s.inspecting());
    assert!(s.editing_allowed());
    assert_eq!(s.view.scale, view.scale);
    assert_eq!(s.display_elements().len(), circuit_len);
}

#[test]
fn only_an_instance_with_parts_can_be_looked_into() {
    let mut s =
        Session::load("$ 1 0.000005 10 50 5 50 5e-11\nr 0 0 64 0 0 1000\n", &Registry::standard())
            .unwrap();
    assert!(!s.can_view_components(0));
    assert!(s.view_components(0).is_err());
    assert!(!s.inspecting());
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn the_parts_show_the_voltages_of_the_running_circuit() {
    use prometheuscs::app::settings::AppSettings;
    use prometheuscs::app::worker::Worker;
    use std::time::Duration;
    let name = "ctx_live";
    outer(name); // defines the model
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(&driven(name), "live", None).unwrap();
    for _ in 0..60 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let at = s.sim.elements.iter().position(|e| e.class_name() == "CustomCompositeElm").unwrap();
    s.view_components(at).unwrap();
    // the resistor inside has the voltage the running circuit gives it, not zero
    let volts: Vec<f64> = s
        .display_elements()
        .iter()
        .filter(|e| e.class_name() == "ResistorElm")
        .map(|e| e.get_voltage_diff().abs())
        .collect();
    assert!(!volts.is_empty());
    assert!(volts.iter().any(|v| *v > 1e-3), "the parts carry the circuit's voltages: {volts:?}");
}
