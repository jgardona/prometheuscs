// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Compares this program with recordings of the original CircuitJS1.
//!
//! `cargo run --bin golden` writes `tests/compat/golden/<example>.json`: for each of a few
//! simulated times, the voltage across, the current through and the post voltages of every
//! element. This test runs the same example here to the same times and compares. Examples in
//! `tests/compat/nondeterministic.json` (random sources and the like) are skipped, and
//! `tests/compat/tolerances.json` may loosen the comparison of one example with a reason:
//! `{"name": {"rel": 0.01, "abs": 1e-3, "reason": "..."}}`. Without recordings the test only
//! checks that the harness itself works.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Golden {
    name: String,
    #[serde(default)]
    timeout: bool,
    #[serde(default)]
    error: Option<String>,
    checkpoints: Vec<Checkpoint>,
}

#[derive(Serialize, Deserialize)]
struct Checkpoint {
    time: f64,
    #[serde(rename = "timeStep")]
    time_step: Option<f64>,
    elements: Vec<Recorded>,
}

#[derive(Serialize, Deserialize)]
struct Recorded {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "voltageDiff")]
    voltage_diff: Option<f64>,
    current: Option<f64>,
    posts: Vec<Option<f64>>,
}

#[derive(Deserialize, Clone, Copy)]
struct Tolerance {
    rel: f64,
    abs: f64,
}

const DEFAULT: Tolerance = Tolerance { rel: 1e-3, abs: 1e-6 };

fn load(text: &str) -> Result<Simulation, String> {
    let r = parse(text, &Registry::standard());
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    // no reset: the original keeps the state saved in the file (capacitor voltages, inductor
    // currents) when it loads a circuit
    sim.analyze().map_err(|e| format!("analysis failed: {e:?}"))?;
    Ok(sim)
}

/// Steps until the simulated time reaches `t` (the original records at the first step at or
/// after the time it asks for, so the same rule brings both to the same step).
fn run_to(sim: &mut Simulation, t: f64) -> Result<(), String> {
    let mut guard = 0u64;
    while sim.time() < t - 1e-15 {
        sim.step().map_err(|e| format!("stopped at t={}: {e:?}", sim.time()))?;
        guard += 1;
        if guard > 20_000_000 {
            return Err("did not reach the time".into());
        }
    }
    Ok(())
}

fn record(sim: &Simulation) -> Checkpoint {
    Checkpoint {
        time: sim.time(),
        time_step: Some(sim.time_step()),
        elements: sim
            .elements
            .iter()
            .map(|e| Recorded {
                kind: e.class_name().to_string(),
                voltage_diff: Some(e.get_voltage_diff()),
                current: Some(e.get_current()),
                posts: (0..e.post_count()).map(|i| Some(e.get_post_voltage(i))).collect(),
            })
            .collect(),
    }
}

fn close(a: Option<f64>, b: f64, tol: Tolerance) -> bool {
    // the original reports NaN and infinities as null: nothing to compare
    let Some(a) = a else { return true };
    (a - b).abs() <= tol.abs + tol.rel * a.abs().max(b.abs())
}

/// What differs between a recording and the state of `sim` (empty when they agree).
fn compare(cp: &Checkpoint, sim: &Simulation, tol: Tolerance) -> Vec<String> {
    let mut out = Vec::new();
    if cp.elements.len() != sim.elements.len() {
        out.push(format!(
            "{} elements, the original has {}",
            sim.elements.len(),
            cp.elements.len()
        ));
        return out;
    }
    for (i, (want, e)) in cp.elements.iter().zip(&sim.elements).enumerate() {
        let name = format!("#{i} {} ({})", e.class_name(), want.kind);
        if !close(want.voltage_diff, e.get_voltage_diff(), tol) {
            out.push(format!(
                "{name}: voltage {:?} vs {}",
                want.voltage_diff,
                e.get_voltage_diff()
            ));
        }
        // the current of a wire or a ground is not solved but derived afterwards from the
        // currents of the other elements, and the split among parallel wires is arbitrary (the
        // cross switch keeps its currents per pole and reports none as its own)
        let derived = matches!(
            want.kind.as_str(),
            "WireElm" | "RoutedWireElm" | "GroundElm" | "CrossSwitchElm"
        );
        if !derived && !close(want.current, e.get_current(), tol) {
            out.push(format!("{name}: current {:?} vs {}", want.current, e.get_current()));
        }
        for (k, v) in want.posts.iter().enumerate().take(e.post_count()) {
            if !close(*v, e.get_post_voltage(k), tol) {
                out.push(format!("{name}: post {k} {:?} vs {}", v, e.get_post_voltage(k)));
            }
        }
    }
    out
}

fn read_json<T: for<'a> Deserialize<'a> + Default>(path: &str) -> T {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
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
fn examples_agree_with_the_original() {
    let dir = Path::new("tests/compat/golden");
    // GOLDEN_ALL=1 compares the examples listed as nondeterministic too
    let skip: Vec<String> = if std::env::var("GOLDEN_ALL").is_ok() {
        Vec::new()
    } else {
        read_json("tests/compat/nondeterministic.json")
    };
    let tolerances: HashMap<String, Tolerance> = {
        #[derive(Deserialize)]
        struct Raw {
            rel: f64,
            abs: f64,
        }
        let raw: HashMap<String, Raw> = read_json("tests/compat/tolerances.json");
        raw.into_iter().map(|(k, r)| (k, Tolerance { rel: r.rel, abs: r.abs })).collect()
    };
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|x| x == "json"));
    files.sort();
    if files.is_empty() {
        eprintln!("no recordings in {}: run `cargo run --bin golden` first", dir.display());
        return;
    }
    // differences found by this test and not yet explained; each has its reason in the file
    let known: HashMap<String, String> = read_json("tests/compat/known_differences.json");
    let (mut checked, mut failures, mut still_known) = (0, Vec::new(), 0);
    for path in files {
        let g: Golden = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        if skip.contains(&g.name) || g.timeout || g.error.is_some() || g.checkpoints.is_empty() {
            continue;
        }
        let tol = tolerances.get(&g.name).copied().unwrap_or(DEFAULT);
        let text = std::fs::read_to_string(format!("assets/circuits/{}.txt", g.name)).unwrap();
        let result = load(&text).and_then(|mut sim| {
            let mut diffs = Vec::new();
            for cp in &g.checkpoints {
                run_to(&mut sim, cp.time)?;
                diffs.extend(
                    compare(cp, &sim, tol).into_iter().map(|d| format!("t={}: {d}", cp.time)),
                );
            }
            Ok(diffs)
        });
        checked += 1;
        match result {
            Ok(d) if d.is_empty() => {
                if known.contains_key(&g.name) {
                    eprintln!("{} now agrees: remove it from known_differences.json", g.name);
                }
            }
            Ok(d) if known.contains_key(&g.name) => {
                still_known += 1;
                eprintln!("known: {}: {} differences ({})", g.name, d.len(), known[&g.name]);
            }
            Ok(d) => failures.push(format!("{}: {} differences, first: {}", g.name, d.len(), d[0])),
            Err(e) => failures.push(format!("{}: {e}", g.name)),
        }
    }
    eprintln!(
        "{checked} recordings compared, {still_known} known differences, {} new",
        failures.len()
    );
    assert!(failures.is_empty(), "{} examples differ:\n{}", failures.len(), failures.join("\n"));
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[test]
fn the_harness_accepts_a_matching_recording_and_rejects_a_changed_one() {
    let text = std::fs::read_to_string("assets/circuits/lrc.txt").unwrap();
    let mut sim = load(&text).unwrap();
    run_to(&mut sim, 0.002).unwrap();
    let mut cp = record(&sim);
    let json = serde_json::to_string(&Golden {
        name: "lrc".into(),
        timeout: false,
        error: None,
        checkpoints: vec![record(&sim)],
    })
    .unwrap();
    let back: Golden = serde_json::from_str(&json).unwrap();
    let mut again = load(&text).unwrap();
    run_to(&mut again, back.checkpoints[0].time).unwrap();
    assert!(compare(&back.checkpoints[0], &again, DEFAULT).is_empty());
    // a different current in one element is noticed
    let victim =
        cp.elements.iter_mut().find(|e| e.current.is_some_and(|c| c.abs() > 1e-6)).unwrap();
    victim.current = victim.current.map(|c| c * 1.5);
    assert!(!compare(&cp, &again, DEFAULT).is_empty());
    // and a different number of elements
    cp.elements.pop();
    assert!(compare(&cp, &again, DEFAULT)[0].contains("elements"));
}

#[test]
fn nulls_from_the_original_are_not_compared() {
    assert!(close(None, 5.0, DEFAULT));
    assert!(close(Some(1.0), 1.0005, DEFAULT));
    assert!(!close(Some(1.0), 1.01, DEFAULT));
}
