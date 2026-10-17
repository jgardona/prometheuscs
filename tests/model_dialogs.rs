// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The model dialogs: creating and editing diode, transistor, MOSFET and relay models for an
//! element, and what the circuit does with them.

use prometheuscs::app::session::Session;
use prometheuscs::elements::model_edit::ModelEditor;
use prometheuscs::elements::models::{DiodeModel, TransistorModel};
use prometheuscs::elements::registry::Registry;

const CIRCUIT: &str = "$ 1 0.000005 10 50 5 50 5e-11
v 160 64 160 160 0 0 40 5 0 0 0.5
r 160 160 224 160 0 1000
d 224 160 224 224 2 default
g 224 224 224 240 0
t 288 160 352 160 0 1 0 0 100
s 352 64 352 160 0 1 false
w 288 64 352 64 0
r 288 64 288 160 0 100
g 352 224 352 240 0
f 400 160 464 160 0 1.5 0.02
r 400 64 400 160 0 100
";

fn session() -> Session {
    Session::load(CIRCUIT, &Registry::standard()).unwrap()
}

fn index_of(s: &Session, class: &str) -> usize {
    s.sim.elements.iter().position(|e| e.class_name() == class).unwrap()
}

#[cfg(feature = "diodes")]
#[cfg(feature = "mosfets")]
#[cfg(feature = "transistors")]
#[test]
fn the_elements_offer_model_buttons() {
    let s = session();
    let d = index_of(&s, "DiodeElm");
    // the default model is built in, so it cannot be edited
    assert_eq!(s.model_buttons(d), ["Create New Simple Model", "Create New Advanced Model"]);
    for class in ["NTransistorElm", "NMosfetElm"] {
        let i = index_of(&s, class);
        assert_eq!(s.model_buttons(i), ["Create New Model"], "{class}");
    }
}

#[cfg(feature = "diodes")]
#[test]
fn a_new_diode_model_is_used_by_the_element_saved_and_undone() {
    let mut s = session();
    let d = index_of(&s, "DiodeElm");
    let mut editor = s.start_model_edit(d, 0).expect("simple model dialog");
    let mut fields = editor.fields();
    fields[2].value = 0.6;
    fields[3].value = 0.005;
    assert_eq!(s.finish_model_edit(d, &mut editor, &mut fields), Ok(false));
    let name = editor.name().to_string();
    assert_eq!(name, "fwdrop=0.6");
    assert!(DiodeModel::lookup(&name).is_some());
    // the element uses it and now the model can be edited
    assert_eq!(s.model_buttons(d).last().unwrap(), "Edit Model");
    let xml = s.circuit_xml();
    assert!(xml.contains("fwdrop=0.6"), "the model is saved with the circuit");
    let again = Session::load(&xml, &Registry::standard()).unwrap();
    assert_eq!(again.model_buttons(d).last().unwrap(), "Edit Model");
    s.undo();
    assert_eq!(s.model_buttons(d).len(), 2, "undo returns to the built-in model");
}

#[cfg(feature = "diodes")]
#[test]
fn editing_a_model_changes_every_element_that_uses_it() {
    let mut s = session();
    let d = index_of(&s, "DiodeElm");
    let mut e = s.start_model_edit(d, 1).unwrap();
    let mut f = e.fields();
    f[0].text = Some("mine".into());
    s.finish_model_edit(d, &mut e, &mut f).unwrap();
    let before = DiodeModel::lookup("mine").unwrap().emission_coefficient;
    // edit it in place: a larger emission coefficient
    let mut e = s.start_model_edit(d, 2).expect("edit model button");
    let mut f = e.fields();
    f[3].value = before * 2.0;
    s.finish_model_edit(d, &mut e, &mut f).unwrap();
    assert_eq!(DiodeModel::lookup("mine").unwrap().emission_coefficient, before * 2.0);
}

#[cfg(feature = "mosfets")]
#[cfg(feature = "transistors")]
#[test]
fn a_program_model_name_is_refused_and_the_other_kinds_work() {
    let mut s = session();
    let t = index_of(&s, "NTransistorElm");
    let mut e = s.start_model_edit(t, 0).unwrap();
    let mut f = e.fields();
    f[0].text = Some("default".into());
    assert!(s.finish_model_edit(t, &mut e, &mut f).is_err());
    f[0].text = Some("highgain".into());
    f[2].value = 5.0;
    assert_eq!(s.finish_model_edit(t, &mut e, &mut f), Ok(false));
    assert_eq!(TransistorModel::lookup("highgain").unwrap().beta_r, 5.0);

    let m = index_of(&s, "NMosfetElm");
    let mut e = s.start_model_edit(m, 0).unwrap();
    assert!(matches!(e, ModelEditor::Mosfet(_)));
    let mut f = e.fields();
    f[1].value = 2.5;
    assert_eq!(s.finish_model_edit(m, &mut e, &mut f), Ok(false));
    assert_eq!(e.name(), "mosfetmodel");
}

#[cfg(feature = "passive")]
#[test]
fn sliders_can_be_added_changed_and_removed_for_a_field() {
    let mut s = session();
    let r = s.sim.elements.iter().position(|e| e.class_name() == "ResistorElm").unwrap();
    s.sim.elements[r].base_mut().selected = true;
    assert_eq!(s.selected_slider_element(), Some(r));
    let mut entries = s.slider_entries(r);
    assert_eq!(entries.len(), 1, "a resistor has one field with a number");
    assert!(!entries[0].enabled);
    assert_eq!(s.adjustables.len(), 0);
    entries[0].enabled = true;
    entries[0].min = 100.0;
    entries[0].max = 10_000.0;
    entries[0].logarithmic = true;
    s.apply_sliders(r, &entries).unwrap();
    assert_eq!(s.adjustables.len(), 1);
    assert_eq!(s.slider_positions.len(), 1);
    // it works and is saved with the circuit
    let value = s.set_slider(0, 50.0).unwrap();
    assert!((value - 1000.0).abs() < 1.0, "{value}: halfway on a logarithmic 100 to 10k");
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    assert_eq!(again.adjustables.len(), 1);
    assert!(again.adjustables[0].logarithmic);
    // a reversed range is refused
    let mut bad = s.slider_entries(r);
    bad[0].min = 5.0;
    bad[0].max = 1.0;
    assert!(s.apply_sliders(r, &bad).is_err());
    // switch it off
    let mut off = s.slider_entries(r);
    off[0].enabled = false;
    s.apply_sliders(r, &off).unwrap();
    assert_eq!(s.adjustables.len(), 0);
}
