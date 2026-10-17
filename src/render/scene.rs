// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CirSim drawing of posts).
//! Drawing of a whole circuit through a [`Painter`].

use super::painter::{Color, Painter, TextAlign};
use crate::elements::element::{DrawCtx, Element};
use crate::elements::geometry::Point;
use std::collections::HashMap;

/// Mapping from circuit coordinates to the drawing surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// Zoom factor.
    pub scale: f64,
    /// Horizontal offset in surface pixels.
    pub x: f64,
    /// Vertical offset in surface pixels.
    pub y: f64,
}

impl Default for View {
    fn default() -> Self {
        View { scale: 1.0, x: 0.0, y: 0.0 }
    }
}

impl View {
    /// Converts a surface position to circuit coordinates.
    pub fn to_circuit(&self, px: f64, py: f64) -> (i32, i32) {
        (((px - self.x) / self.scale).round() as i32, ((py - self.y) / self.scale).round() as i32)
    }

    /// A view that centres the circuit in a `width` x `height` surface, zooming in or out so it
    /// fits with a margin (never beyond `max_scale`).
    pub fn fit(elements: &[Box<dyn Element>], width: f64, height: f64, max_scale: f64) -> View {
        let mut bounds: Option<(i32, i32, i32, i32)> = None;
        for e in elements {
            for i in 0..e.post_count() {
                if let Some(p) = e.get_post(i) {
                    bounds = Some(match bounds {
                        None => (p.x, p.y, p.x, p.y),
                        Some((a, b, c, d)) => (a.min(p.x), b.min(p.y), c.max(p.x), d.max(p.y)),
                    });
                }
            }
        }
        let Some((x0, y0, x1, y1)) = bounds else {
            return View::default();
        };
        let margin = 48.0;
        let (w, h) = (f64::from(x1 - x0).max(1.0), f64::from(y1 - y0).max(1.0));
        let scale = ((width - 2.0 * margin) / w).min((height - 2.0 * margin) / h);
        let scale = scale.clamp(0.1, max_scale);
        View {
            scale,
            x: (width - w * scale) / 2.0 - f64::from(x0) * scale,
            y: (height - h * scale) / 2.0 - f64::from(y0) * scale,
        }
    }
}

/// Points where posts are drawn: any grid point that does not join exactly two element posts.
pub fn post_draw_list(elements: &[Box<dyn Element>]) -> Vec<Point> {
    let mut counts: HashMap<(i32, i32), usize> = HashMap::new();
    let mut order = Vec::new();
    for e in elements {
        for i in 0..e.post_count() {
            if let Some(p) = e.get_post(i) {
                let n = counts.entry((p.x, p.y)).or_insert(0);
                if *n == 0 {
                    order.push(p);
                }
                *n += 1;
            }
        }
    }
    order.into_iter().filter(|p| counts.get(&(p.x, p.y)).copied() != Some(2)).collect()
}

/// Draws the circuit: background, every element, then the junction posts.
pub fn draw_scene(
    g: &mut dyn Painter,
    elements: &mut [Box<dyn Element>],
    posts: &[Point],
    ctx: &DrawCtx,
    view: View,
    size: (f64, f64),
) {
    g.set_color(if ctx.printable { Color::WHITE } else { Color::BLACK });
    g.fill_rect(0.0, 0.0, size.0, size.1);
    g.save();
    g.translate(view.x, view.y);
    g.scale(view.scale, view.scale);
    g.set_line_cap(super::painter::LineCap::Butt);
    for e in elements.iter_mut() {
        g.save();
        g.set_line_width(1.0);
        e.draw(g, ctx);
        g.restore();
    }
    for &p in posts {
        crate::elements::element::draw_post(g, ctx, p);
    }
    g.restore();
}

/// Draws the transient overlays on top of the circuit: the selection rectangle being dragged
/// (circuit coordinates) and the cursor crosshair (circuit coordinates of the snapped pointer).
pub fn draw_overlay(
    g: &mut dyn Painter,
    ctx: &DrawCtx,
    view: View,
    size: (f64, f64),
    rubber: Option<crate::elements::geometry::Rect>,
    crosshair: Option<(i32, i32)>,
) {
    if let Some((cx, cy)) = crosshair {
        let sx = f64::from(cx) * view.scale + view.x;
        let sy = f64::from(cy) * view.scale + view.y;
        g.save();
        g.set_color(Color::GRAY);
        g.set_alpha(0.6);
        g.set_line_width(1.0);
        g.begin_path();
        g.move_to(0.0, sy);
        g.line_to(size.0, sy);
        g.move_to(sx, 0.0);
        g.line_to(sx, size.1);
        g.stroke();
        g.restore();
    }
    if let Some(r) = rubber {
        g.save();
        g.translate(view.x, view.y);
        g.scale(view.scale, view.scale);
        g.set_color(ctx.select_color);
        g.set_line_width(1.0);
        g.set_line_dash(4, 4);
        g.stroke_rect(f64::from(r.x), f64::from(r.y), f64::from(r.width), f64::from(r.height));
        g.restore();
    }
}

// the tests draw resistors
#[cfg(all(test, feature = "passive"))]
mod tests {
    use super::*;
    use crate::elements::passive::ResistorElm;

    fn resistor(x: i32, y: i32, x2: i32, y2: i32) -> Box<dyn Element> {
        let mut r = ResistorElm::new(x, y);
        r.base_mut().set_position(x, y, x2, y2);
        r.set_points();
        Box::new(r)
    }

    #[test]
    fn posts_are_drawn_only_where_not_exactly_two_meet() {
        let elms = vec![resistor(0, 0, 64, 0), resistor(64, 0, 64, 64), resistor(64, 0, 128, 0)];
        let posts = post_draw_list(&elms);
        assert!(posts.contains(&Point::new(64, 0)));
        assert!(posts.contains(&Point::new(0, 0)));
        assert_eq!(posts.len(), 4);
        assert!(post_draw_list(&elms[..2]).iter().all(|p| *p != Point::new(64, 0)));
    }

    #[test]
    fn surface_positions_map_back_to_circuit_coordinates() {
        let v = View { scale: 2.0, x: 10.0, y: 20.0 };
        assert_eq!(v.to_circuit(10.0 + 2.0 * 30.0, 20.0 + 2.0 * 40.0), (30, 40));
    }

    #[test]
    fn fit_view_centres_the_circuit() {
        let elms = vec![resistor(0, 0, 100, 0)];
        let v = View::fit(&elms, 400.0, 300.0, 2.0);
        assert!(v.scale > 0.0 && v.scale <= 2.0);
        let mid = 50.0 * v.scale + v.x;
        assert!((mid - 200.0).abs() < 1e-9);
    }
}

/// Width of the information box.
pub const INFO_BOX_WIDTH: f64 = 160.0;

/// The information box of the original, for a canvas with no scopes under it: a dark panel in
/// the bottom right corner with the lines of text. It is a fifth of the height when it
/// describes an element (`tall`) and a small strip otherwise.
pub fn draw_info_box(
    g: &mut dyn Painter,
    lines: &[String],
    (w, h): (f64, f64),
    tall: bool,
    light: bool,
) {
    let box_h = if tall { h * 0.2 } else { 70.0 }.min(h);
    let x = (w - INFO_BOX_WIDTH).max(0.0);
    g.set_color(if light { Color::new(238, 238, 238) } else { Color::new(17, 17, 17) });
    g.fill_rect(x, h - box_h, w - x, box_h);
    g.set_color(if light { Color::new(0, 0, 0) } else { Color::new(255, 255, 255) });
    g.set_font_size(12);
    g.set_text_align(TextAlign::Left);
    for (i, line) in lines.iter().enumerate() {
        let y = h - box_h + 15.0 * (i + 1) as f64;
        if y > h - 2.0 {
            break;
        }
        g.draw_string(line, x + 5.0, y);
    }
}
