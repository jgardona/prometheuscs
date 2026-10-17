// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The application's action layer works without a display (contract C6).

use prometheuscs::app::commands::{Action, Cmd, Outcome, apply, dispatch};
use prometheuscs::app::edit::Toggle;
use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;
use std::time::Duration;

const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n38 2 0 1e-7 1e-5 Capacitance\n";

fn session() -> Session {
    Session::load(RC, &Registry::standard()).unwrap()
}

fn frame(s: &mut Session) {
    s.update(Duration::from_millis(16)).unwrap();
}

#[test]
fn restart_returns_time_to_zero_and_runs() {
    let mut s = session();
    for _ in 0..5 {
        frame(&mut s);
    }
    assert!(s.time() > 0.0);
    apply(&mut s, Action::Run(false));
    apply(&mut s, Action::Restart);
    assert_eq!(s.time(), 0.0);
    assert!(s.running);
    frame(&mut s);
    assert!(s.time() > 0.0);
}

#[test]
fn stop_freezes_time_and_run_resumes_it() {
    let mut s = session();
    frame(&mut s);
    apply(&mut s, Action::Run(false));
    let frozen = s.time();
    for _ in 0..3 {
        frame(&mut s);
    }
    assert_eq!(s.time(), frozen);
    apply(&mut s, Action::ToggleRun);
    assert!(s.running);
    frame(&mut s);
    assert!(s.time() > frozen);
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn slider_changes_the_value_of_its_element_and_survives_restart() {
    let mut s = session();
    apply(&mut s, Action::Slider { index: 0, position: 100.0 });
    let value = |s: &Session| s.edit_fields(2)[0].value;
    assert!((value(&s) - 1e-5).abs() < 1e-12);
    apply(&mut s, Action::Slider { index: 0, position: 0.0 });
    assert!((value(&s) - 1e-7).abs() < 1e-12);
    apply(&mut s, Action::Restart);
    assert!((value(&s) - 1e-7).abs() < 1e-12);
}

#[test]
fn speed_sliders_change_the_pacing() {
    let mut s = session();
    apply(&mut s, Action::Speed(80.0));
    let slow = s.step_rate();
    apply(&mut s, Action::Speed(120.0));
    assert!(s.step_rate() > slow);
    apply(&mut s, Action::CurrentSpeed(70.0));
    assert_eq!(s.options.current_bar, 70);
}

#[test]
fn menu_commands_and_panel_actions_share_one_session() {
    let mut s = session();
    assert_eq!(dispatch(&mut s, &Cmd::SelectAll), Outcome::Done);
    assert_eq!(dispatch(&mut s, &Cmd::Open), Outcome::Ui(Cmd::Open));
    dispatch(&mut s, &Cmd::Toggle(Toggle::ShowPower));
    assert!(s.toggle_state(Toggle::ShowPower));
    apply(&mut s, Action::Brightness(30.0));
    assert_eq!(s.options.power_bar, 30);
    dispatch(&mut s, &Cmd::NewCircuit);
    assert!(s.sim.elements.is_empty());
}
