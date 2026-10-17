// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The automatic save keeps unsaved work and "Recover autosave" brings it back.
//!
//! The recovery file lives under `XDG_CONFIG_HOME`, which this test points at a folder of its
//! own, so the user's real autosave is never touched. It is the only test in this file because
//! the environment is shared by the whole process.

use prometheuscs::app::session::Session;
use prometheuscs::app::settings::{AppSettings, autosave_path};
use prometheuscs::elements::registry::Registry;

#[test]
fn unsaved_work_is_written_and_recovered() {
    let dir = std::env::temp_dir().join(format!("prometheuscs-autosave-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // SAFETY: nothing else in this test binary reads or writes the environment concurrently.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };
    assert!(autosave_path().starts_with(&dir));

    // nothing to recover yet
    let mut fresh = Session::empty_with(&Registry::standard(), AppSettings::default());
    assert!(fresh.recover_autosave().is_err());

    // a circuit with no unsaved changes is not written
    let text = "$ 1 0.000005 10 50 5 50 5e-11\nr 0 0 64 0 0 1000\nv 0 64 0 0 0 0 40 5 0 0 0.5\nw 64 0 64 64 0\nw 64 64 0 64 0\n";
    let mut s = Session::load(text, &Registry::standard()).unwrap();
    assert!(!s.autosave().unwrap());
    assert!(!autosave_path().exists());

    // an edit makes it dirty, and the next autosave writes the file
    s.select_all();
    s.duplicate();
    assert!(s.autosave().unwrap());
    assert!(autosave_path().exists());
    let edited = s.sim.elements.len();

    // recovery gives the edited circuit back, still marked as unsaved
    let mut other = Session::empty_with(&Registry::standard(), AppSettings::default());
    other.recover_autosave().unwrap();
    assert_eq!(other.sim.elements.len(), edited);
    assert!(other.dirty);

    let _ = std::fs::remove_dir_all(&dir);
}
