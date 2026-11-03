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

#[test]
fn swap_terminals_exchanges_the_posts_and_undo_restores_them() {
    let mut s = session();
    // nothing selected: nothing to swap
    assert!(!Cmd::SwapTerminals.enabled(&s));
    s.sim.elements[1].base_mut().selected = true; // the resistor 0,0 to 64,0
    assert!(Cmd::SwapTerminals.enabled(&s));
    dispatch(&mut s, &Cmd::SwapTerminals);
    let b = s.sim.elements[1].base();
    assert_eq!((b.x, b.y, b.x2, b.y2), (64, 0, 0, 0));
    s.undo();
    let b = s.sim.elements[1].base();
    assert_eq!((b.x, b.y, b.x2, b.y2), (0, 0, 64, 0));
    // the command has a stable id for shortcuts
    assert_eq!(Cmd::SwapTerminals.action_name(), "swap-terminals");
    assert_eq!(Cmd::from_action("swap-terminals", None), Some(Cmd::SwapTerminals));
}

/// Places an element of `id` by dragging from (`x0`, `y`) to (`x1`, `y`).
fn place(s: &mut Session, id: &str, x0: f64, x1: f64, y: f64) {
    s.set_tool(prometheuscs::app::edit::Tool::Place(id.into()));
    s.mouse_down(x0, y, prometheuscs::app::edit::Mods::default(), 1);
    s.mouse_drag(x1, y);
    assert!(s.mouse_up());
}

#[test]
fn properties_open_on_create_only_when_asked_and_not_for_quiet_elements() {
    let mut s = Session::load("$ 1 0.000005 10 50 5 50 5e-11\n", &Registry::standard()).unwrap();
    s.canvas_size = (800.0, 600.0);
    s.fit_pending = false;
    // the default of CircuitJS 5.x: off
    assert!(!s.settings.show_properties_on_create);
    place(&mut s, "ResistorElm", 96.0, 160.0, 96.0);
    assert_eq!(s.take_properties_after_place(), None);
    s.settings.show_properties_on_create = true;
    place(&mut s, "ResistorElm", 96.0, 160.0, 192.0);
    let i = s.sim.elements.len() - 1;
    assert_eq!(s.take_properties_after_place(), Some(i));
    // asking again gives nothing: it is about the last placement only
    assert_eq!(s.take_properties_after_place(), None);
    // wires, grounds and logic inputs are rarely edited when placed
    for id in ["WireElm", "GroundElm", "LogicInputElm"] {
        place(&mut s, id, 300.0, 364.0, 300.0);
        assert_eq!(s.take_properties_after_place(), None, "{id}");
    }
}

#[test]
fn properties_follow_the_click_when_asked() {
    let mut s = session();
    s.canvas_size = (800.0, 600.0);
    s.fit_pending = false;
    // the default of CircuitJS 5.x: on; the resistor is at 0,0 to 64,0
    assert!(s.settings.show_properties_on_click);
    assert_eq!(s.properties_on_click(32.0, 0.0), Some(1));
    assert_eq!(s.properties_on_click(64.0, 32.0), Some(2));
    s.settings.show_properties_on_click = false;
    assert_eq!(s.properties_on_click(32.0, 0.0), None);
}

#[test]
fn a_part_dropped_on_the_canvas_is_placed_through_the_normal_path() {
    let mut s =
        Session::load("$ 1 0.000005 10 50 5 50 5e-11\nw 0 96 192 96 0\n", &Registry::standard())
            .unwrap();
    s.canvas_size = (800.0, 600.0);
    s.fit_pending = false;
    s.settings.show_properties_on_create = true;
    let before = s.sim.elements.len();
    let (px, py) = (s.view.x + 64.0 * s.view.scale, s.view.y + 96.0 * s.view.scale);
    assert!(s.drop_element("ResistorElm", px, py));
    // the resistor, plus the wire split at both of its posts
    assert!(s.sim.elements.len() > before);
    let i = s.sim.elements.iter().position(|e| e.class_name() == "ResistorElm").unwrap();
    let b = s.sim.elements[i].base();
    assert_eq!((b.x, b.y, b.x2, b.y2), (64, 96, 128, 96));
    // the tool in use is kept, and the properties of the new part can open
    assert_eq!(s.tool, prometheuscs::app::edit::Tool::Select);
    assert_eq!(s.take_properties_after_place(), Some(i));
    // one undo removes it
    s.undo();
    assert_eq!(s.sim.elements.len(), before);
    // an unknown id places nothing
    assert!(!s.drop_element("NoSuchElm", px, py));
}

/// Only changes to what the circuit is mark it unsaved (spec FR-024, research R-12): the
/// speed, current-speed and brightness controls, the view and the arrangement of the scopes
/// do not; placing, deleting, moving and editing elements do.
#[test]
fn only_changes_to_the_circuit_mark_it_unsaved() {
    use prometheuscs::app::edit::Mods;
    let text = std::fs::read_to_string("assets/circuits/lrc.txt").unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    s.canvas_size = (800.0, 600.0);
    s.fit_pending = false;
    assert!(!s.dirty, "freshly loaded");
    let clean = |s: &Session, what: &str| assert!(!s.dirty, "{what} marked the circuit unsaved");
    apply(&mut s, Action::Speed(40.0));
    clean(&s, "the simulation speed");
    apply(&mut s, Action::CurrentSpeed(30.0));
    clean(&s, "the current speed");
    apply(&mut s, Action::Brightness(70.0));
    clean(&s, "the power brightness");
    s.zoom_at(400.0, 300.0, 1.5);
    clean(&s, "zooming");
    // a Shift drag pans the view
    s.mouse_down(100.0, 100.0, Mods { shift: true, ctrl: false }, 1);
    s.mouse_drag(160.0, 140.0);
    s.mouse_up();
    clean(&s, "panning");
    for _ in 0..3 {
        frame(&mut s);
    }
    clean(&s, "running");
    if s.scope_count() > 1 {
        dispatch(&mut s, &Cmd::StackAll);
        clean(&s, "stacking the scopes");
        dispatch(&mut s, &Cmd::UnstackAll);
        clean(&s, "unstacking the scopes");
    }

    // what changes the circuit does
    let changed = |s: &mut Session, what: &str, f: &dyn Fn(&mut Session)| {
        s.dirty = false;
        f(s);
        assert!(s.dirty, "{what} did not mark the circuit unsaved");
    };
    changed(&mut s, "placing", &|s| {
        s.set_tool(prometheuscs::app::edit::Tool::Place("ResistorElm".into()));
        s.mouse_down(500.0, 500.0, Mods::default(), 1);
        s.mouse_drag(560.0, 500.0);
        s.mouse_up();
        s.set_tool(prometheuscs::app::edit::Tool::Select);
    });
    changed(&mut s, "editing", &|s| {
        let i = s.sim.elements.len() - 1;
        let f = s.edit_fields(i);
        s.apply_edit_fields(i, f);
    });
    changed(&mut s, "moving", &|s| {
        let i = s.sim.elements.len() - 1;
        s.deselect_all();
        s.sim.elements[i].base_mut().selected = true;
        dispatch(s, &Cmd::RotateCw);
    });
    changed(&mut s, "deleting", &|s| {
        dispatch(s, &Cmd::Delete);
    });
}
