// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (OutputElm, TextElm, BoxElm,
// LabeledNodeElm, ProbeElm).
//! Outputs, labels and annotations: analog output, text, box, labeled node, voltmeter probe.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::{TokenError, Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale::{self, ls};
use crate::render::painter::{Color, Painter, TextAlign, TextBaseline};
use crate::solver::stamper::Stamper;

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "outputs";
const CAT: &str = "Outputs and Labels";

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("OutputElm", FAMILY, "Add Analog Output", CAT, |x, y| {
            Box::new(OutputElm::new(x, y))
        })
        .dump(i32::from(b'O'), |a, b, c, d, f, st| {
            Ok(Box::new(OutputElm::from_dump(a, b, c, d, f, st)))
        }),
    );
    r.add(
        ElementDescriptor::new("TextElm", FAMILY, "Add Text", CAT, |x, y| {
            Box::new(TextElm::new(x, y))
        })
        .dump(i32::from(b'x'), |a, b, c, d, f, st| {
            Ok(Box::new(TextElm::from_dump(a, b, c, d, f, st)?))
        })
        .shortcut('t'),
    );
    r.add(
        ElementDescriptor::new("BoxElm", FAMILY, "Add Box", CAT, |x, y| {
            Box::new(BoxElm::new(x, y))
        })
        .dump(i32::from(b'b'), |a, b, c, d, f, _| Ok(Box::new(BoxElm::from_dump(a, b, c, d, f)))),
    );
    r.add(
        ElementDescriptor::new("LabeledNodeElm", FAMILY, "Add Labeled Node", CAT, |x, y| {
            Box::new(LabeledNodeElm::new(x, y))
        })
        .dump(207, |a, b, c, d, f, st| Ok(Box::new(LabeledNodeElm::from_dump(a, b, c, d, f, st))))
        .shortcut('b'),
    );
    r.add(
        ElementDescriptor::new("ProbeElm", FAMILY, "Add Voltmeter/Scope Probe", CAT, |x, y| {
            Box::new(ProbeElm::new(x, y))
        })
        .dump(i32::from(b'p'), |a, b, c, d, f, st| {
            Ok(Box::new(ProbeElm::from_dump(a, b, c, d, f, st)))
        }),
    );
}

// ---------------------------------------------------------------------------------------------
// OutputElm
// ---------------------------------------------------------------------------------------------

const OUT_FLAG_VALUE: i32 = 1;
const OUT_FLAG_FIXED: i32 = 2;

/// A one-terminal analog output marker that can show its voltage.
#[derive(Clone, Debug)]
pub struct OutputElm {
    base: Base,
    /// Display scale index (0 auto, 1 volts, 2 millivolts, 3 microvolts).
    pub scale: i32,
}

impl OutputElm {
    /// An output being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        OutputElm { base: Base::new(xx, yy), scale: 0 }
    }

    /// An output read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        OutputElm { base: Base::from_dump(xa, ya, xb, yb, f), scale: st.next_i32().unwrap_or(0) }
    }

    fn show_voltage(&self) -> bool {
        self.base.flags & OUT_FLAG_VALUE != 0
    }
    fn is_fixed(&self) -> bool {
        self.base.flags & OUT_FLAG_FIXED != 0
    }
}

impl Element for OutputElm {
    impl_element_base!("OutputElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'O')
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.scale = xml.int_attr("sc", self.scale);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("sc", self.scale);
    }
    fn post_count(&self) -> usize {
        1
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        let selected = self.base.needs_highlight();
        g.set_font_size(14);
        g.set_color(if selected { c.select_color } else { c.light_gray });
        let s = if self.show_voltage() {
            unit_text_with_scale(
                self.base.volts[0],
                "V",
                Scale::from_index(self.scale),
                self.is_fixed(),
            )
        } else {
            ls("out")
        };
        let w = g.measure_text(&s) as i32;
        let dn = self.base.dn;
        self.base.lead1 =
            interp_point(self.base.point1, self.base.point2, 1.0 - (f64::from(w / 2 + 8)) / dn);
        let (p1, l1) = (self.base.point1, self.base.lead1);
        self.base.set_bbox_pts(p1, l1, 0.0);
        let (x2, y2) = (self.base.x2, self.base.y2);
        draw_centered_text(g, &mut self.base, &s, x2, y2, true);
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        if selected {
            g.set_color(c.select_color);
        }
        draw_thick_line(g, self.base.point1, self.base.lead1);
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("output".into());
        arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::checkbox("Show Voltage", self.show_voltage())),
            _ if !self.show_voltage() => None,
            1 => Some(EditInfo::choice(
                "Scale",
                vec!["Auto".into(), "V".into(), "mV".into(), format!("{}V", locale::MU)],
                self.scale.max(0) as usize,
            )),
            2 if self.scale != 0 => Some(EditInfo::checkbox("Fixed Precision", self.is_fixed())),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                self.base.flags = ei.change_flag(self.base.flags, OUT_FLAG_VALUE);
                ei.new_dialog = true;
            }
            1 => {
                self.scale = ei.choice_index as i32;
                ei.new_dialog = true;
            }
            2 => self.base.flags = ei.change_flag(self.base.flags, OUT_FLAG_FIXED),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// TextElm
// ---------------------------------------------------------------------------------------------

const TEXT_FLAG_BAR: i32 = 2;
const TEXT_FLAG_ESCAPE: i32 = 4;

/// A block of text on the drawing.
#[derive(Clone, Debug)]
pub struct TextElm {
    base: Base,
    /// The text; a backslash followed by `n` starts a new line.
    pub text: String,
    lines: Vec<String>,
    /// Font size in pixels.
    pub size: i32,
    /// Colour as `#rrggbb`, or none for the default.
    pub color: Option<String>,
}

fn split_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => {
                    lines.push(std::mem::take(&mut cur));
                }
                Some(other) => cur.push(other),
                None => {}
            }
        } else {
            cur.push(ch);
        }
    }
    lines.push(cur);
    lines
}

impl TextElm {
    /// A text being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        TextElm {
            base: Base::new(xx, yy),
            text: "hello".into(),
            lines: vec!["hello".into()],
            size: 24,
            color: None,
        }
    }

    /// A text read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let size = st.next_i32()?;
        let mut text = st.next_token().ok_or(TokenError::Missing)?.to_string();
        if f & TEXT_FLAG_ESCAPE == 0 {
            while let Some(t) = st.next_token() {
                text.push(' ');
                text.push_str(t);
            }
            text = text.replace("%2b", "+").replace("%2B", "+");
        } else {
            text = unescape(&text);
        }
        let lines = split_lines(&text);
        Ok(TextElm { base: Base::from_dump(xa, ya, xb, yb, f), text, lines, size, color: None })
    }
}

impl Element for TextElm {
    impl_element_base!("TextElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'x')
    }
    fn dump(&self) -> String {
        let mut b = self.base.clone();
        b.flags |= TEXT_FLAG_ESCAPE;
        format!("{} {} {}", b.dump_prefix(self.dump_type()), self.size, escape(&self.text))
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.size = xml.int_attr("si", self.size);
        self.text = xml.string_attr("te", &self.text);
        self.color = xml.attr("co").map(str::to_string).or(self.color.take());
        self.lines = split_lines(&self.text);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("si", self.size);
        out.str_attr("te", &self.text);
        if let Some(c) = &self.color {
            out.str_attr("co", c);
        }
    }
    fn post_count(&self) -> usize {
        0
    }
    fn is_graphic(&self) -> bool {
        true
    }
    fn drag(&mut self, xx: i32, yy: i32) {
        let b = &mut self.base;
        b.x = xx;
        b.y = yy;
        b.x2 = xx + 16;
        b.y2 = yy;
        self.set_points();
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        let color = self.color.as_deref().and_then(Color::parse_hex).unwrap_or(c.light_gray);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { color });
        g.set_font_size(self.size);
        let (x, y) = (self.base.x, self.base.y);
        let mut cury = y;
        self.base.set_bbox(x, y, x, y);
        for line in self.lines.clone() {
            let s = ls(&line);
            let sw = g.measure_text(&s) as i32;
            g.draw_string(&s, f64::from(x), f64::from(cury));
            let fs = g.font_size();
            if self.base.flags & TEXT_FLAG_BAR != 0 {
                let by = cury - fs;
                g.draw_line(x, by, x + sw - 1, by);
            }
            self.base.adjust_bbox(x, cury - fs, x + sw, cury + 3);
            cury += fs + 3;
        }
        self.base.x2 = self.base.bbox.x + self.base.bbox.width;
        self.base.y2 = self.base.bbox.y + self.base.bbox.height;
        g.restore();
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::text("Text", &self.text.replace("\\n", "\n"))),
            1 => Some(EditInfo::new("Size", f64::from(self.size), 5.0, 100.0)),
            2 => Some(EditInfo::checkbox("Draw Bar On Top", self.base.flags & TEXT_FLAG_BAR != 0)),
            3 => Some(
                EditInfo::text("Color", self.color.as_deref().unwrap_or(&Color::LIGHT_GRAY.hex()))
                    .set_is_color(),
            ),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                self.text = ei.text.clone().unwrap_or_default().replace('\n', "\\n");
                self.lines = split_lines(&self.text);
            }
            1 => self.size = ei.value as i32,
            2 => self.base.flags = ei.change_flag(self.base.flags, TEXT_FLAG_BAR),
            3 => {
                let c = ei.text.clone().unwrap_or_default();
                self.color = (c != Color::LIGHT_GRAY.hex()).then_some(c);
            }
            _ => {}
        }
    }
    fn elm_type(&self) -> Option<String> {
        Some("text".into())
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(self.text.clone());
    }
    fn shortcut(&self) -> i32 {
        i32::from(b't')
    }
    fn can_view_in_scope(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------------------------
// BoxElm
// ---------------------------------------------------------------------------------------------

/// A dashed rectangle used to group parts of a drawing.
#[derive(Clone, Debug)]
pub struct BoxElm {
    base: Base,
}

impl BoxElm {
    /// A box being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut base = Base::new(xx, yy);
        base.x2 = xx;
        base.y2 = yy;
        base.init_bounding_box();
        BoxElm { base }
    }

    /// A box read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32) -> Self {
        BoxElm { base: Base::from_dump(xa, ya, xb, yb, f) }
    }
}

impl Element for BoxElm {
    impl_element_base!("BoxElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'b')
    }
    fn post_count(&self) -> usize {
        0
    }
    fn is_graphic(&self) -> bool {
        true
    }
    fn drag(&mut self, xx: i32, yy: i32) {
        self.base.x2 = xx;
        self.base.y2 = yy;
        self.set_points();
    }
    fn creation_failed(&self) -> bool {
        let b = &self.base;
        (b.x2 - b.x).abs() < 32 || (b.y2 - b.y).abs() < 32
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.set_color(if self.base.needs_highlight() { c.select_color } else { Color::GRAY });
        let b = &mut self.base;
        let (x, y, x2, y2) = (b.x, b.y, b.x2, b.y2);
        b.set_bbox(x, y, x2, y2);
        g.set_line_dash(16, 6);
        g.draw_rect(x.min(x2), y.min(y2), (x2 - x).abs(), (y2 - y).abs());
        g.set_line_dash(0, 0);
    }

    fn mouse_distance(&self, gx: i32, gy: i32) -> i32 {
        let b = &self.base;
        let thresh = 10;
        let (dx1, dy1) = ((gx - b.x).abs(), (gy - b.y).abs());
        let (dx2, dy2) = ((gx - b.x2).abs(), (gy - b.y2).abs());
        if dx1 < thresh {
            return dx1 * dx1;
        }
        if dx2 < thresh {
            return dx2 * dx2;
        }
        if dy1 < thresh {
            return dy1 * dy1;
        }
        if dy2 < thresh {
            return dy2 * dy2;
        }
        -1
    }

    fn selected_by_rect(&self, r: &Rect) -> bool {
        let bb = &self.base.bbox;
        r.contains(bb.x, bb.y) && r.contains(bb.x + bb.width - 1, bb.y + bb.height - 1)
    }
    fn shortcut(&self) -> i32 {
        0
    }
}

// ---------------------------------------------------------------------------------------------
// LabeledNodeElm
// ---------------------------------------------------------------------------------------------

const LN_FLAG_ESCAPE: i32 = 4;
const LN_FLAG_INTERNAL: i32 = 1;
const LN_FLAG_ROTATE_TEXT: i32 = 8;
const LN_CIRCLE_SIZE: f64 = 17.0;

/// A named node: all labeled nodes with the same text are connected.
#[derive(Clone, Debug)]
pub struct LabeledNodeElm {
    base: Base,
    /// The node's name.
    pub text: String,
    /// Number of bits when the label names a bus (found by the bus detection).
    bus_width: usize,
    /// Current into each bit of a bus label.
    currents: Option<Vec<f64>>,
}

impl LabeledNodeElm {
    /// A labeled node being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        LabeledNodeElm {
            base: Base::new(xx, yy),
            text: "label".into(),
            bus_width: 1,
            currents: None,
        }
    }

    fn bus_value(&self) -> i32 {
        (0..self.bus_width).filter(|&i| self.base.volts[i] > 2.5).fold(0, |v, i| v | (1 << i))
    }

    /// A labeled node read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut text = st.next_token().unwrap_or("").to_string();
        if f & LN_FLAG_ESCAPE == 0 {
            while let Some(t) = st.next_token() {
                text.push(' ');
                text.push_str(t);
            }
        } else {
            text = unescape(&text);
        }
        LabeledNodeElm {
            base: Base::from_dump(xa, ya, xb, yb, f),
            text,
            bus_width: 1,
            currents: None,
        }
    }

    fn is_rotate_text(&self) -> bool {
        self.base.flags & LN_FLAG_ROTATE_TEXT != 0
    }

    fn draw_rotated_label(&mut self, g: &mut dyn Painter, c: &DrawCtx, pt1: Point, pt2: Point) {
        let (line_over, text) = match self.text.strip_prefix('/') {
            Some(rest) => (true, rest.to_string()),
            None => (false, self.text.clone()),
        };
        g.save();
        g.set_font_size(c.value_font_size);
        let w = g.measure_text(&text) as i32;
        let h = g.font_size();
        g.set_text_baseline(TextBaseline::Middle);
        let dir = sign(pt2.y - pt1.y);
        let offset = h + (w / 2 - h).max(0);
        let (tx, ty) = (pt2.x, pt2.y + dir * offset);
        g.translate(f64::from(tx), f64::from(ty));
        g.rotate(-std::f64::consts::FRAC_PI_2);
        g.set_text_align(TextAlign::Center);
        g.draw_string(&text, 0.0, 0.0);
        self.base.adjust_bbox(tx - h / 2, ty - w / 2, tx + h / 2, ty + w / 2);
        g.restore();
        if line_over {
            let xa = -h / 2 - 1;
            g.save();
            g.translate(f64::from(tx), f64::from(ty));
            g.rotate(-std::f64::consts::FRAC_PI_2);
            g.draw_line(-w / 2, xa, w / 2, xa);
            g.restore();
        }
    }
}

impl Element for LabeledNodeElm {
    impl_element_base!("LabeledNodeElm");

    fn dump_type(&self) -> i32 {
        207
    }
    fn xml_dump_type(&self) -> String {
        "ln".into()
    }
    fn dump(&self) -> String {
        let mut b = self.base.clone();
        b.flags |= LN_FLAG_ESCAPE;
        format!("{} {}", b.dump_prefix(self.dump_type()), escape(&self.text))
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.text = xml.string_attr("te", &self.text);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.str_attr("te", &self.text);
    }
    fn post_count(&self) -> usize {
        self.bus_width
    }
    fn post_width(&self, _n: usize) -> usize {
        self.bus_width
    }
    fn bus_width(&self) -> usize {
        self.bus_width
    }
    fn bus_role(&self) -> BusRole {
        BusRole::Label(self.text.clone())
    }
    fn apply_bus_width(&mut self, width: usize) -> bool {
        if width == self.bus_width {
            return false;
        }
        self.bus_width = width;
        self.currents = (width > 1).then(|| vec![0.0; width]);
        self.alloc_nodes();
        true
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        let p = self.base.point1;
        Some(if self.bus_width == 1 { p } else { Point { x: p.x, y: p.y, z: n as i32 } })
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.lead1 =
            interp_point(self.base.point1, self.base.point2, 1.0 - LN_CIRCLE_SIZE / self.base.dn);
    }

    fn connected_post(&self, bit: usize, labels: &mut LabelRegistry) -> Option<Point> {
        let key =
            if self.bus_width > 1 { format!("{}:{bit}", self.text) } else { self.text.clone() };
        labels.connect(&key, self.get_post(bit)?)
    }
    fn is_wire_equivalent(&self) -> bool {
        true
    }
    fn is_removable_wire(&self) -> bool {
        true
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 == n2
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        let width = if self.bus_width > 1 { 5.0 } else { 3.0 };
        draw_thick_line_w(g, self.base.point1, self.base.lead1, width);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
        let (p1, p2) = (self.base.point1, self.base.point2);
        let ps2 = interp_point(p1, p2, 1.0 + 11.0 / self.base.dn);
        self.base.set_bbox_pts(p1, ps2, LN_CIRCLE_SIZE);
        let (pt1, pt2) = (self.base.point1, self.base.lead1);
        let text = self.text.clone();
        if self.is_rotate_text() && pt1.x == pt2.x {
            self.draw_rotated_label(g, c, pt1, pt2);
        } else {
            draw_labeled_node(g, &mut self.base, c, &text, pt1, pt2);
        }
        if let Some(cur) = &self.currents {
            self.base.current = cur.iter().sum();
        }
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        match &self.currents {
            Some(c) => -c.get(n).copied().unwrap_or(0.0),
            None => -self.base.current,
        }
    }
    fn set_wire_current(&mut self, bit: usize, c: f64) {
        match &mut self.currents {
            Some(cur) => {
                if let Some(slot) = cur.get_mut(bit) {
                    *slot = c;
                }
            }
            None => self.base.current = c,
        }
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn elm_type(&self) -> Option<String> {
        Some("Labeled Node".into())
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(format!("{} ({})", ls(&self.text), ls("Labeled Node")));
        if self.bus_width > 1 {
            let value = self.bus_value();
            arr.push(format!("value = {value}"));
            arr.push(format!("hex = 0x{value:X}"));
        } else {
            arr.push(format!("I = {}", current_text(self.get_current())));
            arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
        }
    }
    fn scope_text(&self, _v: i32) -> Option<String> {
        Some(self.text.clone())
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::text("Text", &self.text)),
            1 => Some(EditInfo::checkbox("Internal Node", self.base.flags & LN_FLAG_INTERNAL != 0)),
            2 => Some(EditInfo::checkbox("Rotate Text When Vertical", self.is_rotate_text())),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.text = ei.text.clone().unwrap_or_default(),
            1 => self.base.flags = ei.change_flag(self.base.flags, LN_FLAG_INTERNAL),
            2 => self.base.flags = ei.change_flag(self.base.flags, LN_FLAG_ROTATE_TEXT),
            _ => {}
        }
    }
    fn shortcut(&self) -> i32 {
        i32::from(b'b')
    }
}

// ---------------------------------------------------------------------------------------------
// ProbeElm
// ---------------------------------------------------------------------------------------------

const TP_VOL: i32 = 0;
const TP_RMS: i32 = 1;
const TP_MAX: i32 = 2;
const TP_MIN: i32 = 3;
const TP_P2P: i32 = 4;
const TP_BIN: i32 = 5;
const TP_FRQ: i32 = 6;
const TP_PER: i32 = 7;
const TP_PWI: i32 = 8;
const TP_DUT: i32 = 9;
const TP_AVG: i32 = 10;
const PROBE_FLAG_SHOWVOLTAGE: i32 = 1;
const PROBE_FLAG_CIRCLE: i32 = 2;
const PROBE_CIRCLE_SIZE: i32 = 12;
/// Meter kinds offered in the edit dialog, in dialog order.
const METER_CHOICES: [i32; 7] = [TP_VOL, TP_RMS, TP_AVG, TP_MAX, TP_MIN, TP_P2P, TP_BIN];

/// A voltmeter probe that can also report RMS, average, peaks and more.
#[derive(Clone, Debug)]
pub struct ProbeElm {
    base: Base,
    meter: i32,
    scale: i32,
    resistance: f64,
    rms_v: f64,
    total: f64,
    count: f64,
    avg_v: f64,
    total_v: f64,
    binary_level: f64,
    zerocount: i32,
    max_v: f64,
    last_max_v: f64,
    min_v: f64,
    last_min_v: f64,
    frequency: f64,
    period: f64,
    pulse_width: f64,
    duty_cycle: f64,
    increasing_v: bool,
    decreasing_v: bool,
    started: bool,
    last_step_count: u64,
    period_start: f64,
    period_length: f64,
    pulse_start: f64,
    center: Point,
}

impl ProbeElm {
    fn blank(base: Base, resistance: f64) -> Self {
        ProbeElm {
            base,
            meter: TP_VOL,
            scale: 0,
            resistance,
            rms_v: 0.0,
            total: 0.0,
            count: 0.0,
            avg_v: 0.0,
            total_v: 0.0,
            binary_level: 0.0,
            zerocount: 0,
            max_v: 0.0,
            last_max_v: 0.0,
            min_v: 0.0,
            last_min_v: 0.0,
            frequency: 0.0,
            period: 0.0,
            pulse_width: 0.0,
            duty_cycle: 0.0,
            increasing_v: true,
            decreasing_v: true,
            started: false,
            last_step_count: 0,
            period_start: 0.0,
            period_length: 0.0,
            pulse_start: 0.0,
            center: Point::default(),
        }
    }

    /// A probe being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut base = Base::new(xx, yy);
        base.flags = PROBE_FLAG_SHOWVOLTAGE | PROBE_FLAG_CIRCLE;
        Self::blank(base, 1e7)
    }

    /// A probe read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut p = Self::blank(Base::from_dump(xa, ya, xb, yb, f), 0.0);
        // these three fields are optional in older files; a missing one keeps its default
        if let Ok(m) = st.next_i32() {
            p.meter = m;
            if let Ok(s) = st.next_i32() {
                p.scale = s;
                if let Ok(r) = st.next_f64() {
                    p.resistance = r;
                }
            }
        }
        p
    }

    fn must_show_voltage(&self) -> bool {
        self.base.flags & PROBE_FLAG_SHOWVOLTAGE != 0
    }
    fn draw_as_circle(&self) -> bool {
        self.base.flags & PROBE_FLAG_CIRCLE != 0
    }

    fn meter_line(&self, m: i32) -> String {
        match m {
            TP_VOL => format!("Vd = {}", voltage_text(self.get_voltage_diff())),
            TP_RMS => format!("V(rms) = {}", voltage_text(self.rms_v)),
            TP_AVG => format!("V(avg) = {}", voltage_text(self.avg_v)),
            TP_MAX => format!("Vmax = {}", voltage_text(self.last_max_v)),
            TP_MIN => format!("Vmin = {}", voltage_text(self.last_min_v)),
            TP_P2P => format!("Vp2p = {}", voltage_text(self.last_max_v - self.last_min_v)),
            TP_BIN => format!("Binary = {}", self.binary_level),
            TP_FRQ => format!("Freq = {}", unit_text(self.frequency, "Hz")),
            TP_PER => format!("Period = {}", unit_text(self.period, "s")),
            TP_PWI => format!("Pulse width = {}", unit_text(self.pulse_width, "s")),
            TP_DUT => format!("Duty cycle = {}", format(self.duty_cycle, false)),
            _ => String::new(),
        }
    }

    fn closed_cycle(&mut self) {
        self.total /= self.count;
        self.rms_v = self.total.sqrt();
        if self.rms_v.is_nan() {
            self.rms_v = 0.0;
        }
        self.avg_v = self.total_v / self.count;
        if self.avg_v.is_nan() {
            self.avg_v = 0.0;
        }
        self.count = 0.0;
        self.total = 0.0;
        self.total_v = 0.0;
    }
}

impl Element for ProbeElm {
    impl_element_base!("ProbeElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'p')
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            self.meter,
            self.scale,
            js_number(self.resistance)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        // the original clears the flags first, so an absent `f` means no flags
        self.base.flags = xml.int_attr("f", 0);
        self.meter = xml.int_attr("me", self.meter);
        self.scale = xml.int_attr("sc", self.scale);
        self.resistance = xml.double_attr("re", 0.0);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("me", self.meter);
        out.int_attr("sc", self.scale);
        out.double_attr("re", self.resistance);
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.current = 0.0;
        self.base.curcount = 0.0;
        self.zerocount = 0;
        self.rms_v = 0.0;
        self.total = 0.0;
        self.count = 0.0;
        self.avg_v = 0.0;
        self.total_v = 0.0;
        self.max_v = 0.0;
        self.last_max_v = 0.0;
        self.min_v = 0.0;
        self.last_min_v = 0.0;
        self.binary_level = 0.0;
        self.period = 0.0;
        self.pulse_width = 0.0;
        self.duty_cycle = 0.0;
        self.period_start = 0.0;
        self.period_length = 0.0;
        self.pulse_start = 0.0;
        self.increasing_v = true;
        self.decreasing_v = true;
        self.started = false;
        self.last_step_count = 0;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.center = interp_point(self.base.point1, self.base.point2, 0.5);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        let show_circle = self.draw_as_circle();
        let hs = if show_circle { PROBE_CIRCLE_SIZE } else { 8 };
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
        let selected = self.base.needs_highlight();
        let mut len = if selected || self.base.creating || self.must_show_voltage() {
            16.0
        } else {
            self.base.dn - 32.0
        };
        if show_circle {
            len = f64::from(PROBE_CIRCLE_SIZE * 2);
        }
        self.base.calc_leads(len as i32);
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        if selected {
            g.set_color(c.select_color);
        }
        draw_thick_line(g, p1, self.base.lead1);
        set_voltage_color(g, &self.base, c, self.base.volts[1]);
        if selected {
            g.set_color(c.select_color);
        }
        draw_thick_line(g, self.base.lead2, p2);
        g.set_font_size(14);
        if self.must_show_voltage() {
            let sc = Scale::from_index(self.scale);
            let s = match self.meter {
                TP_VOL => unit_text_with_scale(self.get_voltage_diff(), "V", sc, false),
                TP_RMS => unit_text_with_scale(self.rms_v, "V(rms)", sc, false),
                TP_AVG => unit_text_with_scale(self.avg_v, "V(avg)", sc, false),
                TP_MAX => unit_text_with_scale(self.last_max_v, "Vpk", sc, false),
                TP_MIN => unit_text_with_scale(self.last_min_v, "Vmin", sc, false),
                TP_P2P => {
                    unit_text_with_scale(self.last_max_v - self.last_min_v, "Vp2p", sc, false)
                }
                TP_BIN => format!("{}", self.binary_level),
                TP_FRQ => unit_text(self.frequency, "Hz"),
                TP_PWI => unit_text(self.pulse_width, "s"),
                TP_DUT => format(self.duty_cycle, false),
                _ => String::new(),
            };
            let hs = if show_circle { f64::from(PROBE_CIRCLE_SIZE + 3) } else { 4.0 };
            draw_values(g, &self.base, c, Some(&s), hs, false, false);
        }
        g.set_color(c.white);
        g.set_font_size(c.value_font_size);
        let b = &self.base;
        let mut plus = interp_point_g(
            b.point1,
            b.point2,
            (b.dn / 2.0 - len / 2.0 - 4.0) / b.dn,
            f64::from(-10 * b.dsign),
        );
        if b.y2 > b.y {
            plus.y += 4;
        }
        if b.y > b.y2 {
            plus.y += 3;
        }
        let w = g.measure_text("+") as i32;
        g.draw_string("+", f64::from(plus.x - w / 2), f64::from(plus.y));
        if show_circle {
            g.set_color(c.light_gray);
            draw_thick_circle(g, self.center.x, self.center.y, PROBE_CIRCLE_SIZE);
            let (cx, cy) = (self.center.x, self.center.y);
            draw_centered_text(g, &mut self.base, "V", cx, cy, true);
        }
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if s.time_step_count == self.last_step_count {
            return;
        }
        self.last_step_count = s.time_step_count;
        self.count += 1.0;
        let v = self.get_voltage_diff();
        self.total += v * v;
        self.total_v += v;
        self.binary_level = if v < 2.5 { 0.0 } else { 1.0 };
        if !self.started {
            self.started = true;
            self.max_v = v;
            self.min_v = v;
            self.increasing_v = true;
            self.decreasing_v = false;
            self.period_start = s.t;
            self.pulse_start = s.t;
        }
        if v > self.max_v && self.increasing_v {
            self.max_v = v;
            self.increasing_v = true;
            self.decreasing_v = false;
        }
        if v < self.max_v && self.increasing_v {
            self.last_max_v = self.max_v;
            self.period_length = s.t - self.period_start;
            self.period_start = s.t;
            self.period = self.period_length;
            self.pulse_width = s.t - self.pulse_start;
            self.duty_cycle = self.pulse_width / self.period_length;
            self.min_v = v;
            self.increasing_v = false;
            self.decreasing_v = true;
            self.closed_cycle();
        }
        if v < self.min_v && self.decreasing_v {
            self.min_v = v;
            self.increasing_v = false;
            self.decreasing_v = true;
        }
        if v > self.min_v && self.decreasing_v {
            self.last_min_v = self.min_v;
            self.pulse_start = s.t;
            self.max_v = v;
            self.increasing_v = true;
            self.decreasing_v = false;
            self.closed_cycle();
        }
        if v == 0.0 {
            self.zerocount += 1;
            if self.zerocount > 5 {
                self.total = 0.0;
                self.rms_v = 0.0;
                self.avg_v = 0.0;
                self.max_v = 0.0;
                self.min_v = 0.0;
            }
        } else {
            self.zerocount = 0;
        }
    }

    fn calculate_current(&mut self) {
        let b = &self.base;
        self.base.current =
            if self.resistance == 0.0 { 0.0 } else { (b.volts[0] - b.volts[1]) / self.resistance };
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if self.resistance != 0.0 {
            s.stamp_resistor(self.base.nodes[0], self.base.nodes[1], self.resistance);
        }
    }

    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        self.resistance != 0.0
    }

    fn elm_type(&self) -> Option<String> {
        Some("voltmeter".into())
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("voltmeter".into());
        arr.push(self.meter_line(self.meter));
        for m in [TP_VOL, TP_MAX, TP_MIN, TP_RMS, TP_AVG, TP_P2P, TP_PER, TP_PWI, TP_DUT] {
            if self.meter != m {
                arr.push(self.meter_line(m));
            }
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::checkbox("Show Voltage", self.must_show_voltage())),
            1 => {
                let choices = [
                    "Voltage",
                    "RMS Voltage",
                    "Average Voltage",
                    "Max Voltage",
                    "Min Voltage",
                    "P2P Voltage",
                    "Binary Value",
                ];
                let selected = METER_CHOICES.iter().position(|m| *m == self.meter).unwrap_or(0);
                Some(EditInfo::choice(
                    "Value",
                    choices.iter().map(|s| (*s).into()).collect(),
                    selected,
                ))
            }
            2 => Some(EditInfo::choice(
                "Scale",
                vec!["Auto".into(), "V".into(), "mV".into(), format!("{}V", locale::MU)],
                self.scale.max(0) as usize,
            )),
            3 => Some(EditInfo::checkbox("Use Circle Symbol", self.draw_as_circle())),
            4 => Some(EditInfo::new("Series Resistance (0 = infinite)", self.resistance, 0.0, 0.0)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                if ei.checkbox.unwrap_or(false) {
                    self.base.flags = PROBE_FLAG_SHOWVOLTAGE;
                } else {
                    self.base.flags &= !PROBE_FLAG_SHOWVOLTAGE;
                }
            }
            1 => self.meter = METER_CHOICES[ei.choice_index.min(METER_CHOICES.len() - 1)],
            2 => self.scale = ei.choice_index as i32,
            3 => self.base.flags = ei.change_flag(self.base.flags, PROBE_FLAG_CIRCLE),
            4 => self.resistance = ei.value,
            _ => {}
        }
    }
    fn can_view_in_scope(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(s: &str) -> Tokenizer<'_> {
        Tokenizer::new(s)
    }

    #[test]
    fn text_splits_on_backslash_n_and_round_trips_escaped() {
        let t =
            TextElm::from_dump(0, 0, 16, 0, TEXT_FLAG_ESCAPE, &mut tok("12 a\\sb\\\\nc")).unwrap();
        assert_eq!(t.text, "a b\\nc");
        assert_eq!(t.lines, vec!["a b", "c"]);
        let d = t.dump();
        assert!(d.starts_with("x 0 0 16 0 4 12 "), "{d}");
        let mut st = Tokenizer::for_circuit_line(&d);
        for _ in 0..6 {
            st.next_token();
        }
        let back = TextElm::from_dump(0, 0, 16, 0, 4, &mut st).unwrap();
        assert_eq!(back.text, t.text);
    }

    #[test]
    fn old_text_lines_join_the_remaining_tokens_and_decode_plus() {
        let t = TextElm::from_dump(0, 0, 16, 0, 0, &mut tok("24 a b%2Bc")).unwrap();
        assert_eq!(t.text, "a b+c");
    }

    #[test]
    fn labeled_node_text_survives_the_text_format() {
        let e = LabeledNodeElm::from_dump(0, 0, 32, 0, LN_FLAG_ESCAPE, &mut tok("q\\sr"));
        assert_eq!(e.text, "q r");
        assert!(e.dump().ends_with("q\\sr"));
        let mut labels = LabelRegistry::default();
        assert_eq!(e.connected_post(0, &mut labels), None);
        assert_eq!(e.connected_post(0, &mut labels), Some(Point::new(0, 0)));
    }

    #[test]
    fn box_needs_32_pixels_each_way_and_is_hit_near_its_edges() {
        let mut b = BoxElm::new(0, 0);
        b.drag(20, 100);
        assert!(b.creation_failed());
        b.drag(64, 100);
        assert!(!b.creation_failed());
        assert!(b.mouse_distance(3, 50) >= 0);
        assert_eq!(b.mouse_distance(30, 50), -1);
    }

    #[test]
    fn probe_reports_rms_of_a_square_wave() {
        let mut p = ProbeElm::new(0, 0);
        p.base.x2 = 64;
        p.set_points();
        p.base.alloc_nodes(2);
        let mut st = Stamper::default();
        for step in 1..=200_u64 {
            st.time_step_count = step;
            st.t = step as f64 * 1e-3;
            p.base.volts = vec![if (step / 20) % 2 == 0 { 5.0 } else { 0.0 }, 0.0];
            p.step_finished(&mut st);
        }
        assert!((p.last_max_v - 5.0).abs() < 1e-9);
        assert!(p.rms_v > 0.0 && p.rms_v < 5.0 + 1e-9);
        assert!(p.meter_line(TP_MAX).contains("Vmax"));
    }

    #[test]
    fn output_scale_choices_map_to_scales() {
        assert_eq!(Scale::from_index(2), Scale::Milli);
        let mut o = OutputElm::new(0, 0);
        assert!(o.get_edit_info(1).is_none());
        let mut ei = o.get_edit_info(0).unwrap();
        ei.checkbox = Some(true);
        o.set_edit_value(0, &mut ei);
        assert!(o.get_edit_info(1).is_some() && o.get_edit_info(2).is_none());
    }
}
