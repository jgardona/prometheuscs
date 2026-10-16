// SPDX-License-Identifier: GPL-2.0-or-later
//! [`Painter`] backend that writes an SVG document. It needs no display, so drawing code can be
//! tested by comparing the text it produces (snapshot tests).

use super::painter::{Color, LineCap, Painter, TextAlign, TextBaseline};
use std::fmt::Write;

/// Affine transform `[a b c d e f]` as in the canvas API.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Matrix([f64; 6]);

impl Matrix {
    const IDENTITY: Matrix = Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// `self` followed by `m` in local coordinates (canvas `transform`).
    fn then(&self, m: [f64; 6]) -> Matrix {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = m;
        Matrix([
            a * a2 + c * b2,
            b * a2 + d * b2,
            a * c2 + c * d2,
            b * c2 + d * d2,
            a * e2 + c * f2 + e,
            b * e2 + d * f2 + f,
        ])
    }

    /// Average scale factor, used for line widths and font sizes.
    fn scale_factor(&self) -> f64 {
        let [a, b, c, d, ..] = self.0;
        (a * d - b * c).abs().sqrt()
    }
}

#[derive(Clone, Debug)]
enum Paint {
    Color(Color),
    Gradient { id: usize },
}

#[derive(Clone, Debug)]
struct State {
    matrix: Matrix,
    paint: Paint,
    color: Color,
    alpha: f64,
    line_width: f64,
    cap: LineCap,
    dash: (i32, i32),
    font_size: i32,
    align: TextAlign,
    baseline: TextBaseline,
}

/// Draws into an SVG string.
pub struct SvgPainter {
    width: f64,
    height: f64,
    state: State,
    stack: Vec<State>,
    path: String,
    defs: String,
    body: String,
    gradients: usize,
}

fn num(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".to_string() } else { s.to_string() }
}

impl SvgPainter {
    /// A painter for a drawing of `width` x `height` pixels.
    pub fn new(width: f64, height: f64) -> Self {
        SvgPainter {
            width,
            height,
            state: State {
                matrix: Matrix::IDENTITY,
                paint: Paint::Color(Color::WHITE),
                color: Color::WHITE,
                alpha: 1.0,
                line_width: 1.0,
                cap: LineCap::Butt,
                dash: (0, 0),
                font_size: 12,
                align: TextAlign::Left,
                baseline: TextBaseline::Alphabetic,
            },
            stack: Vec::new(),
            path: String::new(),
            defs: String::new(),
            body: String::new(),
            gradients: 0,
        }
    }

    /// The finished SVG document.
    pub fn finish(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
            num(self.width),
            num(self.height),
            num(self.width),
            num(self.height)
        );
        if !self.defs.is_empty() {
            let _ = writeln!(out, "<defs>\n{}</defs>", self.defs);
        }
        out.push_str(&self.body);
        out.push_str("</svg>\n");
        out
    }

    fn paint_attr(&self) -> String {
        match &self.state.paint {
            Paint::Color(c) => c.hex(),
            Paint::Gradient { id } => format!("url(#g{id})"),
        }
    }

    fn opacity(&self) -> String {
        if self.state.alpha < 1.0 {
            format!(" opacity=\"{}\"", num(self.state.alpha))
        } else {
            String::new()
        }
    }

    fn point(&mut self, x: f64, y: f64, cmd: char) {
        let (px, py) = self.state.matrix.apply(x, y);
        let _ = write!(self.path, "{cmd}{} {} ", num(px), num(py));
    }

    fn rect_path(&self, x: f64, y: f64, w: f64, h: f64) -> String {
        let m = &self.state.matrix;
        let pts = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
        let mut d = String::new();
        for (i, (px, py)) in pts.iter().enumerate() {
            let (tx, ty) = m.apply(*px, *py);
            let _ = write!(d, "{}{} {} ", if i == 0 { 'M' } else { 'L' }, num(tx), num(ty));
        }
        d.push('Z');
        d
    }

    fn stroke_path(&mut self, d: &str) {
        let s = &self.state;
        let mut attrs = format!(
            "fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"",
            self.paint_attr(),
            num(s.line_width * s.matrix.scale_factor())
        );
        if s.cap == LineCap::Round {
            attrs.push_str(" stroke-linecap=\"round\"");
        }
        if s.dash != (0, 0) {
            let k = s.matrix.scale_factor();
            let _ = write!(
                attrs,
                " stroke-dasharray=\"{} {}\"",
                num(f64::from(s.dash.0) * k),
                num(f64::from(s.dash.1) * k)
            );
        }
        let _ = writeln!(self.body, "<path d=\"{}\" {attrs}{}/>", d.trim_end(), self.opacity());
    }

    fn fill_path(&mut self, d: &str) {
        let _ = writeln!(
            self.body,
            "<path d=\"{}\" fill=\"{}\" stroke=\"none\"{}/>",
            d.trim_end(),
            self.paint_attr(),
            self.opacity()
        );
    }
}

impl Painter for SvgPainter {
    fn set_color(&mut self, color: Color) {
        self.state.color = color;
        self.state.paint = Paint::Color(color);
    }
    fn set_line_width(&mut self, width: f64) {
        self.state.line_width = width;
    }
    fn set_line_cap(&mut self, cap: LineCap) {
        self.state.cap = cap;
    }
    fn set_line_dash(&mut self, on: i32, off: i32) {
        self.state.dash = (on, off);
    }
    fn set_alpha(&mut self, alpha: f64) {
        self.state.alpha = alpha;
    }
    fn set_font_size(&mut self, size: i32) {
        self.state.font_size = size;
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
        // no font metrics without a font backend: a fixed advance keeps snapshots stable
        text.chars().count() as f64 * f64::from(self.state.font_size) * 0.6
    }
    fn save(&mut self) {
        self.stack.push(self.state.clone());
    }
    fn restore(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s;
        }
    }
    fn transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.state.matrix = self.state.matrix.then([a, b, c, d, e, f]);
    }
    fn translate(&mut self, x: f64, y: f64) {
        self.transform(1.0, 0.0, 0.0, 1.0, x, y);
    }
    fn rotate(&mut self, angle: f64) {
        let (s, c) = angle.sin_cos();
        self.transform(c, s, -s, c, 0.0, 0.0);
    }
    fn scale(&mut self, sx: f64, sy: f64) {
        self.transform(sx, 0.0, 0.0, sy, 0.0, 0.0);
    }
    fn begin_path(&mut self) {
        self.path.clear();
    }
    fn move_to(&mut self, x: f64, y: f64) {
        self.point(x, y, 'M');
    }
    fn line_to(&mut self, x: f64, y: f64) {
        let cmd = if self.path.is_empty() { 'M' } else { 'L' };
        self.point(x, y, cmd);
    }
    fn bezier_curve_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) {
        let m = self.state.matrix;
        let (a, b) = m.apply(c1x, c1y);
        let (c, d) = m.apply(c2x, c2y);
        let (e, f) = m.apply(x, y);
        let _ = write!(
            self.path,
            "C{} {} {} {} {} {} ",
            num(a),
            num(b),
            num(c),
            num(d),
            num(e),
            num(f)
        );
    }
    fn arc(&mut self, x: f64, y: f64, radius: f64, start: f64, end: f64) {
        // a polyline of at most 15 degrees per segment, so any transform stays exact enough
        let sweep = end - start;
        let steps = ((sweep.abs() / (std::f64::consts::PI / 12.0)).ceil() as usize).max(1);
        for i in 0..=steps {
            let a = start + sweep * i as f64 / steps as f64;
            let (px, py) = (x + radius * a.cos(), y + radius * a.sin());
            let cmd = if self.path.is_empty() { 'M' } else { 'L' };
            self.point(px, py, cmd);
        }
    }
    fn close_path(&mut self) {
        self.path.push_str("Z ");
    }
    fn stroke(&mut self) {
        let d = self.path.clone();
        if !d.is_empty() {
            self.stroke_path(&d);
        }
    }
    fn fill(&mut self) {
        let d = self.path.clone();
        if !d.is_empty() {
            self.fill_path(&d);
        }
    }
    fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let d = self.rect_path(x, y, w, h);
        self.stroke_path(&d);
    }
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let d = self.rect_path(x, y, w, h);
        self.fill_path(&d);
    }
    fn draw_string(&mut self, text: &str, x: f64, y: f64) {
        let (px, py) = self.state.matrix.apply(x, y);
        let anchor = match self.state.align {
            TextAlign::Left => "start",
            TextAlign::Center => "middle",
        };
        let baseline = match self.state.baseline {
            TextBaseline::Alphabetic => "alphabetic",
            TextBaseline::Middle => "central",
        };
        let size = f64::from(self.state.font_size) * self.state.matrix.scale_factor();
        let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let _ = writeln!(
            self.body,
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{anchor}\" dominant-baseline=\"{baseline}\" fill=\"{}\"{}>{escaped}</text>",
            num(px),
            num(py),
            num(size),
            self.state.color.hex(),
            self.opacity()
        );
    }
    fn set_stroke_gradient(&mut self, len: f64, c0: Color, c1: Color) {
        let (x0, y0) = self.state.matrix.apply(0.0, 0.0);
        let (x1, y1) = self.state.matrix.apply(len, 0.0);
        let id = self.gradients;
        self.gradients += 1;
        let _ = writeln!(
            self.defs,
            "<linearGradient id=\"g{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"><stop offset=\"0\" stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/></linearGradient>",
            num(x0),
            num(y0),
            num(x1),
            num(y1),
            c0.hex(),
            c1.hex()
        );
        self.state.paint = Paint::Gradient { id };
    }
    fn set_stroke_color(&mut self, color: Color) {
        self.set_color(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_transformed_into_device_coordinates() {
        let mut g = SvgPainter::new(100.0, 50.0);
        g.set_color(Color::RED);
        g.translate(10.0, 5.0);
        g.scale(2.0, 2.0);
        g.set_line_width(1.5);
        g.draw_line(0, 0, 10, 0);
        let svg = g.finish();
        assert!(svg.contains("d=\"M10 5 L30 5\""), "{svg}");
        assert!(svg.contains("stroke=\"#ff0000\""));
        assert!(svg.contains("stroke-width=\"3\""));
    }

    #[test]
    fn save_and_restore_bring_back_the_state() {
        let mut g = SvgPainter::new(10.0, 10.0);
        g.save();
        g.translate(5.0, 5.0);
        g.set_color(Color::GREEN);
        g.restore();
        g.draw_line(0, 0, 1, 1);
        let svg = g.finish();
        assert!(svg.contains("M0 0 L1 1") && svg.contains("#ffffff"), "{svg}");
    }

    #[test]
    fn gradients_and_dashes_are_emitted() {
        let mut g = SvgPainter::new(10.0, 10.0);
        g.set_stroke_gradient(20.0, Color::RED, Color::BLUE);
        g.set_line_dash(3, 3);
        g.draw_line(0, 0, 20, 0);
        let svg = g.finish();
        assert!(svg.contains("<linearGradient id=\"g0\""));
        assert!(svg.contains("stroke=\"url(#g0)\"") && svg.contains("stroke-dasharray=\"3 3\""));
    }

    #[test]
    fn text_is_escaped_and_aligned() {
        let mut g = SvgPainter::new(10.0, 10.0);
        g.set_text_align(TextAlign::Center);
        g.draw_string("a<b", 5.0, 5.0);
        let svg = g.finish();
        assert!(svg.contains("text-anchor=\"middle\"") && svg.contains("a&lt;b"));
    }
}
