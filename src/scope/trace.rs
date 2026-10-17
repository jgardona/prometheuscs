// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (ScopePlot2d).
//! The picture an X/Y scope draws: a persistent image the trace is painted into at every time
//! step and that slowly fades, as on an oscilloscope screen.
//!
//! The original keeps a canvas the size of the scope. Here the image is a fixed grid of
//! [`TRACE_W`] by [`TRACE_H`] cells, made on the simulation's thread (which sees every step) and shown
//! stretched to the scope; a trace that scans a raster, like the smiley example, needs every
//! step, not the one sample in sixty-four the time plots keep.

use super::XyAxes;
use std::sync::atomic::{AtomicU32, Ordering};

/// How many time steps make three frames of the screen at the current speed. The original dims
/// its picture by a hundredth every third frame it draws; the simulation sets this so that the
/// picture dims at the same pace of the display whatever the speed.
static FADE_STEPS: AtomicU32 = AtomicU32::new(2000);

/// Tells the traces how many steps the simulation runs in three frames (about 50 ms).
pub fn set_fade_steps(steps: f64) {
    FADE_STEPS.store(steps.clamp(64.0, 1e7) as u32, Ordering::Relaxed);
}

/// Cells across the image. It is wider than tall because the scopes are: shown stretched to a
/// scope of 1600 by 130 pixels it still has cells of a few pixels.
pub const TRACE_W: usize = 512;
/// Cells down the image.
pub const TRACE_H: usize = 192;

/// What decides where and how the trace is drawn; when it changes the image starts again.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceSetup {
    /// Which plots are the axes, modulators and the trail.
    pub axes: XyAxes,
    /// Scales set by hand (units per division and position for the X and Y plots).
    pub manual: Option<ManualAxes>,
    /// How many plots there are.
    pub plots: usize,
}

/// The manual scale of the two axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManualAxes {
    /// Units per division, horizontal and vertical.
    pub scale: (f64, f64),
    /// Position, in parts of the whole height, horizontal and vertical.
    pub pos: (f64, f64),
    /// Divisions across.
    pub divisions: f64,
}

/// The image and the state that goes with it.
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    /// RGB of each cell, row by row from the top.
    pub pixels: Vec<u8>,
    /// Full scale of the horizontal and vertical axes in automatic mode (they only grow).
    pub scale_x: f64,
    /// See `scale_x`.
    pub scale_y: f64,
    scale_mod: [f64; 4],
    setup: Option<TraceSetup>,
    last: Option<(f64, f64)>,
    since_fade: u32,
}

impl Default for Trace {
    fn default() -> Self {
        Trace {
            pixels: vec![0; TRACE_W * TRACE_H * 3],
            scale_x: 5.0,
            scale_y: 0.1,
            scale_mod: [5.0; 4],
            setup: None,
            last: None,
            since_fade: 0,
        }
    }
}

impl Trace {
    /// Empties the image.
    pub fn clear(&mut self) {
        self.pixels.fill(0);
        self.last = None;
    }

    /// The cell the trace head is in, as fractions of the width and the height from the top
    /// left.
    pub fn head(&self) -> Option<(f64, f64)> {
        self.last
    }

    /// Paints one time step: `values` are the current values of the plots.
    pub fn step(&mut self, values: &[f64], setup: TraceSetup) {
        if self.setup != Some(setup) {
            self.setup = Some(setup);
            self.clear();
        }
        let n = values.len();
        if n == 0 {
            return;
        }
        let a = setup.axes;
        let (px, py) = (XyAxes::valid(a.x, 0, n), XyAxes::valid(a.y, 1, n));
        let (vx, vy) = (values[px], values[py]);
        let point = match setup.manual {
            Some(m) => {
                let span = m.divisions + 0.1;
                (
                    0.5 + (vx / m.scale.0.max(1e-12) + m.divisions * m.pos.0) / span,
                    0.5 - (vy / m.scale.1.max(1e-12) + m.divisions * m.pos.1) / span,
                )
            }
            None => {
                let mut grew = false;
                while (vx > self.scale_x || vx < -self.scale_x) && self.scale_x < 1e30 {
                    self.scale_x *= 2.0;
                    grew = true;
                }
                while (vy > self.scale_y || vy < -self.scale_y) && self.scale_y < 1e30 {
                    self.scale_y *= 2.0;
                    grew = true;
                }
                if grew {
                    self.clear();
                }
                (0.499 * (1.0 + vx / self.scale_x), 0.499 * (1.0 - vy / self.scale_y))
            }
        };
        let (color, alpha) = self.paint_of(values, &a);
        if let Some(from) = self.last {
            self.line(from, point, color, alpha);
        }
        self.last = Some(point);
        self.fade(a.trail);
    }

    /// The colour and opacity of the trace from the modulating plots.
    fn paint_of(&mut self, values: &[f64], a: &XyAxes) -> ([u8; 3], f64) {
        let n = values.len();
        let mut take = |index: i32, slot: usize, absolute: bool| {
            let v = XyAxes::modulator(index, n)?;
            let mut v = values[v];
            if absolute {
                v = v.abs();
            }
            while v > self.scale_mod[slot] && self.scale_mod[slot] < 1e30 {
                self.scale_mod[slot] *= 2.0;
            }
            Some((v, self.scale_mod[slot]))
        };
        let brightness = take(a.brightness, 0, true);
        let (r, g, b) = (take(a.red, 1, false), take(a.green, 2, false), take(a.blue, 3, false));
        let alpha = brightness.map_or(1.0, |(v, s)| (v / s).clamp(0.0, 1.0));
        let part =
            |c: Option<(f64, f64)>| c.map_or(0, |(v, s)| ((v / s) * 255.0).clamp(0.0, 255.0) as u8);
        let color = if r.is_none() && g.is_none() && b.is_none() {
            [255, 255, 255]
        } else {
            [part(r), part(g), part(b)]
        };
        (color, alpha)
    }

    /// Paints a line of cells from one point to the next.
    fn line(&mut self, from: (f64, f64), to: (f64, f64), color: [u8; 3], alpha: f64) {
        let (sw, sh) = (TRACE_W as f64, TRACE_H as f64);
        let (dx, dy) = ((to.0 - from.0) * sw, (to.1 - from.1) * sh);
        let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as usize;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let (x, y) = ((from.0 + (to.0 - from.0) * t) * sw, (from.1 + (to.1 - from.1) * t) * sh);
            if x < 0.0 || y < 0.0 || x >= sw || y >= sh {
                continue;
            }
            let at = (y as usize * TRACE_W + x as usize) * 3;
            for (c, new) in self.pixels[at..at + 3].iter_mut().zip(color) {
                *c = (f64::from(*c) * (1.0 - alpha) + f64::from(new) * alpha).round() as u8;
            }
        }
    }

    /// Dims the image a little as time goes on: by a hundredth every three frames (see
    /// [`set_fade_steps`]), or, with a trail of `trail` steps, continuously with that time
    /// constant.
    fn fade(&mut self, trail: i32) {
        self.since_fade += 1;
        let (every, keep) = if trail > 0 {
            (64, (-64.0 / f64::from(trail)).exp())
        } else {
            (FADE_STEPS.load(Ordering::Relaxed), 0.99)
        };
        if self.since_fade < every {
            return;
        }
        self.since_fade = 0;
        for c in &mut self.pixels {
            // always lose at least one level, or a faint cell would never go out
            let dimmed = (f64::from(*c) * keep) as u8;
            *c = if dimmed == *c { c.saturating_sub(1) } else { dimmed };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(manual: bool) -> TraceSetup {
        TraceSetup {
            axes: XyAxes::default(),
            manual: manual.then_some(ManualAxes {
                scale: (1.0, 1.0),
                pos: (0.0, 0.0),
                divisions: 8.0,
            }),
            plots: 2,
        }
    }

    fn lit(t: &Trace) -> usize {
        t.pixels.chunks(3).filter(|p| p.iter().any(|v| *v != 0)).count()
    }

    #[test]
    fn a_moving_point_draws_a_line_of_cells() {
        let mut t = Trace::default();
        let s = setup(true);
        t.step(&[-3.0, -3.0], s);
        assert_eq!(lit(&t), 0, "one point is no line");
        t.step(&[3.0, -3.0], s);
        // six divisions of a nine-and-a-bit span across 512 cells: about 330 cells
        assert!(lit(&t) > 250, "{}", lit(&t));
    }

    #[test]
    fn brightness_and_colour_follow_their_plots() {
        let mut t = Trace::default();
        let mut s = setup(true);
        s.plots = 4;
        s.axes = XyAxes { brightness: 2, red: 3, ..XyAxes::default() };
        t.step(&[0.0, 0.0, 0.0, 0.0], s);
        t.step(&[2.0, 0.0, 5.0, 5.0], s);
        let cell = t.pixels.chunks(3).find(|p| p.iter().any(|v| *v != 0)).unwrap();
        // full brightness, and only red lit
        assert!(cell[0] > 200 && cell[1] == 0 && cell[2] == 0, "{cell:?}");
        // no brightness: nothing is added
        let mut dark = Trace::default();
        dark.step(&[0.0, 0.0, 0.0, 0.0], s);
        dark.step(&[2.0, 0.0, 0.0, 5.0], s);
        assert_eq!(lit(&dark), 0);
    }

    #[test]
    fn the_automatic_scale_grows_and_starts_the_picture_again() {
        let mut t = Trace::default();
        let s = setup(false);
        t.step(&[1.0, 0.05], s);
        t.step(&[2.0, 0.05], s);
        assert!(lit(&t) > 0);
        t.step(&[20.0, 0.05], s);
        assert!(t.scale_x >= 20.0);
        assert_eq!(lit(&t), 0, "the old picture was at the old scale");
    }

    #[test]
    fn changing_the_setup_clears_and_time_fades_the_picture() {
        let mut t = Trace::default();
        t.step(&[-3.0, 0.0], setup(true));
        t.step(&[3.0, 0.0], setup(true));
        assert!(lit(&t) > 0);
        let before: u32 = t.pixels.iter().map(|v| u32::from(*v)).sum();
        for _ in 0..4000 {
            t.step(&[3.0, 0.0], setup(true));
        }
        let after: u32 = t.pixels.iter().map(|v| u32::from(*v)).sum();
        assert!(after < before, "{after} < {before}");
        t.step(&[3.0, 0.0], setup(false));
        assert_eq!(lit(&t), 0);
    }
}
