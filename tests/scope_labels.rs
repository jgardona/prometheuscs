// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The labels that follow the pointer on a scope sit on a translucent plate in the scope's
//! background colour, and the line with the element name and the newest value is gone.

use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;
use prometheuscs::render::scopes::{ScopeScales, draw_scopes, layout};
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
    svg.lines()
        .filter(|l| l.starts_with("<path") && l.contains("opacity=\"0.75\"") && l.contains(fill))
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
            (x0, y0, x1, y1)
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

// ---- US1: plates behind the cursor labels

#[test]
fn each_cursor_label_is_on_a_translucent_plate() {
    let v = views();
    let (x, y, w, h) = first_rect(&v);
    let cursor = Some((x + 0.3 * w, y + 0.4 * h));
    let want = v[0].plots.len() + 1; // a value line per plot, and the time
    for (light, fill) in [(false, "fill=\"#000000\""), (true, "fill=\"#ffffff\"")] {
        let with = svg(&v, light, cursor);
        let without = svg(&v, light, None);
        // the scope background is opaque; only the plates have opacity 0.75
        assert_eq!(only_in(&with, &without, fill).len(), want, "light={light}");
        assert!(!plates(&without, fill).is_empty(), "the scale labels have plates too");
    }
}

#[test]
fn the_text_keeps_its_colour_on_the_plate() {
    let v = views();
    let (x, y, w, h) = first_rect(&v);
    let out = svg(&v, false, Some((x + 0.8 * w, y + 0.4 * h)));
    let time =
        out.lines().find(|l| l.starts_with("<text") && l.contains(">t = ")).expect("time label");
    assert!(time.contains("fill=\"#c0c0c0\""), "{time}");
    let value =
        out.lines().find(|l| l.starts_with("<text") && l.contains(">V = ")).expect("value label");
    assert!(value.contains("fill=\"#00ff00\""), "the voltage trace is green: {value}");
}

#[test]
fn the_plates_are_beside_the_cursor_line_and_inside_the_scope() {
    let v = views();
    let (x, y, w, h) = first_rect(&v);
    // left half: labels on the right of the cursor line; right half: on the left of it
    for (fx, on_right) in [(0.2, true), (0.8, false)] {
        let cx = x + fx * w;
        let out = svg(&v, false, Some((cx, y + 0.4 * h)));
        let found = only_in(&out, &svg(&v, false, None), "fill=\"#000000\"");
        assert!(!found.is_empty(), "plates at {fx}");
        for (x0, _, x1, _) in found {
            if on_right {
                assert!(x0 >= cx, "plate at {x0} should be right of {cx}");
            } else {
                assert!(x1 <= cx + 0.5 * (w / 40.0) + 1.0, "plate ends at {x1}, cursor {cx}");
            }
        }
    }
    // corners and the middle: every plate inside the scope's rectangle
    for (fx, fy) in [(0.02, 0.05), (0.98, 0.05), (0.02, 0.95), (0.98, 0.95), (0.5, 0.5)] {
        let out = svg(&v, false, Some((x + fx * w, y + fy * h)));
        let found = only_in(&out, &svg(&v, false, None), "fill=\"#000000\"");
        assert!(!found.is_empty(), "plates at ({fx}, {fy})");
        for (x0, y0, x1, y1) in found {
            assert!(x0 >= x - 0.5 && x1 <= x + w + 0.5, "x {x0}..{x1} outside {x}..{}", x + w);
            assert!(y0 >= y - 0.5 && y1 <= y + h + 0.5, "y {y0}..{y1} outside {y}..{}", y + h);
        }
    }
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
    assert!(out.contains(">V = ") || out.contains(">I = "), "cursor labels");
    assert!(out.contains(">t = "), "time label");
    assert!(out.contains(">I = 1 mA<"), "the information area");
}

// ---- US3: stacked scopes

#[test]
fn plates_of_a_stacked_scope_stay_in_their_scope() {
    let v = views();
    let rects = layout(&v, W, H);
    // the lowest scope of the first column
    let i = (0..v.len()).max_by(|&a, &b| rects[a].1.partial_cmp(&rects[b].1).unwrap()).unwrap();
    let (x, y, w, h) = rects[i];
    let out = svg(&v, false, Some((x + 0.5 * w, y + 0.5 * h)));
    let found = only_in(&out, &svg(&v, false, None), "fill=\"#000000\"");
    assert!(!found.is_empty());
    for (x0, y0, x1, y1) in found {
        assert!(x0 >= x - 0.5 && x1 <= x + w + 0.5 && y0 >= y - 0.5 && y1 <= y + h + 0.5);
    }
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
fn a_scope_placed_in_the_circuit_has_plates_inside_it_and_none_when_compact() {
    let v = views();
    for (w, h) in [(420.0, 200.0), (200.0, 160.0), (128.0, 64.0), (60.0, 40.0)] {
        for fx in [0.05, 0.5, 0.95] {
            let out = placed(&v[0], (w, h), Some((fx * w, 0.5 * h)), false);
            let found = only_in(&out, &placed(&v[0], (w, h), None, false), "fill=\"#000000\"");
            if w < 160.0 || h < 90.0 {
                // a compact scope has room for the curves only: no labels, so no empty plates
                assert!(found.is_empty(), "{w}x{h} at {fx}");
                continue;
            }
            assert!(!found.is_empty(), "{w}x{h} at {fx}");
            for (x0, y0, x1, y1) in found {
                assert!(x0 >= 19.5 && x1 <= 20.0 + w + 0.5, "{w}x{h} at {fx}: x {x0}..{x1}");
                assert!(y0 >= 19.5 && y1 <= 20.0 + h + 0.5, "{w}x{h} at {fx}: y {y0}..{y1}");
            }
        }
    }
}

#[test]
fn a_placed_scope_on_the_light_theme_has_white_plates() {
    let v = views();
    let out = placed(&v[0], (420.0, 200.0), Some((300.0, 100.0)), true);
    let without = placed(&v[0], (420.0, 200.0), None, true);
    assert_eq!(only_in(&out, &without, "fill=\"#ffffff\"").len(), v[0].plots.len() + 1);
    assert!(plates(&out, "fill=\"#000000\"").is_empty());
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
        // a value line per plot, the time, and the two lines of the measurement
        let found = only_in(&out, &svg(&v, light, None), fill);
        assert_eq!(found.len(), v[0].plots.len() + 1 + 2, "light={light}");
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
