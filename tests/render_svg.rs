// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Snapshot tests of circuit drawing through the SVG backend.

use prometheuscs::elements::element::DrawCtx;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::render::scene::{View, draw_scene, post_draw_list};
use prometheuscs::render::svg::SvgPainter;

const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n";

fn render(text: &str, ctx: &DrawCtx) -> String {
    let mut r = parse(text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let posts = post_draw_list(&r.circuit.elements);
    let mut g = SvgPainter::new(200.0, 120.0);
    let view = View { scale: 1.0, x: 40.0, y: 20.0 };
    draw_scene(&mut g, &mut r.circuit.elements, &posts, ctx, view, (200.0, 120.0));
    g.finish()
}

fn quiet() -> DrawCtx {
    DrawCtx { show_dots: false, sim_running: false, ..DrawCtx::default() }
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn rc_circuit_drawing_is_stable() {
    insta::assert_snapshot!("rc_circuit", render(RC, &quiet()));
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn european_resistor_symbol_changes_the_drawing() {
    let zigzag = render(RC, &quiet());
    let euro = render(RC, &DrawCtx { euro_resistors: true, ..quiet() });
    assert_ne!(zigzag, euro);
    insta::assert_snapshot!("rc_circuit_euro_resistor", euro);
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn values_can_be_hidden() {
    let with = render(RC, &quiet());
    let without = render(RC, &DrawCtx { show_values: false, ..quiet() });
    let count = |s: &str| s.matches("<text").count();
    assert!(count(&with) > count(&without));
}

/// A circuit with a gate, for the options that only show on gates.
const GATE: &str = "$ 1 0.000005 10 50 5 50 5e-11\n150 0 0 64 0 0 2 5 0\nw 0 0 0 64 0\n";

#[cfg(feature = "logic")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn each_display_option_changes_the_drawing() {
    let base = render(RC, &quiet());
    let changed = |name: &str, ctx: DrawCtx| {
        assert_ne!(render(RC, &ctx), base, "{name} changes the drawing");
    };
    changed("euro resistors", DrawCtx { euro_resistors: true, ..quiet() });
    changed("show values", DrawCtx { show_values: false, ..quiet() });
    changed("printable (white background)", DrawCtx { printable: true, ..quiet() });
    changed("show volts", DrawCtx { show_volts: false, ..quiet() });
    // the gate shapes differ between the two styles
    let ansi = render(GATE, &quiet());
    let iec = render(GATE, &DrawCtx { iec_gates: true, ..quiet() });
    assert_ne!(ansi, iec, "IEC gates change the drawing of a gate");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn an_option_that_is_off_draws_what_the_default_draws() {
    // the options default to the original's, and turning an option off then on again is a no-op
    let a = render(RC, &quiet());
    let b = render(RC, &DrawCtx { euro_resistors: false, iec_gates: false, ..quiet() });
    assert_eq!(a, b);
}
