// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Checks the features of CircuitJS 5.x without a 5.x build (spec 006, research R-1).
//!
//! 1. The circuits in `tests/compat/circuits-5x/` against the values worked out by hand in
//!    `expected.json` (derivations in that folder's README).
//! 2. The controlled-source examples, which CircuitJS 5.x redrew with the two-terminal
//!    sources, against the recordings of their old versions (`tests/compat/golden/cs-*.json`,
//!    old circuits in `tests/compat/circuits-5x/old/`): elements found in both versions, by type
//!    and coordinates, must agree.

use prometheuscs::elements::element::Element;
use prometheuscs::elements::geometry::Point;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;
use serde::Deserialize;
use std::collections::HashMap;

const REL: f64 = 1e-3;
const ABS: f64 = 1e-6;

fn close(want: f64, got: f64) -> bool {
    (want - got).abs() <= ABS + REL * want.abs().max(got.abs())
}

fn load(text: &str) -> Simulation {
    let r = parse(text, &Registry::standard());
    assert!(r.errors.is_empty(), "load problems: {:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().unwrap();
    sim
}

fn run_to(sim: &mut Simulation, t: f64) {
    while sim.time() < t - 1e-15 {
        sim.step().unwrap();
    }
}

/// Voltage of the node at grid point (`x`, `y`): a post of some element there.
fn voltage_at(sim: &Simulation, x: i32, y: i32) -> f64 {
    let p = Point::new(x, y);
    for e in &sim.elements {
        for k in 0..e.post_count() {
            if e.get_post(k) == Some(p) {
                return e.get_post_voltage(k);
            }
        }
    }
    panic!("no post at ({x}, {y})");
}

#[derive(Deserialize)]
struct Expected {
    time: f64,
    points: Vec<(i32, i32, f64)>,
    currents: Vec<(usize, f64)>,
    info: (usize, Vec<String>),
}

#[test]
fn circuits_in_the_5x_format_give_the_values_worked_out_by_hand() {
    let all: HashMap<String, Expected> = serde_json::from_str(
        &std::fs::read_to_string("tests/compat/circuits-5x/expected.json").unwrap(),
    )
    .unwrap();
    let mut failures = Vec::new();
    let mut names: Vec<_> = all.keys().cloned().collect();
    names.sort();
    for name in names {
        let want = &all[&name];
        let text = std::fs::read_to_string(format!("tests/compat/circuits-5x/{name}.txt")).unwrap();
        let mut sim = load(&text);
        run_to(&mut sim, want.time);
        for &(x, y, v) in &want.points {
            let got = voltage_at(&sim, x, y);
            if !close(v, got) {
                failures.push(format!("{name}: V({x},{y}) = {got}, expected {v}"));
            }
        }
        for &(i, c) in &want.currents {
            let got = sim.elements[i].get_current();
            if !close(c, got) {
                failures.push(format!(
                    "{name}: I(#{i} {}) = {got}, expected {c}",
                    sim.elements[i].class_name()
                ));
            }
        }
        let mut info = Vec::new();
        sim.elements[want.info.0].get_info(&mut info);
        for line in &want.info.1 {
            if !info.contains(line) {
                failures.push(format!("{name}: info of #{} lacks {line:?}: {info:?}", want.info.0));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[derive(Deserialize)]
struct Golden {
    checkpoints: Vec<Checkpoint>,
}

#[derive(Deserialize)]
struct Checkpoint {
    time: f64,
    elements: Vec<Recorded>,
}

#[derive(Deserialize)]
struct Recorded {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "voltageDiff")]
    voltage_diff: Option<f64>,
    current: Option<f64>,
    posts: Vec<Option<f64>>,
}

/// Type and coordinates of an element, to find the same element in two versions of a circuit.
fn key(e: &dyn Element) -> (String, i32, i32, i32, i32) {
    let b = e.base();
    (e.class_name().to_string(), b.x, b.y, b.x2, b.y2)
}

/// Examples the original recorded with random sources or similar (skipped by the golden test).
fn nondeterministic() -> Vec<String> {
    std::fs::read_to_string("tests/compat/nondeterministic.json")
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

#[test]
fn rewritten_controlled_source_examples_match_their_old_recordings() {
    let skip = nondeterministic();
    // examples that are no longer the recorded circuit, each with its reason
    let known: HashMap<String, String> =
        std::fs::read_to_string("tests/compat/known_differences.json")
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
    let mut files: Vec<_> = std::fs::read_dir("tests/compat/circuits-5x/old")
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    files.sort();
    assert_eq!(files.len(), 12, "the 12 old cs-* circuits");
    let mut failures = Vec::new();
    for path in files {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        if skip.contains(&name) {
            continue;
        }
        let golden: Golden = serde_json::from_str(
            &std::fs::read_to_string(format!("tests/compat/golden/{name}.json")).unwrap(),
        )
        .unwrap();
        // the recording was made from the old circuit: same elements, in file order
        let old = parse(&std::fs::read_to_string(&path).unwrap(), &Registry::standard());
        let old_keys: Vec<_> = old.circuit.elements.iter().map(|e| key(e.as_ref())).collect();
        assert_eq!(
            old_keys.len(),
            golden.checkpoints[0].elements.len(),
            "{name}: the recording does not belong to the old circuit"
        );
        let mut sim =
            load(&std::fs::read_to_string(format!("assets/circuits/{name}.txt")).unwrap());
        if let Some(reason) = known.get(&name) {
            // still has to run to the last recorded time
            run_to(&mut sim, golden.checkpoints.last().map_or(0.0, |c| c.time));
            eprintln!("known: {name}: {reason}");
            continue;
        }
        let mut compared = 0;
        for cp in &golden.checkpoints {
            run_to(&mut sim, cp.time);
            for e in &sim.elements {
                let k = key(e.as_ref());
                let Some(i) = old_keys.iter().position(|o| *o == k) else { continue };
                let rec = &cp.elements[i];
                assert_eq!(rec.kind, k.0, "{name}: element #{i}");
                compared += 1;
                let label = format!("{name} t={} {} at ({},{})", cp.time, k.0, k.1, k.2);
                if let Some(v) = rec.voltage_diff {
                    if !close(v, e.get_voltage_diff()) {
                        failures.push(format!("{label}: V {v} vs {}", e.get_voltage_diff()));
                    }
                }
                let derived = matches!(rec.kind.as_str(), "WireElm" | "GroundElm");
                if let (Some(c), false) = (rec.current, derived) {
                    if !close(c, e.get_current()) {
                        failures.push(format!("{label}: I {c} vs {}", e.get_current()));
                    }
                }
                for (p, v) in rec.posts.iter().enumerate().take(e.post_count()) {
                    if let Some(v) = v {
                        if !close(*v, e.get_post_voltage(p)) {
                            failures.push(format!(
                                "{label}: post {p} {v} vs {}",
                                e.get_post_voltage(p)
                            ));
                        }
                    }
                }
            }
        }
        assert!(compared > 0, "{name}: no element in common with the old circuit");
    }
    assert!(failures.is_empty(), "{} difference(s):\n{}", failures.len(), failures.join("\n"));
}
