// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "subcircuits")]
#![cfg(feature = "outputs")]
#![cfg(feature = "passive")]
#![cfg(feature = "reactive")]
//! Where subcircuit models are kept (this circuit, this session, the user library) and the
//! rule that a model the circuit uses cannot be deleted (spec 006, user story 5).
//!
//! Persistent models live under `XDG_CONFIG_HOME`, which this test points at a folder of its
//! own, so the user's real library is never touched. It is the only test in this file because
//! the environment is shared by the whole process.

use prometheuscs::app::session::Session;
use prometheuscs::app::subcircuits::{ModelScope, load_saved};
use prometheuscs::elements::registry::Registry;
use prometheuscs::elements::subcircuit;

/// An RC low-pass: the labeled nodes `in` and `out` are its pins.
const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11
207 64 96 32 96 0 in
r 64 96 128 96 0 1000
c 128 96 128 160 0 1e-6 0
w 128 96 192 96 0
207 192 96 224 96 0 out
g 128 160 128 176 0
";

/// The model's own description, to compare before and after a move.
fn contents(name: &str) -> String {
    subcircuit::lookup(name).unwrap().to_xml().to_pretty_string()
}

#[test]
fn scopes_move_models_without_changing_them_and_used_models_stay() {
    let dir = std::env::temp_dir().join(format!("prometheuscs-sublib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // SAFETY: nothing else in this test binary reads or writes the environment concurrently.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };

    let reg = Registry::standard();
    let mut s = Session::load(RC, &reg).unwrap();
    let mut m = s.start_subcircuit().unwrap();
    m.name = "rclib".into();
    s.finish_subcircuit(m, ModelScope::Circuit).unwrap();
    let original = contents("rclib");
    let file = dir.join("prometheuscs").join("subcircuits").join("rclib.xml");

    // Local → Global → Persistent → Global → Local: only where it is kept changes
    for scope in [
        ModelScope::Session,
        ModelScope::Saved,
        ModelScope::Session,
        ModelScope::Circuit,
        ModelScope::Saved,
    ] {
        s.set_subcircuit_scope("rclib", scope).unwrap();
        assert_eq!(s.subcircuit_scope("rclib"), Some(scope));
        assert_eq!(contents("rclib"), original, "{scope:?}");
        assert_eq!(file.exists(), scope == ModelScope::Saved, "{scope:?}: {}", file.display());
        assert_eq!(s.circuit_xml().contains(r#"nm="rclib""#), scope == ModelScope::Circuit);
    }

    // a persistent model is read back from the user library at start-up
    subcircuit::remove("rclib");
    assert!(subcircuit::lookup("rclib").is_none());
    load_saved();
    assert_eq!(contents("rclib"), original);

    // a model the circuit uses cannot be deleted, and the message names it
    let used = "$ 1 0.000005 10 50 5 50 5e-11\n410 400 400 432 400 0 rclib\n";
    let mut c = Session::load(used, &reg).unwrap();
    assert_eq!(c.models_in_use(), ["rclib"]);
    let err = c.delete_subcircuit("rclib").unwrap_err();
    assert!(err.contains("rclib"), "{err}");
    assert!(subcircuit::lookup("rclib").is_some() && file.exists());
    // nothing is deleted when one of several is in use
    let mut m2 = Session::load(RC, &reg).unwrap().start_subcircuit().unwrap();
    m2.name = "spare".into();
    c.finish_subcircuit(m2, ModelScope::Session).unwrap();
    assert!(c.delete_subcircuits(&["spare".into(), "rclib".into()]).is_err());
    assert!(subcircuit::lookup("spare").is_some());
    // once no instance uses it, it goes, file and all
    let mut free = Session::load(RC, &reg).unwrap();
    free.delete_subcircuits(&["spare".into(), "rclib".into()]).unwrap();
    assert!(subcircuit::lookup("rclib").is_none() && !file.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
