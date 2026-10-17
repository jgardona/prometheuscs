// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (ScopeElm).
//! A scope placed in the circuit as a rectangle that shows its curves in place.

use super::element::*;
use super::registry::{ElementDescriptor, Registry};
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::Painter;
use crate::render::scopes::ScaleState;
use crate::scope::ScopeView;

/// Family name used for the feature switch.
pub const FAMILY: &str = "outputs";

/// A scope drawn inside the circuit. What it plots is recorded by the simulation; the element
/// only keeps the settings from the file and draws the latest view it was given.
pub struct ScopeElm {
    base: Base,
    source: Option<ScopeSource>,
    view: Option<ScopeView>,
    scale: Option<ScaleState>,
}

impl ScopeElm {
    /// A scope being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut s = ScopeElm { base: Base::new(x, y), source: None, view: None, scale: None };
        s.base.x2 = x + 128;
        s.base.y2 = y + 64;
        s.set_points();
        s
    }

    /// A scope read from a text line: its settings are the one token that follows.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut s = ScopeElm {
            base: Base::from_dump(xa, ya, xb, yb, f),
            source: st.next_token().map(|t| ScopeSource::Line(t.to_string())),
            view: None,
            scale: None,
        };
        s.set_points();
        s
    }
}

impl Element for ScopeElm {
    impl_element_base!("ScopeElm");

    fn dump_type(&self) -> i32 {
        403
    }
    fn dump(&self) -> String {
        match &self.source {
            Some(ScopeSource::Line(l)) => format!("{} {l}", self.base.dump_prefix(403)),
            _ => self.base.dump_prefix(403),
        }
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        match &self.source {
            Some(ScopeSource::Xml(n)) => {
                out.child(n.to_out());
            }
            Some(ScopeSource::Line(l)) => {
                // settings read from the text format are kept in the same field order
                out.str_attr("sc", l);
            }
            None => {}
        }
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        if let Some(o) = xml.children.iter().find(|c| c.tag == "o") {
            self.source = Some(ScopeSource::Xml(o.clone()));
        } else if let Some(l) = xml.attr("sc") {
            self.source = Some(ScopeSource::Line(l.to_string()));
        }
    }

    fn scope_source(&self) -> Option<ScopeSource> {
        self.source.clone()
    }
    fn scope_view(&self) -> Option<&ScopeView> {
        self.view.as_ref()
    }
    fn set_scope_source(&mut self, source: ScopeSource) {
        self.source = Some(source);
        // the new settings start from a fresh scale
        self.scale = None;
    }
    fn set_scope_view(&mut self, view: ScopeView) {
        if self.scale.is_none() {
            self.scale = Some(ScaleState { values: view.scale, negative: false });
        }
        self.view = Some(view);
    }

    fn post_count(&self) -> usize {
        0
    }
    fn can_view_in_scope(&self) -> bool {
        false
    }
    fn num_handles(&self) -> usize {
        2
    }
    fn reset(&mut self) {
        self.view = None;
        self.scale = None;
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let b = &self.base;
        let (x, y) = (b.x.min(b.x2), b.y.min(b.y2));
        let (w, h) = (f64::from((b.x2 - b.x).abs()), f64::from((b.y2 - b.y).abs()));
        if let (Some(view), Some(scale)) = (&self.view, &mut self.scale) {
            let spot = crate::render::scopes::ScopeSpot {
                x: f64::from(x),
                y: f64::from(y),
                w,
                h,
                slot: 0,
                cursor: None,
                cursor_y: None,
                selected: None,
                measure_from: None,
            };
            crate::render::scopes::draw_scope(g, view, scale, &spot, c.printable);
        }
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
        g.set_line_width(1.0);
        g.stroke_rect(f64::from(x), f64::from(y), w, h);
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, 0.0);
        draw_posts(g, &self.base, self, c);
    }
}

/// Registers the scope element. It is made from the scope menu, not the Draw menu.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("ScopeElm", FAMILY, "Add Scope", "Outputs and Labels", |x, y| {
            Box::new(ScopeElm::new(x, y))
        })
        .dump(403, |a, b, c, d, f, st| Ok(Box::new(ScopeElm::from_dump(a, b, c, d, f, st))))
        .hidden(),
    );
}
