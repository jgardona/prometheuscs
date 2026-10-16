// SPDX-License-Identifier: GPL-2.0-or-later
//! [`Painter`] backend drawing with Cairo, used by the GTK drawing area and for image export.

use super::painter::{Color, LineCap, Painter, TextAlign, TextBaseline};
use gtk4::cairo::{Context, FontSlant, FontWeight, LinearGradient, Matrix};

/// Drawing state that Cairo does not keep itself.
#[derive(Clone, Copy)]
struct State {
    color: Color,
    alpha: f64,
    align: TextAlign,
    baseline: TextBaseline,
    font_size: i32,
}

/// Draws onto a Cairo context.
pub struct CairoPainter<'a> {
    cr: &'a Context,
    state: State,
    stack: Vec<State>,
}

impl<'a> CairoPainter<'a> {
    /// Wraps a context; the context's current transform is the circuit-to-surface mapping.
    pub fn new(cr: &'a Context) -> Self {
        cr.select_font_face("sans-serif", FontSlant::Normal, FontWeight::Normal);
        cr.set_font_size(12.0);
        cr.set_line_width(1.0);
        let p = CairoPainter {
            cr,
            state: State {
                color: Color::WHITE,
                alpha: 1.0,
                align: TextAlign::Left,
                baseline: TextBaseline::Alphabetic,
                font_size: 12,
            },
            stack: Vec::new(),
        };
        p.apply_color();
        p
    }

    fn apply_color(&self) {
        let c = self.state.color;
        self.cr.set_source_rgba(
            f64::from(c.r) / 255.0,
            f64::from(c.g) / 255.0,
            f64::from(c.b) / 255.0,
            self.state.alpha,
        );
    }
}

impl Painter for CairoPainter<'_> {
    fn set_color(&mut self, color: Color) {
        self.state.color = color;
        self.apply_color();
    }
    fn set_line_width(&mut self, width: f64) {
        self.cr.set_line_width(width);
    }
    fn set_line_cap(&mut self, cap: LineCap) {
        self.cr.set_line_cap(match cap {
            LineCap::Butt => gtk4::cairo::LineCap::Butt,
            LineCap::Round => gtk4::cairo::LineCap::Round,
        });
    }
    fn set_line_dash(&mut self, on: i32, off: i32) {
        if on == 0 && off == 0 {
            self.cr.set_dash(&[], 0.0);
        } else {
            self.cr.set_dash(&[f64::from(on), f64::from(off)], 0.0);
        }
    }
    fn set_alpha(&mut self, alpha: f64) {
        self.state.alpha = alpha;
        self.apply_color();
    }
    fn set_font_size(&mut self, size: i32) {
        self.state.font_size = size;
        self.cr.set_font_size(f64::from(size));
    }
    fn font_size(&self) -> i32 {
        self.state.font_size
    }
    fn set_text_align(&mut self, align: TextAlign) {
        self.state.align = align;
    }
    fn set_text_baseline(&mut self, baseline: TextBaseline) {
        self.state.baseline = baseline;
    }
    fn measure_text(&self, text: &str) -> f64 {
        self.cr.text_extents(text).map_or(0.0, |e| e.x_advance())
    }
    fn save(&mut self) {
        self.stack.push(self.state);
        self.cr.save().ok();
    }
    fn restore(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s;
            self.cr.restore().ok();
        }
    }
    fn transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.cr.transform(Matrix::new(a, b, c, d, e, f));
    }
    fn translate(&mut self, x: f64, y: f64) {
        self.cr.translate(x, y);
    }
    fn rotate(&mut self, angle: f64) {
        self.cr.rotate(angle);
    }
    fn scale(&mut self, sx: f64, sy: f64) {
        self.cr.scale(sx, sy);
    }
    fn begin_path(&mut self) {
        self.cr.new_path();
    }
    fn move_to(&mut self, x: f64, y: f64) {
        self.cr.move_to(x, y);
    }
    fn line_to(&mut self, x: f64, y: f64) {
        self.cr.line_to(x, y);
    }
    fn bezier_curve_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) {
        self.cr.curve_to(c1x, c1y, c2x, c2y, x, y);
    }
    fn arc(&mut self, x: f64, y: f64, radius: f64, start: f64, end: f64) {
        self.cr.arc(x, y, radius, start, end);
    }
    fn close_path(&mut self) {
        self.cr.close_path();
    }
    fn stroke(&mut self) {
        self.cr.stroke().ok();
    }
    fn fill(&mut self) {
        self.cr.fill().ok();
    }
    fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.cr.rectangle(x, y, w, h);
        self.cr.stroke().ok();
    }
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.cr.rectangle(x, y, w, h);
        self.cr.fill().ok();
    }
    fn draw_string(&mut self, text: &str, x: f64, y: f64) {
        let width = self.measure_text(text);
        let x = match self.state.align {
            TextAlign::Left => x,
            TextAlign::Center => x - width / 2.0,
        };
        let y = match self.state.baseline {
            TextBaseline::Alphabetic => y,
            TextBaseline::Middle => {
                self.cr.font_extents().map_or(y, |e| y + (e.ascent() - e.descent()) / 2.0)
            }
        };
        self.cr.move_to(x, y);
        self.cr.show_text(text).ok();
        self.cr.new_path();
    }
    fn set_stroke_gradient(&mut self, len: f64, c0: Color, c1: Color) {
        let g = LinearGradient::new(0.0, 0.0, len, 0.0);
        let a = self.state.alpha;
        let stop =
            |c: Color| (f64::from(c.r) / 255.0, f64::from(c.g) / 255.0, f64::from(c.b) / 255.0, a);
        let (r, gr, b, al) = stop(c0);
        g.add_color_stop_rgba(0.0, r, gr, b, al);
        let (r, gr, b, al) = stop(c1);
        g.add_color_stop_rgba(1.0, r, gr, b, al);
        self.cr.set_source(&g).ok();
    }
    fn set_stroke_color(&mut self, color: Color) {
        self.set_color(color);
    }
}
