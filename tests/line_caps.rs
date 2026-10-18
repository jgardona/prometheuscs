// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The original draws every stroke of the circuit with round ends (`UIManager.draw` sets the cap
//! to ROUND before the elements are drawn), so wires and pins that meet look continuous.

use prometheuscs::elements::element::DrawCtx;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::render::scene::{View, draw_scene, post_draw_list};
use prometheuscs::render::svg::SvgPainter;

fn svg(text: &str, ctx: &DrawCtx) -> String {
    let mut r = parse(text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let posts = post_draw_list(&r.circuit.elements);
    let mut g = SvgPainter::new(400.0, 300.0);
    let view = View { scale: 1.0, x: 40.0, y: 20.0 };
    draw_scene(&mut g, &mut r.circuit.elements, &posts, ctx, view, (400.0, 300.0));
    g.finish()
}

fn quiet() -> DrawCtx {
    DrawCtx { show_dots: false, sim_running: false, ..DrawCtx::default() }
}

/// The stroked paths (outlines) of the drawing, one string each.
fn strokes(svg: &str) -> Vec<&str> {
    svg.lines().filter(|l| l.starts_with("<path") && l.contains("fill=\"none\"")).collect()
}

#[cfg(feature = "passive")]
#[test]
fn a_wire_is_drawn_with_round_ends() {
    let out = svg("$ 1 0.000005 10 50 5 50 5e-11\nw 0 0 64 0 0\nw 64 0 64 64 0\n", &quiet());
    let s = strokes(&out);
    assert!(s.len() >= 2, "two wires: {out}");
    assert!(s.iter().all(|l| l.contains("stroke-linecap=\"round\"")), "{s:#?}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn every_stroke_of_a_small_circuit_has_round_ends() {
    let rc = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n";
    for ctx in [quiet(), DrawCtx { euro_resistors: true, ..quiet() }] {
        let out = svg(rc, &ctx);
        let s = strokes(&out);
        assert!(s.len() > 10, "a circuit has many strokes");
        let flat: Vec<_> = s.iter().filter(|l| !l.contains("stroke-linecap=\"round\"")).collect();
        assert!(flat.is_empty(), "strokes with flat ends: {flat:#?}");
    }
}
