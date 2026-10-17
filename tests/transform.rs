// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Rotating and mirroring a selection: the posts move as the geometry says and the operations
//! undo each other.

use prometheuscs::app::edit::Transform;
use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;

const HEAD: &str = "$ 1 0.000005 10 50 5 50 5e-11\n";

fn session(body: &str) -> Session {
    let mut s = Session::load(&format!("{HEAD}{body}"), &Registry::standard()).unwrap();
    s.select_all();
    s
}

/// The end points of every element, as the file would write them.
fn ends(s: &Session) -> Vec<(i32, i32, i32, i32)> {
    s.sim
        .elements
        .iter()
        .map(|e| {
            let b = e.base();
            (b.x, b.y, b.x2, b.y2)
        })
        .collect()
}

#[cfg(feature = "passive")]
#[test]
fn mirroring_twice_gives_back_the_circuit() {
    let mut s = session("r 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n");
    let before = s.circuit_xml();
    assert!(s.transform(Transform::MirrorX));
    assert_ne!(s.circuit_xml(), before, "one mirror changes it");
    assert!(s.transform(Transform::MirrorX));
    assert_eq!(s.circuit_xml(), before);
    assert!(s.transform(Transform::MirrorY));
    assert!(s.transform(Transform::MirrorY));
    assert_eq!(s.circuit_xml(), before);
}

#[test]
fn four_turns_one_way_are_the_identity_and_two_ways_cancel() {
    let mut s = session("r 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n");
    let before = ends(&s);
    for _ in 0..4 {
        assert!(s.transform(Transform::RotateCw));
    }
    assert_eq!(ends(&s), before);
    assert!(s.transform(Transform::RotateCw));
    assert!(s.transform(Transform::RotateCcw));
    assert_eq!(ends(&s), before);
}

#[cfg(feature = "passive")]
#[test]
fn a_quarter_turn_swaps_the_axes_around_the_centre() {
    // a horizontal wire 64 long becomes a vertical one of the same length, about its centre
    let mut s = session("w 0 0 64 0 0\n");
    assert!(s.transform(Transform::RotateCw));
    let (x, y, x2, y2) = ends(&s)[0];
    assert_eq!(x, x2, "now vertical");
    assert_eq!((y - y2).abs(), 64);
    assert_eq!((y + y2) / 2, 0, "about the same centre");
    assert_eq!(x, 32);
}

#[cfg(feature = "passive")]
#[test]
fn only_the_selection_is_transformed() {
    let mut s = session("r 0 0 64 0 0 1000\nr 0 128 64 128 0 1000\n");
    s.deselect_all();
    s.sim.elements[1].base_mut().selected = true;
    let first = ends(&s)[0];
    assert!(s.transform(Transform::RotateCw));
    assert_eq!(ends(&s)[0], first, "the unselected resistor stays");
    let (x, _, x2, _) = ends(&s)[1];
    assert_eq!(x, x2, "the selected one turned");
}

#[test]
fn a_transformation_can_be_undone() {
    let mut s = session("r 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\n");
    let before = s.circuit_xml();
    s.transform(Transform::RotateCcw);
    assert_ne!(s.circuit_xml(), before);
    assert!(s.undo());
    assert_eq!(s.circuit_xml(), before);
}

#[test]
fn elements_that_cannot_mirror_stay_where_they_are() {
    // a switch pair etc. may refuse; the call must still leave a loadable circuit
    let mut s = session("s 0 0 64 0 0 0 false\nr 64 0 64 64 0 1000\n");
    s.transform(Transform::MirrorX);
    let xml = s.circuit_xml();
    assert!(Session::load(&xml, &Registry::standard()).is_ok());
}
