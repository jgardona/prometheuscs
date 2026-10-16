// SPDX-License-Identifier: GPL-2.0-or-later
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Rough frame-time measurement of the Cairo backend (run with --release --nocapture).

use gtk4::cairo::{Context, Format, ImageSurface};
use prometheuscs::elements::element::DrawCtx;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::render::cairo::CairoPainter;
use prometheuscs::render::scene::{View, draw_scene, post_draw_list};
use std::time::Instant;

#[test]
fn frame_time() {
    for name in ["lrc.txt", "ladder.txt", "fullrect.txt"] {
        let Ok(text) = std::fs::read_to_string(format!("assets/circuits/{name}")) else { continue };
        let mut r = parse(&text, &Registry::standard());
        let n = r.circuit.elements.len();
        let (w, h) = (1280, 800);
        let surface = ImageSurface::create(Format::ARgb32, w, h).unwrap();
        let ctx =
            DrawCtx { show_volts: true, show_values: true, show_dots: true, ..DrawCtx::default() };
        let posts = post_draw_list(&r.circuit.elements);
        let view = View::fit(&r.circuit.elements, f64::from(w), f64::from(h), 2.0);
        let frames = 200;
        let t = Instant::now();
        for _ in 0..frames {
            let cr = Context::new(&surface).unwrap();
            let mut g = CairoPainter::new(&cr);
            draw_scene(
                &mut g,
                &mut r.circuit.elements,
                &posts,
                &ctx,
                view,
                (f64::from(w), f64::from(h)),
            );
        }
        println!(
            "{name}: {n} elements, {:.2} ms/frame",
            t.elapsed().as_secs_f64() * 1000.0 / f64::from(frames)
        );
    }
}
