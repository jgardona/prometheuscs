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

const XML_HEAD: &str =
    "<cir f=\"1\" ts=\"0.000005\" ic=\"10\" cb=\"50\" pb=\"50\" vr=\"5\" mts=\"5e-11\">\n";

/// Loads a circuit in the XML format, analysed and run `steps` steps.
fn run_xml(body: &str, steps: usize) -> Simulation {
    let parsed = parse(&format!("{XML_HEAD}{body}\n</cir>\n"), &Registry::standard());
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    for _ in 0..steps {
        sim.step().expect("step");
    }
    sim
}

fn info(sim: &Simulation, i: usize) -> Vec<String> {
    let mut arr = Vec::new();
    sim.elements[i].get_info(&mut arr);
    arr
}

/// 5 V on the labeled node `in` (its own ground loop) and a controlled voltage source with a
/// 1 kohm load in another loop that only shares ground.
const SOURCE_AND_LABEL: &str = r#"<v x="96 288 96 192" f="0" wf="0" maxv="5"/>
<g x="96 288 96 320" f="0"/>
<ln x="96 192 96 144" f="0" te="in"/>"#;

fn diamond_with_load(tag: &str, ex: &str) -> String {
    format!(
        r#"<{tag} x="224 288 224 192" f="0" ex="{ex}"/>
<r x="224 192 320 192" f="0" r="1000"/>
<w x="320 192 320 288" f="0"/>
<w x="224 288 320 288" f="0"/>
<g x="224 288 224 320" f="0"/>"#
    )
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "outputs")]
#[test]
fn a_reference_pulls_the_named_node_into_the_sources_matrix() {
    let sim = run_xml(&format!("{SOURCE_AND_LABEL}\n{}", diamond_with_load("cv", "2*v(in)")), 1);
    // the two loops only share ground, so without the reference they would be two matrices
    let node_of = |i: usize, p: usize| sim.elements[i].base().nodes[p];
    let label_node = node_of(2, 0);
    let source_out = node_of(3, 1);
    let st = &sim.st;
    assert_eq!(st.nodes[label_node.0].matrix, st.nodes[source_out.0].matrix);
    // the reference nodes come after the two posts: v(in) reads the label's node and ground
    assert_eq!(node_of(3, 2), label_node);
    assert_eq!(node_of(3, 3).0, 0);
    assert!((sim.elements[3].get_voltage_diff() - 10.0).abs() < 1e-6);
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "outputs")]
#[test]
fn a_labeled_node_wins_over_a_meter_with_the_same_name() {
    // a voltmeter labeled `in` across a 2 V source must not be read: the node `in` is 5 V
    let meter = r#"<v x="400 288 400 192" f="0" wf="0" maxv="2"/>
<g x="400 288 400 320" f="0"/>
<p x="464 192 464 288" f="3" me="0" sc="0" re="0" lb="in"/>
<w x="400 192 464 192" f="0"/>
<w x="400 288 464 288" f="0"/>"#;
    let sim =
        run_xml(&format!("{SOURCE_AND_LABEL}\n{}\n{meter}", diamond_with_load("cv", "v(in)")), 5);
    assert!((sim.elements[3].get_voltage_diff() - 5.0).abs() < 1e-6);
    // and with no labeled node of that name the voltmeter is read
    let alone = format!("{}\n{meter}", diamond_with_load("cv", "v(in)"));
    let sim = run_xml(&alone, 5);
    assert!((sim.elements[0].get_voltage_diff() - 2.0).abs() < 1e-6);
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "outputs")]
#[test]
fn unresolved_references_read_zero_and_say_why() {
    let sim = run_xml(&diamond_with_load("cv", "1+v(nothere)+i(nothere)"), 5);
    assert!((sim.elements[0].get_voltage_diff() - 1.0).abs() < 1e-6);
    let lines = info(&sim, 0);
    assert!(lines.contains(&"unknown name: v(nothere)".to_string()), "{lines:?}");
    assert!(lines.contains(&"unknown name: i(nothere)".to_string()), "{lines:?}");
    // i() of a labeled node is not a current: no element has that reference name
    let sim = run_xml(&format!("{SOURCE_AND_LABEL}\n{}", diamond_with_load("cv", "1+i(in)")), 5);
    assert!(info(&sim, 3).contains(&"unknown name: i(in)".to_string()));
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "outputs")]
#[test]
fn a_vcvs_expression_can_use_references_too() {
    // where the VCVS's pins are: a, then V+ and V-
    let chip = r#"<VCVS x="224 192 256 192" f="0" ic="1" ex="3*v(in)"/>"#;
    let probe = run_xml(chip, 0);
    let post = |k: usize| probe.elements[0].get_post(k).unwrap();
    let (a, plus, minus) = (post(0), post(1), post(2));
    // input pin grounded, V- grounded, 1 kohm from V+ to ground: V+ = 3*v(in) = 15 V
    let body = format!(
        r#"{SOURCE_AND_LABEL}
{chip}
<g x="{} {} {} {}" f="0"/>
<g x="{} {} {} {}" f="0"/>
<r x="{} {} {} {}" f="0" r="1000"/>
<g x="{} {} {} {}" f="0"/>"#,
        a.x,
        a.y,
        a.x,
        a.y + 32,
        minus.x,
        minus.y,
        minus.x,
        minus.y + 32,
        plus.x,
        plus.y,
        plus.x + 64,
        plus.y,
        plus.x + 64,
        plus.y,
        plus.x + 64,
        plus.y + 32,
    );
    let sim = run_xml(&body, 5);
    let r = sim.elements.iter().find(|e| e.class_name() == "ResistorElm").unwrap();
    assert!((r.get_voltage_diff().abs() - 15.0).abs() < 1e-3, "{}", r.get_voltage_diff());
}

#[cfg(feature = "analog")]
#[test]
fn a_bad_expression_keeps_the_one_that_worked() {
    let reg = Registry::standard();
    for id in ["ControlledVoltageElm", "ControlledCurrentElm", "VCVSElm"] {
        let mut e = reg.create(id, 0, 0).unwrap();
        let mut ei = e.get_edit_info(0).unwrap();
        let before = ei.text.clone().unwrap();
        ei.text = Some("2*(".into());
        e.set_edit_value(0, &mut ei);
        let err = ei.error.clone().unwrap_or_default();
        assert!(err.starts_with("Parse error in expression: 2*(: "), "{id}: {err}");
        assert_eq!(e.get_edit_info(0).unwrap().text.unwrap(), before, "{id}");
        // a good one with references changes the number of nodes the element needs: two each
        let without_refs = e.node_count() - 2 * e.expr_refs().len();
        let mut ei = e.get_edit_info(0).unwrap();
        ei.text = Some("v(a)+i(b)".into());
        e.set_edit_value(0, &mut ei);
        assert!(ei.error.is_none(), "{id}: {:?}", ei.error);
        assert_eq!(e.node_count(), without_refs + 4, "{id}");
    }
}
