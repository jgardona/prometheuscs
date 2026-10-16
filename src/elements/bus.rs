// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (BusLogicInputElm,
// InstructionDisplayElm).
//! Bus input and the instruction display: elements whose posts are the bits of a bus, all at
//! one point.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use crate::expr::{Expr, ExprState};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::Painter;
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name used for the feature switch.
pub const FAMILY: &str = "digital";
const CAT: &str = "Digital Chips";

/// Registers the bus elements.
pub fn register(r: &mut Registry) {
    r.add(ElementDescriptor::new(
        "BusLogicInputElm",
        FAMILY,
        "Add Bus Logic Input",
        CAT,
        |x, y| Box::new(BusLogicInputElm::new(x, y)),
    ));
    r.add(ElementDescriptor::new(
        "InstructionDisplayElm",
        FAMILY,
        "Add Instruction Display",
        CAT,
        |x, y| Box::new(InstructionDisplayElm::new(x, y)),
    ));
}

fn bus_post(b: &Base, n: usize) -> Point {
    Point { x: b.x, y: b.y, z: n as i32 }
}

// ---------------------------------------------------------------------------------------------
// Bus logic input
// ---------------------------------------------------------------------------------------------

/// A bus whose value is set by clicking: each click adds one.
#[derive(Clone, Debug)]
pub struct BusLogicInputElm {
    base: Base,
    bus_width: usize,
    value: i64,
    hi: f64,
    lo: f64,
    vs: Vec<Option<VsId>>,
    currents: Vec<f64>,
}

impl BusLogicInputElm {
    /// An input being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut base = Base::new(xx, yy);
        base.no_diagonal = true;
        let mut e = BusLogicInputElm {
            base,
            bus_width: 4,
            value: 0,
            hi: 5.0,
            lo: 0.0,
            vs: Vec::new(),
            currents: Vec::new(),
        };
        e.resize();
        e
    }

    fn resize(&mut self) {
        self.vs = vec![None; self.bus_width];
        self.currents = vec![0.0; self.bus_width];
        self.alloc_nodes();
    }
}

impl Element for BusLogicInputElm {
    impl_element_base!("BusLogicInputElm");

    fn dump_type(&self) -> i32 {
        0
    }
    fn xml_dump_type(&self) -> String {
        "bli".into()
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.bus_width = xml.int_attr("bw", self.bus_width as i32).clamp(1, 32) as usize;
        self.value = i64::from(xml.int_attr("va", 0));
        self.hi = xml.double_attr("hi", self.hi);
        self.lo = xml.double_attr("lo", self.lo);
        self.resize();
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("bw", self.bus_width as i32);
        if self.value != 0 {
            out.int_attr("va", self.value as i32);
        }
        if self.hi != 5.0 {
            out.double_attr("hi", self.hi);
        }
        if self.lo != 0.0 {
            out.double_attr("lo", self.lo);
        }
    }

    fn post_count(&self) -> usize {
        self.bus_width
    }
    fn post_width(&self, _n: usize) -> usize {
        self.bus_width
    }
    fn num_handles(&self) -> usize {
        1
    }
    fn voltage_source_count(&self) -> usize {
        self.bus_width
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        (n < self.bus_width).then(|| bus_post(&self.base, n))
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &mut self.base;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - 12.0 / b.dn);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        g.set_font_size(20);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
        let s = self.value.to_string();
        let b = &mut self.base;
        let half = (g.measure_text(&s) / 2.0) as i32 + 8;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - f64::from(half) / b.dn);
        let (p1, l1, x2, y2) = (b.point1, b.lead1, b.x2, b.y2);
        b.set_bbox_pts(p1, l1, 0.0);
        draw_centered_text(g, b, &s, x2, y2, true);
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        draw_thick_line_w(g, p1, l1, 5.0);
        self.base.current = self.currents.iter().sum();
        self.base.update_dot_count(c);
        let cc = -self.base.curcount;
        draw_dots(g, c, p1, l1, cc);
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn click_rect(&self) -> Option<super::geometry::Rect> {
        Some(super::geometry::Rect::new(self.base.x2 - 10, self.base.y2 - 10, 20, 20))
    }
    fn simple_toggle(&mut self) {
        self.value += 1;
        if self.value >= 1 << self.bus_width {
            self.value = 0;
        }
    }

    fn set_voltage_source(&mut self, n: usize, v: VsId) {
        if let Some(slot) = self.vs.get_mut(n) {
            *slot = Some(v);
        }
    }
    fn voltage_source_nodes_at(&self, j: usize) -> Option<(NodeId, NodeId)> {
        self.base.nodes.get(j).map(|&n| (NodeId::GROUND, n))
    }
    fn set_current(&mut self, vs: VsId, c: f64) {
        if let Some(i) = self.vs.iter().position(|v| *v == Some(vs)) {
            self.currents[i] = c;
            self.base.current = c;
        }
    }
    fn stamp(&mut self, s: &mut Stamper) {
        for (i, vs) in self.vs.iter().enumerate() {
            if let Some(vs) = vs {
                let v = if self.value & (1 << i) != 0 { self.hi } else { self.lo };
                s.stamp_voltage_source(NodeId::GROUND, self.base.nodes[i], *vs, v);
            }
        }
    }
    fn calculate_current(&mut self) {}
    fn get_current_into_node(&self, n: usize) -> f64 {
        self.currents.get(n).copied().unwrap_or(0.0)
    }
    fn has_ground_connection(&self, _n: usize) -> bool {
        true
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(format!("bus input ({})", self.bus_width));
        arr.push(format!("value = {}", self.value));
        arr.push(format!("hex = 0x{:X}", self.value));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("Bus Width", self.bus_width as f64, 2.0, 32.0).set_dimensionless(),
            ),
            1 => Some(EditInfo::number("Value", self.value as f64).set_dimensionless()),
            2 => Some(EditInfo::number("High Voltage", self.hi).set_unit_step()),
            3 => Some(EditInfo::number("Low Voltage", self.lo).set_unit_step()),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                if ei.value >= 2.0 {
                    self.bus_width = (ei.value as usize).min(32);
                    self.resize();
                } else {
                    ei.error = Some("must be >= 2".into());
                }
            }
            1 => self.value = ei.value as i64,
            2 => self.hi = ei.value,
            3 => self.lo = ei.value,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Instruction display
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct LookupEntry {
    lo: i64,
    hi: i64,
    template: String,
}

impl LookupEntry {
    /// The text for `value`: `{expression}` parts are evaluated with `a` = the value.
    fn text(&self, value: i64) -> String {
        let mut out = String::new();
        let mut rest = self.template.as_str();
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let Some(close) = rest[open..].find('}').map(|c| c + open) else {
                out.push_str(&rest[open..]);
                return out;
            };
            let expr = &rest[open + 1..close];
            match Expr::parse(expr) {
                Ok(e) => {
                    let mut es = ExprState::new();
                    es.values[0] = value as f64;
                    let r = e.eval(&es);
                    if r == r.trunc() && r.abs() < 1e15 {
                        out.push_str(&(r as i64).to_string());
                    } else {
                        out.push_str(&crate::format::number::js_number(r));
                    }
                }
                Err(_) => {
                    out.push('{');
                    out.push_str(expr);
                    out.push('}');
                }
            }
            rest = &rest[close + 1..];
        }
        out.push_str(rest);
        out
    }
}

fn parse_number(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(h, 16).ok();
    }
    if let Some(b) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return i64::from_str_radix(b, 2).ok();
    }
    s.parse().ok()
}

fn find_dash(s: &str) -> Option<usize> {
    let start =
        if s.starts_with("0x") || s.starts_with("0X") || s.starts_with("0b") || s.starts_with("0B")
        {
            2
        } else {
            0
        };
    s[start..].find('-').map(|i| i + start)
}

/// Shows a text chosen by the value on a bus.
#[derive(Clone, Debug)]
pub struct InstructionDisplayElm {
    base: Base,
    bus_width: usize,
    threshold: f64,
    lookup_text: String,
    entries: Vec<LookupEntry>,
}

impl InstructionDisplayElm {
    /// A display being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut base = Base::new(xx, yy);
        base.no_diagonal = true;
        let mut e = InstructionDisplayElm {
            base,
            bus_width: 4,
            threshold: 2.5,
            lookup_text: "0=text0\n1=text1\n0x2-0xF=other ({a})\n".into(),
            entries: Vec::new(),
        };
        e.parse_entries().ok();
        e.alloc_nodes();
        e
    }

    /// Parses the lookup table; the error names the first bad line.
    fn parse_entries(&mut self) -> Result<(), String> {
        self.entries.clear();
        let mut error = None;
        for line in self.lookup_text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, val)) = line.split_once('=') else {
                error.get_or_insert(format!("missing =: {line}"));
                continue;
            };
            let key = key.trim();
            let entry = match find_dash(key) {
                Some(d) => parse_number(&key[..d]).zip(parse_number(&key[d + 1..])),
                None => parse_number(key).map(|k| (k, k)),
            };
            if let Some((lo, hi)) = entry {
                self.entries.push(LookupEntry { lo, hi, template: val.to_string() });
            }
        }
        error.map_or(Ok(()), Err)
    }

    fn read_value(&self) -> i64 {
        (0..self.bus_width)
            .filter(|&i| self.base.volts[i] > self.threshold)
            .fold(0, |v, i| v | (1 << i))
    }

    fn display_text(&self) -> String {
        let value = self.read_value();
        self.entries
            .iter()
            .find(|e| value >= e.lo && value <= e.hi)
            .map_or_else(|| value.to_string(), |e| e.text(value))
    }
}

impl Element for InstructionDisplayElm {
    impl_element_base!("InstructionDisplayElm");

    fn dump_type(&self) -> i32 {
        0
    }
    fn xml_dump_type(&self) -> String {
        "ins".into()
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.bus_width = xml.int_attr("bw", self.bus_width as i32).clamp(1, 32) as usize;
        self.threshold = xml.double_attr("th", self.threshold);
        self.lookup_text = xml.text.clone();
        self.parse_entries().ok();
        self.alloc_nodes();
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("bw", self.bus_width as i32);
        if self.threshold != 2.5 {
            out.double_attr("th", self.threshold);
        }
        if !self.lookup_text.is_empty() {
            out.text = Some(self.lookup_text.clone());
        }
    }

    fn post_count(&self) -> usize {
        self.bus_width
    }
    fn post_width(&self, _n: usize) -> usize {
        self.bus_width
    }
    fn num_handles(&self) -> usize {
        1
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        (n < self.bus_width).then(|| bus_post(&self.base, n))
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &mut self.base;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - 12.0 / b.dn);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        let selected = self.base.needs_highlight();
        g.set_font_size(14);
        g.set_color(if selected { c.select_color } else { c.light_gray });
        let s = self.display_text();
        let half = (g.measure_text(&s) / 2.0) as i32 + 8;
        let b = &mut self.base;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - f64::from(half) / b.dn);
        let (p1, l1, x2, y2) = (b.point1, b.lead1, b.x2, b.y2);
        b.set_bbox_pts(p1, l1, 0.0);
        draw_centered_text(g, b, &s, x2, y2, true);
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        if selected {
            g.set_color(c.select_color);
        }
        draw_thick_line_w(g, p1, l1, 5.0);
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let v = self.read_value();
        arr.push("instruction display".into());
        arr.push(format!("in = {v} (0x{v:X})"));
        arr.push(format!("text = {}", self.display_text()));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("Bus Width", self.bus_width as f64, 2.0, 32.0).set_dimensionless(),
            ),
            1 => Some(EditInfo::number("Threshold Voltage", self.threshold)),
            2 => Some(EditInfo::text_area("Lookup Table", &self.lookup_text)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                if (1.0..=32.0).contains(&ei.value) {
                    self.bus_width = ei.value as usize;
                    self.alloc_nodes();
                } else {
                    ei.error = Some("must be between 1 and 32".into());
                }
            }
            1 => self.threshold = ei.value,
            2 => {
                if let Some(t) = &ei.text {
                    self.lookup_text.clone_from(t);
                }
                if let Err(e) = self.parse_entries() {
                    ei.error = Some(e);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_picks_entries_and_evaluates_templates() {
        let mut d = InstructionDisplayElm::new(0, 0);
        let set = |d: &mut InstructionDisplayElm, v: usize| {
            for i in 0..4 {
                d.base.volts[i] = if v & (1 << i) != 0 { 5.0 } else { 0.0 };
            }
        };
        set(&mut d, 0);
        assert_eq!(d.display_text(), "text0");
        set(&mut d, 1);
        assert_eq!(d.display_text(), "text1");
        set(&mut d, 7);
        assert_eq!(d.display_text(), "other (7)");
        d.lookup_text = "0=x{a+1}y\n".into();
        d.parse_entries().unwrap();
        set(&mut d, 0);
        assert_eq!(d.display_text(), "x1y");
        set(&mut d, 5);
        assert_eq!(d.display_text(), "5");
    }

    #[test]
    fn bus_input_counts_up_and_wraps() {
        let mut b = BusLogicInputElm::new(0, 0);
        b.bus_width = 2;
        b.resize();
        for expect in [1, 2, 3, 0] {
            b.simple_toggle();
            assert_eq!(b.value, expect);
        }
        assert_eq!(b.post_count(), 2);
        assert_eq!(b.get_post(1), Some(Point { x: 0, y: 0, z: 1 }));
    }
}
