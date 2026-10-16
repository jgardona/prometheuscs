// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Every bundled example loads (guarantee G1) and survives a save and load (guarantee G2).
//!
//! `coverage_report` prints how many examples load completely and which element types still
//! stop the rest (`cargo test --test compat_load coverage_report -- --nocapture`). The strict
//! tests are ignored until the last element family is ported (task T080).

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

#[test]
#[ignore = "enabled when every element family is ported (T080)"]
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
#[ignore = "enabled when every element family is ported (T080)"]
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
