// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! An element that has no terminals (a text, a box) is picked by a press anywhere inside its
//! bounding box, as in the original; it used to be picked only within 8 px of the centre.

use prometheuscs::app::edit::Mods;
use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;
use prometheuscs::render::scene::{View, draw_scene, post_draw_list};
use prometheuscs::render::svg::SvgPainter;
use std::time::Duration;

/// A session on `text` with the circuit drawn once, so the elements know their bounding boxes.
fn drawn(text: &str) -> Session {
    let mut s = Session::load(text, &Registry::standard()).unwrap();
    s.update(Duration::from_millis(16)).unwrap();
    s.view = View { scale: 1.0, x: 0.0, y: 0.0 };
    let posts = post_draw_list(&s.sim.elements);
    let ctx = s.draw_ctx.clone();
    let mut g = SvgPainter::new(600.0, 400.0);
    draw_scene(&mut g, &mut s.sim.elements, &posts, &ctx, s.view, (600.0, 400.0));
    s
}

const TEXT: &str = "$ 1 5e-6 10 63 10 62\nx 100 100 116 100 4 24 hello\n";

/// Presses at (`x`, `y`) with nothing selected and lets go; true when the press found an element
/// (and so would start moving it), not when it merely began a rubber band.
fn press(s: &mut Session, x: i32, y: i32) -> bool {
    s.deselect_all();
    let hit = s.mouse_down(f64::from(x), f64::from(y), Mods::default(), 1);
    s.mouse_up();
    hit
}

#[cfg(feature = "outputs")]
#[test]
fn a_press_anywhere_in_a_text_selects_it() {
    let mut s = drawn(TEXT);
    let b = s.sim.elements[0].base().bbox;
    let (cx, cy) = (b.x + b.width / 2, b.y + b.height / 2);
    for (name, x, y) in [
        ("centre", cx, cy),
        ("left", b.x + 2, cy),
        ("right", b.x + b.width - 3, cy),
        ("top", cx, b.y + 2),
        ("bottom", cx, b.y + b.height - 3),
        ("corner", b.x + 2, b.y + 2),
    ] {
        assert!(press(&mut s, x, y), "{name} ({x}, {y}) of {b:?}");
        assert_eq!(s.selected_indices(), vec![0], "{name}");
    }
}

#[cfg(feature = "outputs")]
#[test]
fn a_press_well_outside_the_text_does_not_select_it() {
    let mut s = drawn(TEXT);
    let b = s.sim.elements[0].base().bbox;
    for (x, y) in [
        (b.x - 20, b.y + 5),
        (b.x + b.width + 20, b.y + 5),
        (b.x + 5, b.y - 20),
        (b.x + 5, b.y + b.height + 20),
    ] {
        assert!(!press(&mut s, x, y), "({x}, {y}) of {b:?}");
        assert!(s.selected_indices().is_empty(), "({x}, {y}) of {b:?}");
    }
}

#[cfg(feature = "outputs")]
#[test]
fn a_text_dragged_from_its_edge_moves() {
    let mut s = drawn(TEXT);
    let b = s.sim.elements[0].base().bbox;
    let before = (s.sim.elements[0].base().x, s.sim.elements[0].base().y);
    let (x, y) = (b.x + 3, b.y + b.height - 4);
    s.mouse_down(f64::from(x), f64::from(y), Mods::default(), 1);
    s.mouse_drag(f64::from(x + 48), f64::from(y + 32));
    s.mouse_up();
    let after = (s.sim.elements[0].base().x, s.sim.elements[0].base().y);
    assert_eq!((after.0 - before.0, after.1 - before.1), (48, 32), "moved by the drag");
}

#[cfg(feature = "passive")]
#[cfg(feature = "outputs")]
#[test]
fn a_part_under_a_text_still_wins_on_its_own_line() {
    // a resistor along y = 100 and the text over it: the line is the resistor, the rest is text
    let mut s =
        drawn("$ 1 5e-6 10 63 10 62\nr 100 100 164 100 0 1000\nx 100 100 116 100 4 24 hello\n");
    let tb = s.sim.elements[1].base().bbox;
    assert!(press(&mut s, 132, 100));
    assert_eq!(s.selected_indices(), vec![0], "on the resistor's line");
    assert!(press(&mut s, tb.x + 4, tb.y + 2));
    assert_eq!(s.selected_indices(), vec![1], "inside the text, away from the line");
}

#[cfg(feature = "outputs")]
#[test]
fn a_box_is_grabbed_by_its_edge_not_by_its_inside() {
    let mut s = drawn("$ 1 5e-6 10 63 10 62\nb 100 100 260 220 0\n");
    assert!(press(&mut s, 100 + 9, 160), "9 px inside the left edge (the original grabs 10)");
    assert_eq!(s.selected_indices(), vec![0]);
    assert!(!press(&mut s, 180, 160), "the inside is not the box");
    assert!(!press(&mut s, 100 + 12, 160), "12 px inside the edge is out of its band");
}

/// The elements that have their own, narrower rule for what is under the mouse (wires and the
/// like, as in the original): a press far from their line must not pick them.
const OWN_RULE: [&str; 6] =
    ["WireElm", "RoutedWireElm", "LineElm", "BoxElm", "ScopeElm", "ProbeElm"];

#[test]
fn a_press_inside_the_box_of_any_element_picks_it() {
    use prometheuscs::format::writer::write_elements_xml;
    let registry = Registry::standard();
    let mut weak = Vec::new();
    let mut checked = 0;
    for id in registry.ids() {
        if OWN_RULE.contains(&id) {
            continue;
        }
        // the optocoupler is built from a diode, a controlled source and a transistor
        let optocoupler_parts =
            cfg!(all(feature = "diodes", feature = "analog", feature = "transistors"));
        if id == "OptocouplerElm" && !optocoupler_parts {
            continue;
        }
        let Some(mut e) = registry.create(id, 256, 128) else { continue };
        e.drag(320, 128);
        e.set_points();
        let text = write_elements_xml(std::iter::once(e.as_ref()));
        let Ok(mut s) = Session::load(&text, &registry) else { continue };
        if s.sim.elements.is_empty() {
            continue;
        }
        let _ = s.update(Duration::from_millis(16));
        s.view = View { scale: 1.0, x: 0.0, y: 0.0 };
        let posts = post_draw_list(&s.sim.elements);
        let ctx = s.draw_ctx.clone();
        let mut g = SvgPainter::new(900.0, 700.0);
        draw_scene(&mut g, &mut s.sim.elements, &posts, &ctx, s.view, (900.0, 700.0));
        let b = s.sim.elements[0].base().bbox;
        if b.width < 24 || b.height < 24 {
            continue;
        }
        checked += 1;
        let (mut hit, mut total) = (0, 0);
        for y in (b.y + 2..b.y + b.height - 2).step_by(4) {
            for x in (b.x + 2..b.x + b.width - 2).step_by(4) {
                total += 1;
                if press(&mut s, x, y) {
                    hit += 1;
                }
            }
        }
        if hit != total {
            weak.push(format!("{id}: {hit}/{total} of {b:?}"));
        }
    }
    assert!(checked > 50, "the sweep looked at {checked} elements");
    assert!(weak.is_empty(), "presses inside the box that miss the element:\n{}", weak.join("\n"));
}
