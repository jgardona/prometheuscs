// SPDX-License-Identifier: GPL-2.0-or-later
//! Exporting a circuit as a picture: PNG, SVG, clipboard and printing share one drawing path.

use super::cairo::CairoPainter;
use super::scene::{View, draw_scene, post_draw_list};
use crate::app::session::Session;
use gtk4::cairo::{Context, Format, ImageSurface, SvgSurface};
use std::path::Path;

/// Space left around the circuit in exported pictures, in circuit pixels.
const MARGIN: i32 = 32;

/// Size and placement of the circuit's picture.
struct Frame {
    width: f64,
    height: f64,
    view: View,
}

fn frame(s: &Session) -> Option<Frame> {
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for e in &s.sim.elements {
        let b = &e.base().bbox;
        x0 = x0.min(b.x);
        y0 = y0.min(b.y);
        x1 = x1.max(b.x + b.width);
        y1 = y1.max(b.y + b.height);
    }
    if x0 > x1 {
        return None;
    }
    let (w, h) = (x1 - x0 + 2 * MARGIN, y1 - y0 + 2 * MARGIN);
    Some(Frame {
        width: f64::from(w),
        height: f64::from(h),
        view: View { scale: 1.0, x: f64::from(MARGIN - x0), y: f64::from(MARGIN - y0) },
    })
}

fn draw(cr: &Context, s: &mut Session, f: &Frame) {
    let mut g = CairoPainter::new(cr);
    let posts = post_draw_list(&s.sim.elements);
    let mut ctx = s.draw_ctx.clone();
    ctx.dragging_row_or_column = true;
    // pictures never show selection or hover highlights
    for e in &mut s.sim.elements {
        e.base_mut().selected = false;
        e.base_mut().highlight = false;
    }
    draw_scene(&mut g, &mut s.sim.elements, &posts, &ctx, f.view, (f.width, f.height));
}

/// Draws the circuit on a new image surface sized to fit it.
pub fn render_image(s: &mut Session) -> Result<ImageSurface, String> {
    let f = frame(s).ok_or("O circuito está vazio")?;
    let surface = ImageSurface::create(Format::ARgb32, f.width as i32, f.height as i32)
        .map_err(|e| e.to_string())?;
    {
        let cr = Context::new(&surface).map_err(|e| e.to_string())?;
        draw(&cr, s, &f);
    }
    Ok(surface)
}

/// PNG bytes of the circuit picture.
pub fn png_bytes(s: &mut Session) -> Result<Vec<u8>, String> {
    let surface = render_image(s)?;
    let mut out = Vec::new();
    surface.write_to_png(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Writes the circuit picture as a PNG file.
pub fn save_png(s: &mut Session, path: &Path) -> Result<(), String> {
    let bytes = png_bytes(s)?;
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes the circuit picture as an SVG file.
pub fn save_svg(s: &mut Session, path: &Path) -> Result<(), String> {
    let f = frame(s).ok_or("O circuito está vazio")?;
    let surface = SvgSurface::new(f.width, f.height, Some(path)).map_err(|e| e.to_string())?;
    {
        let cr = Context::new(&surface).map_err(|e| e.to_string())?;
        draw(&cr, s, &f);
    }
    surface.finish();
    Ok(())
}

/// Draws the circuit scaled to fit a page of `width` x `height` points (for printing).
pub fn draw_page(cr: &Context, s: &mut Session, width: f64, height: f64) {
    let Some(f) = frame(s) else { return };
    let k = (width / f.width).min(height / f.height).min(2.0);
    cr.scale(k, k);
    draw(cr, s, &f);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::Session;
    use crate::elements::registry::Registry;

    fn session() -> Session {
        let text = std::fs::read_to_string("assets/circuits/lrc.txt").unwrap();
        Session::load(&text, &Registry::standard()).unwrap()
    }

    #[test]
    fn png_and_svg_exports_are_valid_files() {
        let mut s = session();
        let png = png_bytes(&mut s).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let dir = std::env::temp_dir().join(format!("prometheuscs-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let svg = dir.join("c.svg");
        save_svg(&mut s, &svg).unwrap();
        let text = std::fs::read_to_string(&svg).unwrap();
        assert!(text.contains("<svg"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn exporting_an_empty_circuit_reports_it() {
        let mut s = Session::empty();
        assert!(png_bytes(&mut s).is_err());
    }
}
