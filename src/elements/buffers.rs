// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (DelayBufferElm, TriStateElm).
//! The delay buffer and the tri-state buffer. The tri-state buffer handles one bit; the bus
//! version of the original is not implemented.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::Painter;
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name used for the feature switch.
pub const FAMILY: &str = "logic";
const CAT: &str = "Logic Gates, Input and Output";

/// Registers the buffers.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("DelayBufferElm", FAMILY, "Add Delay Buffer", CAT, |x, y| {
            Box::new(DelayBuffer::new(x, y))
        })
        .dump(422, |a, b, c, d, f, st| Ok(Box::new(DelayBuffer::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("TriStateElm", FAMILY, "Add Tri-State Buffer", CAT, |x, y| {
            Box::new(TriState::new(x, y))
        })
        .dump(180, |a, b, c, d, f, st| Ok(Box::new(TriState::from_dump(a, b, c, d, f, st)))),
    );
}

// ---------------------------------------------------------------------------------------------
// Delay buffer
// ---------------------------------------------------------------------------------------------

/// A buffer whose output follows the input after a delay.
pub struct DelayBuffer {
    base: Base,
    delay: f64,
    threshold: f64,
    high_voltage: f64,
    delay_end: f64,
    source: Option<VsId>,
    triangle: Polygon,
    boxed: Polygon,
    center: Point,
}

impl DelayBuffer {
    fn blank(base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        DelayBuffer {
            base,
            delay: 0.0,
            threshold: 2.5,
            high_voltage: 5.0,
            delay_end: 0.0,
            source: None,
            triangle: Polygon::default(),
            boxed: Polygon::default(),
            center: Point::default(),
        }
    }

    /// A buffer being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y))
    }

    /// A buffer read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut d = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        d.delay = st.next_f64().unwrap_or(0.0);
        d.threshold = st.next_f64().unwrap_or(2.5);
        d.high_voltage = st.next_f64().unwrap_or(5.0);
        d
    }
}

impl Element for DelayBuffer {
    impl_element_base!("DelayBufferElm");

    fn dump_type(&self) -> i32 {
        422
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {}",
            self.base.dump_prefix(422),
            js_number(self.delay),
            js_number(self.threshold),
            js_number(self.high_voltage)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("dl", self.delay);
        out.double_attr("th", self.threshold);
        out.double_attr("hv", self.high_voltage);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.delay = xml.double_attr("dl", self.delay);
        self.threshold = xml.double_attr("th", self.threshold);
        self.high_voltage = xml.double_attr("hv", self.high_voltage);
    }
    fn set_high_voltage(&mut self, hv: f64) {
        self.high_voltage = hv;
    }

    fn voltage_source_count(&self) -> usize {
        1
    }
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.source = Some(v);
    }
    // no current path through the input, but an indirect one through the output to ground
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n1: usize) -> bool {
        n1 == 1
    }
    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.delay_end = 0.0;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &mut self.base;
        let hs = 16.0;
        let mut ww = 14.0;
        if ww > b.dn / 2.0 {
            ww = (b.dn / 2.0).floor();
        }
        b.lead1 = interp_point(b.point1, b.point2, 0.5 - ww / b.dn);
        b.lead2 = interp_point(b.point1, b.point2, 0.5 + ww / b.dn);
        let (p1, p2, l1, l2, dn) = (b.point1, b.point2, b.lead1, b.lead2, b.dn);
        let end = interp_point(p1, p2, 0.5 + (ww - 5.0) / dn);
        let (a0, a1) = interp_point2(l1, end, 0.0, hs);
        let (d3, d2) = interp_point2(l1, end, 1.0, hs);
        self.boxed = Polygon::from_points(&[a0, a1, d2, d3]);
        self.center = interp_point(l1, end, 0.5);
        let (t0, t1) = interp_point2(l1, l2, 0.0, hs);
        let t2 = interp_point(p1, p2, 0.5 + ww / dn);
        self.triangle = Polygon::from_points(&[t0, t1, t2]);
        self.base.set_bbox_pts(p1, p2, hs);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.source {
            s.set_vs_nodes(vs, NodeId::GROUND, self.base.nodes[1]);
            s.stamp_voltage_source_var(NodeId::GROUND, self.base.nodes[1], vs);
        }
    }
    fn do_step(&mut self, s: &mut Stamper) {
        let in_state = self.base.volts[0] > self.threshold;
        let mut out_state = self.base.volts[1] > self.threshold;
        if in_state != out_state {
            if s.t >= self.delay_end {
                out_state = in_state;
            }
        } else {
            self.delay_end = s.t + self.delay;
        }
        if let Some(vs) = self.source {
            s.update_voltage_source(vs, if out_state { self.high_voltage } else { 0.0 });
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        draw_posts(g, &self.base, self, c);
        draw_2_leads(g, &self.base, c);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        if c.iec_gates {
            draw_thick_polygon(g, &self.boxed);
            let (cx, cy) = (self.center.x, self.center.y - 6);
            draw_centered_text(g, &mut self.base, "1", cx, cy, true);
        } else {
            draw_thick_polygon(g, &self.triangle);
        }
        self.base.update_dot_count(c);
        let (l2, p2) = (self.base.lead2, self.base.point2);
        draw_dots(g, c, l2, p2, self.base.curcount);
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 1 { self.base.current } else { 0.0 }
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(crate::locale::ls("buffer"));
        arr.push(format!("{}{}", crate::locale::ls("delay = "), unit_text(self.delay, "s")));
        arr.push(format!("Vi = {}", voltage_text(self.base.volts[0])));
        arr.push(format!("Vo = {}", voltage_text(self.base.volts[1])));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Delay (s)", self.delay, 0.0, 0.0)),
            1 => Some(EditInfo::new("Threshold (V)", self.threshold, 0.0, 0.0)),
            2 => Some(
                EditInfo::new("High Logic Voltage", self.high_voltage, 0.0, 0.0).set_unit_step(),
            ),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.delay = ei.value,
            1 => self.threshold = ei.value,
            2 => self.high_voltage = ei.value,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Tri-state buffer
// ---------------------------------------------------------------------------------------------

const FLAG_FLIP: i32 = 1;
const FLAG_FLIP_X: i32 = 2;
const FLAG_FLIP_Y: i32 = 4;

/// A buffer with an enable input: with the enable low the output floats (a large resistor, with
/// an optional weak pulldown).
pub struct TriState {
    base: Base,
    r_on: f64,
    r_off: f64,
    r_off_ground: f64,
    high_voltage: f64,
    resistance: f64,
    open: bool,
    source: Option<VsId>,
    gate: Polygon,
    point3: Point,
    lead3: Point,
}

impl TriState {
    fn blank(base: Base, r_off_ground: f64, high_voltage: f64) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        TriState {
            base,
            r_on: 0.1,
            r_off: 1e10,
            r_off_ground,
            high_voltage,
            resistance: 1e10,
            open: false,
            source: None,
            gate: Polygon::default(),
            point3: Point::default(),
            lead3: Point::default(),
        }
    }

    /// A buffer being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y), 1e8, 5.0)
    }

    /// A buffer read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut t = Self::blank(Base::from_dump(xa, ya, xb, yb, f), 0.0, 5.0);
        t.r_on = st.next_f64().unwrap_or(t.r_on);
        t.r_off = st.next_f64().unwrap_or(t.r_off);
        t.r_off_ground = st.next_f64().unwrap_or(0.0);
        t.high_voltage = st.next_f64().unwrap_or(5.0);
        t
    }
}

impl Element for TriState {
    impl_element_base!("TriStateElm");

    fn dump_type(&self) -> i32 {
        180
    }
    fn xml_dump_type(&self) -> String {
        "ts".into()
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(180),
            js_number(self.r_on),
            js_number(self.r_off),
            js_number(self.r_off_ground),
            js_number(self.high_voltage)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("ron", self.r_on);
        out.double_attr("roff", self.r_off);
        out.double_attr("rog", self.r_off_ground);
        out.double_attr("hi", self.high_voltage);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.r_on = xml.double_attr("ron", self.r_on);
        self.r_off = xml.double_attr("roff", self.r_off);
        self.r_off_ground = xml.double_attr("rog", self.r_off_ground);
        self.high_voltage = xml.double_attr("hi", self.high_voltage);
    }
    fn set_high_voltage(&mut self, hv: f64) {
        self.high_voltage = hv;
    }

    // posts: input, output, enable
    fn post_count(&self) -> usize {
        3
    }
    fn internal_node_count(&self) -> usize {
        1
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.base.point1,
            1 => self.base.point2,
            _ => self.point3,
        })
    }
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.source = Some(v);
    }
    fn voltage_source_nodes_at(&self, _j: usize) -> Option<(NodeId, NodeId)> {
        Some((NodeId::GROUND, self.base.nodes[3]))
    }
    // the output and its internal node are joined by the switch resistor
    fn get_matrix_connection(&self, n1: usize, n2: usize) -> bool {
        (n1 == 1 && n2 == 3) || (n1 == 3 && n2 == 1)
    }
    // no current path through the input, but an indirect one through the output to ground
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n1: usize) -> bool {
        n1 == 1
    }
    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(32);
        let b = &self.base;
        let (l1, l2, dn) = (b.lead1, b.lead2, b.dn);
        let hs = 16.0;
        let mut ww = 16.0;
        if ww > dn / 2.0 {
            ww = (dn / 2.0).floor();
        }
        let (t0, t1) = interp_point2(l1, l2, 0.0, hs + 2.0);
        let t2 = interp_point(l1, l2, 0.5 + (ww - 2.0) / 32.0);
        self.gate = Polygon::from_points(&[t0, t1, t2]);
        let sign = if b.flags & FLAG_FLIP == 0 { -1.0 } else { 1.0 };
        self.point3 = interp_point_g(l1, l2, 0.5, sign * hs);
        self.lead3 = interp_point_g(l1, l2, 0.5, sign * (hs / 2.0 + 2.0));
        self.alloc_nodes();
    }

    // the mouse picks the side of the enable input while dragging
    fn drag(&mut self, xx: i32, yy: i32) {
        let b = &mut self.base;
        let mut flip = (xx < b.x) == (yy < b.y);
        let (mut xx, mut yy) = (snap_grid(xx), snap_grid(yy));
        if (b.x - xx).abs() < (b.y - yy).abs() {
            xx = b.x;
        } else {
            flip = !flip;
            yy = b.y;
        }
        b.flags = if flip { b.flags | FLAG_FLIP } else { b.flags & !FLAG_FLIP };
        b.drag(xx, yy);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        if let Some(vs) = self.source {
            s.stamp_voltage_source_var(NodeId::GROUND, n[3], vs);
        }
        s.stamp_non_linear(n[3]);
        s.stamp_non_linear(n[1]);
    }
    fn do_step(&mut self, s: &mut Stamper) {
        let (n, v) = (self.base.nodes.clone(), self.base.volts.clone());
        self.open = v[2] < self.high_voltage * 0.5;
        self.resistance = if self.open { self.r_off } else { self.r_on };
        s.stamp_resistor(n[3], n[1], self.resistance);
        if self.r_off_ground > 0.0 {
            s.stamp_resistor(n[1], NodeId::GROUND, self.r_off_ground);
        }
        if let Some(vs) = self.source {
            s.update_voltage_source(
                vs,
                if v[0] > self.high_voltage * 0.5 { self.high_voltage } else { 0.0 },
            );
        }
    }
    fn calculate_current(&mut self) {
        let v = &self.base.volts;
        let i31 = (v[3] - v[1]) / self.resistance;
        let i10 = if self.r_off_ground == 0.0 { 0.0 } else { v[1] / self.r_off_ground };
        self.base.current = i31 - i10;
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 1 { self.base.current } else { 0.0 }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let hs = 16.0;
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, hs);
        set_voltage_color(g, &self.base, c, self.base.volts[2]);
        draw_thick_line(g, self.point3, self.lead3);
        draw_2_leads(g, &self.base, c);
        g.set_color(c.light_gray);
        draw_thick_polygon(g, &self.gate);
        self.base.update_dot_count(c);
        let l2 = self.base.lead2;
        draw_dots(g, c, l2, p2, self.base.curcount);
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("tri-state buffer".into());
        arr.push(if self.open { "open" } else { "closed" }.into());
        arr.push(format!("Vd = {}", voltage_d_text(self.get_voltage_diff())));
        arr.push(format!("I = {}", current_d_text(self.base.current)));
        arr.push(format!("Vc = {}", voltage_text(self.base.volts[2])));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("On Resistance (ohms)", self.r_on, 0.0, 0.0).set_positive()),
            1 => Some(EditInfo::new("Off Resistance (ohms)", self.r_off, 0.0, 0.0).set_positive()),
            2 => Some(
                EditInfo::new("Output Pulldown Resistance (ohms)", self.r_off_ground, 0.0, 0.0)
                    .set_positive(),
            ),
            3 => Some(
                EditInfo::new("High Logic Voltage", self.high_voltage, 1.0, 10.0).set_unit_step(),
            ),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 if ei.value > 0.0 => self.r_on = ei.value,
            1 if ei.value > 0.0 => self.r_off = ei.value,
            2 if ei.value > 0.0 => self.r_off_ground = ei.value,
            3 => self.high_voltage = ei.value,
            _ => {}
        }
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIP | FLAG_FLIP_X;
        self.base.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIP | FLAG_FLIP_Y;
        self.base.flip_y(center2);
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIP;
        self.base.flip_xy(xmy);
        self.set_points();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tri_state_dump_round_trips() {
        let mut st = Tokenizer::new("0.1 1e10 1e8 5");
        let t = TriState::from_dump(0, 0, 64, 0, 0, &mut st);
        assert_eq!(t.dump(), "180 0 0 64 0 0 0.1 10000000000 100000000 5");
    }
}
