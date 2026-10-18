// SPDX-License-Identifier: GPL-2.0-or-later
//! The schematic of an example as an SVG picture, drawn by the same painter the program uses.

use crate::elements::element::DrawCtx;
use crate::elements::registry::Registry;
use crate::format::parser::parse;
use crate::render::scene::{View, draw_scene, post_draw_list};
use crate::render::svg::SvgPainter;

/// Space left around the circuit, in circuit pixels.
const MARGIN: i32 = 40;

/// The picture of the circuit in `text` (drawn as a running simulation: the sweep source
/// otherwise animates its icon from the wall clock, and a picture must not change between runs), or `None` when it is empty or does not load.
pub fn schematic_svg(text: &str, registry: &Registry) -> Option<String> {
    let mut r = parse(text, registry);
    if !r.errors.is_empty() {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for e in &r.circuit.elements {
        let b = &e.base().bbox;
        x0 = x0.min(b.x);
        y0 = y0.min(b.y);
        x1 = x1.max(b.x + b.width);
        y1 = y1.max(b.y + b.height);
    }
    if x0 > x1 {
        return None;
    }
    let (w, h) = (f64::from(x1 - x0 + 2 * MARGIN), f64::from(y1 - y0 + 2 * MARGIN));
    let view = View { scale: 1.0, x: f64::from(MARGIN - x0), y: f64::from(MARGIN - y0) };
    let ctx = DrawCtx { show_dots: false, sim_running: true, ..DrawCtx::default() };
    let posts = post_draw_list(&r.circuit.elements);
    let mut g = SvgPainter::new(w, h);
    draw_scene(&mut g, &mut r.circuit.elements, &posts, &ctx, view, (w, h));
    Some(g.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "passive", feature = "sources"))]
    #[test]
    fn a_circuit_gives_a_picture_and_an_empty_one_gives_none() {
        let rc = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 128 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nw 64 0 64 128 0\nw 64 128 0 128 0\n";
        let svg = schematic_svg(rc, &Registry::standard()).unwrap();
        assert!(svg.starts_with("<svg ") && svg.trim_end().ends_with("</svg>"));
        assert_eq!(svg, schematic_svg(rc, &Registry::standard()).unwrap(), "not deterministic");
        assert_eq!(schematic_svg("$ 1 0.000005 10 50 5 50 5e-11\n", &Registry::standard()), None);
    }
}
