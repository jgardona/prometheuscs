// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! A scope plotting a selected (or hovered) element is drawn with a soft blue background, for
//! docked scopes and for scopes placed in the circuit, with and without a worker.

use prometheuscs::app::session::Session;
use prometheuscs::app::settings::AppSettings;
use prometheuscs::app::worker::Worker;
use prometheuscs::elements::registry::Registry;
use prometheuscs::render::scopes::{ScopeScales, draw_scopes, glow_color};
use prometheuscs::render::svg::SvgPainter;
use std::collections::BTreeSet;
use std::time::Duration;

fn lrc() -> String {
    std::fs::read_to_string("assets/circuits/lrc.txt").unwrap()
}

fn loaded() -> Session {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    for _ in 0..10 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    s
}

/// For every scope, the elements its plots show.
fn plotted(s: &Session) -> Vec<BTreeSet<usize>> {
    s.sim.scopes.iter().map(|sc| sc.plots.iter().map(|p| p.elm).collect()).collect()
}

fn select_only(s: &mut Session, elm: usize) {
    s.deselect_all();
    s.sim.elements[elm].base_mut().selected = true;
}

/// The background colour of the scope drawn by `draw_scopes` with these flags.
fn drawn(s: &Session, glow: &[bool], light: bool) -> String {
    let mut g = SvgPainter::new(900.0, 400.0);
    let mut scales = ScopeScales::default();
    scales.glow = glow.to_vec();
    draw_scopes(&mut g, &s.scope_views(), &mut scales, 900.0, 400.0, light, None);
    g.finish()
}

fn hex(light: bool, glow: bool) -> String {
    let c = if glow {
        glow_color(light)
    } else if light {
        prometheuscs::render::painter::Color::new(255, 255, 255)
    } else {
        prometheuscs::render::painter::Color::new(0, 0, 0)
    };
    format!("fill=\"{}\"", c.hex())
}

// ---- US1: the selected component lights its scope

#[test]
fn a_scope_glows_when_it_plots_the_selected_element() {
    let mut s = loaded();
    let sets = plotted(&s);
    assert!(sets.len() >= 2, "{sets:?}");
    assert_eq!(s.scope_glow(), vec![false; sets.len()], "nothing selected");
    let elm = *sets[0].iter().next().unwrap();
    select_only(&mut s, elm);
    let want: Vec<bool> = sets.iter().map(|p| p.contains(&elm)).collect();
    assert!(want[0]);
    assert_eq!(s.scope_glow(), want);
    s.deselect_all();
    assert_eq!(s.scope_glow(), vec![false; sets.len()], "deselected");
}

#[test]
fn an_element_no_scope_plots_lights_nothing() {
    let mut s = loaded();
    let all: BTreeSet<usize> = plotted(&s).into_iter().flatten().collect();
    let other = (0..s.sim.elements.len()).find(|i| !all.contains(i)).expect("an unplotted element");
    select_only(&mut s, other);
    assert!(s.scope_glow().iter().all(|g| !g));
}

#[test]
fn the_glowing_scope_has_the_blue_background_and_the_others_do_not() {
    let s = loaded();
    let n = s.sim.scopes.len();
    for light in [false, true] {
        let mut flags = vec![false; n];
        flags[0] = true;
        let svg = drawn(&s, &flags, light);
        assert_eq!(svg.matches(&hex(light, true)).count(), 1, "one glowing panel, light={light}");
        assert_eq!(svg.matches(&hex(light, false)).count(), n - 1, "the others, light={light}");
        let none = drawn(&s, &vec![false; n], light);
        assert_eq!(none.matches(&hex(light, true)).count(), 0);
        // the grid and the traces keep their colours
        let grid = if light { "#c8c8c8" } else { "#404040" };
        assert!(svg.contains(grid), "grid colour {grid}");
    }
}

#[test]
fn the_glow_colour_is_a_soft_blue() {
    let dark = glow_color(false);
    assert!(dark.b > dark.g && dark.g >= dark.r, "{dark:?}");
    assert!(dark.b <= 90, "soft, not saturated: {dark:?}");
    let light = glow_color(true);
    assert!(light.b == 255 && light.r < 255 && light.r > 200, "{light:?}");
}

// ---- US2: hover

#[test]
fn pointing_at_a_plotted_element_lights_its_scope() {
    let mut s = loaded();
    s.view = prometheuscs::render::scene::View { scale: 1.0, x: 0.0, y: 0.0 };
    let sets = plotted(&s);
    let elm = *sets[0].iter().next().unwrap();
    let p = s.sim.elements[elm].get_post(0).unwrap();
    let q = s.sim.elements[elm].get_post(1).unwrap();
    s.hover_at(Some((f64::from(p.x + q.x) / 2.0, f64::from(p.y + q.y) / 2.0)));
    let glow = s.scope_glow();
    assert!(glow[0], "{glow:?}");
    s.hover_at(None);
    assert!(s.scope_glow().iter().all(|g| !g), "the pointer left");
}

#[test]
fn the_hovered_and_the_selected_elements_light_their_scopes_together() {
    let mut s = loaded();
    s.view = prometheuscs::render::scene::View { scale: 1.0, x: 0.0, y: 0.0 };
    let sets = plotted(&s);
    let (a, b) = (0..sets.len())
        .flat_map(|i| (0..sets.len()).map(move |j| (i, j)))
        .find(|&(i, j)| {
            i != j
                && sets[i].iter().any(|e| !sets[j].contains(e))
                && sets[j].iter().any(|e| !sets[i].contains(e))
        })
        .expect("two scopes with elements of their own");
    let ea = *sets[a].difference(&sets[b]).next().unwrap();
    let eb = *sets[b].difference(&sets[a]).next().unwrap();
    select_only(&mut s, ea);
    let p = s.sim.elements[eb].get_post(0).unwrap();
    let q = s.sim.elements[eb].get_post(1).unwrap();
    s.hover_at(Some((f64::from(p.x + q.x) / 2.0, f64::from(p.y + q.y) / 2.0)));
    let glow = s.scope_glow();
    assert!(glow[a] && glow[b], "{glow:?}");
}

// ---- US3: other arrangements

#[test]
fn a_scope_with_several_plots_glows_for_any_of_them() {
    let mut s = loaded();
    let sets = plotted(&s);
    // make scope 0 plot an element of scope 1 as well
    let other = *sets[1].difference(&sets[0]).next().expect("an element only scope 1 plots");
    let last = s.sim.scopes[0].plots.len() - 1;
    s.sim.scopes[0].plots[last].elm = other;
    let mine =
        *sets[0].iter().find(|e| s.sim.scopes[0].plots.iter().any(|p| p.elm == **e)).unwrap();
    for elm in [mine, other] {
        select_only(&mut s, elm);
        assert!(s.scope_glow()[0], "element {elm}");
    }
    assert!(s.scope_glow()[1], "scope 1 still plots {other}");
}

#[test]
fn an_element_index_that_does_not_exist_never_matches() {
    let mut s = loaded();
    s.sim.scopes[0].plots[0].elm = usize::MAX;
    assert!(s.scope_glow().iter().all(|g| !g));
}

/// The SVG of the scope placed in the circuit as element `elm`.
fn placed_svg(s: &mut Session, elm: usize) -> String {
    let mut g = SvgPainter::new(900.0, 700.0);
    let ctx = s.draw_ctx.clone();
    s.sim.elements[elm].draw(&mut g, &ctx);
    g.finish()
}

fn placed_scope_is_tinted_only_when_its_element_is_selected(
    mut s: Session,
    frames: impl Fn(&mut Session),
) {
    assert!(s.undock_scope(0));
    frames(&mut s);
    let elm = s.sim.embedded[0].0;
    let plotted: Vec<usize> = s.embedded_scope(elm).unwrap().plots.iter().map(|p| p.elm).collect();
    let glow = format!("fill=\"{}\"", glow_color(false).hex());
    assert!(!placed_svg(&mut s, elm).contains(&glow), "nothing selected");
    select_only(&mut s, plotted[0]);
    frames(&mut s);
    assert!(placed_svg(&mut s, elm).contains(&glow), "its element is selected");
    s.deselect_all();
    frames(&mut s);
    assert!(!placed_svg(&mut s, elm).contains(&glow), "deselected");
}

#[test]
fn a_scope_placed_in_the_circuit_glows_for_its_selected_element() {
    placed_scope_is_tinted_only_when_its_element_is_selected(loaded(), |s| {
        for _ in 0..3 {
            s.update(Duration::from_millis(16)).unwrap();
        }
        // without a worker the views of placed scopes are pushed by whoever draws
        s.sim.push_embedded_views();
    });
}

#[test]
fn a_scope_placed_in_the_circuit_glows_with_the_simulation_on_its_own_thread() {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(&lrc(), "lrc", None).unwrap();
    placed_scope_is_tinted_only_when_its_element_is_selected(s, |s| {
        for _ in 0..12 {
            s.update(Duration::from_millis(16)).unwrap();
            std::thread::sleep(Duration::from_millis(16));
        }
    });
}

#[test]
fn a_big_and_a_compact_placed_scope_are_both_tinted() {
    let mut s = loaded();
    assert!(s.undock_scope(0));
    let elm = s.sim.embedded[0].0;
    let plotted = s.embedded_scope(elm).unwrap().plots[0].elm;
    select_only(&mut s, plotted);
    s.update(Duration::from_millis(16)).unwrap();
    s.sim.push_embedded_views();
    let glow = format!("fill=\"{}\"", glow_color(false).hex());
    for (w, h) in [(128, 64), (60, 40), (420, 200)] {
        let b = s.sim.elements[elm].base_mut();
        b.x2 = b.x + w;
        b.y2 = b.y + h;
        assert!(placed_svg(&mut s, elm).contains(&glow), "{w}x{h}");
    }
}
