// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The information box of the bottom right corner: what it says about the element under the
//! pointer, the selected one, a terminal, or nothing, and how it is drawn.

use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;
use prometheuscs::render::scene::{View, draw_info_box};
use prometheuscs::render::svg::SvgPainter;
use std::time::Duration;

// 5 V across two 1 kohm resistors in parallel: the first at x 128 to 192, the second at 256 to 320
const CIRCUIT: &str = "$ 1 0.000005 10 50 5 50 5e-11
v 64 128 64 0 0 0 40 5 0 0 0.5
r 128 64 192 64 0 1000
r 256 64 320 64 0 1000
w 64 64 128 64 0
w 192 64 256 64 0
w 320 64 320 128 0
w 320 128 64 128 0
g 64 128 64 160 0
";

fn session() -> Session {
    let mut s = Session::load(CIRCUIT, &Registry::standard()).unwrap();
    s.fit_pending = false;
    s.view = View { scale: 1.0, x: 0.0, y: 0.0 };
    for _ in 0..20 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    s
}

#[cfg(feature = "passive")]
fn select(s: &mut Session, circuit_x: i32) {
    s.deselect_all();
    let i = s
        .sim
        .elements
        .iter()
        .position(|e| e.class_name() == "ResistorElm" && e.base().x == circuit_x);
    s.sim.elements[i.unwrap()].base_mut().selected = true;
}

#[test]
fn with_nothing_under_the_pointer_or_selected_it_tells_the_time() {
    let s = session();
    let l = s.info_lines();
    assert!(l[0].starts_with("t = "), "{l:?}");
    assert!(l[1].starts_with("time step = "), "{l:?}");
    assert!(!s.info_is_about_an_element());
}

#[cfg(feature = "passive")]
#[test]
fn a_selected_element_is_described_when_the_pointer_is_elsewhere() {
    let mut s = session();
    select(&mut s, 128);
    let l = s.info_lines();
    assert_eq!(l[0], "resistor", "{l:?}");
    assert!(
        l.iter().any(|x| x.starts_with("I = ")) && l.iter().any(|x| x.starts_with("P = ")),
        "{l:?}"
    );
    assert!(s.info_is_about_an_element());
}

#[cfg(feature = "passive")]
#[test]
fn the_element_under_the_pointer_comes_before_the_selected_one() {
    let mut s = session();
    select(&mut s, 128);
    // the pointer over the middle of the other resistor: it is that one that is described
    s.hover_at(Some((288.0, 64.0)));
    let l = s.info_lines();
    assert_eq!(l[0], "resistor", "{l:?}");
    let hovered = s.sim.elements.iter().position(|e| e.base().x == 256).unwrap();
    let mut expected = Vec::new();
    s.sim.elements[hovered].get_info(&mut expected);
    assert_eq!(l, expected);
    // leaving it falls back to the selection
    s.hover_at(None);
    let mut selected = Vec::new();
    let i =
        s.sim.elements.iter().position(|e| e.base().x == 128 && e.class_name() == "ResistorElm");
    s.sim.elements[i.unwrap()].get_info(&mut selected);
    assert_eq!(s.info_lines(), selected);
}

#[cfg(feature = "passive")]
#[test]
fn the_pointer_on_a_terminal_gives_the_voltage_there() {
    let mut s = session();
    s.hover_at(Some((128.0, 64.0)));
    let l = s.info_lines();
    assert_eq!(l.len(), 1, "{l:?}");
    assert!(l[0].starts_with("V = "), "{l:?}");
}

#[cfg(feature = "passive")]
#[test]
fn several_selected_elements_do_not_pick_one() {
    let mut s = session();
    s.select_all();
    assert!(s.info_lines()[0].starts_with("t = "));
}

#[test]
fn the_box_is_drawn_in_the_bottom_right_corner() {
    let lines = vec!["resistor".to_string(), "I = 5 mA".to_string()];
    let draw = |tall: bool| {
        let mut g = SvgPainter::new(800.0, 600.0);
        draw_info_box(&mut g, &lines, (800.0, 600.0), tall, false);
        g.finish()
    };
    let svg = draw(true);
    // the panel: 160 wide at the right, a fifth of the height; the text inside it
    assert!(svg.contains("M640 480 L800 480 L800 600 L640 600 Z"), "{svg}");
    assert!(svg.contains(">resistor<") && svg.contains(">I = 5 mA<"));
    assert!(svg.contains("x=\"645\""), "text starts 5 px inside the panel: {svg}");
    // without an element it is a strip of 70 pixels
    assert!(draw(false).contains("M640 530 L800 530 L800 600 L640 600 Z"));
}
