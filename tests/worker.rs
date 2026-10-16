// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The simulation worker: edits and parameter messages produce snapshots without blocking the
//! caller (task T037).

use prometheuscs::app::session::Session;
use prometheuscs::app::settings::AppSettings;
use prometheuscs::app::worker::{Command, Worker};
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;
use prometheuscs::solver::snapshot::{FrameSnapshot, Status};
use std::time::{Duration, Instant};

const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n38 2 0 1e-7 1e-5 Capacitance\n";

fn xml_of(text: &str) -> String {
    let mut s = Session::load(text, &Registry::standard()).unwrap();
    s.circuit_xml()
}

fn worker() -> Worker {
    Worker::spawn(Registry::standard(), AppSettings::default())
}

/// Loads a circuit into the worker, runs `steps` and returns the snapshot of that moment.
fn run(w: &Worker, xml: String, steps: usize) -> FrameSnapshot {
    w.send(Command::Load { xml, keep_time: false });
    w.send(Command::SetRunning(false));
    w.send(Command::RunSteps(steps));
    assert!(w.sync());
    w.latest().expect("a snapshot")
}

#[test]
fn a_loaded_circuit_runs_and_matches_a_headless_run() {
    let w = worker();
    let xml = xml_of(RC);
    let snap = run(&w, xml.clone(), 640);
    assert!(snap.time > 0.0 && snap.element_currents.len() == 4);
    assert_eq!(snap.status, Status::Stopped);
    // the same circuit stepped here gives the same numbers, bit for bit
    let parsed = parse(&xml, &Registry::standard());
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().unwrap();
    sim.run_steps(640).unwrap();
    assert_eq!(snap.time, sim.time());
    for (i, e) in sim.elements.iter().enumerate() {
        assert_eq!(snap.element_currents[i], e.get_current(), "element {i}");
        assert_eq!(snap.element_volts[i], e.base().volts, "element {i}");
    }
}

#[test]
fn a_parameter_message_changes_the_next_snapshot_without_a_reload() {
    let w = worker();
    let snap_a = run(&w, xml_of(RC), 200);
    let w2 = worker();
    w2.send(Command::Load { xml: xml_of(RC), keep_time: false });
    w2.send(Command::SetRunning(false));
    w2.send(Command::Edit { elm: 2, n: 0, value: 1e-5 });
    w2.send(Command::RunSteps(200));
    assert!(w2.sync());
    let snap_b = w2.latest().unwrap();
    assert_eq!(snap_a.time, snap_b.time);
    // ten times the capacitance charges more slowly
    let vc = |s: &FrameSnapshot| s.element_volts[2][0] - s.element_volts[2][1];
    assert!(vc(&snap_a).abs() > vc(&snap_b).abs() * 2.0, "{} vs {}", vc(&snap_a), vc(&snap_b));
}

#[test]
fn a_running_worker_publishes_snapshots_with_advancing_time() {
    let w = worker();
    w.send(Command::Load { xml: xml_of(RC), keep_time: false });
    w.send(Command::SetRunning(true));
    let start = Instant::now();
    let mut times = Vec::new();
    while times.len() < 3 && start.elapsed() < Duration::from_secs(5) {
        if let Some(s) = w.latest() {
            if s.status == Status::Running && times.last().is_none_or(|t| s.time > *t) {
                times.push(s.time);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(times.len(), 3, "snapshots: {times:?}");
    assert!(times.windows(2).all(|p| p[1] > p[0]));
    w.send(Command::SetRunning(false));
    assert!(w.sync());
}

#[test]
fn sending_commands_does_not_wait_for_the_solver() {
    let text = std::fs::read_to_string("assets/circuits/ladder.txt").unwrap();
    let w = worker();
    w.send(Command::Load { xml: xml_of(&text), keep_time: false });
    w.send(Command::SetRunning(true));
    assert!(w.sync());
    let start = Instant::now();
    for i in 0..1000 {
        w.send(Command::Edit { elm: 0, n: 0, value: 1.0 + f64::from(i) });
    }
    assert!(start.elapsed() < Duration::from_millis(50), "{:?}", start.elapsed());
    w.send(Command::SetRunning(false));
    assert!(w.sync());
}

#[test]
fn the_dump_carries_the_running_state() {
    let w = worker();
    let before = xml_of(RC);
    run(&w, before.clone(), 1000);
    let after = w.dump().unwrap();
    assert_ne!(before, after, "capacitor voltage must have moved");
    assert!(after.contains("<c "));
}

#[test]
fn a_failing_circuit_is_reported_in_the_snapshot() {
    // a polarized capacitor wired backwards charges past its reverse-voltage limit and stops
    let text = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 100\n209 64 64 64 0 0 0.000001 0 0.001 1\nw 64 64 0 64 0\n";
    let w = worker();
    let snap = run(&w, xml_of(text), 20_000);
    assert!(matches!(snap.status, Status::Errored { .. }), "{:?}", snap.status);
    let err = snap.error.expect("error");
    assert!(err.message.contains("reverse voltage"), "{}", err.message);
}

#[test]
fn a_session_with_a_worker_draws_what_the_worker_computed() {
    let mut s = Session::load(RC, &Registry::standard()).unwrap();
    s.attach_worker(worker());
    let w = s.worker_handle().unwrap();
    w.send(Command::SetRunning(false));
    w.send(Command::RunSteps(640));
    assert!(w.sync());
    s.update(Duration::from_millis(16)).unwrap();
    assert!(s.time() > 0.0);
    let cap = &s.sim.elements[2];
    assert!(cap.base().volts[0] - cap.base().volts[1] != 0.0);
    // restart goes through the worker and brings the clock back
    s.reset();
    assert!(s.worker_handle().unwrap().sync());
    s.update(Duration::from_millis(16)).unwrap();
    assert!(s.time() < 640.0 * 5e-6);
}

#[test]
fn an_edit_of_a_running_circuit_keeps_its_state_and_clock() {
    let mut s = Session::load(RC, &Registry::standard()).unwrap();
    s.attach_worker(worker());
    let w = s.worker_handle().unwrap();
    w.send(Command::SetRunning(false));
    w.send(Command::RunSteps(1000));
    assert!(w.sync());
    s.update(Duration::from_millis(16)).unwrap();
    let t = s.time();
    // a structural edit: rename nothing, just rewrite the circuit through the worker
    s.set_toggle(prometheuscs::app::edit::Toggle::SmallGrid, true);
    s.select_all();
    assert!(s.duplicate());
    assert!(s.worker_handle().unwrap().sync());
    s.update(Duration::from_millis(16)).unwrap();
    assert!(s.time() >= t, "the clock must not restart: {} < {t}", s.time());
    // the original capacitor still holds its charge
    let xml = s.circuit_xml();
    assert!(xml.matches("<c ").count() == 2);
    let first_c = xml.lines().find(|l| l.contains("<c ")).unwrap();
    assert!(!first_c.contains("vd=\"0\""), "{first_c}");
}
