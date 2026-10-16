// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp.
//! Grid geometry helpers: points, rectangles and interpolation along element axes.

/// A point on the circuit grid. `z` is used by elements that need a third coordinate to tell
/// apart posts at the same position (for example bus bits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: i32,
    /// Vertical coordinate.
    pub y: i32,
    /// Extra coordinate used to tell apart coincident posts.
    pub z: i32,
}

impl Point {
    /// Creates a point with `z = 0`.
    pub const fn new(x: i32, y: i32) -> Self {
        Point { x, y, z: 0 }
    }

    /// Moves the point by the given offsets.
    pub fn move_by(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }
}

/// An axis-aligned integer rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
}

impl Rect {
    /// Creates a rectangle.
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Rect { x, y, width, height }
    }

    /// Replaces all four fields.
    pub fn set_bounds(&mut self, x: i32, y: i32, width: i32, height: i32) {
        *self = Rect { x, y, width, height };
    }

    /// Moves the rectangle by the given offsets.
    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    /// True when the point lies inside the rectangle (right and bottom edges exclusive).
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.width && py < self.y + self.height
    }

    /// True when the two rectangles overlap.
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.width
            && o.x < self.x + self.width
            && self.y < o.y + o.height
            && o.y < self.y + self.height
    }

    /// Grows the rectangle so that it also contains `o`.
    pub fn union_with(&mut self, o: &Rect) {
        let x1 = self.x.min(o.x);
        let y1 = self.y.min(o.y);
        let x2 = (self.x + self.width).max(o.x + o.width);
        let y2 = (self.y + self.height).max(o.y + o.height);
        self.set_bounds(x1, y1, x2 - x1, y2 - y1);
    }
}

/// A closed or open sequence of points.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Polygon {
    /// The vertices, in drawing order.
    pub points: Vec<Point>,
}

impl Polygon {
    /// Creates an empty polygon.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a polygon from a slice of points.
    pub fn from_points(points: &[Point]) -> Self {
        Polygon { points: points.to_vec() }
    }

    /// Appends a vertex.
    pub fn add_point(&mut self, x: i32, y: i32) {
        self.points.push(Point::new(x, y));
    }
}

/// Point a fraction `f` of the way from `a` to `b` (rounded as the original does).
pub fn interp_point(a: Point, b: Point, f: f64) -> Point {
    Point::new(
        (f64::from(a.x) * (1.0 - f) + f64::from(b.x) * f + 0.48).floor() as i32,
        (f64::from(a.y) * (1.0 - f) + f64::from(b.y) * f + 0.48).floor() as i32,
    )
}

/// Point a fraction `f` along `a`-`b`, offset perpendicular to the line by `g` pixels.
pub fn interp_point_g(a: Point, b: Point, f: f64, g: f64) -> Point {
    let gx = f64::from(b.y - a.y);
    let gy = f64::from(a.x - b.x);
    let g = g / (gx * gx + gy * gy).sqrt();
    Point::new(
        (f64::from(a.x) * (1.0 - f) + f64::from(b.x) * f + g * gx + 0.48).floor() as i32,
        (f64::from(a.y) * (1.0 - f) + f64::from(b.y) * f + g * gy + 0.48).floor() as i32,
    )
}

/// Two points a fraction `f` along `a`-`b`, offset perpendicular by `+g` and `-g`.
pub fn interp_point2(a: Point, b: Point, f: f64, g: f64) -> (Point, Point) {
    let gx = f64::from(b.y - a.y);
    let gy = f64::from(a.x - b.x);
    let g = g / (gx * gx + gy * gy).sqrt();
    let bx = f64::from(a.x) * (1.0 - f) + f64::from(b.x) * f;
    let by = f64::from(a.y) * (1.0 - f) + f64::from(b.y) * f;
    (
        Point::new((bx + g * gx + 0.48).floor() as i32, (by + g * gy + 0.48).floor() as i32),
        Point::new((bx - g * gx + 0.48).floor() as i32, (by - g * gy + 0.48).floor() as i32),
    )
}

/// Euclidean distance between two points.
pub fn distance(p1: Point, p2: Point) -> f64 {
    let x = f64::from(p1.x - p2.x);
    let y = f64::from(p1.y - p2.y);
    (x * x + y * y).sqrt()
}

/// Squared distance between two points.
pub fn distance_sq(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    dx * dx + dy * dy
}

/// Sign of an integer: -1, 0 or 1.
pub fn sign(x: i32) -> i32 {
    x.signum()
}

/// Triangle (arrow head) with its tip at `b`, of length `al` and half-width `aw`.
pub fn calc_arrow(a: Point, b: Point, al: f64, aw: f64) -> Polygon {
    let adx = f64::from(b.x - a.x);
    let ady = f64::from(b.y - a.y);
    let l = (adx * adx + ady * ady).sqrt();
    let (p1, p2) = interp_point2(a, b, 1.0 - al / l, aw);
    Polygon::from_points(&[b, p1, p2])
}

static GRID_SIZE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(16);

/// Grid spacing in circuit pixels (16, or 8 with the small grid).
pub fn grid_size() -> i32 {
    GRID_SIZE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Sets the grid spacing used by elements that lay themselves out on the grid.
pub fn set_grid_size(size: i32) {
    GRID_SIZE.store(size, std::sync::atomic::Ordering::Relaxed);
}

/// Rounds `v` to the grid the way the original does.
pub fn snap_grid(v: i32) -> i32 {
    let g = grid_size();
    (v + g / 2 - 1) & !(g - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_midpoint_with_original_rounding() {
        let p = interp_point(Point::new(0, 0), Point::new(10, 0), 0.5);
        assert_eq!(p, Point::new(5, 0));
        // 0.48 bias rounds 2.52 down and 2.53 up
        assert_eq!(interp_point(Point::new(0, 0), Point::new(5, 0), 0.5).x, 2);
    }

    #[test]
    fn perpendicular_offset_is_orthogonal_to_the_axis() {
        let p = interp_point_g(Point::new(0, 0), Point::new(10, 0), 0.5, 4.0);
        assert_eq!(p, Point::new(5, -4));
        let (q, r) = interp_point2(Point::new(0, 0), Point::new(0, 10), 0.5, 3.0);
        assert_eq!((q.y, r.y), (5, 5));
        assert_eq!((q.x - r.x).abs(), 6);
    }

    #[test]
    fn rectangles_intersect_and_contain() {
        let a = Rect::new(0, 0, 10, 10);
        assert!(a.contains(0, 0) && !a.contains(10, 10));
        assert!(a.intersects(&Rect::new(9, 9, 5, 5)));
        assert!(!a.intersects(&Rect::new(10, 0, 5, 5)));
    }
}
