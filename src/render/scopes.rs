// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Scope drawing).
//! Drawing of the time diagrams: grid, curves and readouts of every scope.

use super::painter::{Color, Painter, TextAlign};
use crate::elements::units::{current_text, unit_text, voltage_text};
use crate::scope::{
    POINTS, PlotView, ScopeView, UNITS_A, UNITS_C, UNITS_COUNT, UNITS_OHMS, UNITS_V, UNITS_W,
    V_POSITION_STEPS, VAL_CURRENT, VAL_POWER, VAL_VOLTAGE,
};

/// What a scope remembers between frames about how it fits its data.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScaleState {
    /// Full-scale value for each units; they follow the data.
    pub values: [f64; UNITS_COUNT as usize],
    /// The signal has been negative: the zero line stays in the middle instead of going to the
    /// bottom of an automatic scale.
    pub negative: bool,
}

/// Full-scale values that follow the data, kept between frames.
#[derive(Clone, Debug, Default)]
pub struct ScopeScales {
    scales: Vec<ScaleState>,
    /// A measurement is being dragged out: (scope, simulated time where the drag began).
    pub drag_from: Option<(usize, f64)>,
    /// The plot under the pointer: (scope, plot). Kept while a plot is being dragged.
    pub hover: Option<(usize, usize)>,
    /// A plot is being dragged, so the pointer no longer chooses the plot.
    pub dragging: bool,
    /// The lines of the information area at the right of the scopes.
    pub info: Vec<String>,
}

impl ScopeScales {
    fn sync(&mut self, views: &[ScopeView]) {
        if self.scales.len() != views.len() {
            self.scales =
                views.iter().map(|v| ScaleState { values: v.scale, negative: false }).collect();
        }
    }
}

const PLOT_COLORS: [(u8, u8, u8); 8] = [
    (255, 0, 0),
    (255, 128, 0),
    (255, 0, 255),
    (127, 0, 255),
    (0, 0, 255),
    (0, 128, 255),
    (255, 255, 0),
    (0, 255, 255),
];

fn plot_color(units: i32, index: usize, same_units_before: usize, light: bool) -> Color {
    if same_units_before > 0 {
        let (r, g, b) = PLOT_COLORS[(index - 1) % 8];
        return Color::new(r, g, b);
    }
    match units {
        UNITS_V => Color::new(0, 255, 0),
        UNITS_A => {
            if light {
                Color::new(160, 160, 0)
            } else {
                Color::new(255, 255, 0)
            }
        }
        _ => {
            if light {
                Color::new(0, 0, 0)
            } else {
                Color::new(255, 255, 255)
            }
        }
    }
}

fn units_text(units: i32, v: f64) -> String {
    match units {
        UNITS_V => voltage_text(v),
        UNITS_A => current_text(v),
        UNITS_OHMS => unit_text(v, "\u{3a9}"),
        UNITS_W => unit_text(v, "W"),
        UNITS_C => unit_text(v, "C"),
        _ => unit_text(v, ""),
    }
}

fn value_name(value: i32, units: i32) -> &'static str {
    match (value, units) {
        (VAL_VOLTAGE, UNITS_V) => "V",
        (VAL_CURRENT, UNITS_A) => "I",
        (VAL_POWER, _) => "P",
        (_, UNITS_V) => "V",
        (_, UNITS_A) => "I",
        (_, UNITS_OHMS) => "R",
        (_, UNITS_W) => "P",
        _ => "",
    }
}

/// Time between vertical grid lines: the first of 1-2-5 style steps that leaves at least
/// twenty pixels between lines.
fn grid_step(seconds_per_pixel: f64) -> f64 {
    const MULT: [f64; 3] = [2.0, 2.5, 2.0];
    let mut step = 1e-15;
    let mut i = 0;
    while step < seconds_per_pixel * 20.0 {
        step *= MULT[i % 3];
        i += 1;
    }
    step
}

/// Next value of the 1-2-5 series above `s`.
fn step_up(s: f64) -> f64 {
    let m = 10f64.powf(s.log10().floor());
    let r = s / m;
    if r < 1.5 {
        2.0 * m
    } else if r < 3.5 {
        5.0 * m
    } else {
        10.0 * m
    }
}

/// Next value of the 1-2-5 series below `s`.
fn step_down(s: f64) -> f64 {
    let m = 10f64.powf(s.log10().floor());
    let r = s / m;
    if r < 1.5 {
        0.5 * m
    } else if r < 3.5 {
        m
    } else {
        2.0 * m
    }
}

/// Short name of what a plot shows (V, I, P...), for menus.
pub fn plot_label(p: &crate::scope::PlotView) -> &'static str {
    value_name(p.value, p.units)
}

/// How many of the newest samples fit the scope: one per pixel, as in the original.
fn visible(w: f64) -> usize {
    (w.floor().max(16.0) as usize).min(POINTS)
}

/// Which samples of the buffer a scope of width `w` shows.
struct Window {
    /// Samples shown (one per pixel).
    n: usize,
    /// Samples in the buffer.
    len: usize,
    /// Index of the sample at the left edge (negative when the window starts before the buffer).
    start: isize,
    /// The shown part of the buffer, `lo..hi`.
    lo: usize,
    hi: usize,
}

/// The window of `view`: the newest samples, or, once the trigger has fired, the ones around
/// the trigger sample with it in the middle.
fn window(view: &ScopeView, w: f64) -> Window {
    let n = visible(w);
    let len = view.plots.first().map_or(0, |p| p.min.len());
    let held = view.trigger.as_ref().filter(|t| t.triggered);
    let start: isize = match held {
        Some(t) => len as isize - 1 - t.back as isize - (n / 2) as isize,
        None => len as isize - n as isize,
    };
    Window {
        n,
        len,
        start,
        lo: start.clamp(0, len as isize) as usize,
        hi: (start + n as isize).clamp(0, len as isize) as usize,
    }
}

/// The value in the middle of an automatic scale and the span from there to either edge, when
/// every plot has the same units and zero may move: a signal that never goes negative gets
/// zero at the bottom, as in the original.
type Frame = Option<(f64, f64)>;

/// The frame of `view` (see [`Frame`]); `None` when the scale is manual, or the plots have
/// different units, and zero stays in the middle.
fn frame_of(view: &ScopeView, st: &ScaleState, win: &Window) -> Frame {
    if view.manual {
        return None;
    }
    let first = view.plots.first()?;
    if view.plots.iter().any(|p| p.units != first.units) {
        return None;
    }
    let gm = st.values[first.units as usize].max(1e-12);
    let (mut lowest, mut highest) = (f64::MAX, f64::MIN);
    for p in &view.plots {
        let from = win.lo.max(p.min.len().saturating_sub(p.valid));
        for k in from..win.hi.max(from) {
            lowest = lowest.min(p.min[k]);
            highest = highest.max(p.max[k]);
        }
    }
    let have = lowest <= highest;
    if view.max_scale && have && highest > lowest {
        return Some(((highest + lowest) / 2.0, (highest - lowest) / 2.0 * 1.05));
    }
    if st.negative || (have && lowest < -0.05 * gm) {
        return Some((0.0, gm));
    }
    Some((gm / 2.0, gm / 2.0))
}

/// How a scope maps values to pixels.
struct Axes<'a> {
    values: &'a [f64; UNITS_COUNT as usize],
    frame: Frame,
}

/// Vertical position of value `v` of plot `p` in a scope drawn from `y` with height `h`.
fn plot_y(view: &ScopeView, ax: &Axes, p: &PlotView, v: f64, (y, h): (f64, f64)) -> f64 {
    let mid = y + h / 2.0;
    let half = (h / 2.0 - 4.0).max(1.0);
    if view.manual {
        let gm = ((f64::from(view.man_divisions) / 2.0 + 0.05) * p.man_scale).max(1e-12);
        let offset = gm * 2.0 * f64::from(p.man_pos) / f64::from(V_POSITION_STEPS);
        (mid - ((v + offset) / gm).clamp(-1.05, 1.05) * half).clamp(y, y + h)
    } else if let Some((centre, span)) = ax.frame {
        (mid - ((v - centre) / span.max(1e-12)).clamp(-1.05, 1.05) * half).clamp(y, y + h)
    } else {
        let gm = ax.values[p.units as usize].max(1e-12);
        (mid - (v / gm).clamp(-1.05, 1.05) * half).clamp(y, y + h)
    }
}

/// The measurements a scope shows about its first plot, as lines of text.
#[derive(Debug, Default, PartialEq)]
pub struct Measures {
    /// Lines for the top left corner, in order.
    pub top: Vec<String>,
    /// The line for the bottom left corner (the smallest value).
    pub bottom: Option<String>,
}

/// The cycles of a signal as the original finds them: the span, in samples, from the first
/// rising crossing of `mid` to the last, and what `accumulate` added up over those samples
/// up to the last crossing.
fn cycles(
    samples: &[(f64, f64)],
    mid: f64,
    mut accumulate: impl FnMut(&mut f64, (f64, f64)),
) -> Option<(usize, f64)> {
    // leading empty samples do not count
    let first = samples.iter().position(|s| s.1 != 0.0)?;
    let mut state = if samples[first].1 > mid { 1 } else { -1 };
    let (mut waves, mut start, mut end) = (0, 0, 0);
    let (mut acc, mut at_end) = (0.0, 0.0);
    for (i, &(mn, mx)) in samples.iter().enumerate().skip(first) {
        let switched = if state == 1 { mx < mid } else { mn > mid };
        if switched {
            state = -state;
            if state == 1 {
                if waves == 0 {
                    start = i;
                    acc = 0.0;
                } else {
                    end = i;
                    at_end = acc;
                }
                waves += 1;
            }
        }
        if waves > 0 {
            accumulate(&mut acc, (mn, mx));
        }
    }
    // (`then`, not `then_some`: the difference must not be computed when it would go below zero)
    (end > start).then(|| (end - start, at_end))
}

/// The frequency of the plot from the spacing of its peaks, or 0 when it is not steady.
fn frequency(samples: &[(f64, f64)], time_per_sample: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let avg: f64 = samples.iter().map(|s| s.0 + s.1).sum::<f64>() / (samples.len() * 2) as f64;
    let thresh = avg * 0.05;
    let (mut state, mut oi) = (0, 0usize);
    let (mut avperiod, mut avperiod2) = (0.0, 0.0);
    let mut periodct = -1;
    for (i, s) in samples.iter().enumerate() {
        let q = s.1 - avg;
        let old = state;
        if q < thresh {
            state = 1;
        } else if q > -thresh {
            state = 2;
        }
        if state == 2 && old == 1 {
            let pd = (i - oi) as f64;
            oi = i;
            if pd < 12.0 {
                continue;
            }
            if periodct >= 0 {
                avperiod += pd;
                avperiod2 += pd * pd;
            }
            periodct += 1;
        }
    }
    if periodct < 1 {
        return 0.0;
    }
    avperiod /= f64::from(periodct);
    avperiod2 /= f64::from(periodct);
    let std = (avperiod2 - avperiod * avperiod).max(0.0).sqrt();
    if std > 2.0 { 0.0 } else { 1.0 / (avperiod * time_per_sample) }
}

/// The measurements `view`'s flags ask for, over the samples `lo..hi` of its first plot.
/// `step_x` is the time per division and `step_y` the units per division of the first plot.
fn measures(view: &ScopeView, win: &Window, step_x: f64, step_y: f64) -> Measures {
    let mut m = Measures::default();
    let flags = view.flags;
    let Some(first) = view.plots.first() else {
        m.top.extend(view.elm_info.iter().cloned());
        return m;
    };
    // the recorded part of a plot inside the window
    let recorded = |p: &PlotView| {
        let from = win.lo.max(p.min.len().saturating_sub(p.valid));
        (from..win.hi.max(from)).map(|k| (p.min[k], p.max[k])).collect::<Vec<_>>()
    };
    let samples = recorded(first);
    let (mut max, mut min) = (f64::NEG_INFINITY, f64::INFINITY);
    for p in view.plots.iter().filter(|p| p.units == first.units) {
        for (lo, hi) in recorded(p) {
            max = max.max(hi);
            min = min.min(lo);
        }
    }
    let have = max >= min;
    let text = |v: f64| units_text(first.units, v);
    if flags & crate::scope::FLAG_SCALE != 0
        && !view.manual
        && !(flags & crate::scope::FLAG_SHOW_V != 0 && flags & crate::scope::FLAG_SHOW_I != 0)
    {
        m.top.push(format!("H={}/div V={}/div", unit_text(step_x, "s"), text(step_y)));
    }
    if flags & crate::scope::FLAG_NO_MAX == 0 && have {
        m.top.push(format!("Max={}", text(max)));
    }
    if flags & crate::scope::FLAG_MIN != 0 && have {
        m.bottom = Some(format!("Min={}", text(min)));
    }
    if flags & crate::scope::FLAG_P2P != 0 && have {
        m.top.push(format!("P-P={}", text(max - min)));
    }
    let mid = (max + min) / 2.0;
    let average = |m: &mut Measures| {
        if let Some((span, sum)) = cycles(&samples, mid, |acc, (lo, hi)| *acc += (hi + lo) * 0.5) {
            m.top.push(format!("{}{}", text(sum / span as f64), crate::locale::ls(" average")));
        }
    };
    if flags & crate::scope::FLAG_RMS != 0 {
        if first.units == UNITS_V || first.units == UNITS_A {
            let r = cycles(&samples, mid, |acc, (lo, hi)| {
                let v = (hi + lo) * 0.5;
                *acc += v * v;
            });
            if let Some((span, sum)) = r {
                m.top.push(format!("{}rms", text((sum / span as f64).sqrt())));
            }
        } else {
            // the original shows the average when the RMS makes no sense
            average(&mut m);
        }
    }
    if flags & crate::scope::FLAG_AVERAGE != 0 {
        average(&mut m);
    }
    if flags & crate::scope::FLAG_DUTY != 0 {
        let r = cycles(&samples, mid, |acc, (_, hi)| {
            if hi > mid {
                *acc += 1.0;
            }
        });
        if let Some((span, on)) = r {
            m.top.push(format!(
                "{}{}%",
                crate::locale::ls("Duty cycle "),
                (100.0 * on / span as f64) as i64
            ));
        }
    }
    // the label of the scope comes among the measurements, as in the original
    if let Some(t) = view.text.as_ref().filter(|t| !t.is_empty()) {
        m.top.push(t.clone());
    }
    if flags & crate::scope::FLAG_FREQ != 0 {
        let f = frequency(&samples, view.time_per_sample);
        if f != 0.0 {
            m.top.push(unit_text(f, "Hz"));
        }
    }
    m.top.extend(phase_text(view, win));
    m.top.extend(view.elm_info.iter().cloned());
    m
}

/// Groups the scopes by their place: the scopes of one place are one column.
pub fn bands(views: &[ScopeView]) -> Vec<Vec<usize>> {
    let mut places: Vec<i32> = views.iter().map(|v| v.position).collect();
    places.sort_unstable();
    places.dedup();
    places
        .into_iter()
        .map(|p| (0..views.len()).filter(|&i| views[i].position == p).collect())
        .collect()
}

/// Where a scope is: left, top, width and height.
pub type ScopeRect = (f64, f64, f64, f64);

/// The space between two columns of scopes.
const COLUMN_GAP: f64 = 10.0;

/// Width of the information area at the right of the scopes (wider when there are few columns,
/// as in the original).
pub fn info_width(views: &[ScopeView], w: f64) -> f64 {
    let width = if bands(views).len() <= 2 { 240.0 } else { 160.0 };
    // a narrow area is all scopes
    if w < 4.0 * width { 0.0 } else { width }
}

/// The rectangle of each scope in an area of `w` by `h`, as the original lays them out: one
/// column for each place, side by side; the scopes of a place share its column, one above the
/// other.
pub fn layout(views: &[ScopeView], w: f64, h: f64) -> Vec<ScopeRect> {
    let columns = bands(views);
    let mut rects = vec![(0.0, 0.0, 0.0, 0.0); views.len()];
    let cw = (w - info_width(views, w)) / columns.len().max(1) as f64;
    for (c, column) in columns.iter().enumerate() {
        let rh = h / column.len().max(1) as f64;
        for (r, &i) in column.iter().enumerate() {
            rects[i] =
                (cw * c as f64, rh * r as f64, (cw - COLUMN_GAP).max(2.0 * COLUMN_GAP), rh - 2.0);
        }
    }
    rects
}

/// The scale line of a scope in manual mode: the time per division, then, for each plot, a
/// dot in its colour and its units per division, wrapped to the width of the scope. Returns
/// the baseline of its last line.
fn draw_manual_scale(
    g: &mut dyn Painter,
    view: &ScopeView,
    (x, y, w, h): (f64, f64, f64, f64),
    step: f64,
    (first_line, text, light): (f64, Color, bool),
) -> f64 {
    let mut ty = first_line;
    if ty + 5.0 >= y + h {
        return ty;
    }
    g.set_font_size(10);
    g.set_text_align(TextAlign::Left);
    g.set_color(text);
    let head = format!("H={}/div", unit_text(step, "s"));
    g.draw_string(&head, x + 3.0, ty);
    let mut at = g.measure_text(&head);
    const BULLET: f64 = 17.0;
    for (i, p) in view.plots.iter().enumerate() {
        let per = format!("={}/div", units_text(p.units, p.man_scale));
        let width = g.measure_text(&per);
        if at + BULLET + width > w {
            at = 0.0;
            ty += 15.0;
            if ty + 5.0 >= y + h {
                return ty - 15.0;
            }
        }
        let before = view.plots[..i].iter().filter(|q| q.units == p.units).count();
        g.set_color(plot_color(p.units, i, before, light));
        g.fill_oval((x + at) as i32 + 10, (ty - 9.0) as i32, 8, 8);
        at += BULLET;
        g.set_color(text);
        g.draw_string(&per, x + 3.0 + at, ty);
        at += width;
    }
    ty + 15.0
}

/// Whether a scope of this size shows the settings wheel in its bottom left corner.
fn shows_wheel(w: f64, h: f64) -> bool {
    w > 100.0 && h > 100.0
}

/// Whether (`px`, `py`), relative to the scope's top left corner, is on its settings wheel.
pub fn on_wheel(rect: ScopeRect, px: f64, py: f64) -> bool {
    shows_wheel(rect.2, rect.3)
        && (0.0..=36.0).contains(&px)
        && (rect.3 - 36.0..=rect.3).contains(&py)
}

/// The settings wheel: eight spokes round a small circle; it lights up under the pointer.
pub fn draw_wheel(g: &mut dyn Painter, (x, y, w, h): (f64, f64, f64, f64), lit: bool) {
    if !shows_wheel(w, h) {
        return;
    }
    g.save();
    g.set_color(if lit { Color::CYAN } else { Color::new(64, 64, 64) });
    g.set_line_width(3.0);
    let (cx, cy) = (x + 18.0, y + h - 18.0);
    g.begin_path();
    g.arc(cx, cy, 5.0, 0.0, std::f64::consts::TAU);
    g.stroke();
    let spoke = |g: &mut dyn Painter, dx: f64, dy: f64, from: f64, to: f64| {
        g.draw_line(
            (cx + dx * from) as i32,
            (cy + dy * from) as i32,
            (cx + dx * to) as i32,
            (cy + dy * to) as i32,
        );
    };
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        spoke(g, dx, dy, 5.0, 8.0);
    }
    for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        spoke(g, dx, dy, 4.0, 6.0);
    }
    g.restore();
}

/// The scope under a point of the area (the gap after a scope counts as part of it).
pub fn scope_at(views: &[ScopeView], w: f64, h: f64, x: f64, y: f64) -> Option<usize> {
    let cw = (w - info_width(views, w)) / bands(views).len().max(1) as f64;
    layout(views, w, h)
        .iter()
        .position(|&(rx, ry, _, rh)| x >= rx && x < rx + cw && y >= ry && y < ry + rh + 2.0)
}

/// How many scopes the tallest column has: the area is as many rows high.
pub fn rows(views: &[ScopeView]) -> usize {
    bands(views).iter().map(Vec::len).max().unwrap_or(0)
}

/// Draws all scopes in the area (`w` by `h`) at the origin of the painter. `cursor` is the
/// pointer position in the area, when it is over it: the scope under it shows a readout.
pub fn draw_scopes(
    g: &mut dyn Painter,
    views: &[ScopeView],
    scales: &mut ScopeScales,
    w: f64,
    h: f64,
    light_background: bool,
    cursor: Option<(f64, f64)>,
) {
    scales.sync(views);
    if views.is_empty() || w < 8.0 || h < 8.0 {
        return;
    }
    // the area is a little lighter than the scopes, which shows the gaps between them
    g.set_color(if light_background { Color::new(235, 235, 235) } else { Color::new(16, 16, 16) });
    g.fill_rect(0.0, 0.0, w, h);
    let rects = layout(views, w, h);
    let under = cursor.and_then(|(cx, cy)| scope_at(views, w, h, cx, cy));
    // the plot under the pointer: the one whose curve is closest at the pointer's column
    if !scales.dragging {
        scales.hover = None;
        if let (Some((cx, cy)), Some(i)) = (cursor, under) {
            let (rx, ry, rw, rh) = rects[i];
            let (view, win) = (&views[i], window(&views[i], rw));
            let j = (((cx - rx) / rw * win.n as f64).max(0.0) as usize).min(win.n - 1);
            let k = win.start + j as isize;
            if let Some(st) = scales.scales.get(i) {
                let ax = Axes { values: &st.values, frame: frame_of(view, st, &win) };
                let mut best = f64::INFINITY;
                for (pi, p) in view.plots.iter().enumerate() {
                    let recorded = k >= 0
                        && (k as usize) < win.len
                        && (k as usize) >= p.min.len().saturating_sub(p.valid);
                    if !recorded {
                        continue;
                    }
                    let d = (cy - plot_y(view, &ax, p, p.max[k as usize], (ry, rh))).abs();
                    if d < best {
                        best = d;
                        scales.hover = Some((i, pi));
                    }
                }
            }
        }
    }
    let hover = scales.hover;
    for (i, &(rx, ry, rw, rh)) in rects.iter().enumerate() {
        let measure = scales.drag_from.filter(|d| d.0 == i).map(|d| d.1);
        let Some(scale) = scales.scales.get_mut(i) else { continue };
        let selected = hover.filter(|h| h.0 == i).map(|h| h.1);
        let wheel_lit = cursor
            .filter(|_| under == Some(i))
            .is_some_and(|(cx, cy)| on_wheel(rects[i], cx - rx, cy - ry));
        draw_scope(
            g,
            &views[i],
            scale,
            &ScopeSpot {
                x: rx,
                y: ry,
                w: rw,
                h: rh,
                slot: 0,
                cursor: cursor.filter(|_| under == Some(i)).map(|c| c.0),
                cursor_y: cursor.filter(|_| under == Some(i)).map(|c| c.1 - ry),
                selected,
                measure_from: measure,
            },
            light_background,
        );
        draw_wheel(g, rects[i], wheel_lit);
    }
    // the information area: what the pointer is over, in a column to the right
    let iw = info_width(views, w);
    if iw > 0.0 {
        g.set_color(if light_background { Color::new(0, 0, 0) } else { Color::new(255, 255, 255) });
        g.set_font_size(12);
        g.set_text_align(TextAlign::Left);
        let mut ty = 20.0;
        for line in &scales.info {
            if ty > h - 4.0 {
                break;
            }
            g.draw_string(line, w - iw + 12.0, ty);
            ty += 15.0;
        }
    }
}

/// The simulated time at pixel `x` of a scope of width `w`, for a scope that draws its plots
/// against time; `None` for one that shows a spectrum or an X/Y plot, or has no plots.
pub fn time_at(view: &ScopeView, w: f64, x: f64) -> Option<f64> {
    if view.fft || view.xy || view.plots.is_empty() || w < 8.0 {
        return None;
    }
    let win = window(view, w);
    if win.len == 0 {
        return None;
    }
    let j = ((x / w * win.n as f64).max(0.0) as usize).min(win.n - 1);
    let at = (win.start + j as isize).clamp(0, win.len as isize - 1);
    Some(view.time - (win.len as isize - 1 - at) as f64 * view.time_per_sample)
}

/// Where a scope is drawn.
pub struct ScopeSpot {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
    /// Position among the scopes sharing the place (0 draws the background and the grid).
    pub slot: usize,
    /// Pointer position along the width, when it is over the scope.
    pub cursor: Option<f64>,
    /// Pointer position down the height (from the top of this scope), when it is over it.
    pub cursor_y: Option<f64>,
    /// The plot under the pointer, drawn heavier.
    pub selected: Option<usize>,
    /// Simulated time where a drag to measure a stretch of the signal began.
    pub measure_from: Option<f64>,
}

/// Draws one scope in the rectangle (`x`, `y`, `w`, `h`); `scale` is its full-scale values,
/// which follow the data.
pub fn draw_scope(
    g: &mut dyn Painter,
    view: &ScopeView,
    state: &mut ScaleState,
    spot: &ScopeSpot,
    light: bool,
) {
    // nothing is drawn outside the rectangle of the scope
    g.save();
    g.clip_rect(spot.x, spot.y, spot.w, spot.h);
    draw_scope_inside(g, view, state, spot, light);
    g.restore();
}

fn draw_scope_inside(
    g: &mut dyn Painter,
    view: &ScopeView,
    state: &mut ScaleState,
    spot: &ScopeSpot,
    light: bool,
) {
    let ScopeSpot { x, y, w, h, slot, cursor, cursor_y, selected, measure_from } = *spot;
    let (bg, grid, text) = if light {
        (Color::new(255, 255, 255), Color::new(200, 200, 200), Color::new(0, 0, 0))
    } else {
        (Color::new(0, 0, 0), Color::new(64, 64, 64), Color::new(192, 192, 192))
    };
    if slot == 0 {
        g.set_color(bg);
        g.fill_rect(x, y, w, h);
    }
    // a small scope placed in the circuit has room for the curves only
    let compact = w < 160.0 || h < 90.0;
    let mid = y + h / 2.0;
    let half = (h / 2.0 - 4.0).max(1.0);
    let win = window(view, w);
    let (n, len, start, lo, hi) = (win.n, win.len, win.start, win.lo, win.hi);

    // follow the data on the 1-2-5 series: step up when it does not fit, down when it fits the
    // step below
    for u in 0..UNITS_COUNT {
        let peak = view
            .plots
            .iter()
            .filter(|p| p.units == u)
            .flat_map(|p| p.min[lo..hi].iter().chain(&p.max[lo..hi]))
            .fold(0.0_f64, |m, v| m.max(v.abs()));
        let s = &mut state.values[u as usize];
        if view.plots.iter().any(|p| p.units == u) {
            while peak > *s && *s < 1e30 {
                *s = step_up(*s);
            }
            while !view.max_scale && *s > 1e-4 && peak < step_down(*s) * 0.95 {
                *s = step_down(*s);
            }
        }
    }

    let frame = frame_of(view, state, &win);
    if let Some((centre, _)) = frame {
        // once the signal has gone below zero, zero stays in the middle
        if centre == 0.0 && !view.max_scale {
            state.negative = true;
        }
    }
    let scale = &state.values;
    let axes = Axes { values: scale, frame };
    if slot == 0 && view.fft && !view.plots.is_empty() {
        draw_spectrum(g, view, (x, y, w, h), (grid, text), compact, cursor);
        return;
    }
    if slot == 0 && view.xy && view.plots.len() >= 2 {
        let pointer = cursor.zip(cursor_y);
        draw_xy(g, view, (x, y, w, h), (grid, text, light), pointer);
        return;
    }
    let px_time = view.time_per_sample * n as f64 / w;
    let step = grid_step(px_time);
    let dx = w / n as f64;
    // simulated time of sample `k` (which may lie outside the buffer)
    let time_of = |k: isize| view.time - (len as isize - 1 - k) as f64 * view.time_per_sample;
    let t_left = time_of(start);
    // how far the text in the left corner reaches: the readouts keep clear of it
    let mut left_w = 0.0_f64;
    // the grid is drawn once for the scopes that share a place
    if slot == 0 {
        // vertical grid: time
        g.set_line_width(1.0);
        g.set_color(grid);
        let mut k = (t_left.max(0.0) / step).ceil();
        loop {
            let gx = x + (k * step - t_left) / px_time;
            if gx > x + w {
                break;
            }
            if gx >= x {
                g.draw_line(gx as i32, y as i32, gx as i32, (y + h) as i32);
            }
            k += 1.0;
        }

        // horizontal grid, when every plot has the same units
        let same_units = view.plots.windows(2).all(|p| p[0].units == p[1].units);
        if view.manual {
            // one line per division; each plot names its own scale
            let divs = view.man_divisions.max(1);
            let top = f64::from(divs) / 2.0 + 0.05;
            g.set_color(grid);
            for q in -(divs / 2)..=(divs / 2) {
                let gy = mid - f64::from(q) / top * half;
                g.draw_line(x as i32, gy as i32, (x + w) as i32, gy as i32);
            }
        } else if let (true, Some(first)) = (same_units, view.plots.first()) {
            let gm = scale[first.units as usize];
            let (centre, span) = frame.unwrap_or((0.0, gm));
            // a line every quarter of the full scale, wherever zero has gone
            for q in -4i32..=4 {
                let gy = plot_y(view, &axes, first, f64::from(q) * gm / 4.0, (y, h));
                if gy <= y || gy >= y + h - 1.0 {
                    continue;
                }
                g.set_color(if q == 0 { text } else { grid });
                g.draw_line(x as i32, gy as i32, (x + w) as i32, gy as i32);
            }
            if !compact {
                g.set_color(text);
                g.set_font_size(10);
                for (s, ly) in [
                    (units_text(first.units, centre + span), y + 12.0),
                    (units_text(first.units, centre - span), y + h - 3.0),
                ] {
                    left_w = left_w.max(g.measure_text(&s));
                    g.draw_string(&s, x + 3.0, ly);
                }
            }
        } else {
            g.set_color(grid);
            g.draw_line(x as i32, mid as i32, (x + w) as i32, mid as i32);
            // each units has its own full scale: name them in the corner
            g.set_font_size(10);
            let mut ly = y + 12.0;
            let mut shown: Vec<i32> = Vec::new();
            for (i, p) in view.plots.iter().enumerate() {
                if shown.contains(&p.units) {
                    continue;
                }
                shown.push(p.units);
                if compact {
                    continue;
                }
                let before = view.plots[..i].iter().filter(|q| q.units == p.units).count();
                g.set_color(plot_color(p.units, i, before, light));
                let label = format!("\u{b1}{}", units_text(p.units, scale[p.units as usize]));
                left_w = left_w.max(g.measure_text(&label));
                g.draw_string(&label, x + 3.0, ly);
                ly += 12.0;
            }
        }
    }

    // curves
    let manual_gm = |p: &PlotView| (f64::from(view.man_divisions) / 2.0 + 0.05) * p.man_scale;
    let place = |p: &PlotView, v: f64| plot_y(view, &axes, p, v, (y, h));
    // the plot under the pointer is drawn last, on top
    let mut order: Vec<usize> = (0..view.plots.len()).filter(|&i| Some(i) != selected).collect();
    order.extend(selected.filter(|&i| i < view.plots.len()));
    for i in order {
        let p = &view.plots[i];
        let before = view.plots[..i].iter().filter(|q| q.units == p.units).count();
        g.set_color(plot_color(p.units, i, before, light));
        g.set_line_width(if Some(i) == selected { 3.0 } else { 1.0 });
        let flat =
            if view.manual { manual_gm(p) } else { scale[p.units as usize].max(1e-12) } * 1e-6;
        g.begin_path();
        // only what was recorded is drawn: after the history is emptied the left stays blank
        let first_valid = lo.max(p.min.len().saturating_sub(p.valid));
        let mut began = false;
        for k in first_valid..hi {
            let (lov, hiv) = (p.min[k], p.max[k]);
            let cx = x + ((k as isize - start) as f64 + 0.5) * dx;
            if began {
                g.line_to(cx, place(p, hiv));
            } else {
                g.move_to(cx, place(p, hiv));
                began = true;
            }
            // the span of the sample is a vertical stroke of its own; the line goes on from
            // the top of it, as in the original, instead of zigzagging from top to bottom
            // (which overshoots wherever the signal is steep, and shows when the line is thick)
            if (hiv - lov).abs() > flat {
                g.line_to(cx, place(p, lov));
                g.move_to(cx, place(p, hiv));
            }
        }
        g.stroke();
    }
    if let (Some(t), Some(p)) = (&view.trigger, view.plots.first()) {
        draw_trigger(g, t, place(p, t.level), (x, y, w, h));
    }

    if compact {
        return;
    }

    // measurements, under the corner labels
    if slot == 0 {
        let labels = if view.manual {
            0
        } else if view.plots.windows(2).all(|p| p[0].units == p[1].units) {
            // the full scale is written in the top corner, one line
            1
        } else {
            let mut u: Vec<i32> = view.plots.iter().map(|p| p.units).collect();
            u.sort_unstable();
            u.dedup();
            u.len()
        };
        let first_units = view.plots.first().map_or(UNITS_V, |p| p.units);
        let step_y = scale[first_units as usize] / 4.0;
        let m = measures(view, &win, step, step_y);
        g.set_color(text);
        g.set_font_size(10);
        g.set_text_align(TextAlign::Left);
        let mut ty = y + 14.0 + 12.0 * labels as f64;
        if view.manual && view.flags & crate::scope::FLAG_SCALE != 0 {
            ty = draw_manual_scale(g, view, (x, y, w, h), step, (ty, text, light));
            g.set_color(text);
            g.set_font_size(10);
            g.set_text_align(TextAlign::Left);
        }
        for line in &m.top {
            if ty + 5.0 >= y + h {
                break;
            }
            left_w = left_w.max(g.measure_text(line));
            g.draw_string(line, x + 3.0, ty);
            ty += 15.0;
        }
        if let Some(b) = &m.bottom {
            // the lowest value of the full scale is written in the bottom corner, when the
            // plots share it
            let shared = !view.manual && view.plots.windows(2).all(|p| p[0].units == p[1].units);
            left_w = left_w.max(g.measure_text(b));
            g.draw_string(b, x + 3.0, y + h - if shared { 17.0 } else { 5.0 });
        }
    }

    // readouts
    g.set_font_size(11);
    let mut ty = y + 14.0 + 26.0 * slot as f64;
    g.set_text_align(TextAlign::Left);
    let lines: Vec<String> = view
        .plots
        .iter()
        .map(|p| {
            let last = p.max.last().copied().unwrap_or(0.0);
            format!("{} {} = {}", p.title, value_name(p.value, p.units), units_text(p.units, last))
        })
        .collect();
    let widest = lines.iter().map(|l| g.measure_text(l)).fold(0.0, f64::max);
    // centred on the scope, unless that runs into the text in the corner: then to the right of
    // it, and left out when there is no room there either
    let (from, to) = if (w - widest) / 2.0 >= left_w + 4.0 {
        (x, x + w)
    } else {
        (x + left_w + 6.0, x + w - 4.0)
    };
    let fits = widest <= to - from;
    for (i, (p, line)) in view.plots.iter().zip(&lines).enumerate() {
        if !fits {
            break;
        }
        let before = view.plots[..i].iter().filter(|q| q.units == p.units).count();
        g.set_color(plot_color(p.units, i, before, light));
        g.draw_string(line, (from + to) / 2.0 - g.measure_text(line) / 2.0, ty);
        ty += 13.0;
    }
    if slot == 0 {
        g.set_color(text);
        g.set_font_size(10);
        let span = unit_text(step, "s");
        let label = format!("{span}/div");
        g.draw_string(&label, x + w - g.measure_text(&label) - 4.0, y + h - 3.0);
    }
    if let Some(cx) = cursor {
        draw_cursor(g, view, (x, y, w, h), (cx, start, n), slot, (grid, text, light));
        if slot == 0 {
            if let Some(t0) = measure_from {
                draw_measure(g, view, (x, y, w, h), (cx, start, n), (t0, selected), text);
            }
        }
    }
}

/// The newest `count` samples of a plot, as the mean of the extremes of each (which keeps the
/// fast wiggle inside a sample out of the spectrum).
fn means(p: &PlotView, count: usize) -> Vec<f64> {
    let from = p.min.len().saturating_sub(count);
    p.min[from..].iter().zip(&p.max[from..]).map(|(a, b)| 0.5 * (a + b)).collect()
}

/// The largest power of two that is at most `n` (at least 16).
fn pow2_floor(n: usize) -> usize {
    if n < 16 { 16 } else { 1 << n.ilog2() }
}

/// The frequency spectrum of the first plot, with the frequency of its tallest peak; in
/// decibels below the peak when the scope asks for a logarithmic spectrum. `cursor` is the
/// pointer position along the width, when it is over the scope.
fn draw_spectrum(
    g: &mut dyn Painter,
    view: &ScopeView,
    (x, y, w, h): (f64, f64, f64, f64),
    (grid, text): (Color, Color),
    compact: bool,
    cursor: Option<f64>,
) {
    let p = &view.plots[0];
    // the newest samples that make a power of two, as many as the scope shows
    let samples = means(p, pow2_floor(visible(w)));
    let spec = crate::scope::fft::spectrum(&samples);
    let max = spec.iter().copied().fold(1e-8, f64::max);
    let log = view.flags & crate::scope::FLAG_LOG_SPECTRUM != 0;
    let fmax = 0.5 / view.time_per_sample;
    g.set_line_width(1.0);
    g.set_font_size(10);
    let divs = if compact { 4 } else { 10 };
    for i in 0..=divs {
        let gx = x + w * f64::from(i) / f64::from(divs);
        g.set_color(grid);
        g.draw_line(gx as i32, y as i32, gx as i32, (y + h) as i32);
        if !compact && i < divs {
            g.set_color(Color::new(255, 0, 0));
            g.draw_string(
                &unit_text(fmax * f64::from(i) / f64::from(divs), "Hz"),
                gx + 2.0,
                y + h - 3.0,
            );
        }
    }
    let top = y + 14.0;
    let base = y + h - 14.0;
    const DB_RANGE: f64 = 80.0;
    if log && !compact {
        for db in [20.0, 40.0, 60.0, 80.0] {
            let gy = top + (base - top) * db / DB_RANGE;
            g.set_color(grid);
            g.draw_line(x as i32, gy as i32, (x + w) as i32, gy as i32);
            g.set_color(Color::new(255, 0, 0));
            g.draw_string(&format!("-{db:.0} dB"), x + 2.0, gy - 2.0);
        }
    }
    g.set_color(Color::new(255, 0, 0));
    g.begin_path();
    for (k, m) in spec.iter().enumerate() {
        let gx = x + w * k as f64 / spec.len() as f64;
        let level = if log {
            -crate::scope::fft::decibels(*m, max, DB_RANGE) / DB_RANGE
        } else {
            1.0 - m / max
        };
        let gy = top + (base - top) * level;
        if k == 0 {
            g.move_to(gx, gy);
        } else {
            g.line_to(gx, gy);
        }
    }
    g.stroke();
    if !compact {
        let peak = (1..spec.len()).max_by(|&a, &b| spec[a].total_cmp(&spec[b])).unwrap_or(0);
        g.set_color(text);
        g.set_font_size(11);
        let label = format!(
            "{} {}: {} @ {}",
            p.title,
            value_name(p.value, p.units),
            units_text(p.units, spec[peak]),
            unit_text(fmax * peak as f64 / spec.len() as f64, "Hz")
        );
        g.draw_string(&label, x + 6.0, y + 12.0);
        if let Some(cx) = cursor.filter(|c| *c >= x && *c < x + w) {
            let bin = (((cx - x) / w * spec.len() as f64) as usize).min(spec.len() - 1);
            let db = crate::scope::fft::decibels(spec[bin], max, 1000.0);
            let lines = [unit_text(fmax * (cx - x) / w, "Hz"), format!("{} dB", db.round())];
            g.set_color(text);
            g.draw_line(cx as i32, y as i32, cx as i32, (y + h) as i32);
            let on_right = cx < x + w / 2.0;
            for (i, line) in lines.iter().enumerate() {
                let tx = if on_right { cx + 6.0 } else { cx - 6.0 - g.measure_text(line) };
                g.draw_string(line, tx, y + 38.0 + 13.0 * i as f64);
            }
        }
    }
}

/// The phase angle between a voltage and a current plot, as a line of text; nothing unless the
/// scope plots exactly one of each and has been asked to show it.
fn phase_text(view: &ScopeView, win: &Window) -> Option<String> {
    if view.flags & crate::scope::FLAG_PHASE_ANGLE == 0 {
        return None;
    }
    let mut volts = None;
    let mut amps = None;
    for p in &view.plots {
        let slot = match p.units {
            UNITS_V => &mut volts,
            UNITS_A => &mut amps,
            _ => return None,
        };
        if slot.replace(p).is_some() {
            return None;
        }
    }
    let (v, i) = (volts?, amps?);
    let count = pow2_floor(win.n.min(v.valid));
    let angle = crate::scope::fft::phase_angle(&means(v, count), &means(i, count))?;
    Some(format!("{}{angle:.1}\u{b0}", crate::locale::ls("Phase angle: ")))
}

/// The picture of an X/Y plot: its cells as small squares, and a dot where the trace is now.
/// In automatic mode the picture fills the scope; with a manual scale it is a square, as the
/// divisions are.
fn draw_trace(
    g: &mut dyn Painter,
    tr: &crate::scope::trace::Trace,
    manual: bool,
    (x, y, w, h): (f64, f64, f64, f64),
) {
    use crate::scope::trace::{TRACE_H, TRACE_W};
    let (ox, oy, sw, sh) = if manual {
        let m = w.min(h);
        (x + (w - m) / 2.0, y + (h - m) / 2.0, m, m)
    } else {
        (x, y, w, h)
    };
    let (cw, ch) = (sw / TRACE_W as f64, sh / TRACE_H as f64);
    for row in 0..TRACE_H {
        let mut col = 0;
        while col < TRACE_W {
            let at = (row * TRACE_W + col) * 3;
            let c = [tr.pixels[at], tr.pixels[at + 1], tr.pixels[at + 2]];
            if c == [0, 0, 0] {
                col += 1;
                continue;
            }
            // a run of cells of one colour is one rectangle
            let mut end = col + 1;
            while end < TRACE_W && tr.pixels[(row * TRACE_W + end) * 3..][..3] == c {
                end += 1;
            }
            g.set_color(Color::new(c[0], c[1], c[2]));
            // a little over the cell, so that neighbours leave no seams
            g.fill_rect(
                ox + col as f64 * cw,
                oy + row as f64 * ch,
                (end - col) as f64 * cw + 0.5,
                ch + 0.5,
            );
            col = end;
        }
    }
    if let Some((u, v)) = tr.head() {
        g.set_color(Color::WHITE);
        g.fill_oval((ox + u * sw) as i32 - 2, (oy + v * sh) as i32 - 2, 5, 5);
    }
}

/// The first plot against the second, as a curve through the samples; plots named for it
/// modulate the brightness and the colour of the trace. `pointer` is the pointer position in
/// the scope, when it is over it.
fn draw_xy(
    g: &mut dyn Painter,
    view: &ScopeView,
    (x, y, w, h): (f64, f64, f64, f64),
    (_, text, light): (Color, Color, bool),
    pointer: Option<(f64, f64)>,
) {
    use crate::scope::XyAxes;
    let count = view.plots.len();
    let (px, py) = (XyAxes::valid(view.axes.x, 0, count), XyAxes::valid(view.axes.y, 1, count));
    let (plot_x, plot_y) = (&view.plots[px], &view.plots[py]);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let len = plot_x.min.len();
    let from = len.saturating_sub(visible(w));
    // with a trail, only the samples that have not faded away yet
    let steps = f64::from(view.speed.max(1));
    let trail = f64::from(view.axes.trail.max(0));
    let from = if trail > 0.0 {
        from.max(len.saturating_sub((4.0 * trail / steps).ceil() as usize + 1))
    } else {
        from
    };
    let mean = |p: &PlotView, k: usize| 0.5 * (p.min[k] + p.max[k]);
    // the scale of an automatic axis: it doubles to fit the signal, as in the original
    let fit = |p: &PlotView, start: f64| {
        let peak = (from..len).fold(0.0_f64, |m, k| m.max(mean(p, k).abs()));
        let mut s = start;
        while peak > s && s < 1e30 {
            s *= 2.0;
        }
        while s > 1e-9 && peak < s / 4.0 {
            s /= 2.0;
        }
        s
    };
    let (sx, sy) = match &view.trace {
        Some(t) => (t.scale_x, t.scale_y),
        None => (fit(plot_x, 5.0), fit(plot_y, 0.1)),
    };
    let grid_px = (w.min(h) / 2.0) / (f64::from(view.man_divisions) / 2.0 + 0.05);
    let steps_v = f64::from(V_POSITION_STEPS);
    let divs = f64::from(view.man_divisions);
    // pixel position of a pair of values
    let place = |vx: f64, vy: f64| -> (f64, f64) {
        if view.manual {
            (
                x + w * 0.499
                    + vx / plot_x.man_scale.max(1e-12) * grid_px
                    + grid_px * divs * f64::from(plot_x.man_pos) / steps_v,
                y + h * 0.499
                    - vy / plot_y.man_scale.max(1e-12) * grid_px
                    - grid_px * divs * f64::from(plot_y.man_pos) / steps_v,
            )
        } else {
            (
                x + w * 0.499 * (1.0 + (vx / sx).clamp(-1.05, 1.05)),
                y + h * 0.499 * (1.0 - (vy / sy).clamp(-1.05, 1.05)),
            )
        }
    };
    g.set_line_width(1.0);
    if view.manual {
        g.set_color(Color::new(64, 64, 64));
        let d = view.man_divisions;
        for i in -d..=d {
            let off = grid_px * f64::from(i);
            if i != 0 {
                g.draw_line((cx + off) as i32, y as i32, (cx + off) as i32, (y + h) as i32);
            }
            g.draw_line(x as i32, (cy + off) as i32, (x + w) as i32, (cy + off) as i32);
        }
    }
    // the axes: a plot of voltage against current has its vertical axis in yellow
    // green axes, the vertical one yellow when it is voltage against current
    g.set_color(Color::new(0, 255, 0));
    g.draw_line(x as i32, cy as i32, (x + w) as i32, cy as i32);
    if !view.plot_xy {
        g.set_color(Color::new(255, 255, 0));
    }
    g.draw_line(cx as i32, y as i32, cx as i32, (y + h) as i32);

    if let Some(tr) = &view.trace {
        draw_trace(g, tr, view.manual, (x, y, w, h));
    } else {
        // modulators: brightness and colour follow other plots
        let modulate = |index: i32| XyAxes::modulator(index, count);
        let channel = |i: Option<usize>| {
            i.map(|i| {
                let p = &view.plots[i];
                let peak = (from..len).fold(0.0_f64, |m, k| m.max(mean(p, k)));
                let mut s = 5.0;
                while peak > s && s < 1e30 {
                    s *= 2.0;
                }
                (p, s)
            })
        };
        let brightness = channel(modulate(view.axes.brightness));
        let (red, green, blue) = (
            channel(modulate(view.axes.red)),
            channel(modulate(view.axes.green)),
            channel(modulate(view.axes.blue)),
        );
        let colored = red.is_some() || green.is_some() || blue.is_some();
        let base = if colored {
            None
        } else {
            Some(if light { Color::new(0, 0, 0) } else { Color::new(255, 255, 255) })
        };
        let part = |c: Option<(&PlotView, f64)>, k: usize| {
            c.map_or(0, |(p, s)| ((mean(p, k) / s) * 255.0).clamp(0.0, 255.0) as u8)
        };
        let plain = brightness.is_none() && !colored && trail <= 0.0;
        let mut last = place(
            mean(plot_x, from.min(len.saturating_sub(1))),
            mean(plot_y, from.min(len.saturating_sub(1))),
        );
        if len > 0 && plain {
            g.set_color(base.unwrap_or(Color::WHITE));
            g.begin_path();
            g.move_to(last.0, last.1);
            for k in from + 1..len {
                last = place(mean(plot_x, k), mean(plot_y, k));
                g.line_to(last.0, last.1);
            }
            g.stroke();
        } else if len > 0 {
            for k in from + 1..len {
                let next = place(mean(plot_x, k), mean(plot_y, k));
                let mut alpha = brightness.map_or(1.0, |(p, s)| (mean(p, k).abs() / s).min(1.0));
                if trail > 0.0 {
                    alpha *= (-((len - 1 - k) as f64) * steps / trail).exp();
                }
                g.set_color(
                    base.unwrap_or_else(|| Color::new(part(red, k), part(green, k), part(blue, k))),
                );
                g.set_alpha(alpha.clamp(0.0, 1.0));
                g.begin_path();
                g.move_to(last.0, last.1);
                g.line_to(next.0, next.1);
                g.stroke();
                last = next;
            }
            g.set_alpha(1.0);
        }
        if len > 0 {
            g.set_color(Color::WHITE);
            g.fill_oval(last.0 as i32 - 2, last.1 as i32 - 2, 5, 5);
        }
    }
    if h > 60.0 {
        g.set_color(text);
        g.set_font_size(10);
        let (xs, ys) = if view.manual { (plot_x.man_scale, plot_y.man_scale) } else { (sx, sy) };
        let per = if view.manual { "/div" } else { "" };
        let sign = if view.manual { "" } else { "\u{b1}" };
        g.draw_string(
            &format!(
                "X: {} {sign}{}{per}",
                value_name(plot_x.value, plot_x.units),
                units_text(plot_x.units, xs)
            ),
            x + 4.0,
            y + 12.0,
        );
        g.draw_string(
            &format!(
                "Y: {} {sign}{}{per}",
                value_name(plot_y.value, plot_y.units),
                units_text(plot_y.units, ys)
            ),
            x + 4.0,
            y + 24.0,
        );
        if let Some(t) = &view.text {
            g.draw_string(t, x + 4.0, y + h - 6.0);
        }
        // the values under the pointer
        if let Some((mx, my)) = pointer {
            let (vx, vy) = if view.manual {
                (
                    plot_x.man_scale
                        * ((mx - w * 0.499) / grid_px - divs * f64::from(plot_x.man_pos) / steps_v),
                    plot_y.man_scale
                        * ((h * 0.499 - my) / grid_px - divs * f64::from(plot_y.man_pos) / steps_v),
                )
            } else {
                ((mx - x) / (0.499 * w) * sx - sx, -((my - y) / (0.499 * h) * sy - sy))
            };
            let lines = [units_text(plot_x.units, vx), units_text(plot_y.units, vy)];
            g.set_color(text);
            g.draw_line((x + mx) as i32, y as i32, (x + mx) as i32, (y + h) as i32);
            g.draw_line(x as i32, (y + my) as i32, (x + w) as i32, (y + my) as i32);
            for (i, line) in lines.iter().enumerate() {
                g.draw_string(line, x + mx + 6.0, y + my - 4.0 - 13.0 * (1 - i) as f64);
            }
        }
    }
}

/// The stretch being measured by dragging: a line where the drag began, and the time and the
/// change in the value of the plot (the one under the pointer, else the first) since then.
fn draw_measure(
    g: &mut dyn Painter,
    view: &ScopeView,
    (x, y, w, h): (f64, f64, f64, f64),
    (cx, start, n): (f64, isize, usize),
    (from, selected): (f64, Option<usize>),
    text: Color,
) {
    let Some(p) = view.plots.get(selected.unwrap_or(0)).or(view.plots.first()) else { return };
    let len = p.min.len();
    if len == 0 || view.time_per_sample <= 0.0 {
        return;
    }
    let at = |t: f64| len as f64 - 1.0 - (view.time - t) / view.time_per_sample;
    let j0 = at(from) - start as f64;
    if j0 < 0.0 || j0 >= n as f64 {
        return;
    }
    let gx0 = x + (j0 + 0.5) * (w / n as f64);
    g.set_line_width(1.0);
    g.set_color(Color::new(192, 192, 192));
    g.draw_line(gx0 as i32, y as i32, gx0 as i32, (y + h) as i32);
    let value = |k: f64| {
        let k = (k.round().max(0.0) as usize).min(len - 1);
        0.5 * (p.min[k] + p.max[k])
    };
    let j1 = (((cx - x) / w * n as f64) as usize).min(n - 1);
    let k1 = (start + j1 as isize).clamp(0, len as isize - 1) as f64;
    let t1 = view.time - (len as f64 - 1.0 - k1) * view.time_per_sample;
    let lines = [
        format!("\u{394}t={}", crate::elements::units::time_text((t1 - from).abs())),
        format!("\u{394}={}", units_text(p.units, value(k1) - value(at(from)))),
    ];
    g.set_color(text);
    g.set_font_size(11);
    let gx = x + (j1 as f64 + 0.5) * (w / n as f64);
    let on_right = gx < x + w / 2.0;
    for (i, line) in lines.iter().enumerate() {
        let tx = if on_right { gx + 6.0 } else { gx - 6.0 - g.measure_text(line) };
        g.draw_string(line, tx, y + h - 32.0 - 13.0 * (1 - i) as f64);
    }
}

/// The dashed line at the trigger level and the state of the trigger.
fn draw_trigger(
    g: &mut dyn Painter,
    t: &crate::scope::TriggerView,
    level_y: f64,
    (x, y, w, h): (f64, f64, f64, f64),
) {
    let orange = Color::new(255, 128, 0);
    g.set_color(orange);
    g.set_line_width(1.0);
    if level_y > y && level_y < y + h {
        let mut gx = x;
        while gx < x + w {
            g.draw_line(
                gx as i32,
                level_y as i32,
                (gx + 4.0).min(x + w - 1.0) as i32,
                level_y as i32,
            );
            gx += 8.0;
        }
        g.set_font_size(10);
        let edge =
            if t.edge == crate::scope::TRIGGER_EDGE_RISING { "T\u{2191}" } else { "T\u{2193}" };
        g.draw_string(edge, x + w - 25.0, level_y - 3.0);
    }
    let status = match t.state {
        crate::scope::TriggerState::Armed if t.waiting => "WAIT",
        crate::scope::TriggerState::Armed => "ARMED",
        crate::scope::TriggerState::Triggered => "TRIG",
        crate::scope::TriggerState::AutoRun => "AUTO",
    };
    g.set_font_size(10);
    g.draw_string(status, x + w - g.measure_text(status) - 5.0, y + h - 15.0);
}

/// The vertical line under the pointer and the values of every plot at that time.
fn draw_cursor(
    g: &mut dyn Painter,
    view: &ScopeView,
    (x, y, w, h): (f64, f64, f64, f64),
    (cx, start, n): (f64, isize, usize),
    slot: usize,
    (_, text, light): (Color, Color, bool),
) {
    let j = (((cx - x) / w * n as f64) as usize).min(n - 1);
    let gx = x + (j as f64 + 0.5) * (w / n as f64);
    let len = view.plots.first().map_or(0, |p| p.min.len());
    // the sample under the pointer, in the buffer
    let at = (start + j as isize).clamp(0, len as isize - 1) as usize;
    g.set_line_width(1.0);
    g.set_color(text);
    if slot == 0 {
        g.draw_line(gx as i32, y as i32, gx as i32, (y + h) as i32);
    }
    // time of the sample, counted back from the newest
    let when = view.time - (len - 1 - at) as f64 * view.time_per_sample;
    g.set_font_size(11);
    g.set_text_align(TextAlign::Left);
    let mut ty = y + 14.0 + 26.0 * slot as f64;
    let on_right = gx < x + w / 2.0;
    for (i, p) in view.plots.iter().enumerate() {
        let before = view.plots[..i].iter().filter(|q| q.units == p.units).count();
        g.set_color(plot_color(p.units, i, before, light));
        let v = (p.min[at] + p.max[at]) / 2.0;
        let line = format!("{} = {}", value_name(p.value, p.units), units_text(p.units, v));
        let tx = if on_right { gx + 6.0 } else { gx - 6.0 - g.measure_text(&line) };
        ty += 13.0;
        g.draw_string(&line, tx, ty + 26.0);
    }
    if slot == 0 {
        g.set_color(text);
        let label = if when < 0.0 {
            "t < 0".to_string()
        } else {
            format!("t = {}", crate::elements::units::time_text(when))
        };
        let tx = if on_right { gx + 6.0 } else { gx - 6.0 - g.measure_text(&label) };
        g.draw_string(&label, tx, y + h - 18.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::svg::SvgPainter;
    use crate::scope::PlotView;

    fn view(values: &[f64]) -> ScopeView {
        let mut max = vec![0.0; POINTS];
        let n = values.len().min(POINTS);
        max[POINTS - n..].copy_from_slice(&values[..n]);
        ScopeView {
            trace: None,
            position: 0,
            text: None,
            scale: [0.5; UNITS_COUNT as usize],
            time_per_sample: 1e-4,
            time: 0.05,
            plots: vec![PlotView {
                units: UNITS_V,
                value: 0,
                title: "R1".into(),
                min: max.clone(),
                max,
                valid: POINTS,
                man_scale: 0.5,
                man_pos: 0,
            }],
            xy: false,
            plot_xy: false,
            axes: crate::scope::XyAxes::default(),
            speed: 1,
            fft: false,
            // the original's default flags: current and voltage plots, peak value shown
            flags: crate::scope::FLAG_NO_MAX,
            elm_info: Vec::new(),
            manual: false,
            man_divisions: 8,
            max_scale: false,
            trigger: None,
        }
    }

    #[test]
    fn full_scale_follows_the_data() {
        let mut g = SvgPainter::new(400.0, 200.0);
        let mut scales = ScopeScales::default();
        draw_scopes(&mut g, &[view(&[3.0; 40])], &mut scales, 400.0, 200.0, false, None);
        assert!(
            scales.scales[0].values[0] >= 3.0 && scales.scales[0].values[0] < 8.0,
            "{:?}",
            scales.scales[0].values
        );
        let svg = g.finish();
        assert!(svg.contains("R1 V ="), "{svg}");
    }

    #[test]
    fn full_scale_steps_follow_the_one_two_five_series() {
        let mut v = 0.5;
        let mut up = Vec::new();
        for _ in 0..5 {
            v = step_up(v);
            up.push(v);
        }
        assert_eq!(up, [1.0, 2.0, 5.0, 10.0, 20.0]);
        let mut v = 5.0;
        let mut down = Vec::new();
        for _ in 0..4 {
            v = step_down(v);
            down.push(v);
        }
        assert_eq!(down, [2.0, 1.0, 0.5, 0.2]);
    }

    #[test]
    fn scopes_sharing_a_position_are_drawn_in_one_band() {
        let mut a = view(&[1.0; 10]);
        let mut b = view(&[2.0; 10]);
        let mut c = view(&[3.0; 10]);
        a.position = 0;
        b.position = 0;
        c.position = 1;
        assert_eq!(bands(&[a, b, c]), vec![vec![0, 1], vec![2]]);
    }

    #[test]
    fn manual_mode_names_the_scale_of_each_plot_when_asked() {
        let mut v = view(&[3.0; 40]);
        v.manual = true;
        v.plots[0].man_scale = 2.0;
        let draw = |v: &ScopeView| {
            let mut g = SvgPainter::new(400.0, 200.0);
            draw_scopes(
                &mut g,
                std::slice::from_ref(v),
                &mut ScopeScales::default(),
                400.0,
                200.0,
                false,
                None,
            );
            g.finish()
        };
        // the scale line is a choice of the scope ("Show Scale"), as in the original
        assert!(!draw(&v).contains("V/div"));
        v.flags |= crate::scope::FLAG_SCALE;
        let svg = draw(&v);
        assert!(svg.contains("H=") && svg.contains("=2 V/div"), "{svg}");
        // the label of the scope is among the measurements, not a line of its own
        v.text = Some("my label".into());
        assert!(draw(&v).contains("my label"));
    }

    #[test]
    fn the_trigger_shows_its_level_and_state() {
        use crate::scope::{TRIGGER_EDGE_RISING, TRIGGER_NORMAL, TriggerState, TriggerView};
        let mut v = view(&[3.0; 40]);
        v.trigger = Some(TriggerView {
            mode: TRIGGER_NORMAL,
            edge: TRIGGER_EDGE_RISING,
            level: 1.0,
            state: TriggerState::Armed,
            waiting: true,
            triggered: false,
            back: 0,
            time: 0.0,
        });
        let mut g = SvgPainter::new(400.0, 200.0);
        let mut scales = ScopeScales::default();
        draw_scopes(&mut g, &[v.clone()], &mut scales, 400.0, 200.0, false, None);
        let svg = g.finish();
        assert!(svg.contains("WAIT"), "{svg}");
        // once triggered the display is held around the trigger sample: the newest samples
        // are not at the right edge any more
        if let Some(t) = &mut v.trigger {
            t.triggered = true;
            t.state = TriggerState::Triggered;
            t.back = 10;
        }
        let mut g = SvgPainter::new(400.0, 200.0);
        draw_scopes(&mut g, &[v], &mut scales, 400.0, 200.0, false, None);
        assert!(g.finish().contains("TRIG"));
    }

    /// A view of one voltage plot made of `values` (the newest samples) with the given flags.
    fn signal(values: &[f64], flags: i32) -> ScopeView {
        let mut v = view(&[]);
        let mut data = vec![0.0; POINTS];
        let n = values.len().min(POINTS);
        data[POINTS - n..].copy_from_slice(&values[..n]);
        v.plots[0].min = data.clone();
        v.plots[0].max = data;
        v.plots[0].valid = n;
        v.flags = flags;
        v
    }

    fn lines(v: &ScopeView) -> Measures {
        measures(v, &window(v, 600.0), 1e-3, 1.0)
    }

    #[test]
    fn a_sine_has_its_rms_average_peaks_and_frequency_measured() {
        use crate::scope::{FLAG_AVERAGE, FLAG_FREQ, FLAG_MIN, FLAG_P2P, FLAG_RMS};
        // 5 V amplitude, 100 samples a period, 5 periods, 1e-4 s per sample: 100 Hz
        let sine: Vec<f64> =
            (0..500).map(|i| 5.0 * (std::f64::consts::TAU * i as f64 / 100.0).sin()).collect();
        let flags = FLAG_RMS | FLAG_AVERAGE | FLAG_FREQ | FLAG_MIN | FLAG_P2P;
        let m = lines(&signal(&sine, flags));
        let text = m.top.join(" | ");
        assert!(text.contains("Max=5"), "{text}");
        assert!(text.contains("P-P=10"), "{text}");
        let rms = m.top.iter().find(|l| l.ends_with("rms")).expect("rms");
        let value: f64 = rms.trim_end_matches("rms").trim_end_matches(" V").trim().parse().unwrap();
        assert!((value - 5.0 / 2f64.sqrt()).abs() < 0.1, "{rms}");
        assert!(text.contains("100 Hz") || text.contains("99.") || text.contains("100."), "{text}");
        assert!(m.bottom.as_deref().is_some_and(|b| b.starts_with("Min=-5")), "{:?}", m.bottom);
    }

    #[test]
    fn a_square_wave_has_its_duty_cycle_measured() {
        use crate::scope::FLAG_DUTY;
        let square: Vec<f64> = (0..600).map(|i| if i % 100 < 25 { 5.0 } else { 0.0 }).collect();
        let m = lines(&signal(&square, FLAG_DUTY));
        let line = m.top.iter().find(|l| l.contains('%')).expect("duty cycle");
        let pct: i64 = line.rsplit(' ').next().unwrap().trim_end_matches('%').parse().unwrap();
        assert!((24..=26).contains(&pct), "{line}");
    }

    #[test]
    fn nothing_is_measured_before_anything_is_recorded() {
        use crate::scope::FLAG_RMS;
        let m = lines(&signal(&[], FLAG_RMS | crate::scope::FLAG_NO_MAX));
        assert_eq!(m, Measures::default());
    }

    #[test]
    fn the_plot_nearest_to_the_pointer_is_the_selected_one() {
        let mut v = view(&[]);
        v.plots[0] = signal(&[1.0; 600], 0).plots.remove(0);
        let mut second = v.plots[0].clone();
        second.min = vec![-1.0; POINTS];
        second.max = vec![-1.0; POINTS];
        v.plots.push(second);
        let mut g = SvgPainter::new(600.0, 200.0);
        let mut scales = ScopeScales::default();
        // 1 V is above the middle of the scope and -1 V below it
        draw_scopes(&mut g, &[v.clone()], &mut scales, 600.0, 200.0, false, Some((300.0, 10.0)));
        assert_eq!(scales.hover, Some((0, 0)));
        draw_scopes(&mut g, &[v.clone()], &mut scales, 600.0, 200.0, false, Some((300.0, 190.0)));
        assert_eq!(scales.hover, Some((0, 1)));
        // dragging keeps the plot whatever the pointer does
        scales.dragging = true;
        draw_scopes(&mut g, &[v], &mut scales, 600.0, 200.0, false, Some((300.0, 10.0)));
        assert_eq!(scales.hover, Some((0, 1)));
    }

    #[test]
    fn grid_step_keeps_lines_apart() {
        let s = grid_step(1e-6);
        assert!((2e-5..1e-4).contains(&s), "{s}");
    }

    fn sine(cycles: f64, lag: f64, amp: f64) -> Vec<f64> {
        (0..256)
            .map(|i| amp * (std::f64::consts::TAU * cycles * i as f64 / 256.0 - lag).sin())
            .collect()
    }

    fn two_plots(v: &[f64], a: &[f64]) -> ScopeView {
        let mut view = signal(v, crate::scope::FLAG_NO_MAX);
        let mut cur = view.plots[0].clone();
        let mut data = vec![0.0; POINTS];
        data[POINTS - a.len()..].copy_from_slice(a);
        cur.min = data.clone();
        cur.max = data;
        cur.valid = a.len();
        cur.units = UNITS_A;
        cur.value = crate::scope::VAL_CURRENT;
        view.plots.push(cur);
        view
    }

    #[test]
    fn zero_sits_at_the_bottom_until_the_signal_goes_negative() {
        let positive = signal(&[1.0, 2.0, 3.0, 2.0, 1.0], 0);
        let st = ScaleState { values: [4.0; UNITS_COUNT as usize], negative: false };
        let win = window(&positive, 400.0);
        assert_eq!(frame_of(&positive, &st, &win), Some((2.0, 2.0)));
        let mut both = signal(&[1.0, -2.0, 3.0], 0);
        both.max_scale = false;
        assert_eq!(frame_of(&both, &st, &window(&both, 400.0)), Some((0.0, 4.0)));
        // and it stays there once it has been negative
        let mut g = SvgPainter::new(400.0, 200.0);
        let mut scales = ScopeScales::default();
        draw_scopes(&mut g, &[both], &mut scales, 400.0, 200.0, false, None);
        assert!(scales.scales[0].negative);
    }

    #[test]
    fn manual_scale_and_different_units_keep_zero_in_the_middle() {
        let mut v = signal(&[1.0, 2.0], 0);
        let st = ScaleState { values: [4.0; UNITS_COUNT as usize], negative: false };
        v.manual = true;
        assert_eq!(frame_of(&v, &st, &window(&v, 400.0)), None);
        let w = two_plots(&[1.0, 2.0], &[0.1, 0.2]);
        assert_eq!(frame_of(&w, &st, &window(&w, 400.0)), None);
    }

    #[test]
    fn the_phase_angle_is_shown_for_one_voltage_and_one_current() {
        let mut v = two_plots(&sine(8.0, 0.0, 1.0), &sine(8.0, 60f64.to_radians(), 0.5));
        v.flags |= crate::scope::FLAG_PHASE_ANGLE;
        let text = phase_text(&v, &window(&v, 400.0)).unwrap();
        assert!(text.contains("60.0"), "{text}");
        v.flags &= !crate::scope::FLAG_PHASE_ANGLE;
        assert_eq!(phase_text(&v, &window(&v, 400.0)), None);
        // a second voltage plot makes the question meaningless
        let mut twice = two_plots(&sine(8.0, 0.0, 1.0), &sine(8.0, 0.0, 1.0));
        twice.plots.push(twice.plots[0].clone());
        twice.flags |= crate::scope::FLAG_PHASE_ANGLE;
        assert_eq!(phase_text(&twice, &window(&twice, 400.0)), None);
    }

    #[test]
    fn the_spectrum_can_be_in_decibels_and_reads_the_pointer() {
        let mut v = signal(&sine(8.0, 0.0, 1.0), 0);
        v.fft = true;
        let draw = |v: &ScopeView, cursor| {
            let mut g = SvgPainter::new(400.0, 200.0);
            draw_scopes(
                &mut g,
                std::slice::from_ref(v),
                &mut ScopeScales::default(),
                400.0,
                200.0,
                false,
                cursor,
            );
            g.finish()
        };
        assert!(!draw(&v, None).contains("-20 dB"));
        v.flags |= crate::scope::FLAG_LOG_SPECTRUM;
        assert!(draw(&v, None).contains("-20 dB"));
        let with_pointer = draw(&v, Some((100.0, 50.0)));
        assert!(with_pointer.contains(" dB") && with_pointer.contains("Hz"), "{with_pointer}");
    }

    #[test]
    fn an_xy_plot_follows_its_axes_modulators_and_trail() {
        let mut v = two_plots(&sine(4.0, 0.0, 2.0), &sine(4.0, 1.0, 0.05));
        v.xy = true;
        v.plot_xy = true;
        let draw = |v: &ScopeView| {
            let mut g = SvgPainter::new(400.0, 300.0);
            draw_scopes(
                &mut g,
                std::slice::from_ref(v),
                &mut ScopeScales::default(),
                400.0,
                300.0,
                false,
                Some((200.0, 100.0)),
            );
            g.finish()
        };
        let plain = draw(&v);
        assert!(plain.contains("X: V") && plain.contains("Y: I"), "{plain}");
        // swapping the axes swaps the labels
        v.axes.x = 1;
        v.axes.y = 0;
        let swapped = draw(&v);
        assert!(swapped.contains("X: I") && swapped.contains("Y: V"), "{swapped}");
        v.axes = crate::scope::XyAxes { brightness: 1, red: 0, trail: 20, ..v.axes };
        v.manual = true;
        assert!(draw(&v).contains("/div"));
        // axes that point at no plot fall back to the first two
        v.manual = false;
        v.axes.x = 17;
        v.axes.y = -3;
        assert!(draw(&v).contains("X: V"));
    }

    #[test]
    fn dragging_across_the_signal_reads_the_time_and_the_change() {
        let v = signal(&sine(8.0, 0.0, 1.0), 0);
        let from = time_at(&v, 600.0, 300.0).unwrap();
        let mut scales = ScopeScales { drag_from: Some((0, from)), ..Default::default() };
        let mut g = SvgPainter::new(600.0, 200.0);
        draw_scopes(
            &mut g,
            std::slice::from_ref(&v),
            &mut scales,
            600.0,
            200.0,
            false,
            Some((450.0, 100.0)),
        );
        let svg = g.finish();
        assert!(svg.contains("\u{394}t=") && svg.contains("\u{394}="), "{svg}");
        // 150 pixels of a window of 600 samples, 100 microseconds each
        let to = time_at(&v, 600.0, 450.0).unwrap();
        assert!(((to - from) - 150.0 * 1e-4).abs() < 2e-4, "{}", to - from);
        // a spectrum or an X/Y plot has no time axis to measure on
        let mut spec = v;
        spec.fft = true;
        assert_eq!(time_at(&spec, 600.0, 10.0), None);
    }

    /// The text of one corner of a scope must not be drawn over the text under it.
    #[test]
    fn the_scale_and_the_measurements_do_not_share_a_line() {
        use crate::scope::{FLAG_MIN, FLAG_NO_MAX};
        let mut v = signal(&[1.0, -2.0, 3.0, -1.0], FLAG_MIN);
        v.flags &= !FLAG_NO_MAX; // the peak value is shown
        let mut g = SvgPainter::new(400.0, 200.0);
        draw_scopes(&mut g, &[v], &mut ScopeScales::default(), 400.0, 200.0, false, None);
        let svg = g.finish();
        // the y of every text that starts at the left edge
        let mut ys: Vec<f64> = svg
            .split("<text ")
            .skip(1)
            .filter(|t| t.contains("x=\"3\""))
            .filter_map(|t| t.split("y=\"").nth(1)?.split('"').next()?.parse().ok())
            .collect();
        assert!(ys.len() >= 4, "{svg}");
        ys.sort_by(f64::total_cmp);
        for pair in ys.windows(2) {
            assert!(pair[1] - pair[0] >= 10.0, "texts at {} and {} overlap", pair[0], pair[1]);
        }
    }

    #[test]
    fn scopes_of_different_places_are_columns_and_those_of_one_place_are_rows() {
        let (mut a, mut b, mut c) = (view(&[1.0]), view(&[1.0]), view(&[1.0]));
        (a.position, b.position, c.position) = (0, 1, 1);
        let views = [a, b, c];
        let r = layout(&views, 600.0, 260.0);
        // two columns of 300, the second holding two scopes of 130
        assert_eq!((r[0].0, r[0].1, r[0].3), (0.0, 0.0, 258.0));
        assert_eq!((r[1].0, r[1].1), (300.0, 0.0));
        assert_eq!((r[2].0, r[2].1), (300.0, 130.0));
        assert!(r[0].2 < 300.0, "a gap between the columns");
        assert_eq!(rows(&views), 2);
        assert_eq!(scope_at(&views, 600.0, 260.0, 10.0, 200.0), Some(0));
        assert_eq!(scope_at(&views, 600.0, 260.0, 400.0, 10.0), Some(1));
        assert_eq!(scope_at(&views, 600.0, 260.0, 400.0, 200.0), Some(2));
        assert_eq!(scope_at(&views, 600.0, 260.0, 700.0, 10.0), None);
    }

    #[test]
    fn a_signal_with_a_single_rising_edge_has_no_cycle_and_no_overflow() {
        // one crossing: the start is found, the end never is (a debug build used to panic
        // computing `end - start` before testing it)
        let samples = [(-1.0, -1.0), (-1.0, -1.0), (1.0, 1.0), (1.0, 1.0)];
        assert_eq!(cycles(&samples, 0.0, |a, (lo, hi)| *a += lo + hi), None);
        // and through the whole drawing, with the measurements that use it
        let v = signal(
            &[-1.0, -1.0, 1.0, 1.0],
            crate::scope::FLAG_RMS | crate::scope::FLAG_AVERAGE | crate::scope::FLAG_DUTY,
        );
        let mut g = SvgPainter::new(400.0, 200.0);
        draw_scopes(&mut g, &[v], &mut ScopeScales::default(), 400.0, 200.0, false, None);
    }
}
