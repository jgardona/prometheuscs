// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (WireElm, GroundElm, ResistorElm).
//! Wires, ground and resistors.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::{TokenError, Tokenizer};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale;
use crate::render::painter::Painter;
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "passive";

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("WireElm", FAMILY, "Add Wire", "", |x, y| {
            Box::new(WireElm::new(x, y))
        })
        .dump(i32::from(b'w'), |a, b, c, d, f, _| Ok(Box::new(WireElm::from_dump(a, b, c, d, f))))
        .icon("wire")
        .shortcut('w')
        .quick(),
    );
    r.add(
        ElementDescriptor::new("RoutedWireElm", FAMILY, "Add Routed Wire", "", |x, y| {
            Box::new(WireElm::new_routed(x, y))
        })
        .shortcut('W'),
    );
    r.add(
        ElementDescriptor::new("ResistorElm", FAMILY, "Add Resistor", "", |x, y| {
            Box::new(ResistorElm::new(x, y))
        })
        .dump(i32::from(b'r'), |a, b, c, d, f, st| {
            Ok(Box::new(ResistorElm::from_dump(a, b, c, d, f, st)?))
        })
        .icon("resistor")
        .shortcut('r')
        .quick(),
    );
    r.add(
        ElementDescriptor::new("GroundElm", FAMILY, "Add Ground", "Inputs and Sources", |x, y| {
            Box::new(GroundElm::new(x, y))
        })
        .dump(i32::from(b'g'), |a, b, c, d, f, st| {
            Ok(Box::new(GroundElm::from_dump(a, b, c, d, f, st)))
        })
        .icon("ground")
        .shortcut('g')
        .quick(),
    );
}

// ---------------------------------------------------------------------------------------------
// Wire
// ---------------------------------------------------------------------------------------------

/// Flag: show the current next to the wire.
pub const FLAG_SHOWCURRENT: i32 = 1;
/// Flag: show the voltage next to the wire.
pub const FLAG_SHOWVOLTAGE: i32 = 2;
/// Flag: show the value of a bus.
pub const FLAG_SHOW_BUS_VALUE: i32 = 4;
/// Flag: show the value of a bus in hexadecimal.
pub const FLAG_SHOW_BUS_VALUE_HEX: i32 = 8;

/// A wire. Wires are removed from the matrix; their currents are derived afterwards.
#[derive(Clone, Debug)]
pub struct WireElm {
    base: Base,
    /// Width of the bus in bits (1 for an ordinary wire).
    pub bus_width: usize,
    currents: Option<Vec<f64>>,
    /// Corner points of a routed wire (`rw`); none for an ordinary wire.
    route: Option<Vec<Point>>,
}

impl WireElm {
    /// A routed wire being dragged out from (`xx`, `yy`).
    pub fn new_routed(xx: i32, yy: i32) -> Self {
        let mut w = WireElm::new(xx, yy);
        w.route = Some(Vec::new());
        w
    }

    /// A routed wire through `points` (at least two, first to last).
    pub fn from_route(points: Vec<Point>) -> Self {
        let (first, last) = (points[0], points[points.len() - 1]);
        let mut w = WireElm::new_routed(first.x, first.y);
        w.base.x2 = last.x;
        w.base.y2 = last.y;
        w.route = Some(points);
        w.set_points();
        w
    }

    /// Whether this is an ordinary one-bit wire, which can be turned into a routed one.
    pub fn is_plain(&self) -> bool {
        self.route.is_none() && self.bus_width == 1
    }

    fn route_points(&self) -> Vec<Point> {
        match &self.route {
            Some(r) if r.len() >= 2 => r.clone(),
            _ => vec![self.base.point1, self.base.point2],
        }
    }

    /// A wire being dragged out from (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut w = WireElm { base: Base::new(xx, yy), bus_width: 1, currents: None, route: None };
        w.alloc_nodes();
        w
    }

    /// A wire read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32) -> Self {
        let mut w = WireElm {
            base: Base::from_dump(xa, ya, xb, yb, f),
            bus_width: 1,
            currents: None,
            route: None,
        };
        w.alloc_nodes();
        w
    }

    /// Changes the bus width, resizing the per-bit state.
    pub fn set_bus_width(&mut self, w: usize) {
        if w == self.bus_width {
            return;
        }
        self.bus_width = w;
        self.currents = (w > 1).then(|| vec![0.0; w]);
        self.alloc_nodes();
    }

    fn must_show_current(&self) -> bool {
        self.base.flags & FLAG_SHOWCURRENT != 0
    }
    fn must_show_voltage(&self) -> bool {
        self.base.flags & FLAG_SHOWVOLTAGE != 0
    }
    fn must_show_bus_value(&self) -> bool {
        self.base.flags & FLAG_SHOW_BUS_VALUE != 0
    }
    fn must_show_bus_value_hex(&self) -> bool {
        self.base.flags & FLAG_SHOW_BUS_VALUE_HEX != 0
    }

    fn bus_value(&self) -> u32 {
        (0..self.bus_width).filter(|&i| self.base.volt(i) > 2.5).fold(0, |v, i| v | (1 << i))
    }
}

impl Element for WireElm {
    impl_element_base!();

    fn dump_type(&self) -> i32 {
        if self.route.is_some() { 0 } else { i32::from(b'w') }
    }
    fn xml_dump_type(&self) -> String {
        if self.route.is_some() { "rw".into() } else { "w".into() }
    }
    fn class_name(&self) -> &'static str {
        if self.route.is_some() { "RoutedWireElm" } else { "WireElm" }
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        if let Some(r) = self.route.as_ref().filter(|r| !r.is_empty()) {
            let text: Vec<String> = r.iter().map(|p| format!("{},{}", p.x, p.y)).collect();
            out.text = Some(text.join(";"));
        }
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        if self.route.is_some() {
            let pts: Vec<Point> = xml
                .text
                .trim()
                .split(';')
                .filter_map(|pair| {
                    let (a, b) = pair.split_once(',')?;
                    Some(Point::new(a.trim().parse().ok()?, b.trim().parse().ok()?))
                })
                .collect();
            if pts.len() >= 2 {
                self.route = Some(pts);
            }
        }
    }
    fn moved(&mut self, dx: i32, dy: i32) {
        if let Some(r) = &mut self.route {
            for p in r {
                p.x += dx;
                p.y += dy;
            }
        }
    }
    fn set_points(&mut self) {
        self.base.set_points();
        if let Some(r) = &mut self.route {
            let b = &self.base;
            let ok = r.len() >= 2
                && r[0] == Point::new(b.x, b.y)
                && r[r.len() - 1] == Point::new(b.x2, b.y2);
            if !ok {
                // no automatic router: an L-shaped route, horizontal first
                *r = if b.x != b.x2 && b.y != b.y2 {
                    vec![b.point1, Point::new(b.x2, b.y), b.point2]
                } else {
                    vec![b.point1, b.point2]
                };
            }
        }
    }
    fn post_count(&self) -> usize {
        self.bus_width * 2
    }
    fn bus_width(&self) -> usize {
        self.bus_width
    }
    fn bus_role(&self) -> BusRole {
        BusRole::Wire
    }
    fn apply_bus_width(&mut self, width: usize) -> bool {
        if width == self.bus_width {
            return false;
        }
        self.set_bus_width(width);
        true
    }
    fn post_width(&self, _n: usize) -> usize {
        self.bus_width
    }

    fn get_post(&self, n: usize) -> Option<Point> {
        let b = &self.base;
        if self.bus_width == 1 {
            return Some(if n == 0 { b.point1 } else { b.point2 });
        }
        if n < self.bus_width {
            return Some(Point { x: b.point1.x, y: b.point1.y, z: n as i32 });
        }
        Some(Point { x: b.point2.x, y: b.point2.y, z: (n - self.bus_width) as i32 })
    }

    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        if self.bus_width == 1 {
            return true;
        }
        // only matching bits connect: post n1 connects to n1 +/- bus width
        n1.abs_diff(n2) == self.bus_width
    }

    fn connected_post(&self, n: usize, _labels: &mut LabelRegistry) -> Option<Point> {
        let b = &self.base;
        if self.bus_width == 1 {
            return Some(if n == 0 { b.point2 } else { b.point1 });
        }
        if n < self.bus_width {
            return Some(Point { x: b.point2.x, y: b.point2.y, z: n as i32 });
        }
        Some(Point { x: b.point1.x, y: b.point1.y, z: (n - self.bus_width) as i32 })
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        if let Some(cur) = &self.currents {
            self.base.current = cur.iter().sum();
        }
        let b = &self.base;
        set_voltage_color(g, b, c, b.volt(0));
        let width = if self.bus_width > 1 { 5.0 } else { 3.0 };
        let pts = self.route_points();
        for w in pts.windows(2) {
            draw_thick_line_w(g, w[0], w[1], width);
        }
        if self.route.is_some() {
            self.base.update_dot_count(c);
            if !self.base.creating && self.base.curcount != 0.0 {
                let mut cc = self.base.curcount;
                for w in pts.windows(2) {
                    draw_dots(g, c, w[0], w[1], cc);
                    cc = Base::add_cur_count(cc, distance(w[0], w[1]));
                }
            }
            let (mut x0, mut y0, mut x1, mut y1) = (pts[0].x, pts[0].y, pts[0].x, pts[0].y);
            for p in &pts {
                x0 = x0.min(p.x);
                y0 = y0.min(p.y);
                x1 = x1.max(p.x);
                y1 = y1.max(p.y);
            }
            self.base.set_bbox(x0 - 5, y0 - 5, x1 + 5, y1 + 5);
        } else {
            do_dots(g, &mut self.base, c);
            let (p1, p2) = (self.base.point1, self.base.point2);
            self.base.set_bbox_pts(p1, p2, 3.0);
        }
        let mut s = String::new();
        if self.bus_width > 1 && (self.must_show_bus_value() || self.must_show_bus_value_hex()) {
            let value = self.bus_value();
            if self.must_show_bus_value() {
                s = value.to_string();
            }
            if self.must_show_bus_value_hex() {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(&format!("0x{value:X}"));
            }
        } else if self.bus_width == 1 {
            if self.must_show_current() {
                s = short_unit_text(self.get_current().abs(), "A");
            }
            if self.must_show_voltage() {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(&short_unit_text(self.base.volt(0), "V"));
            }
        }
        draw_values(g, &self.base, c, Some(&s), 4.0, false, false);
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        if self.bus_width > 1 {
            arr.push(format!("bus wire ({})", self.bus_width));
            let value = self.bus_value();
            arr.push(format!("value = {value}"));
            arr.push(format!("hex = 0x{value:X}"));
        } else {
            arr.push("wire".into());
            arr.push(format!("I = {}", current_d_text(self.get_current())));
            arr.push(format!("V = {}", voltage_text(self.base.volt(0))));
        }
    }

    fn get_power(&self) -> f64 {
        0.0
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volt(0)
    }
    fn is_wire_equivalent(&self) -> bool {
        true
    }
    fn is_removable_wire(&self) -> bool {
        true
    }

    fn set_wire_current(&mut self, bit: usize, c: f64) {
        match &mut self.currents {
            Some(cur) => cur[bit] = c,
            None => self.base.current = c,
        }
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        if let Some(cur) = &self.currents {
            return if n < self.bus_width { -cur[n] } else { cur[n - self.bus_width] };
        }
        if n == 0 { -self.base.current } else { self.base.current }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::checkbox("Show Current", self.must_show_current())),
            1 => Some(EditInfo::checkbox("Show Voltage", self.must_show_voltage())),
            2 => Some(EditInfo::checkbox("Show Bus Value", self.must_show_bus_value())),
            3 => Some(EditInfo::checkbox("Show Bus Value (Hex)", self.must_show_bus_value_hex())),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        let bit = match n {
            0 => FLAG_SHOWCURRENT,
            1 => FLAG_SHOWVOLTAGE,
            2 => FLAG_SHOW_BUS_VALUE,
            3 => FLAG_SHOW_BUS_VALUE_HEX,
            _ => return,
        };
        self.base.flags = ei.change_flag(self.base.flags, bit);
    }

    fn shortcut(&self) -> i32 {
        i32::from(b'w')
    }

    fn mouse_distance(&self, gx: i32, gy: i32) -> i32 {
        let pts = self.route_points();
        let d2 = if pts.len() > 2 {
            pts.windows(2)
                .map(|w| segment_distance_sq(w[0], w[1], gx, gy))
                .min()
                .unwrap_or(i32::MAX)
        } else {
            self.base.line_distance_sq(gx, gy)
        };
        if d2 <= 100 { d2 } else { -1 }
    }
}

fn segment_distance_sq(a: Point, b: Point, gx: i32, gy: i32) -> i32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0 {
        return (gx - a.x) * (gx - a.x) + (gy - a.y) * (gy - a.y);
    }
    let t = ((f64::from(gx - a.x) * f64::from(dx) + f64::from(gy - a.y) * f64::from(dy))
        / f64::from(len_sq))
    .clamp(0.0, 1.0);
    let (ex, ey) = (
        f64::from(gx) - (f64::from(a.x) + t * f64::from(dx)),
        f64::from(gy) - (f64::from(a.y) + t * f64::from(dy)),
    );
    (ex * ex + ey * ey) as i32
}

// ---------------------------------------------------------------------------------------------
// Ground
// ---------------------------------------------------------------------------------------------

/// Flag marking old subcircuits that dumped a ground as a voltage source.
pub const FLAG_OLD_STYLE: i32 = 1;

static LAST_GROUND_SYMBOL: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// A ground connection; all grounds in a circuit are the same node.
#[derive(Clone, Debug)]
pub struct GroundElm {
    base: Base,
    /// Symbol style: 0 earth, 1 chassis, 2 signal, 3 common.
    pub symbol_type: i32,
}

impl GroundElm {
    /// A ground being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        GroundElm {
            base: Base::new(xx, yy),
            symbol_type: LAST_GROUND_SYMBOL.load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// A ground read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let symbol_type = st.next_i32().unwrap_or(0);
        GroundElm { base: Base::from_dump(xa, ya, xb, yb, f), symbol_type }
    }

    /// Marks the ground as an old-style one (a zero volt source).
    pub fn set_old_style(&mut self) {
        self.base.flags |= FLAG_OLD_STYLE;
    }

    fn is_old_style(&self) -> bool {
        self.base.flags & FLAG_OLD_STYLE != 0
    }
}

impl Element for GroundElm {
    impl_element_base!("GroundElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'g')
    }
    fn dump(&self) -> String {
        format!("{} {}", self.base.dump_prefix(self.dump_type()), self.symbol_type)
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.symbol_type = xml.int_attr("sy", 0);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        if self.symbol_type != 0 {
            out.int_attr("sy", self.symbol_type);
        }
    }
    fn post_count(&self) -> usize {
        1
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let b = &self.base;
        set_voltage_color(g, b, c, 0.0);
        draw_thick_line(g, b.point1, b.point2);
        let (dx, dy, dn) = (f64::from(b.dx), f64::from(b.dy), b.dn);
        match self.symbol_type {
            0 => {
                for i in 0..3 {
                    let a = f64::from(10 - i * 4);
                    let bb = f64::from(i * 5);
                    let (ps1, ps2) = interp_point2(b.point1, b.point2, 1.0 + bb / dn, a);
                    draw_thick_line(g, ps1, ps2);
                }
            }
            1 => {
                let (ps1, ps2) = interp_point2(b.point1, b.point2, 1.0, 10.0);
                draw_thick_line(g, ps1, ps2);
                for i in 0..=2 {
                    let p = interp_point(ps1, ps2, f64::from(i) / 2.0);
                    let end = Point::new(
                        (f64::from(p.x) - 5.0 * b.dpx1 + 8.0 * dx / dn) as i32,
                        (f64::from(p.y) + 8.0 * dy / dn - 5.0 * b.dpy1) as i32,
                    );
                    draw_thick_line(g, p, end);
                }
            }
            2 => {
                let (ps1, ps2) = interp_point2(b.point1, b.point2, 1.0, 10.0);
                draw_thick_line(g, ps1, ps2);
                let ps3 = Point::new(
                    (f64::from(b.point2.x) + 10.0 * dx / dn) as i32,
                    (f64::from(b.point2.y) + 10.0 * dy / dn) as i32,
                );
                draw_thick_line(g, ps1, ps3);
                draw_thick_line(g, ps2, ps3);
            }
            _ => {
                let (ps1, ps2) = interp_point2(b.point1, b.point2, 1.0, 10.0);
                draw_thick_line(g, ps1, ps2);
            }
        }
        let ps2 = interp_point(b.point1, b.point2, 1.0 + 11.0 / dn);
        do_dots(g, &mut self.base, c);
        let p1 = self.base.point1;
        self.base.set_bbox_pts(p1, ps2, 11.0);
        draw_posts(g, &self.base, self, c);
    }

    fn voltage_source_count(&self) -> usize {
        usize::from(self.is_old_style())
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if self.is_old_style() {
            if let Some(vs) = self.base.volt_source {
                s.stamp_voltage_source(NodeId::GROUND, self.base.nodes[0], vs, 0.0);
            }
        }
    }

    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base.volt_source = Some(v);
    }

    fn set_current(&mut self, _vs: VsId, c: f64) {
        self.base.current = if self.is_old_style() { -c } else { c };
    }

    fn is_wire_equivalent(&self) -> bool {
        true
    }
    fn is_removable_wire(&self) -> bool {
        true
    }
    fn is_ground(&self) -> bool {
        true
    }

    fn connected_post(&self, _bit: usize, labels: &mut LabelRegistry) -> Option<Point> {
        labels.connect_ground(self.base.point1)
    }

    fn get_voltage_diff(&self) -> f64 {
        0.0
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("ground".into());
        arr.push(format!("I = {}", current_text(self.get_current())));
    }
    fn has_ground_connection(&self, _n1: usize) -> bool {
        true
    }
    fn shortcut(&self) -> i32 {
        i32::from(b'g')
    }
    fn drag_length(&self) -> i32 {
        32
    }
    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| {
            EditInfo::choice(
                "Symbol",
                ["Earth", "Chassis", "Signal", "Common"].iter().map(|s| (*s).to_string()).collect(),
                self.symbol_type.max(0) as usize,
            )
        })
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            self.symbol_type = ei.choice_index as i32;
            LAST_GROUND_SYMBOL.store(self.symbol_type, std::sync::atomic::Ordering::Relaxed);
        }
    }
    fn get_current_into_node(&self, _n: usize) -> f64 {
        -self.base.current
    }
}

// ---------------------------------------------------------------------------------------------
// Resistor
// ---------------------------------------------------------------------------------------------

/// A resistor.
#[derive(Clone, Debug)]
pub struct ResistorElm {
    base: Base,
    /// Resistance in ohms.
    pub resistance: f64,
}

impl ResistorElm {
    /// A resistor being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        ResistorElm { base: Base::new(xx, yy), resistance: 1000.0 }
    }

    /// A resistor read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        Ok(ResistorElm { base: Base::from_dump(xa, ya, xb, yb, f), resistance: st.next_f64()? })
    }
}

impl Element for ResistorElm {
    impl_element_base!("ResistorElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'r')
    }
    fn dump(&self) -> String {
        format!(
            "{} {}",
            self.base.dump_prefix(self.dump_type()),
            crate::format::number::js_number(self.resistance)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.resistance = xml.double_attr("r", self.resistance);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("r", self.resistance);
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(32);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let mut hs = 6;
        let (v1, v2) = (self.base.volts[0], self.base.volts[1]);
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
        draw_2_leads(g, &self.base, c);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let len = distance(l1, l2);
        g.save();
        g.set_line_width(3.0);
        let (ex, ey) = (f64::from(l2.x - l1.x), f64::from(l2.y - l1.y));
        g.transform(ex / len, ey / len, -ey / len, ex / len, f64::from(l1.x), f64::from(l1.y));
        if c.show_volts {
            g.set_stroke_gradient(
                len,
                voltage_color(&self.base, c, v1),
                voltage_color(&self.base, c, v2),
            );
        } else {
            set_power_color_w(g, &self.base, c, self.get_power());
        }
        if self.base.dn < 30.0 {
            hs = 2;
        }
        let hsf = f64::from(hs);
        if !c.euro_resistors {
            g.begin_path();
            g.move_to(0.0, 0.0);
            for i in 0..4 {
                let i = f64::from(i);
                g.line_to((1.0 + 4.0 * i) * len / 16.0, hsf);
                g.line_to((3.0 + 4.0 * i) * len / 16.0, -hsf);
            }
            g.line_to(len, 0.0);
            g.stroke();
        } else {
            g.stroke_rect(0.0, -hsf, len, 2.0 * hsf);
        }
        g.restore();
        if c.show_values {
            let ohm = if c.show_ohm { locale::OHM } else { "" };
            let s = short_unit_text(self.resistance, ohm);
            draw_values(g, &self.base, c, Some(&s), hsf + 2.0, false, false);
        }
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn calculate_current(&mut self) {
        self.base.current = (self.base.volts[0] - self.base.volts[1]) / self.resistance;
    }

    fn stamp(&mut self, s: &mut Stamper) {
        s.stamp_resistor(self.base.nodes[0], self.base.nodes[1], self.resistance);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("resistor".into());
        basic_info(self, arr);
        arr.push(format!("R = {}", unit_text(self.resistance, locale::OHM)));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }

    fn scope_text(&self, _v: i32) -> Option<String> {
        Some(format!("{}, {}", locale::ls("resistor"), unit_text(self.resistance, locale::OHM)))
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| EditInfo::new("Resistance (ohms)", self.resistance, 0.0, 0.0))
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            self.resistance = if ei.value <= 0.0 { 1e-9 } else { ei.value };
        }
    }

    fn shortcut(&self) -> i32 {
        i32::from(b'r')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resistor(line: &str) -> ResistorElm {
        let mut st = Tokenizer::new(line);
        ResistorElm::from_dump(0, 0, 64, 0, 0, &mut st).unwrap()
    }

    #[test]
    fn resistor_dump_round_trips() {
        let r = resistor("4700");
        assert_eq!(r.dump(), "r 0 0 64 0 0 4700");
    }

    #[test]
    fn resistor_current_follows_ohms_law() {
        let mut r = resistor("100");
        r.alloc_nodes();
        r.set_node_voltage(0, 5.0);
        r.set_node_voltage(1, 0.0);
        assert!((r.get_current() - 0.05).abs() < 1e-12);
        assert!((r.get_power() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn resistor_edit_clamps_to_positive() {
        let mut r = resistor("100");
        r.set_edit_value(0, &mut EditInfo::number("", -3.0));
        assert_eq!(r.resistance, 1e-9);
    }

    #[test]
    fn wire_posts_follow_bus_width() {
        let mut w = WireElm::from_dump(0, 0, 32, 0, 0);
        w.set_points();
        assert_eq!(w.post_count(), 2);
        w.set_bus_width(4);
        assert_eq!(w.post_count(), 8);
        assert_eq!(w.get_post(5), Some(Point { x: 32, y: 0, z: 1 }));
        assert!(w.get_connection(1, 5) && !w.get_connection(1, 6));
    }

    #[test]
    fn wires_draw_before_any_analysis() {
        let mut g = crate::render::svg::SvgPainter::new(100.0, 100.0);
        let ctx = DrawCtx::default();
        let mut plain = WireElm::new(0, 0);
        plain.base.volts.clear();
        let mut routed =
            WireElm::from_route(vec![Point::new(0, 0), Point::new(16, 0), Point::new(16, 16)]);
        routed.base.volts.clear();
        let mut bus = WireElm::from_dump(0, 0, 32, 0, 0);
        bus.set_bus_width(8);
        bus.base.volts.clear();
        for w in [&mut plain, &mut routed, &mut bus] {
            w.set_points();
            w.draw(&mut g, &ctx);
            let mut info = Vec::new();
            w.get_info(&mut info);
            assert_eq!(w.get_voltage_diff(), 0.0);
        }
    }

    #[test]
    fn new_wires_have_their_node_arrays() {
        assert_eq!(WireElm::new(0, 0).base.volts.len(), 2);
        assert_eq!(WireElm::from_dump(0, 0, 32, 0, 0).base.volts.len(), 2);
    }

    #[test]
    fn grounds_share_the_first_ground_post() {
        let mut labels = LabelRegistry::default();
        let mut g1 = GroundElm::from_dump(0, 0, 0, 16, 0, &mut Tokenizer::new(""));
        let mut g2 = GroundElm::from_dump(64, 0, 64, 16, 0, &mut Tokenizer::new(""));
        g1.set_points();
        g2.set_points();
        assert_eq!(g1.connected_post(0, &mut labels), None);
        assert_eq!(g2.connected_post(0, &mut labels), Some(Point::new(0, 0)));
    }
}
