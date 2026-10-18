// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(all(
    feature = "subcircuits",
    feature = "diodes",
    feature = "analog",
    feature = "transistors"
))]
//! The optocoupler's pins are short, and nothing of it is drawn at the origin of the circuit.

use prometheuscs::elements::element::{DrawCtx, Element};
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::render::svg::SvgPainter;

fn placed(x: i32, y: i32) -> Box<dyn Element> {
    let mut e = Registry::standard().create("OptocouplerElm", x, y).unwrap();
    e.set_points();
    e
}

/// Every coordinate pair of the paths of a drawing.
fn coordinates(svg: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for d in svg.split("d=\"").skip(1) {
        let d = d.split('"').next().unwrap();
        let nums: Vec<f64> = d
            .split(|c: char| c.is_ascii_alphabetic() || c.is_whitespace())
            .filter_map(|t| t.parse().ok())
            .collect();
        out.extend(nums.as_chunks::<2>().0.iter().map(|p| (p[0], p[1])));
    }
    out
}

/// No part of the element is drawn far from where it is: the pins are short and nothing is at
/// the origin.
fn assert_drawn_in_place(e: &mut Box<dyn Element>, what: &str) {
    let (bx, by) = (e.base().x, e.base().y);
    let mut g = SvgPainter::new(1000.0, 1000.0);
    e.draw(&mut g, &DrawCtx::default());
    let svg = g.finish();
    let points = coordinates(&svg);
    assert!(points.len() > 20, "{what}: something is drawn");
    for (x, y) in points {
        // the chip is about 100 by 50 around its position
        assert!(
            (x - f64::from(bx)).abs() < 200.0 && (y - f64::from(by)).abs() < 200.0,
            "{what}: a point at ({x}, {y}) is far from the element at ({bx}, {by})"
        );
    }
}

#[test]
fn the_parts_of_a_placed_optocoupler_have_their_terminals() {
    let e = placed(300, 400);
    for (i, child) in e.child_elements().iter().enumerate() {
        // the current source has no terminals of its own to draw
        if child.class_name() == "CCCSElm" {
            continue;
        }
        for n in 0..child.post_count() {
            let p = child.get_post(n).unwrap();
            assert!(
                (p.x, p.y) != (0, 0),
                "{} (part {i}) has terminal {n} at the origin",
                child.class_name()
            );
        }
    }
}

#[test]
fn the_pins_are_short_and_nothing_is_drawn_at_the_origin() {
    let mut e = placed(300, 400);
    assert_drawn_in_place(&mut e, "placed");
}

#[test]
fn it_stays_right_when_flipped_and_when_moved() {
    for (fx, fy) in [(true, false), (false, true), (true, true)] {
        let mut e = placed(300, 400);
        if fx {
            e.flip_x(600, 1);
        }
        if fy {
            e.flip_y(800, 1);
        }
        assert_drawn_in_place(&mut e, &format!("flipped x={fx} y={fy}"));
    }
    let mut e = placed(300, 400);
    e.base_mut().x = 640;
    e.base_mut().y = 320;
    e.set_points();
    assert_drawn_in_place(&mut e, "moved");
}

#[test]
fn one_read_from_a_file_is_drawn_in_place() {
    let text = "$ 1 0.000005 10 50 5 50 5e-11\n407 300 400 316 400 0\n";
    let mut parsed = parse(text, &Registry::standard());
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    let e = parsed.circuit.elements.first_mut().unwrap();
    assert_eq!(e.class_name(), "OptocouplerElm");
    assert_drawn_in_place(e, "read from a file");
}

/// The same defect (a part left at the origin) in any element of the registry: placed away from
/// the origin, nothing it draws may be far from it.
#[test]
fn no_element_draws_anything_at_the_origin_when_placed_elsewhere() {
    let registry = Registry::standard();
    let mut far = Vec::new();
    for id in registry.ids() {
        let Some(mut e) = registry.create(id, 640, 480) else { continue };
        // as when placed: dragged out to some length
        e.drag(704, 480);
        e.set_points();
        let mut g = SvgPainter::new(2000.0, 2000.0);
        e.draw(&mut g, &DrawCtx::default());
        let svg = g.finish();
        if coordinates(&svg).iter().any(|&(x, y)| x < 200.0 && y < 200.0) {
            far.push(id);
        }
    }
    assert!(far.is_empty(), "drawn near the origin though placed at (640, 480): {far:?}");
}
