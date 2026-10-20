// SPDX-License-Identifier: GPL-2.0-or-later
//! The geometry of the pin layout preview of a subcircuit: where each pin sits on the chip
//! drawn in the dialog, and which place of the chip a dragged pin lands on. It knows nothing
//! about GTK, so it is unit tested.

use crate::elements::chip::{SIDE_E, SIDE_N, SIDE_S, SIDE_W};

/// The chip of the preview: `size_x` by `size_y` pin spacings of `unit` pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// Width in pin spacings.
    pub size_x: i32,
    /// Height in pin spacings.
    pub size_y: i32,
    /// Pixels in one pin spacing.
    pub unit: f64,
}

impl Geometry {
    /// The unit that fits a chip of `size_x` by `size_y` spacings, with `margin` pixels all
    /// around it, in an area of `width` by `height` pixels (never above 40, never below 8).
    pub fn fitting(size_x: i32, size_y: i32, width: f64, height: f64, margin: f64) -> Geometry {
        let ux = (width - 2.0 * margin) / f64::from(size_x.max(1));
        let uy = (height - 2.0 * margin) / f64::from(size_y.max(1));
        Geometry { size_x, size_y, unit: ux.min(uy).clamp(8.0, 40.0) }
    }

    /// Width and height of the body in pixels.
    pub fn body(&self) -> (f64, f64) {
        (f64::from(self.size_x) * self.unit, f64::from(self.size_y) * self.unit)
    }

    /// How many places a side has.
    fn places(&self, side: usize) -> i32 {
        if side == SIDE_N || side == SIDE_S { self.size_x } else { self.size_y }
    }

    /// Where the pin at `pos` on `side` meets the body, from the body's top-left corner.
    pub fn anchor(&self, side: usize, pos: i32) -> (f64, f64) {
        let (w, h) = self.body();
        let along = (f64::from(pos) + 0.5) * self.unit;
        match side {
            SIDE_N => (along, 0.0),
            SIDE_S => (along, h),
            SIDE_W => (0.0, along),
            _ => (w, along),
        }
    }

    /// The place of the chip nearest to the point (`x`, `y`), from the body's top-left corner.
    pub fn nearest_place(&self, x: f64, y: f64) -> Option<(usize, i32)> {
        let mut best: Option<(f64, usize, i32)> = None;
        for side in [SIDE_N, SIDE_S, SIDE_W, SIDE_E] {
            for pos in 0..self.places(side) {
                let (ax, ay) = self.anchor(side, pos);
                let d = (ax - x).hypot(ay - y);
                if best.is_none_or(|(b, ..)| d < b) {
                    best = Some((d, side, pos));
                }
            }
        }
        best.map(|(_, side, pos)| (side, pos))
    }

    /// The pin of `pins` (side and place of each) closest to the point, within `radius`.
    pub fn pin_at(&self, pins: &[(usize, i32)], x: f64, y: f64, radius: f64) -> Option<usize> {
        pins.iter()
            .enumerate()
            .map(|(i, &(side, pos))| {
                let (ax, ay) = self.anchor(side, pos);
                (i, (ax - x).hypot(ay - y))
            })
            .filter(|&(_, d)| d <= radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
}

/// Whether pin `moving` of `pins` can go to (`side`, `pos`): the place must be on the chip and
/// no other pin may be there.
pub fn can_move(
    geometry: &Geometry,
    pins: &[(usize, i32)],
    moving: usize,
    side: usize,
    pos: i32,
) -> bool {
    let places = if side == SIDE_N || side == SIDE_S { geometry.size_x } else { geometry.size_y };
    pos >= 0
        && pos < places
        && !pins.iter().enumerate().any(|(i, &p)| i != moving && p == (side, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: Geometry = Geometry { size_x: 3, size_y: 1, unit: 40.0 };

    #[test]
    fn anchors_sit_on_the_edges() {
        assert_eq!(G.anchor(SIDE_N, 1), (60.0, 0.0));
        assert_eq!(G.anchor(SIDE_S, 0), (20.0, 40.0));
        assert_eq!(G.anchor(SIDE_W, 0), (0.0, 20.0));
        assert_eq!(G.anchor(SIDE_E, 0), (120.0, 20.0));
    }

    #[test]
    fn a_dragged_point_lands_on_the_nearest_place() {
        assert_eq!(G.nearest_place(-30.0, 22.0), Some((SIDE_W, 0)));
        assert_eq!(G.nearest_place(150.0, 10.0), Some((SIDE_E, 0)));
        assert_eq!(G.nearest_place(65.0, -30.0), Some((SIDE_N, 1)));
        assert_eq!(G.nearest_place(110.0, 80.0), Some((SIDE_S, 2)));
    }

    #[test]
    fn pins_are_picked_by_distance() {
        let pins = [(SIDE_W, 0), (SIDE_N, 1), (SIDE_E, 0)];
        assert_eq!(G.pin_at(&pins, 62.0, 4.0, 14.0), Some(1));
        assert_eq!(G.pin_at(&pins, 60.0, 80.0, 14.0), None);
    }

    #[test]
    fn a_pin_cannot_take_a_place_that_is_used_or_off_the_chip() {
        let pins = [(SIDE_W, 0), (SIDE_N, 1), (SIDE_E, 0)];
        assert!(can_move(&G, &pins, 1, SIDE_N, 2));
        assert!(can_move(&G, &pins, 1, SIDE_N, 1), "its own place is free for it");
        assert!(!can_move(&G, &pins, 1, SIDE_E, 0), "used by another pin");
        assert!(!can_move(&G, &pins, 0, SIDE_W, 1), "off the chip: the chip has one row");
    }

    #[test]
    fn the_unit_shrinks_for_big_chips_and_stays_readable() {
        assert_eq!(Geometry::fitting(2, 2, 400.0, 240.0, 40.0).unit, 40.0);
        assert_eq!(Geometry::fitting(20, 2, 400.0, 240.0, 40.0).unit, 16.0);
        assert_eq!(Geometry::fitting(40, 40, 400.0, 240.0, 40.0).unit, 8.0);
    }
}
