// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Graphics, Color).
//! Backend-agnostic 2D drawing interface, modelled on the canvas calls the original uses.

/// An RGB colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red component.
    pub r: u8,
    /// Green component.
    pub g: u8,
    /// Blue component.
    pub b: u8,
}

impl Color {
    /// White.
    pub const WHITE: Color = Color::new(255, 255, 255);
    /// Light gray.
    pub const LIGHT_GRAY: Color = Color::new(192, 192, 192);
    /// Gray.
    pub const GRAY: Color = Color::new(128, 128, 128);
    /// Dark gray.
    pub const DARK_GRAY: Color = Color::new(64, 64, 64);
    /// Black.
    pub const BLACK: Color = Color::new(0, 0, 0);
    /// Red.
    pub const RED: Color = Color::new(255, 0, 0);
    /// Pink.
    pub const PINK: Color = Color::new(255, 175, 175);
    /// Orange.
    pub const ORANGE: Color = Color::new(255, 200, 0);
    /// Yellow.
    pub const YELLOW: Color = Color::new(255, 255, 0);
    /// Green.
    pub const GREEN: Color = Color::new(0, 255, 0);
    /// Magenta.
    pub const MAGENTA: Color = Color::new(255, 0, 255);
    /// Cyan.
    pub const CYAN: Color = Color::new(0, 255, 255);
    /// Blue.
    pub const BLUE: Color = Color::new(0, 0, 255);

    /// Creates a colour from components.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// Mixture of `c1` and `c2`; `mix` of 0 gives `c1`, 1 gives `c2` (components truncated).
    pub fn mix(c1: Color, c2: Color, mix: f64) -> Self {
        let m0 = 1.0 - mix;
        let blend = |a: u8, b: u8| (f64::from(a) * m0 + f64::from(b) * mix) as u8;
        Color::new(blend(c1.r, c2.r), blend(c1.g, c2.g), blend(c1.b, c2.b))
    }

    /// Parses `#rrggbb` (or `rrggbb`); `None` for anything else.
    pub fn parse_hex(text: &str) -> Option<Color> {
        let h = text.trim().trim_start_matches('#');
        if h.len() != 6 || !h.is_ascii() {
            return None;
        }
        let v = u32::from_str_radix(h, 16).ok()?;
        Some(Color::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }

    /// CSS-style hexadecimal form, for example `#ff0000`.
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Horizontal text alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    /// Text starts at the given x.
    Left,
    /// Text is centred on the given x.
    Center,
}

/// Vertical text baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBaseline {
    /// Default alphabetic baseline.
    Alphabetic,
    /// Text is vertically centred on the given y.
    Middle,
}

/// Line end style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineCap {
    /// Flat ends.
    Butt,
    /// Rounded ends.
    Round,
}

/// A 2D drawing surface. Coordinates are in circuit pixels.
///
/// The methods mirror the HTML canvas operations used by the original element drawing code, so
/// each element's `draw` can be translated one to one.
pub trait Painter {
    /// Sets stroke and fill colour.
    fn set_color(&mut self, color: Color);
    /// Sets the stroke width.
    fn set_line_width(&mut self, width: f64);
    /// Sets the line end style.
    fn set_line_cap(&mut self, cap: LineCap);
    /// Sets the dash pattern (`on`, `off`), or solid when both are zero.
    fn set_line_dash(&mut self, on: i32, off: i32);
    /// Sets the global opacity from 0 to 1.
    fn set_alpha(&mut self, alpha: f64);
    /// Sets the font size in pixels.
    fn set_font_size(&mut self, size: i32);
    /// Current font size in pixels.
    fn font_size(&self) -> i32;
    /// Sets text alignment.
    fn set_text_align(&mut self, align: TextAlign);
    /// Sets text baseline.
    fn set_text_baseline(&mut self, baseline: TextBaseline);
    /// Width of `text` in the current font.
    fn measure_text(&self, text: &str) -> f64;
    /// Saves the drawing state (colour, transform, font, ...).
    fn save(&mut self);
    /// Restores the last saved state.
    fn restore(&mut self);
    /// Limits what is drawn from now on (until the matching `restore`) to the rectangle. The
    /// default draws unclipped, which is right for backends that cannot clip.
    fn clip_rect(&mut self, _x: f64, _y: f64, _w: f64, _h: f64) {}
    /// Multiplies the current transform by the given matrix `[a b c d e f]`.
    fn transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64);
    /// Translates the origin.
    fn translate(&mut self, x: f64, y: f64);
    /// Rotates by `angle` radians.
    fn rotate(&mut self, angle: f64);
    /// Scales the axes.
    fn scale(&mut self, sx: f64, sy: f64);
    /// Starts a new path.
    fn begin_path(&mut self);
    /// Moves the pen without drawing.
    fn move_to(&mut self, x: f64, y: f64);
    /// Adds a line to the path.
    fn line_to(&mut self, x: f64, y: f64);
    /// Adds a cubic Bézier curve to the path.
    fn bezier_curve_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64);
    /// Adds a circular arc to the path.
    fn arc(&mut self, x: f64, y: f64, radius: f64, start: f64, end: f64);
    /// Closes the current sub-path.
    fn close_path(&mut self);
    /// Strokes the current path.
    fn stroke(&mut self);
    /// Fills the current path.
    fn fill(&mut self);
    /// Strokes a rectangle outline.
    fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64);
    /// Fills a rectangle.
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64);
    /// Draws text with its origin at (`x`, `y`).
    fn draw_string(&mut self, text: &str, x: f64, y: f64);
    /// Strokes with a horizontal linear gradient from `c0` at x = 0 to `c1` at x = `len`
    /// (in the current transform), used to show voltage along a lead.
    fn set_stroke_gradient(&mut self, len: f64, c0: Color, c1: Color);
    /// Restores a plain stroke colour after [`Painter::set_stroke_gradient`].
    fn set_stroke_color(&mut self, color: Color);

    /// Draws a straight line.
    fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.begin_path();
        self.move_to(f64::from(x1), f64::from(y1));
        self.line_to(f64::from(x2), f64::from(y2));
        self.stroke();
    }

    /// Strokes a closed polyline through `pts`.
    fn draw_polyline(&mut self, pts: &[(i32, i32)]) {
        self.begin_path();
        for (i, &(x, y)) in pts.iter().enumerate() {
            if i == 0 {
                self.move_to(f64::from(x), f64::from(y));
            } else {
                self.line_to(f64::from(x), f64::from(y));
            }
        }
        self.close_path();
        self.stroke();
    }

    /// Fills a closed polygon through `pts`.
    fn fill_polygon(&mut self, pts: &[(i32, i32)]) {
        self.begin_path();
        for (i, &(x, y)) in pts.iter().enumerate() {
            if i == 0 {
                self.move_to(f64::from(x), f64::from(y));
            } else {
                self.line_to(f64::from(x), f64::from(y));
            }
        }
        self.close_path();
        self.fill();
    }

    /// Fills the oval inscribed in the given box (a circle, as in the original).
    fn fill_oval(&mut self, x: i32, y: i32, w: i32, _h: i32) {
        self.begin_path();
        self.arc(
            f64::from(x + w / 2),
            f64::from(y + w / 2),
            f64::from(w / 2),
            0.0,
            2.0 * std::f64::consts::PI,
        );
        self.close_path();
        self.fill();
    }

    /// Strokes a rectangle outline with integer coordinates.
    fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        self.stroke_rect(f64::from(x), f64::from(y), f64::from(w), f64::from(h));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_truncates_like_the_original() {
        let c = Color::mix(Color::GRAY, Color::GREEN, 0.5);
        assert_eq!((c.r, c.g, c.b), (64, 191, 64));
        assert_eq!(Color::mix(Color::RED, Color::BLUE, 0.0), Color::RED);
        assert_eq!(Color::RED.hex(), "#ff0000");
        assert_eq!(Color::parse_hex("#00ff7f"), Some(Color::new(0, 255, 127)));
        assert_eq!(Color::parse_hex("zzz"), None);
    }
}
