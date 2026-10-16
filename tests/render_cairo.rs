// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Renders bundled examples headlessly through the Cairo painter.

use gtk4::cairo::{Context, Format, ImageSurface};
use prometheuscs::elements::element::DrawCtx;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::render::cairo::CairoPainter;
use prometheuscs::render::scene::{View, draw_scene, post_draw_list};
use prometheuscs::solver::sim::Simulation;

fn render_example(name: &str, run_to: f64, out: &str) -> ImageSurface {
    let text = std::fs::read_to_string(format!("assets/circuits/{name}")).expect("example");
    let r = parse(&text, &Registry::standard());
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.run_until(run_to).unwrap();
    let (w, h) = (800, 600);
    let surface = ImageSurface::create(Format::ARgb32, w, h).expect("surface");
    {
        let cr = Context::new(&surface).expect("context");
        let mut g = CairoPainter::new(&cr);
        let ctx = DrawCtx {
            show_volts: true,
            show_values: true,
            show_dots: true,
            sim_running: true,
            current_mult: 1.0,
            voltage_range: 5.0,
            ..DrawCtx::default()
        };
        let posts = post_draw_list(&sim.elements);
        let view = View::fit(&sim.elements, f64::from(w), f64::from(h), 2.0);
        draw_scene(&mut g, &mut sim.elements, &posts, &ctx, view, (f64::from(w), f64::from(h)));
    }
    let mut file = std::fs::File::create(out).expect("png file");
    surface.write_to_png(&mut file).expect("png");
    surface
}

#[test]
fn lrc_renders_something_other_than_the_background() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let mut surface = render_example("lrc.txt", 1e-3, &format!("{dir}/lrc.png"));
    let data = surface.data().expect("pixels");
    let lit = data.chunks(4).filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0).count();
    assert!(lit > 500, "only {lit} non-background pixels");
}
