// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The scale labels of a scope sit on a translucent plate in the scope's background colour, the
//! line with the element name and the newest value is gone, and the readout of the pointer is
//! written in a box above the scope, as in the original, not over the curves.

use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;
use prometheuscs::render::painter::Painter;
use prometheuscs::render::scopes::{ScopeScales, draw_readout, draw_scopes, layout};
use prometheuscs::render::svg::SvgPainter;
use prometheuscs::scope::ScopeView;
use std::time::Duration;

const W: f64 = 900.0;
const H: f64 = 400.0;

fn views() -> Vec<ScopeView> {
    let text = std::fs::read_to_string("assets/circuits/lrc.txt").unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    for _ in 0..600 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    s.scope_views()
}

fn svg(views: &[ScopeView], light: bool, cursor: Option<(f64, f64)>) -> String {
    let mut g = SvgPainter::new(W, H);
    let mut scales = ScopeScales::default();
    draw_scopes(&mut g, views, &mut scales, W, H, light, cursor);
    g.finish()
}

/// The plates: translucent fills of the given colour, as rectangles (x0, y0, x1, y1).
fn plates(svg: &str, fill: &str) -> Vec<(f64, f64, f64, f64)> {
    fills(svg, fill)
        .into_iter()
        .filter(|(l, _)| l.contains("opacity=\"0.75\""))
        .map(|f| f.1)
        .collect()
}

/// A rectangle (x0, y0, x1, y1).
type Rect = (f64, f64, f64, f64);

/// Every fill of the given colour: its line, and its rectangle.
fn fills<'a>(svg: &'a str, fill: &str) -> Vec<(&'a str, Rect)> {
    svg.lines()
        .filter(|l| l.starts_with("<path") && l.contains(fill))
        .map(|l| {
            let d = l.split("d=\"").nth(1).unwrap().split('"').next().unwrap();
            let n: Vec<f64> = d
                .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
                .filter(|t| !t.is_empty())
                .map(|t| t.parse().unwrap())
                .collect();
            let xs: Vec<f64> = n.iter().step_by(2).copied().collect();
            let ys: Vec<f64> = n.iter().skip(1).step_by(2).copied().collect();
            let (x0, x1) = (
                xs.iter().cloned().fold(f64::MAX, f64::min),
                xs.iter().cloned().fold(f64::MIN, f64::max),
            );
            let (y0, y1) = (
                ys.iter().cloned().fold(f64::MAX, f64::min),
                ys.iter().cloned().fold(f64::MIN, f64::max),
            );
            (l, (x0, y0, x1, y1))
        })
        .collect()
}

/// The plates that are there because of the pointer: those of the drawing with it, less those of
/// the same drawing without it (the scale labels have plates of their own).
fn only_in(with: &str, without: &str, fill: &str) -> Vec<(f64, f64, f64, f64)> {
    let base = plates(without, fill);
    plates(with, fill).into_iter().filter(|p| !base.contains(p)).collect()
}

fn first_rect(views: &[ScopeView]) -> (f64, f64, f64, f64) {
    layout(views, W, H)[0]
}

/// How far down the strip of the scopes is drawn in [`readout`]: room for the box of the top row.
const TOP: f64 = 100.0;

/// The readout of the pointer drawn as the window does: the strip `TOP` below the top.
fn readout(views: &[ScopeView], light: bool, cursor: Option<(f64, f64)>) -> String {
    let mut g = SvgPainter::new(W, H + TOP);
    g.translate(0.0, TOP);
    draw_readout(&mut g, views, W, H, light, cursor);
    g.finish()
}

/// The box of the readout, in the coordinates of the strip.
fn readout_box(svg: &str, fill: &str) -> (f64, f64, f64, f64) {
    let found = fills(svg, fill);
    assert_eq!(found.len(), 1, "one box: {svg}");
    let (x0, y0, x1, y1) = found[0].1;
    (x0, y0 - TOP, x1, y1 - TOP)
}

/// The texts of a drawing, with the point where they start.
fn texts(svg: &str) -> Vec<(String, f64, f64)> {
    svg.lines()
        .filter(|l| l.starts_with("<text"))
        .map(|l| {
            let attr = |name: &str| -> f64 {
                l.split(&format!(" {name}=\""))
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap()
            };
            let text = l.split('>').nth(1).unwrap().split('<').next().unwrap().to_string();
            (text, attr("x"), attr("y"))
        })
        .collect()
}

// ---- the readout of the pointer: above the scope, not over the curves

#[test]
fn the_scope_under_the_pointer_writes_no_readout_over_its_curves() {
    let v = views();
    let (x, y, w, h) = first_rect(&v);
    for light in [false, true] {
        let with = svg(&v, light, Some((x + 0.3 * w, y + 0.4 * h)));
        assert!(!with.contains(">V = ") && !with.contains(">I = "), "no values in the scope");
        assert!(!with.contains(">t = ") && !with.contains(">t < 0"), "no time in the scope");
        // and no plates for them: only those of the scale labels
        let fill = if light { "fill=\"#ffffff\"" } else { "fill=\"#000000\"" };
        assert_eq!(plates(&with, fill), plates(&svg(&v, light, None), fill), "light={light}");
    }
}

#[test]
fn the_readout_is_a_box_on_top_of_the_scope_centred_on_the_cursor() {
    let v = views();
    let (x, y, w, h) = first_rect(&v);
    let lines = v[0].plots.len() + 1; // a value line per plot, and the time
    for (light, fill) in [(false, "fill=\"#000000\""), (true, "fill=\"#ffffff\"")] {
        let cx = x + 0.4 * w;
        let out = readout(&v, light, Some((cx, y + 0.5 * h)));
        let (x0, y0, x1, y1) = readout_box(&out, fill);
        assert!((y1 - y).abs() < 0.5, "the box ends on the top edge {y}: {y1}");
        assert!((y1 - y0 - 15.0 * lines as f64).abs() < 0.5, "a line of 15 px per value");
        // the cursor line is at the middle of its pixel column
        let col = w / v[0].plots[0].min.len().min(w as usize).max(1) as f64;
        assert!(((x0 + x1) / 2.0 - cx).abs() <= col + 1.0, "centred on {cx}: {x0}..{x1}");
        // every text is in the box, above the scope
        let found = texts(&out);
        assert_eq!(found.len(), lines, "{found:?}");
        for (text, tx, ty) in found {
            let ty = ty - TOP;
            assert!(tx >= x0 && tx < x1 && ty > y0 && ty <= y1, "{text:?} at ({tx}, {ty})");
        }
    }
}

#[test]
fn the_readout_keeps_the_colours_of_the_plots() {
    let v = views();
    let (x, y, w, h) = first_rect(&v);
    let out = readout(&v, false, Some((x + 0.8 * w, y + 0.4 * h)));
    let time = out.lines().find(|l| l.contains(">t = ")).expect("time label");
    assert!(time.contains("fill=\"#c0c0c0\""), "{time}");
    let value = out.lines().find(|l| l.contains(">V = ")).expect("value label");
    assert!(value.contains("fill=\"#00ff00\""), "the voltage trace is green: {value}");
}

#[test]
fn the_readout_stays_inside_the_width_of_the_area() {
    let v = views();
    let rects = layout(&v, W, H);
    for &(x, y, w, h) in &rects {
        for fx in [0.0, 0.01, 0.99] {
            let out = readout(&v, false, Some((x + fx * w, y + 0.5 * h)));
            let (x0, _, x1, _) = readout_box(&out, "fill=\"#000000\"");
            assert!(x0 >= -0.5 && x1 <= W + 0.5, "box {x0}..{x1} at {fx} of {x}");
        }
    }
}

#[test]
fn a_readout_of_a_lower_scope_is_on_its_top_edge() {
    let v = views();
    let rects = layout(&v, W, H);
    let i = (0..v.len()).max_by(|&a, &b| rects[a].1.partial_cmp(&rects[b].1).unwrap()).unwrap();
    let (x, y, w, h) = rects[i];
    let out = readout(&v, false, Some((x + 0.5 * w, y + 0.5 * h)));
    let (x0, _, x1, y1) = readout_box(&out, "fill=\"#000000\"");
    assert!((y1 - y).abs() < 0.5, "on the top edge {y} of scope {i}: {y1}");
    assert!(x0 < x + 0.5 * w && x1 > x + 0.5 * w - 2.0);
}

#[test]
fn there_is_no_readout_without_the_pointer_or_outside_the_scopes() {
    let v = views();
    assert!(texts(&readout(&v, false, None)).is_empty());
    assert!(fills(&readout(&v, false, None), "fill=").is_empty());
    assert!(texts(&readout(&v, false, Some((W + 50.0, H + 50.0)))).is_empty());
}

// ---- US2: no latest-value line

#[test]
fn no_scope_draws_the_line_with_the_element_name_and_the_newest_value() {
    let v = views();
    for light in [false, true] {
        let out = svg(&v, light, None);
        for view in &v {
            for p in &view.plots {
                let head = format!(">{} ", p.title);
                assert!(!out.contains(&head), "found the latest-value line of {}", p.title);
            }
        }
        // what stays: the time per division
        assert!(out.contains("/div"), "the time per division stays");
    }
}

#[test]
fn the_values_are_still_shown_with_the_pointer_and_in_the_information_area() {
    let v = views();
    let (x, y, w, h) = first_rect(&v);
    let mut g = SvgPainter::new(W, H);
    let mut scales = ScopeScales::default();
    scales.info = vec!["resistor".to_string(), "I = 1 mA".to_string()];
    draw_scopes(&mut g, &v, &mut scales, W, H, false, Some((x + 0.8 * w, y + 0.4 * h)));
    let out = g.finish();
    assert!(out.contains(">I = 1 mA<"), "the information area");
    let r = readout(&v, false, Some((x + 0.8 * w, y + 0.4 * h)));
    assert!(r.contains(">V = ") || r.contains(">I = "), "cursor labels");
    assert!(r.contains(">t = "), "time label");
}

/// The plates of one scope drawn on its own, as a scope placed in the circuit is.
fn placed(view: &ScopeView, (w, h): (f64, f64), cursor: Option<(f64, f64)>, light: bool) -> String {
    use prometheuscs::render::scopes::{ScaleState, ScopeSpot, draw_scope};
    let mut g = SvgPainter::new(w + 40.0, h + 40.0);
    let mut state = ScaleState { values: view.scale, negative: false };
    let spot = ScopeSpot {
        x: 20.0,
        y: 20.0,
        w,
        h,
        slot: 0,
        cursor: cursor.map(|c| 20.0 + c.0),
        cursor_y: cursor.map(|c| c.1),
        selected: None,
        measure_from: None,
        glow: false,
    };
    draw_scope(&mut g, view, &mut state, &spot, light);
    g.finish()
}

#[test]
fn a_scope_placed_in_the_circuit_adds_no_plates_for_the_pointer() {
    let v = views();
    for (w, h) in [(420.0, 200.0), (200.0, 160.0), (128.0, 64.0), (60.0, 40.0)] {
        for fx in [0.05, 0.5, 0.95] {
            for (light, fill) in [(false, "fill=\"#000000\""), (true, "fill=\"#ffffff\"")] {
                let out = placed(&v[0], (w, h), Some((fx * w, 0.5 * h)), light);
                let found = only_in(&out, &placed(&v[0], (w, h), None, light), fill);
                assert!(found.is_empty(), "{w}x{h} at {fx}, light={light}");
            }
        }
    }
}

#[test]
fn a_long_label_keeps_its_whole_plate_by_moving_the_text() {
    // a narrow scope: the label is wider than half of it, and the plate is moved inside
    let v = views();
    let out = placed(&v[0], (90.0, 120.0), Some((45.0, 60.0)), false);
    for (x0, _, x1, _) in plates(&out, "fill=\"#000000\"") {
        assert!(x0 >= 19.5 && x1 <= 110.5, "plate {x0}..{x1} in a scope 20..110");
    }
}

// ---- the labels of a measurement (dragging over the curve) have plates too

#[test]
fn the_labels_of_a_measurement_are_on_plates_inside_the_scope() {
    let v = views();
    let (x, y, w, h) = first_rect(&v);
    let from = v[0].time - 20.0 * v[0].time_per_sample;
    for (light, fill) in [(false, "fill=\"#000000\""), (true, "fill=\"#ffffff\"")] {
        let mut g = SvgPainter::new(W, H);
        let mut scales = ScopeScales::default();
        scales.drag_from = Some((0, from));
        draw_scopes(&mut g, &v, &mut scales, W, H, light, Some((x + 0.8 * w, y + 0.4 * h)));
        let out = g.finish();
        assert!(out.contains(">\u{394}t="), "the time of the measurement is drawn");
        // the two lines of the measurement (the readout of the pointer is above the scope)
        let found = only_in(&out, &svg(&v, light, None), fill);
        assert_eq!(found.len(), 2, "light={light}");
        for (x0, y0, x1, y1) in found {
            assert!(x0 >= x - 0.5 && x1 <= x + w + 0.5 && y0 >= y - 0.5 && y1 <= y + h + 0.5);
        }
        // the text keeps the usual colour
        let delta =
            out.lines().find(|l| l.starts_with("<text") && l.contains(">\u{394}t=")).unwrap();
        let colour = if light { "#000000" } else { "#c0c0c0" };
        assert!(delta.contains(&format!("fill=\"{colour}\"")), "{delta}");
    }
}

// ---- the scale labels (corner values, measurements, time per division) have plates too

/// The scale texts of a drawing (font size 10): their text and the point where they start.
fn scale_texts(svg: &str) -> Vec<(String, f64, f64)> {
    svg.lines()
        .filter(|l| l.starts_with("<text") && l.contains("font-size=\"10\""))
        .map(|l| {
            let attr = |name: &str| -> f64 {
                l.split(&format!(" {name}=\""))
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap()
            };
            let text = l.split('>').nth(1).unwrap().split('<').next().unwrap().to_string();
            (text, attr("x"), attr("y"))
        })
        .collect()
}

#[test]
fn every_scale_label_is_on_a_plate() {
    let mut same_units = views();
    let units = same_units[0].plots[0].units;
    same_units[0].plots.retain(|p| p.units == units);
    for (name, v) in [("mixed units", views()), ("same units", same_units)] {
        for (light, fill) in [(false, "fill=\"#000000\""), (true, "fill=\"#ffffff\"")] {
            let out = svg(&v, light, None);
            let plates = plates(&out, fill);
            let texts = scale_texts(&out);
            assert!(texts.len() >= 4, "{name}: the scale texts: {texts:?}");
            for (text, x, y) in texts {
                // the text starts on its plate and its baseline is inside it
                let covered =
                    plates.iter().any(|&(x0, y0, x1, y1)| x0 <= x && x < x1 && y0 < y && y <= y1);
                assert!(covered, "{name}, light={light}: no plate under {text:?} at ({x}, {y})");
            }
        }
    }
}

// ---- the plates are painted over the curves, not under them

/// The first line (index in the SVG) of each translucent plate with this fill, with its rectangle.
fn plate_lines(svg: &str, fill: &str) -> Vec<(usize, (f64, f64, f64, f64))> {
    let all = plates(svg, fill);
    let mut k = 0;
    svg.lines()
        .enumerate()
        .filter(|(_, l)| {
            l.starts_with("<path") && l.contains("opacity=\"0.75\"") && l.contains(fill)
        })
        .map(|(i, _)| {
            k += 1;
            (i, all[k - 1])
        })
        .collect()
}

#[test]
fn every_scale_label_plate_is_painted_after_the_curves() {
    let mut same_units = views();
    let units = same_units[0].plots[0].units;
    same_units[0].plots.retain(|p| p.units == units);
    for (name, mut v) in [("mixed units", views()), ("same units", same_units)] {
        // one scope, so that the order in the drawing is the order of its own painting
        v.truncate(1);
        for (light, fill) in [(false, "fill=\"#000000\""), (true, "fill=\"#ffffff\"")] {
            let out = svg(&v, light, None);
            // the curves are the only strokes in the colours of the plots
            let last_curve = out
                .lines()
                .enumerate()
                .filter(|(_, l)| {
                    l.starts_with("<path")
                        && (l.contains("stroke=\"#00ff00\"")
                            || l.contains("stroke=\"#ffff00\"")
                            || l.contains("stroke=\"#a0a000\"")
                            || l.contains("stroke=\"#00a000\""))
                })
                .map(|(i, _)| i)
                .max()
                .expect("the curves are drawn");
            let found = plate_lines(&out, fill);
            for (text, x, y) in scale_texts(&out) {
                let (line, _) = found
                    .iter()
                    .find(|(_, (x0, y0, x1, y1))| *x0 <= x && x < *x1 && *y0 < y && y <= *y1)
                    .unwrap_or_else(|| panic!("{name}: no plate under {text:?}"));
                assert!(
                    *line > last_curve,
                    "{name}, light={light}: the plate of {text:?} (line {line}) is painted before the curves (last at {last_curve})"
                );
            }
        }
    }
}
