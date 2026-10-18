// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! GTK emits some signals synchronously (hiding a widget makes its pointer controller send
//! "leave"). A handler that needs the session while the code that hid the widget still holds it
//! used to panic across the C boundary, which aborts the program ("RefCell already borrowed").

use prometheuscs::app::session::{Session, try_with_session};
use prometheuscs::elements::registry::Registry;
use std::cell::RefCell;
use std::time::Duration;

fn load(name: &str) -> Session {
    let text = std::fs::read_to_string(format!("assets/circuits/{name}")).unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    for _ in 0..5 {
        let _ = s.update(Duration::from_millis(16));
    }
    s
}

#[cfg(feature = "passive")]
#[test]
fn a_handler_that_finds_the_session_busy_skips_instead_of_panicking() {
    let session = RefCell::new(load("lrc.txt"));
    // the frame callback holds the session while it hides the scope area...
    let held = session.borrow_mut();
    // ...and GTK calls the "leave" handler synchronously
    let ran = try_with_session(&session, |s| s.set_scope_hover(None));
    assert!(ran.is_none(), "the handler must not run on a borrowed session");
    drop(held);
    // with the session free the handler runs
    assert_eq!(try_with_session(&session, |s| s.scope_count()), Some(3));
}

#[test]
fn the_scope_area_state_follows_the_scopes_of_the_circuit() {
    // no scopes: the area is hidden and has no height
    assert_eq!(load("ledarray.txt").scope_area_state(), (0, false));
    // with scopes: 130 pixels for each row of the tallest column, at most 390, and visible
    let s = load("lrc.txt");
    let rows = prometheuscs::render::scopes::rows(&s.scope_views());
    assert!(rows > 0);
    assert_eq!(s.scope_area_state(), ((rows as i32 * 130).min(390), true));
}

#[test]
fn opening_a_circuit_without_scopes_hides_the_area() {
    let mut s = load("lrc.txt");
    assert!(s.scope_area_state().1);
    let text = std::fs::read_to_string("assets/circuits/ledarray.txt").unwrap();
    s.open_text(&text, "LED Array", None).unwrap();
    let _ = s.update(Duration::from_millis(16));
    assert_eq!(s.scope_area_state(), (0, false));
}
