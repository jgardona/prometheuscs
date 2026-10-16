// SPDX-License-Identifier: GPL-2.0-or-later
//! GPU backend: femtovg drawing into the OpenGL context of a `GtkGLArea`.
//!
//! This is the only module of the program that uses `unsafe`: loading OpenGL entry points from
//! libepoxy (the library GTK itself uses) and handing them to femtovg.
#![allow(unsafe_code)]

use femtovg::renderer::OpenGl;
use femtovg::{Canvas, Color};
use std::ffi::c_void;

/// A femtovg canvas bound to the current OpenGL context.
pub struct GlCanvas {
    /// The canvas to draw on.
    pub canvas: Canvas<OpenGl>,
    probe: glow::Context,
    framebuffer: Option<glow::NativeFramebuffer>,
    // keeps the entry points alive for as long as the renderer uses them
    _library: libloading::Library,
}

/// Address of the GL function `name` in libepoxy.
fn epoxy_symbol(library: &libloading::Library, name: &str) -> *const c_void {
    // epoxy exports each GL entry point as a function-pointer variable named `epoxy_<name>`,
    // holding a dispatcher that works with whatever context is current.
    let symbol = format!("epoxy_{name}");
    // SAFETY: the symbol is a variable of pointer type, so `library.get` yields the address of
    // that variable; reading it gives the function address, which is only handed to glow and
    // femtovg as an opaque address.
    match unsafe { library.get::<*const *const c_void>(symbol.as_bytes()) } {
        Ok(variable) => unsafe { **variable },
        Err(_) => std::ptr::null(),
    }
}

impl GlCanvas {
    /// Creates the canvas. The GL context of the `GtkGLArea` must be current and its buffers
    /// attached (call this from the `realize` handler after `make_current` and
    /// `attach_buffers`), because the framebuffer bound at that moment becomes the screen.
    pub fn new() -> Result<GlCanvas, String> {
        // SAFETY: loading libepoxy runs its library initialisers only, and it is already loaded
        // into the process by GTK, so this just takes another reference to it.
        let library = unsafe { libloading::Library::new("libepoxy.so.0") }
            .map_err(|e| format!("libepoxy: {e}"))?;
        // SAFETY: the caller made the GL context current, and `epoxy_symbol` returns either
        // valid function addresses of that context's driver or null for unknown names.
        let probe = unsafe { glow::Context::from_loader_function(|n| epoxy_symbol(&library, n)) };
        // SAFETY: querying an integer state of the current context.
        let fbo = unsafe { glow::HasContext::get_parameter_i32(&probe, glow::FRAMEBUFFER_BINDING) };
        // SAFETY: as above, for the renderer's own entry points.
        let mut renderer = unsafe { OpenGl::new_from_function(|n| epoxy_symbol(&library, n)) }
            .map_err(|e| format!("femtovg: {e}"))?;
        // GtkGLArea draws into its own framebuffer, not the default one
        let screen = u32::try_from(fbo)
            .ok()
            .and_then(std::num::NonZeroU32::new)
            .map(glow::NativeFramebuffer);
        renderer.set_screen_target(screen);
        let canvas = Canvas::new(renderer).map_err(|e| format!("femtovg: {e}"))?;
        Ok(GlCanvas { canvas, probe, framebuffer: screen, _library: library })
    }

    /// Starts a frame of `width` x `height` pixels at the given scale factor and clears it.
    pub fn begin(&mut self, width: u32, height: u32, scale: f32, background: Color) {
        use glow::HasContext;
        // errors left over from GTK's own drawing would be blamed on femtovg
        // SAFETY: the GL context is current; `get_error` takes no pointers.
        while unsafe { self.probe.get_error() } != glow::NO_ERROR {}
        self.canvas.set_size(width, height, scale);
        self.canvas.clear_rect(0, 0, width, height, background);
    }

    /// Counts the pixels of the last frame that are not black, reading the framebuffer back
    /// (slow; for tests and diagnostics).
    pub fn non_black_pixels(&self, width: u32, height: u32) -> usize {
        use glow::HasContext;
        let mut data = vec![0_u8; width as usize * height as usize * 4];
        // SAFETY: the GL context is current (the caller is inside the render handler) and
        // `data` has room for width x height RGBA pixels.
        unsafe {
            self.probe.bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer);
            self.probe.finish();
            self.probe.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut data)),
            );
        }
        data.chunks(4).filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0).count()
    }

    /// Waits until the GPU has finished every command sent so far (to time whole frames).
    pub fn wait_for_gpu(&self) {
        use glow::HasContext;
        // SAFETY: the GL context is current; `finish` takes no pointers.
        unsafe { self.probe.finish() };
    }

    /// Sends the frame to the GPU.
    pub fn end(&mut self) {
        self.canvas.flush();
    }
}

// ---------------------------------------------------------------------------------------------
// Painter
// ---------------------------------------------------------------------------------------------

use super::painter::{Color as Rgb, LineCap, Painter, TextAlign, TextBaseline};
use femtovg::{Align, Baseline, FontId, Paint, Path};

/// Finds a sans-serif font file on the system: asks fontconfig first, then tries well-known
/// locations.
pub fn find_system_font() -> Option<std::path::PathBuf> {
    let asked = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", "sans"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| std::path::PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()));
    let known = [
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    asked.into_iter().chain(known.iter().map(std::path::PathBuf::from)).find(|p| p.is_file())
}

impl GlCanvas {
    /// Loads the text font. Without a font, text is skipped (and measured as zero wide).
    pub fn load_font(&mut self) -> Option<FontId> {
        let path = find_system_font()?;
        self.canvas.add_font(path).ok()
    }
}

#[derive(Clone, Copy)]
struct State {
    color: Rgb,
    alpha: f64,
    line_width: f64,
    cap: LineCap,
    dash: (i32, i32),
    font_size: i32,
    align: TextAlign,
    baseline: TextBaseline,
    gradient: Option<(f64, Rgb, Rgb)>,
}

/// Draws through femtovg onto the current GL context.
pub struct GlPainter<'a> {
    canvas: &'a mut Canvas<OpenGl>,
    font: Option<FontId>,
    state: State,
    stack: Vec<State>,
    path: Path,
    path_open: bool,
}

fn fcolor(c: Rgb, alpha: f64) -> Color {
    Color::rgba(c.r, c.g, c.b, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

impl<'a> GlPainter<'a> {
    /// Wraps the canvas of a frame started with [`GlCanvas::begin`].
    pub fn new(canvas: &'a mut Canvas<OpenGl>, font: Option<FontId>) -> Self {
        GlPainter {
            canvas,
            font,
            state: State {
                color: Rgb::WHITE,
                alpha: 1.0,
                line_width: 1.0,
                cap: LineCap::Butt,
                dash: (0, 0),
                font_size: 12,
                align: TextAlign::Left,
                baseline: TextBaseline::Alphabetic,
                gradient: None,
            },
            stack: Vec::new(),
            path: Path::new(),
            path_open: false,
        }
    }

    fn stroke_paint(&self) -> Paint {
        let s = &self.state;
        let mut p = match s.gradient {
            Some((len, c0, c1)) => Paint::linear_gradient(
                0.0,
                0.0,
                len as f32,
                0.0,
                fcolor(c0, s.alpha),
                fcolor(c1, s.alpha),
            ),
            None => Paint::color(fcolor(s.color, s.alpha)),
        };
        p.set_line_width(s.line_width as f32);
        p.set_line_cap(match s.cap {
            LineCap::Butt => femtovg::LineCap::Butt,
            LineCap::Round => femtovg::LineCap::Round,
        });
        if s.dash != (0, 0) {
            p.set_line_dash(&[s.dash.0 as f32, s.dash.1 as f32]);
        }
        p
    }

    fn fill_paint(&self) -> Paint {
        Paint::color(fcolor(self.state.color, self.state.alpha))
    }

    fn text_paint(&self) -> Option<Paint> {
        let font = self.font?;
        let s = &self.state;
        Some(
            Paint::color(fcolor(s.color, s.alpha))
                .with_font(&[font])
                .with_font_size(s.font_size as f32)
                .with_text_align(match s.align {
                    TextAlign::Left => Align::Left,
                    TextAlign::Center => Align::Center,
                })
                .with_text_baseline(match s.baseline {
                    TextBaseline::Alphabetic => Baseline::Alphabetic,
                    TextBaseline::Middle => Baseline::Middle,
                }),
        )
    }
}

impl Painter for GlPainter<'_> {
    fn set_color(&mut self, color: Rgb) {
        self.state.color = color;
        self.state.gradient = None;
    }
    fn set_line_width(&mut self, width: f64) {
        self.state.line_width = width;
    }
    fn set_line_cap(&mut self, cap: LineCap) {
        self.state.cap = cap;
    }
    fn set_line_dash(&mut self, on: i32, off: i32) {
        self.state.dash = (on, off);
    }
    fn set_alpha(&mut self, alpha: f64) {
        self.state.alpha = alpha;
    }
    fn set_font_size(&mut self, size: i32) {
        self.state.font_size = size;
    }
    fn font_size(&self) -> i32 {
        self.state.font_size
    }
    fn set_text_align(&mut self, align: TextAlign) {
        self.state.align = align;
    }
    fn set_text_baseline(&mut self, baseline: TextBaseline) {
        self.state.baseline = baseline;
    }
    fn measure_text(&self, text: &str) -> f64 {
        let Some(paint) = self.text_paint() else { return 0.0 };
        self.canvas.measure_text(0.0, 0.0, text, &paint).map_or(0.0, |m| f64::from(m.width()))
    }
    fn save(&mut self) {
        self.stack.push(self.state);
        self.canvas.save();
    }
    fn restore(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s;
            self.canvas.restore();
        }
    }
    fn transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.canvas.set_transform(&femtovg::Transform2D::new(
            a as f32, b as f32, c as f32, d as f32, e as f32, f as f32,
        ));
    }
    fn translate(&mut self, x: f64, y: f64) {
        self.canvas.translate(x as f32, y as f32);
    }
    fn rotate(&mut self, angle: f64) {
        self.canvas.rotate(angle as f32);
    }
    fn scale(&mut self, sx: f64, sy: f64) {
        self.canvas.scale(sx as f32, sy as f32);
    }
    fn begin_path(&mut self) {
        self.path = Path::new();
        self.path_open = false;
    }
    fn move_to(&mut self, x: f64, y: f64) {
        self.path.move_to(x as f32, y as f32);
        self.path_open = true;
    }
    fn line_to(&mut self, x: f64, y: f64) {
        if self.path_open {
            self.path.line_to(x as f32, y as f32);
        } else {
            self.path.move_to(x as f32, y as f32);
            self.path_open = true;
        }
    }
    fn bezier_curve_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) {
        self.path.bezier_to(c1x as f32, c1y as f32, c2x as f32, c2y as f32, x as f32, y as f32);
    }
    fn arc(&mut self, x: f64, y: f64, radius: f64, start: f64, end: f64) {
        // a polyline of at most 10 degrees per segment, the way the SVG backend does it
        let sweep = end - start;
        let steps = ((sweep.abs() / (std::f64::consts::PI / 18.0)).ceil() as usize).max(1);
        for i in 0..=steps {
            let a = start + sweep * i as f64 / steps as f64;
            self.line_to(x + radius * a.cos(), y + radius * a.sin());
        }
    }
    fn close_path(&mut self) {
        self.path.close();
    }
    fn stroke(&mut self) {
        let paint = self.stroke_paint();
        self.canvas.stroke_path(&self.path, &paint);
    }
    fn fill(&mut self) {
        let paint = self.fill_paint();
        self.canvas.fill_path(&self.path, &paint);
    }
    fn stroke_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let mut p = Path::new();
        p.rect(x as f32, y as f32, w as f32, h as f32);
        let paint = self.stroke_paint();
        self.canvas.stroke_path(&p, &paint);
    }
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        let mut p = Path::new();
        p.rect(x as f32, y as f32, w as f32, h as f32);
        let paint = self.fill_paint();
        self.canvas.fill_path(&p, &paint);
    }
    fn draw_string(&mut self, text: &str, x: f64, y: f64) {
        if let Some(paint) = self.text_paint() {
            // a string the font cannot shape is left out rather than stopping the frame
            let _ = self.canvas.fill_text(x as f32, y as f32, text, &paint);
        }
    }
    fn set_stroke_gradient(&mut self, len: f64, c0: Rgb, c1: Rgb) {
        self.state.gradient = Some((len, c0, c1));
    }
    fn set_stroke_color(&mut self, color: Rgb) {
        self.set_color(color);
    }
}
