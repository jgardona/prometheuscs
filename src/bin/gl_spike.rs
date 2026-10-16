// SPDX-License-Identifier: GPL-2.0-or-later
//! Spike for task T044: can a `GtkGLArea` with femtovg draw hundreds of animated circuit
//! elements at 60 frames per second? Draws N resistor-like elements with animated current dots
//! for a few seconds and prints the measured frame rate and per-frame cost.
//!
//! Usage: `gl_spike [elements] [seconds]`

use femtovg::{Color, Paint, Path};
use prometheuscs::render::gl::GlCanvas;
use relm4::gtk::{self, gio, glib, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct Stats {
    frames: u32,
    render_time: Duration,
    worst: Duration,
    started: Instant,
}

fn draw_elements(c: &mut GlCanvas, n: usize, t: f32, w: u32, h: u32, scale: f32) {
    c.begin(w, h, scale, Color::rgb(0, 0, 0));
    let cols = 25usize;
    let cell_w = 64.0_f32;
    let cell_h = 32.0_f32;
    let canvas = &mut c.canvas;
    let lead = Paint::color(Color::rgb(0, 200, 0)).with_line_width(3.0);
    let body = Paint::color(Color::rgb(200, 200, 60)).with_line_width(3.0);
    let dot = Paint::color(Color::rgb(255, 255, 0));
    for i in 0..n {
        let (cx, cy) = ((i % cols) as f32 * cell_w + 16.0, (i / cols) as f32 * cell_h + 16.0);
        let mut p = Path::new();
        p.move_to(cx, cy);
        p.line_to(cx + 12.0, cy);
        canvas.stroke_path(&p, &lead);
        let mut z = Path::new();
        z.move_to(cx + 12.0, cy);
        for k in 0..4 {
            let x = cx + 12.0 + k as f32 * 8.0;
            z.line_to(x + 2.0, cy + 5.0);
            z.line_to(x + 6.0, cy - 5.0);
        }
        z.line_to(cx + 44.0, cy);
        canvas.stroke_path(&z, &body);
        let mut q = Path::new();
        q.move_to(cx + 44.0, cy);
        q.line_to(cx + 56.0, cy);
        canvas.stroke_path(&q, &lead);
        let mut dots = Path::new();
        for k in 0..5 {
            let x = cx + ((k as f32 * 12.0 + t * 40.0) % 56.0);
            dots.rect(x - 1.5, cy - 1.5, 3.0, 3.0);
        }
        canvas.fill_path(&dots, &dot);
    }
    c.end();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(500);
    let seconds: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(5);
    let app = gtk::Application::builder()
        .application_id("org.prometheuscs.GlSpike")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| {
        let window = gtk::ApplicationWindow::builder().application(app).default_width(1700).default_height(900).title("GL spike").build();
        let area = gtk::GLArea::new();
        area.set_has_stencil_buffer(true);
        area.set_auto_render(false);
        let canvas: Rc<RefCell<Option<GlCanvas>>> = Rc::new(RefCell::new(None));
        let stats = Rc::new(RefCell::new(Stats { frames: 0, render_time: Duration::ZERO, worst: Duration::ZERO, started: Instant::now() }));
        {
            let canvas = canvas.clone();
            area.connect_realize(move |a| {
                a.make_current();
                a.attach_buffers();
                if let Some(e) = a.error() {
                    println!("GL context error: {e}");
                    return;
                }
                match GlCanvas::new() {
                    Ok(c) => *canvas.borrow_mut() = Some(c),
                    Err(e) => println!("canvas error: {e}"),
                }
            });
        }
        {
            let (canvas, stats) = (canvas.clone(), stats.clone());
            let t0 = Instant::now();
            area.connect_render(move |a, _ctx| {
                let start = Instant::now();
                if let Some(c) = canvas.borrow_mut().as_mut() {
                    let scale = a.scale_factor() as f32;
                    let (w, h) = ((a.width() as f32 * scale) as u32, (a.height() as f32 * scale) as u32);
                    draw_elements(c, n, t0.elapsed().as_secs_f32(), w, h, scale);
                    if std::env::var_os("GL_SPIKE_FINISH").is_some() {
                        c.wait_for_gpu();
                    }
                    if stats.borrow().frames == 30 {
                        println!("pixels not black after 30 frames: {} of {}", c.non_black_pixels(w, h), w * h);
                    }
                }
                let spent = start.elapsed();
                let mut s = stats.borrow_mut();
                s.frames += 1;
                s.render_time += spent;
                s.worst = s.worst.max(spent);
                glib::Propagation::Stop
            });
        }
        area.add_tick_callback(|a, _| {
            a.queue_render();
            glib::ControlFlow::Continue
        });
        window.set_child(Some(&area));
        window.present();
        let stats = stats.clone();
        let app = app.clone();
        glib::timeout_add_local_once(Duration::from_secs(seconds), move || {
            let s = stats.borrow();
            let secs = s.started.elapsed().as_secs_f64();
            println!(
                "{n} elements: {} frames in {secs:.1}s = {:.1} fps, render call avg {:.2} ms, worst {:.2} ms",
                s.frames,
                f64::from(s.frames) / secs,
                s.render_time.as_secs_f64() * 1000.0 / f64::from(s.frames.max(1)),
                s.worst.as_secs_f64() * 1000.0
            );
            app.quit();
        });
    });
    app.run_with_args::<&str>(&[]);
}
