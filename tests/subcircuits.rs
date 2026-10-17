// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Subcircuit instances (`cc`) in small circuits with known answers.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::{parser::parse, writer};
use prometheuscs::solver::sim::Simulation;

const DIVIDER: &str = r#"<ccm nm="div" f="1" sx="2" sy="2">
  <ext nm="A" nd="1" ps="0" sd="2"/>
  <ext nm="Y" nd="2" ps="0" sd="3"/>
  <r nn="1 2" x="0 0 16 0" f="0" r="1000"/>
  <r nn="2 0" x="0 16 16 16" f="0" r="1000"/>
</ccm>"#;

const INVERTER: &str = r#"<ccm nm="inv" f="1" sx="2" sy="2">
  <ext nm="A" nd="1" ps="0" sd="2"/>
  <ext nm="Y" nd="2" ps="0" sd="3"/>
  <I nn="1 2" x="0 0 48 0" f="0" sl="0.5" hi="5"/>
</ccm>"#;

fn cir(body: &str) -> String {
    format!(
        "<cir f=\"1\" ts=\"0.000005\" ic=\"10\" cb=\"50\" pb=\"50\" vr=\"5\" mts=\"5e-11\">\n{body}\n</cir>"
    )
}

/// Positions of the pins of the instance of `model` placed at the origin of a trial load.
fn pins(model: &str, name: &str) -> Vec<(i32, i32)> {
    let text = cir(&format!("{model}\n<cc x=\"160 160 176 160\" f=\"1\" mo=\"{name}\"/>"));
    let r = parse(&text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let e = r.circuit.elements.last().unwrap();
    (0..e.post_count()).map(|i| e.get_post(i).map(|p| (p.x, p.y)).unwrap()).collect()
}

fn build(model: &str, name: &str, volts: f64) -> (Simulation, usize) {
    let p = pins(model, name);
    let (ax, ay) = p[0];
    let text = cir(&format!(
        "{model}\n<cc x=\"160 160 176 160\" f=\"1\" mo=\"{name}\"/>\n\
         <v x=\"{sx} {ay} {ax} {ay}\" f=\"0\" wf=\"0\" maxv=\"{volts}\"/>\n\
         <g x=\"{sx} {ay} {sx} {gy}\" f=\"0\"/>",
        sx = ax - 64,
        gy = ay + 16,
    ));
    let r = parse(&text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    for _ in 0..50 {
        sim.step().expect("step");
    }
    let cc = sim.elements.iter().position(|e| e.xml_dump_type() == "cc").unwrap();
    (sim, cc)
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[test]
fn a_resistor_divider_inside_a_subcircuit_divides() {
    let (sim, cc) = build(DIVIDER, "div", 10.0);
    let e = &sim.elements[cc];
    assert!((e.get_post_voltage(0) - 10.0).abs() < 1e-6);
    assert!((e.get_post_voltage(1) - 5.0).abs() < 1e-6, "{}", e.get_post_voltage(1));
    // the source delivers 10 V across 2 k
    let src = sim.elements.iter().find(|e| e.xml_dump_type() == "v").unwrap();
    assert!((src.get_current().abs() - 5e-3).abs() < 1e-6, "{}", src.get_current());
    assert!((e.get_current_into_node(0).abs() - 5e-3).abs() < 1e-6);
}

#[cfg(feature = "logic")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[test]
fn gates_inside_a_subcircuit_drive_their_output() {
    let (high, cc) = build(INVERTER, "inv", 5.0);
    assert!(high.elements[cc].get_post_voltage(1).abs() < 0.1);
    let (low, cc) = build(INVERTER, "inv", 0.0);
    assert!((low.elements[cc].get_post_voltage(1) - 5.0).abs() < 0.1);
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[test]
fn saving_writes_the_models_and_loading_again_gives_the_same_circuit() {
    let (sim, _) = build(DIVIDER, "div", 10.0);
    let mut circuit = prometheuscs::format::circuit::Circuit::new();
    circuit.elements = sim.elements;
    let text = writer::write_xml(&circuit);
    assert!(text.contains("<ccm nm=\"div\""), "{text}");
    let again = parse(&text, &Registry::standard());
    assert!(again.errors.is_empty(), "{:?}", again.errors);
    let mut sim = Simulation::new(again.circuit.elements, again.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    for _ in 0..50 {
        sim.step().expect("step");
    }
    let cc = sim.elements.iter().position(|e| e.xml_dump_type() == "cc").unwrap();
    assert!((sim.elements[cc].get_post_voltage(1) - 5.0).abs() < 1e-6);
}

#[cfg(feature = "digital")]
#[cfg(feature = "diodes")]
#[cfg(feature = "logic")]
#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[cfg(feature = "switches")]
#[test]
fn nested_subcircuits_work_and_the_example_files_run() {
    for name in ["adder4-sc", "brentkung", "td4", "fullrect-sc", "td4-ctr"] {
        let text = std::fs::read_to_string(format!("assets/circuits/{name}.txt")).unwrap();
        let r = parse(&text, &Registry::standard());
        assert!(r.errors.is_empty(), "{name}: {:?}", r.errors);
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        sim.analyze().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        sim.reset();
        for _ in 0..200 {
            sim.step().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        }
    }
}

fn run_text(text: &str, steps: usize) -> Simulation {
    let r = parse(&format!("$ 1 0.000005 10 50 5 50 5e-11\n{text}"), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    for _ in 0..steps {
        sim.step().expect("step");
    }
    sim
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[cfg(feature = "transistors")]
#[test]
fn a_darlington_pair_saturates_from_a_small_base_current() {
    let line = "400 160 160 208 160 0";
    let r = parse(&format!("$ 1 0.000005 10 50 5 50 5e-11\n{line}\n"), &Registry::standard());
    let e = &r.circuit.elements[0];
    let p: Vec<(i32, i32)> = (0..3).map(|i| e.get_post(i).map(|p| (p.x, p.y)).unwrap()).collect();
    let ((bx, by), (cx, cy), (ex, ey)) = (p[0], p[1], p[2]);
    let text = format!(
        "{line}\n\
         v {cx} {c0} {cx} {c1} 0 0 40 10 0 0 0.5\n\
         g {cx} {c0} {cx} {c0g} 0\n\
         r {cx} {c1} {cx} {cy} 0 1000\n\
         v {b0} {by} {b1} {by} 0 0 40 5 0 0 0.5\n\
         g {b0} {by} {b0} {byg} 0\n\
         r {b1} {by} {bx} {by} 0 100000\n\
         g {ex} {ey} {ex} {eyg} 0\n",
        c0 = cy - 96,
        c0g = cy - 80,
        c1 = cy - 64,
        b0 = bx - 128,
        b1 = bx - 64,
        byg = by + 16,
        eyg = ey + 16,
    );
    let sim = run_text(&text, 2000);
    let d = &sim.elements[0];
    let (vb, vc) = (d.get_post_voltage(0), d.get_post_voltage(1));
    assert!(vb > 1.0 && vb < 2.0, "two junction drops at the base: {vb}");
    assert!(vc < 1.5, "saturated collector: {vc}");
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[cfg(feature = "switches")]
#[test]
fn the_ota_examples_load_and_run() {
    for name in ["ota-gain", "ota-vca", "ota-ringmod", "ota-vcf-single"] {
        let text = std::fs::read_to_string(format!("assets/circuits/{name}.txt")).unwrap();
        let r = parse(&text, &Registry::standard());
        assert!(r.errors.is_empty(), "{name}: {:?}", r.errors);
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        sim.analyze().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        sim.reset();
        for _ in 0..300 {
            sim.step().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        }
        let ota = sim.elements.iter().find(|e| e.class_name() == "OTAElm").unwrap();
        assert!((0..5).all(|i| ota.get_post_voltage(i).is_finite()), "{name}");
    }
}

#[cfg(feature = "analog")]
#[cfg(feature = "digital")]
#[cfg(feature = "logic")]
#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "transistors")]
#[test]
fn scopes_placed_in_the_circuit_record_and_hand_their_view_to_the_element() {
    for name in ["multivib-a", "qam-256"] {
        let text = std::fs::read_to_string(format!("assets/circuits/{name}.txt")).unwrap();
        let r = parse(&text, &Registry::standard());
        assert!(r.errors.is_empty(), "{name}: {:?}", r.errors);
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        assert!(!sim.embedded.is_empty(), "{name}: the scope elements resolved");
        sim.analyze().unwrap();
        sim.reset();
        for _ in 0..300 {
            sim.step().unwrap();
        }
        let snap = sim.snapshot(1, true);
        assert_eq!(snap.embedded.len(), sim.embedded.len());
        assert!(
            snap.embedded
                .iter()
                .any(|(_, v)| v.plots.iter().any(|p| p.max.iter().any(|m| *m != 0.0)))
        );
    }
}
