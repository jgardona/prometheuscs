// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "subcircuits")]
//! Creating a subcircuit from the circuit (or its selection) and using it.

use prometheuscs::app::session::Session;
use prometheuscs::app::subcircuits::ModelScope;
use prometheuscs::elements::registry::Registry;
use prometheuscs::elements::subcircuit;

/// An RC low-pass: the labeled nodes `in` (west) and `out` (east) are its pins.
const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11
207 64 96 32 96 0 in
r 64 96 128 96 0 1000
c 128 96 128 160 0 1e-6 0
w 128 96 192 96 0
207 192 96 224 96 0 out
g 128 160 128 176 0
";

fn made() -> (Session, subcircuit::CompositeModel) {
    let mut s = Session::load(RC, &Registry::standard()).unwrap();
    let model = s.start_subcircuit().expect("a model");
    (s, model)
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[test]
fn the_labeled_nodes_become_the_pins_laid_out_on_the_chip() {
    let (_, m) = made();
    let pins: Vec<(&str, i32, i32)> =
        m.ext.iter().map(|p| (p.name.as_str(), p.side, p.pos)).collect();
    // west is side 2 and east side 3; pins are sorted by name
    assert_eq!(pins, [("in", 2, 0), ("out", 3, 0)]);
    assert_eq!((m.size_x, m.size_y), (2, 1));
    assert_ne!(m.ext[0].node, m.ext[1].node);
    // the parts come first and every element lists the nodes of its posts
    let tags: Vec<&str> = m.elms.iter().map(|e| e.tag.as_str()).collect();
    assert_eq!(tags[0], "r");
    assert!(m.elms.iter().all(|e| e.attr("nn").is_some()));
    assert!(tags.contains(&"g"));
}

#[cfg(feature = "outputs")]
#[test]
fn a_model_needs_a_name_and_pins_that_fit_the_chip() {
    let (mut s, mut m) = made();
    assert!(s.finish_subcircuit(m.clone(), ModelScope::Session).unwrap_err().contains("name"));
    m.name = "rc".into();
    m.ext[0].pos = 5;
    assert!(s.finish_subcircuit(m.clone(), ModelScope::Session).unwrap_err().contains("outside"));
    m.ext[0].pos = 0;
    m.ext[1].side = 2;
    assert!(s.finish_subcircuit(m, ModelScope::Session).unwrap_err().contains("shares"));
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn an_instance_of_the_new_subcircuit_behaves_like_the_circuit_it_came_from() {
    let (mut s, mut m) = made();
    m.name = "rclowpass".into();
    s.finish_subcircuit(m, ModelScope::Circuit).unwrap();
    assert!(s.subcircuit_names().contains(&"rclowpass".to_string()));
    // an instance, to learn where its pins are
    let alone = "$ 1 0.000005 10 50 5 50 5e-11\n410 200 100 232 100 0 rclowpass\n";
    let reg = Registry::standard();
    let r = prometheuscs::format::parser::parse(alone, &reg);
    let (pin_in, pin_out) = {
        let e = &r.circuit.elements[0];
        assert_eq!(e.post_count(), 2);
        (e.get_post(0).unwrap(), e.get_post(1).unwrap())
    };
    // drive it with 5 V through the `in` pin and read the `out` pin after 3 time constants
    let text = format!(
        "{alone}v {x} {y0} {x} {y} 0 0 40 5 0 0 0.5\ng {x} {y0} {x} {gy} 0\nr {ox} {oy} {ox} {oy2} 0 1000000\ng {ox} {oy2} {ox} {oy3} 0\n",
        x = pin_in.x,
        y = pin_in.y,
        y0 = pin_in.y - 64,
        gy = pin_in.y - 48,
        ox = pin_out.x,
        oy = pin_out.y,
        oy2 = pin_out.y + 32,
        oy3 = pin_out.y + 48,
    );
    let mut s2 = Session::load(&text, &reg).unwrap();
    for _ in 0..600 {
        s2.sim.step().unwrap();
    }
    let inst = s2.sim.elements.iter().find(|e| e.class_name() == "CustomCompositeElm").unwrap();
    let v_out = inst.get_post_voltage(1);
    let want = 5.0 * (1.0 - (-3.0f64).exp());
    assert!((v_out - want).abs() < 0.1, "output {v_out} V, expected about {want} V");
}

#[cfg(feature = "outputs")]
#[test]
fn circuit_models_are_saved_with_the_circuit_even_unused() {
    let (mut s, mut m) = made();
    m.name = "kept".into();
    s.finish_subcircuit(m, ModelScope::Circuit).unwrap();
    let xml = s.circuit_xml();
    assert_eq!(xml.matches("nm=\"kept\"").count(), 1, "{xml}");
    subcircuit::remove("kept");
    let again = Session::load(&xml, &Registry::standard()).unwrap();
    assert!(subcircuit::lookup("kept").is_some(), "the file defines it again");
    // saving it again does not write it twice
    let mut again = again;
    assert_eq!(again.circuit_xml().matches("nm=\"kept\"").count(), 1);
}

#[cfg(feature = "outputs")]
#[test]
fn deleting_a_subcircuit_forgets_it() {
    let (mut s, mut m) = made();
    m.name = "gone".into();
    s.finish_subcircuit(m, ModelScope::Circuit).unwrap();
    s.delete_subcircuit("gone").unwrap();
    assert!(!s.subcircuit_names().contains(&"gone".to_string()));
    assert!(!s.circuit_xml().contains("nm=\"gone\""));
}

#[test]
fn a_circuit_without_pins_cannot_become_a_subcircuit() {
    let mut s = Session::load(
        "$ 1 0.000005 10 50 5 50 5e-11\nr 0 0 64 0 0 1000\ng 64 0 64 16 0\n",
        &Registry::standard(),
    )
    .unwrap();
    assert!(s.start_subcircuit().unwrap_err().contains("no external"));
}

/// A doubler: `out` = 2 * v(in), with the reference inside the subcircuit.
const DOUBLER: &str = r#"<cir f="1" ts="0.000005" ic="10" cb="50" pb="50" vr="5" mts="5e-11">
<ln x="64 96 32 96" f="0" te="in"/>
<r x="64 96 64 160" f="0" r="1000"/>
<g x="64 160 64 176" f="0"/>
<cv x="160 160 160 96" f="0" ex="2*v(in)"/>
<w x="160 96 192 96" f="0"/>
<ln x="192 96 224 96" f="0" te="out"/>
<g x="160 160 160 176" f="0"/>
</cir>
"#;

#[cfg(feature = "analog")]
#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn each_copy_of_a_subcircuit_reads_its_own_named_node() {
    let mut s = Session::load(DOUBLER, &Registry::standard()).unwrap();
    let mut m = s.start_subcircuit().expect("a model");
    // the reference was written as model node numbers: the node of `in`, and ground
    let cv = m.elms.iter().find(|e| e.tag == "cv").unwrap();
    let rn: Vec<i32> = cv.attr("rn").unwrap().split(' ').map(|t| t.parse().unwrap()).collect();
    let pin_in = m.ext.iter().find(|p| p.name == "in").unwrap().node;
    assert_eq!(rn, vec![pin_in, 0]);
    m.name = "doubler".into();
    s.finish_subcircuit(m, ModelScope::Circuit).unwrap();
    // the model is saved with the circuit, references included
    let xml = s.circuit_xml();
    assert!(xml.contains(&format!("rn=\"{pin_in} 0\"")), "{xml}");
    // two copies, driven with 1 V and 2 V
    let reg = Registry::standard();
    let mut text = String::from("$ 1 0.000005 10 50 5 50 5e-11\n");
    for (k, volts) in [(0, 1), (1, 2)] {
        let y = 100 + 200 * k;
        let one = format!("410 200 {y} 232 {y} 0 doubler\n");
        let r = prometheuscs::format::parser::parse(
            &format!("$ 1 0.000005 10 50 5 50 5e-11\n{one}"),
            &reg,
        );
        let e = &r.circuit.elements[0];
        let (pin_in, pin_out) = (e.get_post(0).unwrap(), e.get_post(1).unwrap());
        text += &one;
        text += &format!(
            "v {x} {y0} {x} {y1} 0 0 40 {volts} 0 0 0.5\ng {x} {y0} {x} {gy} 0\nr {ox} {oy} {ox} {oy2} 0 1000000\ng {ox} {oy2} {ox} {oy3} 0\n",
            x = pin_in.x,
            y1 = pin_in.y,
            y0 = pin_in.y - 64,
            gy = pin_in.y - 48,
            ox = pin_out.x,
            oy = pin_out.y,
            oy2 = pin_out.y + 32,
            oy3 = pin_out.y + 48,
        );
    }
    let mut s2 = Session::load(&text, &reg).unwrap();
    for _ in 0..20 {
        s2.sim.step().unwrap();
    }
    let outs: Vec<f64> = s2
        .sim
        .elements
        .iter()
        .filter(|e| e.class_name() == "CustomCompositeElm")
        .map(|e| e.get_post_voltage(1))
        .collect();
    assert_eq!(outs.len(), 2);
    assert!((outs[0] - 2.0).abs() < 1e-6 && (outs[1] - 4.0).abs() < 1e-6, "{outs:?}");
}
