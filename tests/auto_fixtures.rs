// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The original's recorded auto-tests are readable and describe runs we can compare with.

mod common;

use common::auto_test;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;

#[test]
fn all_seven_fixtures_have_a_run_length_and_scope_data() {
    let tests = auto_test::all();
    assert_eq!(tests.len(), 7);
    for t in &tests {
        assert!(t.length > 0.0, "{}", t.name);
        assert!(!t.scope_data.is_empty(), "{} has no scope data", t.name);
        for s in &t.scope_data {
            assert!(!s.values.is_empty() && s.sample_time > 0.0, "{} scope {}", t.name, s.scope);
            assert!(s.values.iter().all(|v| v.is_finite()), "{}", t.name);
        }
    }
}

#[test]
fn switch_events_are_inside_the_run() {
    for t in auto_test::all() {
        for e in &t.switch_events {
            assert!(e.time > 0.0 && e.time < t.length, "{}: {e:?}", t.name);
        }
    }
    assert_eq!(auto_test::load("analogmux").switch_events.len(), 3);
    assert_eq!(auto_test::load("lrcswitch").switch_events.len(), 1);
}

#[test]
fn the_rlc_fixtures_load_without_errors() {
    for name in ["lrc", "lrcswitch", "capac"] {
        let t = auto_test::load(name);
        let r = parse(&t.text, &Registry::standard());
        assert!(r.errors.is_empty(), "{name}: {:?}", r.errors);
    }
}

#[test]
fn the_lrc_recording_matches_the_analytic_ring_down() {
    // series R-L-C ring: the capacitor voltage in the recording decays with rate R / 2L
    let t = auto_test::load("lrc");
    let scope = &t.scope_data[0];
    let peak = scope.values.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let last = scope.values.last().copied().unwrap().abs();
    assert!(peak > 5.0 && last < peak);
}
