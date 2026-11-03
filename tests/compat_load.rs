// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Every bundled example loads (guarantee G1) and survives a save and load (guarantee G2).
//!
//! `coverage_report` prints how many examples load completely and which element types still
//! stop the rest (`cargo test --test compat_load coverage_report -- --nocapture`). The strict
//! tests check every bundled example.

use prometheuscs::elements::registry::Registry;
use prometheuscs::error::LoadError;
use prometheuscs::format::parser::parse;
use prometheuscs::format::writer::write_xml;
use std::collections::BTreeMap;

fn examples() -> Vec<(String, String)> {
    let mut v: Vec<_> = std::fs::read_dir("assets/circuits")
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap_or_default(),
            )
        })
        .collect();
    v.sort();
    v
}

/// The element type or tag an error is about.
fn culprit(e: &LoadError) -> String {
    match e {
        LoadError::UnknownType { type_code, .. } => format!("type {type_code}"),
        LoadError::Malformed { type_code, message, .. } => {
            format!("malformed {type_code}: {message}")
        }
        LoadError::Xml(m) => format!("xml: {m}"),
    }
}

#[test]
fn coverage_report() {
    let registry = Registry::standard();
    let all = examples();
    let mut clean = 0;
    let mut blockers: BTreeMap<String, usize> = BTreeMap::new();
    for (_, text) in &all {
        let r = parse(text, &registry);
        if r.errors.is_empty() {
            clean += 1;
        }
        let mut seen: Vec<String> = r.errors.iter().map(culprit).collect();
        seen.sort();
        seen.dedup();
        for c in seen {
            *blockers.entry(c).or_default() += 1;
        }
    }
    println!("{clean} of {} examples load without errors", all.len());
    let mut rows: Vec<_> = blockers.into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1));
    for (what, n) in rows.iter().take(40) {
        println!("  {n:4} examples: {what}");
    }
    assert!(!all.is_empty());
}

#[cfg(feature = "analog")]
#[cfg(feature = "digital")]
#[cfg(feature = "diodes")]
#[cfg(feature = "logic")]
#[cfg(feature = "mosfets")]
#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[cfg(feature = "switches")]
#[cfg(feature = "transistors")]
#[test]
fn every_example_loads_without_errors() {
    let registry = Registry::standard();
    let mut failing = Vec::new();
    for (name, text) in examples() {
        let r = parse(&text, &registry);
        if !r.errors.is_empty() {
            failing.push(format!("{name}: {}", culprit(&r.errors[0])));
        }
    }
    assert!(
        failing.is_empty(),
        "{} examples fail: {:?}",
        failing.len(),
        &failing[..failing.len().min(10)]
    );
}

#[test]
fn every_example_round_trips_through_save_and_load() {
    let registry = Registry::standard();
    for (name, text) in examples() {
        let first = parse(&text, &registry);
        let xml = write_xml(&first.circuit);
        let second = parse(&xml, &registry);
        assert!(second.errors.is_empty(), "{name}: {:?}", second.errors);
        assert_eq!(write_xml(&second.circuit), xml, "{name}");
        assert_eq!(first.circuit.elements.len(), second.circuit.elements.len(), "{name}");
    }
}

/// Reads `body` as the elements of a circuit in the XML format.
fn xml(body: &str) -> prometheuscs::format::parser::ParseResult {
    let text = format!(
        "<cir f=\"1\" ts=\"0.000005\" ic=\"10\" cb=\"50\" pb=\"50\" vr=\"5\" mts=\"5e-11\">\n{body}\n</cir>\n"
    );
    let r = parse(&text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    r
}

#[cfg(feature = "analog")]
#[test]
fn gyrator_resistance_and_width_come_from_5x_files() {
    // CircuitJS 5.x writes the gyration resistance as `r` and the width as `wi`
    let r = xml(r#"<Gyrator x="0 0 64 0" f="0" r="2200" wi="48"/>"#);
    let g = &r.circuit.elements[0];
    assert_eq!(g.get_post(2).map(|p| p.y.abs()), Some(48), "width 48");
    assert_eq!(g.get_edit_info(0).unwrap().value, 2200.0);
    let out = write_xml(&r.circuit);
    assert!(out.contains(r#"r="2200""#) && out.contains(r#"wi="48""#), "{out}");
    // without `wi` the width comes from the geometry, and files from earlier versions of this
    // program kept the resistance in `gr`
    let r = xml(r#"<Gyrator x="0 0 64 64" f="0" gr="330"/>"#);
    let g = &r.circuit.elements[0];
    assert_eq!(g.get_edit_info(0).unwrap().value, 330.0);
    assert_eq!(g.get_post(2).map(|p| p.y.abs()), Some(64));
}

#[cfg(feature = "sources")]
#[test]
fn editing_a_source_restarts_its_waveform_in_phase_a_slider_does_not() {
    use prometheuscs::solver::sim::Simulation;
    // a 50 Hz sine running for 3 ms, then its amplitude edited
    let r = xml(r#"<v x="0 64 0 0" f="0" wf="1" fr="50" maxv="5"/>
<r x="0 0 64 0" f="0" r="1000"/>
<w x="64 0 64 64" f="0"/>
<w x="0 64 64 64" f="0"/>"#);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().unwrap();
    let change_freq = |sim: &mut Simulation, continuous: bool| {
        let freq_field = (0..12)
            .find(|&n| {
                sim.elements[0].get_edit_info(n).is_some_and(|e| e.name.contains("Frequency"))
            })
            .unwrap();
        while sim.time() < 0.003 {
            sim.step().unwrap();
        }
        let mut ei = sim.elements[0].get_edit_info(freq_field).unwrap();
        assert!(ei.name.contains("Frequency"), "{}", ei.name);
        ei.value = 100.0;
        ei.continuous = continuous;
        sim.apply_edit(0, freq_field, &mut ei);
        sim.analyze().unwrap();
        // the source takes its value at the time the step starts
        let t = sim.time();
        sim.step().unwrap();
        // the sine of a waveform that restarts in phase is the sine of the absolute time
        let in_phase = 5.0 * (2.0 * std::f64::consts::PI * 100.0 * t).sin();
        (sim.elements[0].get_voltage_diff() - in_phase).abs()
    };
    assert!(change_freq(&mut sim, false) < 1e-6, "an edit restarts the waveform in phase");
    let r = xml(r#"<v x="0 64 0 0" f="0" wf="1" fr="50" maxv="5"/>
<r x="0 0 64 0" f="0" r="1000"/>
<w x="64 0 64 64" f="0"/>
<w x="0 64 64 64" f="0"/>"#);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().unwrap();
    assert!(change_freq(&mut sim, true) > 1e-3, "a slider keeps the waveform continuous");
}

#[cfg(feature = "analog")]
#[cfg(feature = "outputs")]
#[test]
fn export_as_text_and_as_link_keep_the_5x_elements() {
    use prometheuscs::app::session::Session;
    use prometheuscs::format::link::{make_link, read_link};
    let text = std::fs::read_to_string("tests/compat/circuits-5x/ci-didt.txt").unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    let exported = s.export_text();
    for want in [r#"<ci "#, r#"ex="didt(ia)/1000""#, r#"lb="ia""#] {
        assert!(exported.contains(want), "{want} missing from:\n{exported}");
    }
    // pasting the text back gives the same circuit, and so does the link
    let mut again = Session::load(&exported, &Registry::standard()).unwrap();
    assert_eq!(again.export_text(), exported);
    let back = read_link(&make_link(&exported)).unwrap();
    assert_eq!(back, exported);
}
