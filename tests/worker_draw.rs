// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! What the interface draws from the worker's snapshot is what the simulation would draw.
//!
//! The simulation runs on a worker thread; the interface keeps its own copy of the elements and
//! only receives voltages, currents and each element's "visual state" every frame. An element
//! that draws something else it learned while stepping (a buffer, a position, a temperature)
//! looks frozen in the window, though nothing is wrong in the simulation. This test runs every
//! bundled example, hands the snapshot to a second copy of the circuit the way the interface
//! does, and compares the drawing of each element between the two.

use prometheuscs::elements::element::DrawCtx;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::render::svg::SvgPainter;
use prometheuscs::solver::sim::Simulation;
use std::collections::BTreeMap;

fn draw_all(elements: &mut [Box<dyn prometheuscs::elements::element::Element>]) -> Vec<String> {
    // running, so that elements that animate by the wall clock while paused draw the same
    let ctx = DrawCtx { show_dots: false, sim_running: true, ..DrawCtx::default() };
    elements
        .iter_mut()
        .map(|e| {
            let mut g = SvgPainter::new(2000.0, 2000.0);
            e.draw(&mut g, &ctx);
            g.finish()
        })
        .collect()
}

/// For each element class that draws differently on the interface's copy, the examples that
/// showed it.
fn differences(steps: usize) -> BTreeMap<String, Vec<String>> {
    let registry = Registry::standard();
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut names: Vec<_> =
        std::fs::read_dir("assets/circuits").unwrap().map(|e| e.unwrap().path()).collect();
    names.sort();
    for path in names {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let worker = parse(&text, &registry);
        let ui = parse(&text, &registry);
        // the interface's own copy is analysed too (it needs the nodes), but never stepped
        let mut ui_sim = Simulation::new(ui.circuit.elements, ui.circuit.options.sim_settings());
        if ui_sim.analyze().is_err() {
            continue;
        }
        let mut sim =
            Simulation::new(worker.circuit.elements, worker.circuit.options.sim_settings());
        if sim.analyze().is_err() {
            continue;
        }
        sim.reset();
        // until the circuit stops by itself, if it does
        if sim.run_steps(steps).is_err() {
            continue;
        }
        sim.calc_wire_currents();
        let snap = sim.snapshot(1, true);
        // the interface's copy takes what the engine applies from a snapshot
        for (i, e) in ui_sim.elements.iter_mut().enumerate() {
            if let Some(volts) = snap.element_volts.get(i) {
                let b = e.base_mut();
                if b.volts.len() == volts.len() {
                    b.volts.copy_from_slice(volts);
                }
                b.current = snap.element_currents[i];
            }
            if let Some(state) = snap.element_states.get(i).filter(|s| !s.is_empty()) {
                e.set_visual_state(state);
            }
        }
        for (i, v) in &snap.embedded {
            if let Some(e) = ui_sim.elements.get_mut(*i) {
                e.set_scope_view(v.clone());
            }
        }
        sim.push_embedded_views();
        let mine = draw_all(&mut sim.elements);
        let theirs = draw_all(&mut ui_sim.elements);
        for (i, (a, b)) in mine.iter().zip(&theirs).enumerate() {
            if a != b {
                let class = sim.elements[i].class_name().to_string();
                let list = out.entry(class).or_default();
                if list.len() < 3 && !list.iter().any(|n| n.starts_with(&name)) {
                    let (la, lb): (Vec<&str>, Vec<&str>) =
                        (a.lines().collect(), b.lines().collect());
                    let first = la.iter().zip(&lb).position(|(x, y)| x != y).unwrap_or(0);
                    let show = |l: &[&str]| {
                        l.get(first).map_or(String::new(), |s| s.chars().take(110).collect())
                    };
                    list.push(format!("{name}[{i}] {} <> {}", show(&la), show(&lb)));
                }
            }
        }
    }
    out
}

/// Set `STEPS` to run longer than the default (slow states such as a lamp's heat need more).
#[test]
fn every_element_draws_the_same_from_a_snapshot_as_from_the_simulation() {
    let steps = std::env::var("STEPS").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let d = differences(steps);
    let report: Vec<String> = d.iter().map(|(c, ex)| format!("{c}: {}", ex.join("; "))).collect();
    assert!(
        d.is_empty(),
        "{} classes draw differently on the interface's copy (the state they draw is not in \
         `visual_state`):\n{}",
        d.len(),
        report.join("\n")
    );
}
