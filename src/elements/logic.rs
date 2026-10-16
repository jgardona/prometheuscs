// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (GateElm and its subclasses,
// InverterElm, SchmittElm, InvertingSchmittElm, LogicInputElm, LogicOutputElm).
//! Logic gates, inverter, Schmitt triggers and logic inputs and outputs.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::switches::SwitchElm;
use super::units::*;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Painter, TextBaseline};
use crate::solver::path::Validator;
use crate::solver::stamper::{NodeId, Stamper, VsId};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "logic";
const CAT: &str = "Logic Gates, Input and Output";
const CAT_ACTIVE: &str = "Active Building Blocks";

static LAST_HIGH_VOLTAGE: AtomicU64 = AtomicU64::new(0x4014_0000_0000_0000); // 5.0
static LAST_SCHMITT: AtomicBool = AtomicBool::new(false);

fn last_high_voltage() -> f64 {
    f64::from_bits(LAST_HIGH_VOLTAGE.load(Ordering::Relaxed))
}

fn set_last_high_voltage(v: f64) {
    LAST_HIGH_VOLTAGE.store(v.to_bits(), Ordering::Relaxed);
}

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("LogicInputElm", FAMILY, "Add Logic Input", CAT, |x, y| {
            Box::new(LogicInputElm::new(x, y))
        })
        .dump(i32::from(b'L'), |a, b, c, d, f, st| {
            Ok(Box::new(LogicInputElm::from_dump(a, b, c, d, f, st)?))
        })
        .shortcut('i'),
    );
    r.add(
        ElementDescriptor::new("LogicOutputElm", FAMILY, "Add Logic Output", CAT, |x, y| {
            Box::new(LogicOutputElm::new(x, y))
        })
        .dump(i32::from(b'M'), |a, b, c, d, f, st| {
            Ok(Box::new(LogicOutputElm::from_dump(a, b, c, d, f, st)))
        })
        .shortcut('o'),
    );
    r.add(
        ElementDescriptor::new("InverterElm", FAMILY, "Add Inverter", CAT, |x, y| {
            Box::new(InverterElm::new(x, y))
        })
        .dump(i32::from(b'I'), |a, b, c, d, f, st| {
            Ok(Box::new(InverterElm::from_dump(a, b, c, d, f, st)))
        })
        .shortcut('1'),
    );
    macro_rules! gate {
        ($kind:expr) => {
            r.add(
                ElementDescriptor::new(
                    $kind.class_name(),
                    FAMILY,
                    $kind.menu_name(),
                    CAT,
                    |x, y| Box::new(GateElm::new($kind, x, y)),
                )
                .dump($kind.dump_code(), |a, b, c, d, f, st| {
                    Ok(Box::new(GateElm::from_dump($kind, a, b, c, d, f, st)?))
                })
                .shortcut($kind.shortcut()),
            );
        };
    }
    gate!(GateKind::Nand);
    gate!(GateKind::Nor);
    gate!(GateKind::And);
    gate!(GateKind::Or);
    gate!(GateKind::Xor);
    gate!(GateKind::Xnor);
    r.add(
        ElementDescriptor::new("SchmittElm", FAMILY, "Add Schmitt Trigger", CAT_ACTIVE, |x, y| {
            Box::new(SchmittElm::new(false, x, y))
        })
        .dump(182, |a, b, c, d, f, st| {
            Ok(Box::new(SchmittElm::from_dump(false, a, b, c, d, f, st)))
        }),
    );
    r.add(
        ElementDescriptor::new(
            "InvertingSchmittElm",
            FAMILY,
            "Add Schmitt Trigger (Inverting)",
            CAT_ACTIVE,
            |x, y| Box::new(SchmittElm::new(true, x, y)),
        )
        .dump(183, |a, b, c, d, f, st| {
            Ok(Box::new(SchmittElm::from_dump(true, a, b, c, d, f, st)))
        }),
    );
}

// ---------------------------------------------------------------------------------------------
// Gates
// ---------------------------------------------------------------------------------------------

const FLAG_SMALL: i32 = 1;
const FLAG_SCHMITT: i32 = 2;
const FLAG_INVERT_INPUTS: i32 = 4;
const FLAG_DEMORGAN: i32 = 8;

/// Which logic function a [`GateElm`] computes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateKind {
    /// AND.
    And,
    /// NAND.
    Nand,
    /// OR.
    Or,
    /// NOR.
    Nor,
    /// XOR.
    Xor,
    /// XNOR.
    Xnor,
}

impl GateKind {
    /// The original's class name.
    pub fn class_name(self) -> &'static str {
        match self {
            GateKind::And => "AndGateElm",
            GateKind::Nand => "NandGateElm",
            GateKind::Or => "OrGateElm",
            GateKind::Nor => "NorGateElm",
            GateKind::Xor => "XorGateElm",
            GateKind::Xnor => "XnorGateElm",
        }
    }

    fn menu_name(self) -> &'static str {
        match self {
            GateKind::And => "Add AND Gate",
            GateKind::Nand => "Add NAND Gate",
            GateKind::Or => "Add OR Gate",
            GateKind::Nor => "Add NOR Gate",
            GateKind::Xor => "Add XOR Gate",
            GateKind::Xnor => "Add XNOR Gate",
        }
    }

    fn dump_code(self) -> i32 {
        match self {
            GateKind::And => 150,
            GateKind::Nand => 151,
            GateKind::Or => 152,
            GateKind::Nor => 153,
            GateKind::Xor => 154,
            GateKind::Xnor => 431,
        }
    }

    fn shortcut(self) -> char {
        match self {
            GateKind::And => '2',
            GateKind::Nand => '@',
            GateKind::Or => '3',
            GateKind::Nor => '#',
            GateKind::Xor => '4',
            GateKind::Xnor => '$',
        }
    }

    fn inverting(self) -> bool {
        matches!(self, GateKind::Nand | GateKind::Nor | GateKind::Xnor)
    }

    fn is_xor(self) -> bool {
        matches!(self, GateKind::Xor | GateKind::Xnor)
    }

    fn is_and_family(self) -> bool {
        matches!(self, GateKind::And | GateKind::Nand)
    }

    fn gate_text(self) -> &'static str {
        match self {
            GateKind::And | GateKind::Nand => "&",
            GateKind::Or | GateKind::Nor => "\u{2265}1",
            GateKind::Xor | GateKind::Xnor => "=1",
        }
    }
}

/// A logic gate with one to eight inputs.
#[derive(Clone, Debug)]
pub struct GateElm {
    base: Base,
    kind: GateKind,
    /// Number of inputs.
    pub input_count: usize,
    last_output: bool,
    /// Output voltage for a logic one.
    pub high_voltage: f64,
    /// Delay between an input change and the output change, in seconds (0 for none).
    pub propagation_delay: f64,
    delay_end_time: f64,
    gsize: i32,
    gwidth: i32,
    gwidth2: i32,
    gheight: i32,
    hs2: f64,
    ww: i32,
    in_posts: Vec<Point>,
    in_gates: Vec<Point>,
    input_states: Vec<bool>,
    gate_poly: Polygon,
    schmitt_poly: Polygon,
    pcircle: Option<Point>,
    icircles: Option<Vec<Point>>,
    oscillation_count: i32,
    last_time: f64,
}

impl GateElm {
    fn blank(kind: GateKind, base: Base, high_voltage: f64) -> Self {
        let mut g = GateElm {
            base,
            kind,
            input_count: 2,
            last_output: false,
            high_voltage,
            propagation_delay: 0.0,
            delay_end_time: 0.0,
            gsize: 2,
            gwidth: 14,
            gwidth2: 28,
            gheight: 16,
            hs2: 0.0,
            ww: 0,
            in_posts: Vec::new(),
            in_gates: Vec::new(),
            input_states: Vec::new(),
            gate_poly: Polygon::new(),
            schmitt_poly: Polygon::new(),
            pcircle: None,
            icircles: None,
            oscillation_count: 0,
            last_time: 0.0,
        };
        g.base.no_diagonal = true;
        g
    }

    /// A gate being placed at (`xx`, `yy`).
    pub fn new(kind: GateKind, xx: i32, yy: i32) -> Self {
        let mut g = Self::blank(kind, Base::new(xx, yy), last_high_voltage());
        if LAST_SCHMITT.load(Ordering::Relaxed) {
            g.base.flags |= FLAG_SCHMITT;
        }
        g.set_size(2);
        g.alloc_nodes();
        g.setup_volts();
        g
    }

    /// A gate read from a file.
    pub fn from_dump(
        kind: GateKind,
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, crate::format::tokenizer::TokenError> {
        let mut g = Self::blank(kind, Base::from_dump(xa, ya, xb, yb, f), 5.0);
        g.input_count = usize::try_from(st.next_i32()?).unwrap_or(2).max(1);
        let last_output_voltage = st.next_f64()?;
        if let Ok(hv) = st.next_f64() {
            g.high_voltage = hv;
        }
        g.last_output = last_output_voltage > g.high_voltage * 0.5;
        g.set_size(if f & FLAG_SMALL != 0 { 1 } else { 2 });
        g.alloc_nodes();
        g.setup_volts();
        Ok(g)
    }

    fn set_size(&mut self, s: i32) {
        self.gsize = s;
        self.gwidth = 7 * s;
        self.gwidth2 = 14 * s;
        self.gheight = 8 * s;
        self.base.flags &= !FLAG_SMALL;
        if s == 1 {
            self.base.flags |= FLAG_SMALL;
        }
    }

    fn setup_volts(&mut self) {
        let v = if self.last_output ^ self.kind.inverting() { self.high_voltage } else { 0.0 };
        for i in 0..self.input_count.min(self.base.volts.len()) {
            self.base.volts[i] = v;
        }
    }

    fn has_flag(&self, f: i32) -> bool {
        self.base.flags & f != 0
    }

    fn draws_as_and(&self) -> bool {
        match self.kind {
            GateKind::And | GateKind::Nand => !self.has_flag(FLAG_DEMORGAN),
            GateKind::Or | GateKind::Nor => self.has_flag(FLAG_DEMORGAN),
            _ => false,
        }
    }

    fn lead_adjustment(&self, ix: usize, euro: bool) -> f64 {
        if self.kind.is_and_family() || euro {
            return 0.0;
        }
        let n = self.input_count;
        if n > 3 && (ix == 0 || ix == n - 1) {
            return -0.15;
        }
        if n > 7 && (ix == 1 || ix == n - 2) {
            return -0.25;
        }
        if n >= 12 && (ix == 2 || ix == n - 3) {
            return -0.35;
        }
        0.0
    }

    fn gate_name(&self) -> &'static str {
        let inv = self.has_flag(FLAG_INVERT_INPUTS);
        match (self.kind, inv) {
            (GateKind::And, false) => "AND gate",
            (GateKind::And, true) => "NOR gate",
            (GateKind::Nand, false) => "NAND gate",
            (GateKind::Nand, true) => "OR gate",
            (GateKind::Or, false) => "OR gate",
            (GateKind::Or, true) => "NAND gate",
            (GateKind::Nor, false) => "NOR gate",
            (GateKind::Nor, true) => "AND gate",
            (GateKind::Xor, _) => "XOR gate",
            (GateKind::Xnor, _) => "XNOR gate",
        }
    }

    fn input(&mut self, x: usize) -> bool {
        let high = !self.has_flag(FLAG_INVERT_INPUTS);
        let v = self.base.volts[x];
        if !self.has_flag(FLAG_SCHMITT) {
            return if v > self.high_voltage * 0.5 { high } else { !high };
        }
        if self.input_states.len() != self.input_count {
            self.input_states = vec![false; self.input_count];
        }
        let res = v > self.high_voltage * if self.input_states[x] { 0.35 } else { 0.55 };
        self.input_states[x] = res;
        if res { high } else { !high }
    }

    fn calc_function(&mut self) -> bool {
        let n = self.input_count;
        match self.kind {
            GateKind::And | GateKind::Nand => (0..n).fold(true, |f, i| f & self.input(i)),
            GateKind::Or | GateKind::Nor => (0..n).fold(false, |f, i| f | self.input(i)),
            GateKind::Xor | GateKind::Xnor => (0..n).fold(false, |f, i| f ^ self.input(i)),
        }
    }

    fn draw_gate_polygon(&self, g: &mut dyn Painter) {
        let p = &self.gate_poly.points;
        if p.len() < 7 {
            return;
        }
        let pt = |i: usize| (f64::from(p[i].x), f64::from(p[i].y));
        g.set_line_width(3.0);
        g.begin_path();
        if self.draws_as_and() {
            g.move_to(pt(0).0, pt(0).1);
            g.line_to(pt(1).0, pt(1).1);
            g.bezier_curve_to(pt(1).0, pt(1).1, pt(2).0, pt(2).1, pt(3).0, pt(3).1);
            g.bezier_curve_to(pt(3).0, pt(3).1, pt(4).0, pt(4).1, pt(5).0, pt(5).1);
            g.line_to(pt(6).0, pt(6).1);
        } else {
            g.move_to(pt(0).0, pt(0).1);
            g.line_to(pt(1).0, pt(1).1);
            g.bezier_curve_to(pt(2).0, pt(2).1, pt(2).0, pt(2).1, pt(3).0, pt(3).1);
            g.bezier_curve_to(pt(4).0, pt(4).1, pt(4).0, pt(4).1, pt(5).0, pt(5).1);
            g.line_to(pt(6).0, pt(6).1);
            g.bezier_curve_to(pt(7).0, pt(7).1, pt(7).0, pt(7).1, pt(0).0, pt(0).1);
        }
        g.close_path();
        if self.kind.is_xor() && p.len() >= 11 {
            g.move_to(pt(8).0, pt(8).1);
            g.bezier_curve_to(pt(10).0, pt(10).1, pt(10).0, pt(10).1, pt(9).0, pt(9).1);
        }
        g.stroke();
        g.set_line_width(1.0);
    }
}

impl Element for GateElm {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        self.kind.class_name()
    }
    fn dump_type(&self) -> i32 {
        self.kind.dump_code()
    }
    fn xml_dump_type(&self) -> String {
        self.kind.class_name().replace("GateElm", "")
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        let out = self.base.volts.get(self.input_count).copied().unwrap_or(0.0);
        format!(
            "{} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            self.input_count,
            js_number(out),
            js_number(self.high_voltage)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", 0);
        self.high_voltage = xml.double_attr("hi", self.high_voltage);
        self.input_count =
            usize::try_from(xml.int_attr("in", self.input_count as i32)).unwrap_or(2).max(1);
        self.propagation_delay = xml.double_attr("pd", self.propagation_delay);
        self.last_output = xml.double_attr("o", 0.0) > self.high_voltage * 0.5;
        self.set_size(if self.has_flag(FLAG_SMALL) { 1 } else { 2 });
        self.alloc_nodes();
        self.setup_volts();
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        if self.high_voltage != 5.0 {
            out.double_attr("hi", self.high_voltage);
        }
        if self.input_count != 2 {
            out.int_attr("in", self.input_count as i32);
        }
        if self.propagation_delay != 0.0 {
            out.double_attr("pd", self.propagation_delay);
        }
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        let v = self.base.volts.get(self.input_count).copied().unwrap_or(0.0);
        if v != 0.0 {
            out.double_attr("o", v);
        }
    }

    fn post_count(&self) -> usize {
        self.input_count + 1
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn drag_length(&self) -> i32 {
        96
    }
    fn shortcut(&self) -> i32 {
        i32::from(self.kind.shortcut() as u8)
    }

    fn get_post(&self, n: usize) -> Option<Point> {
        if n == self.input_count { Some(self.base.point2) } else { self.in_posts.get(n).copied() }
    }

    fn set_points(&mut self) {
        self.base.no_diagonal = true;
        self.base.set_points();
        self.input_states = vec![false; self.input_count];
        if self.base.dn > 150.0 && self.base.creating {
            self.set_size(2);
        }
        let hs = self.gheight;
        self.ww = self.gwidth2;
        if f64::from(self.ww) > self.base.dn / 2.0 {
            self.ww = (self.base.dn / 2.0) as i32;
        }
        let inverting = self.kind.inverting();
        if inverting && f64::from(self.ww + 8) > self.base.dn / 2.0 {
            self.ww = (self.base.dn / 2.0 - 8.0) as i32;
        }
        self.base.calc_leads(self.ww * 2);
        let (p1, p2) = (self.base.point1, self.base.point2);
        let euro = EURO_GATES.with(|e| e.get());
        let circles = self.has_flag(FLAG_INVERT_INPUTS) || self.has_flag(FLAG_DEMORGAN);
        let mut icircles = Vec::new();
        self.in_posts.clear();
        self.in_gates.clear();
        let mut i0 = -(self.input_count as i32) / 2;
        for i in 0..self.input_count {
            if i0 == 0 && self.input_count & 1 == 0 {
                i0 += 1;
            }
            let adj = self.lead_adjustment(i, euro);
            self.in_posts.push(interp_point_g(p1, p2, 0.0, f64::from(hs * i0)));
            let f = if circles { -8.0 / f64::from(self.ww * 2) + adj } else { adj };
            self.in_gates.push(interp_point_g(
                self.base.lead1,
                self.base.lead2,
                f,
                f64::from(hs * i0),
            ));
            if circles {
                icircles.push(interp_point_g(
                    self.base.lead1,
                    self.base.lead2,
                    -4.0 / f64::from(self.ww * 2),
                    f64::from(hs * i0),
                ));
            }
            i0 += 1;
        }
        self.icircles = circles.then_some(icircles);
        self.hs2 = f64::from(self.gwidth * (self.input_count as i32 / 2 + 1));
        self.base.set_bbox_pts(p1, p2, self.hs2);
        if self.has_flag(FLAG_SCHMITT) {
            self.schmitt_poly = schmitt_polygon(&self.base, self.gsize as f32, 0.47);
        }
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        if euro {
            let (a, b) = interp_point2(l1, l2, 0.0, self.hs2);
            let (d, c) = interp_point2(l1, l2, 1.0, self.hs2);
            self.gate_poly = Polygon::from_points(&[a, b, c, d]);
        } else {
            let mut t = [Point::default(); 11];
            if self.draws_as_and() {
                (t[0], t[6]) = interp_point2(l1, l2, 0.0, self.hs2);
                (t[1], t[5]) = interp_point2(l1, l2, 0.5, self.hs2);
                (t[2], t[4]) = interp_point2(l1, l2, 1.0, self.hs2);
                t[3] = interp_point(l1, l2, 1.0);
            } else {
                (t[0], t[6]) = interp_point2(l1, l2, 0.0, self.hs2);
                (t[1], t[5]) = interp_point2(l1, l2, 0.3, self.hs2);
                t[3] = l2;
                (t[2], t[4]) = interp_point2(l1, l2, 0.733, self.hs2 * 0.85);
                t[7] = interp_point(l1, l2, 0.105);
            }
            if self.kind.is_xor() {
                let ww2 = if self.ww == 0 { self.base.dn * 2.0 } else { f64::from(self.ww * 2) };
                (t[8], t[9]) = interp_point2(l1, l2, -0.05 - 8.0 / ww2, self.hs2);
                t[10] = interp_point(l1, l2, 0.1 - 8.0 / ww2);
            }
            self.gate_poly = Polygon::from_points(&t);
        }
        if inverting ^ self.has_flag(FLAG_DEMORGAN) {
            let dn = self.base.dn;
            self.pcircle = Some(interp_point(p1, p2, 0.5 + f64::from(self.ww + 4) / dn));
            self.base.lead2 = interp_point(p1, p2, 0.5 + f64::from(self.ww + 8) / dn);
        } else {
            self.pcircle = None;
        }
        self.alloc_nodes();
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        if self.in_posts.len() != self.input_count {
            return;
        }
        EURO_GATES.with(|e| e.set(c.iec_gates));
        for i in 0..self.input_count {
            set_voltage_color(g, &self.base, c, self.base.volts[i]);
            draw_thick_line(g, self.in_posts[i], self.in_gates[i]);
        }
        set_voltage_color(g, &self.base, c, self.base.volts[self.input_count]);
        draw_thick_line(g, self.base.lead2, self.base.point2);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        if c.iec_gates {
            draw_thick_polygon(g, &self.gate_poly);
            let center = interp_point(self.base.point1, self.base.point2, 0.5);
            draw_centered_text(
                g,
                &mut self.base,
                self.kind.gate_text(),
                center.x,
                center.y - 6 * self.gsize,
                true,
            );
        } else {
            self.draw_gate_polygon(g);
        }
        g.set_line_width(2.0);
        if self.has_flag(FLAG_SCHMITT) {
            draw_polygon(g, &self.schmitt_poly);
        }
        g.set_line_width(1.0);
        if let Some(p) = self.pcircle {
            draw_thick_circle(g, p.x, p.y, 3);
        }
        if let Some(circles) = &self.icircles {
            for p in circles {
                draw_thick_circle(g, p.x, p.y, 3);
            }
        }
        self.base.update_dot_count(c);
        draw_dots(g, c, self.base.lead2, self.base.point2, self.base.curcount);
        draw_posts(g, &self.base, self, c);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.base.volt_source {
            s.stamp_voltage_source_var(NodeId::GROUND, self.base.nodes[self.input_count], vs);
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let mut f = self.calc_function();
        if self.kind.inverting() {
            f = !f;
        }
        if self.propagation_delay == 0.0 && self.last_time != s.t {
            if self.last_output == !f {
                self.oscillation_count += 1;
                if self.oscillation_count > 50 {
                    self.oscillation_count = 0;
                    if (s.random() * 10.0) as i32 > 5 {
                        f = self.last_output;
                    }
                }
            } else {
                self.oscillation_count = 0;
            }
            self.last_time = s.t;
        }
        if self.propagation_delay > 0.0 {
            if f != self.last_output {
                if self.delay_end_time == 0.0 {
                    self.delay_end_time = s.t + self.propagation_delay;
                } else if s.t >= self.delay_end_time {
                    self.last_output = f;
                    self.delay_end_time = 0.0;
                }
            } else {
                self.delay_end_time = 0.0;
            }
        } else {
            self.last_output = f;
        }
        let res = if self.last_output { self.high_voltage } else { 0.0 };
        if let Some(vs) = self.base.volt_source {
            s.update_voltage_source(vs, res);
        }
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        validate_rail_node(v, idx, self.base.nodes[self.input_count])
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n1: usize) -> bool {
        n1 == self.input_count
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == self.input_count { self.base.current } else { 0.0 }
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(self.gate_name().into());
        arr.push(format!("Vout = {}", voltage_text(self.base.volts[self.input_count])));
        arr.push(format!("Iout = {}", current_text(self.get_current())));
        if self.propagation_delay > 0.0 {
            arr.push(format!("delay = {}", unit_text(self.propagation_delay, "s")));
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        // an XOR gate has no "invert inputs" option, so its later entries shift up by one
        let n = if self.kind.is_xor() && n >= 3 { n + 1 } else { n };
        match n {
            0 => Some(
                EditInfo::new("# of Inputs", self.input_count as f64, 1.0, 8.0).set_dimensionless(),
            ),
            1 => Some(
                EditInfo::new("High Logic Voltage", self.high_voltage, 1.0, 10.0).set_unit_step(),
            ),
            2 => Some(EditInfo::checkbox("Schmitt Inputs", self.has_flag(FLAG_SCHMITT))),
            3 => Some(EditInfo::checkbox("Invert Inputs", self.has_flag(FLAG_INVERT_INPUTS))),
            4 => Some(EditInfo::new("Propagation Delay (s)", self.propagation_delay, 0.0, 0.0)),
            5 => Some(EditInfo::checkbox("DeMorgan's Symbol", self.has_flag(FLAG_DEMORGAN))),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        let n = if self.kind.is_xor() && n >= 3 { n + 1 } else { n };
        match n {
            0 => {
                if ei.value >= 1.0 {
                    self.input_count = ei.value as usize;
                    self.alloc_nodes();
                    self.setup_volts();
                    self.set_points();
                } else {
                    ei.error = Some("must be >= 1".into());
                }
            }
            1 => {
                self.high_voltage = ei.value;
                set_last_high_voltage(ei.value);
            }
            2 => {
                self.base.flags = ei.change_flag(self.base.flags, FLAG_SCHMITT);
                LAST_SCHMITT.store(self.has_flag(FLAG_SCHMITT), Ordering::Relaxed);
                self.set_points();
            }
            3 => {
                if ei.checkbox.unwrap_or(false) {
                    self.base.flags |= FLAG_INVERT_INPUTS;
                    self.base.flags &= !FLAG_DEMORGAN;
                } else {
                    self.base.flags &= !FLAG_INVERT_INPUTS;
                }
                self.set_points();
                ei.new_dialog = true;
            }
            4 => self.propagation_delay = ei.value,
            5 => {
                if ei.checkbox.unwrap_or(false) {
                    self.base.flags |= FLAG_DEMORGAN;
                    self.base.flags &= !FLAG_INVERT_INPUTS;
                } else {
                    self.base.flags &= !FLAG_DEMORGAN;
                }
                self.set_points();
                ei.new_dialog = true;
            }
            _ => {}
        }
    }
}

thread_local! {
    /// IEC gate drawing was on for the last frame; gate geometry depends on it.
    static EURO_GATES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

// ---------------------------------------------------------------------------------------------
// Inverter
// ---------------------------------------------------------------------------------------------

/// An inverter with a limited slew rate.
#[derive(Clone, Debug)]
pub struct InverterElm {
    base: Base,
    /// Output slew rate in volts per nanosecond.
    pub slew_rate: f64,
    /// Output voltage for a logic one.
    pub high_voltage: f64,
    last_output_voltage: f64,
    gate_poly: Polygon,
    pcircle: Point,
    center: Point,
}

impl InverterElm {
    fn blank(base: Base, high_voltage: f64) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        InverterElm {
            base,
            slew_rate: 0.5,
            high_voltage,
            last_output_voltage: 0.0,
            gate_poly: Polygon::new(),
            pcircle: Point::default(),
            center: Point::default(),
        }
    }

    /// An inverter being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        Self::blank(Base::new(xx, yy), last_high_voltage())
    }

    /// An inverter read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut e = Self::blank(Base::from_dump(xa, ya, xb, yb, f), 5.0);
        if let (Ok(sr), Ok(hv)) = (st.next_f64(), st.next_f64()) {
            e.slew_rate = sr;
            e.high_voltage = hv;
        }
        e
    }

    fn demorgan(&self) -> bool {
        self.base.flags & FLAG_DEMORGAN != 0
    }
}

impl Element for InverterElm {
    impl_element_base!("InverterElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'I')
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.slew_rate),
            js_number(self.high_voltage)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.slew_rate = xml.double_attr("sl", self.slew_rate);
        self.high_voltage = xml.double_attr("hi", self.high_voltage);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("sl", self.slew_rate);
        out.double_attr("hi", self.high_voltage);
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn shortcut(&self) -> i32 {
        i32::from(b'1')
    }

    fn set_points(&mut self) {
        self.base.no_diagonal = true;
        self.base.set_points();
        let hs = 16;
        let mut ww = 16;
        if f64::from(ww) > self.base.dn / 2.0 {
            ww = (self.base.dn / 2.0) as i32;
        }
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let wwf = f64::from(ww);
        self.base.lead1 = interp_point(p1, p2, 0.5 - wwf / dn);
        self.base.lead2 = interp_point(p1, p2, 0.5 + (wwf + 2.0) / dn);
        let (start, end);
        if self.demorgan() {
            self.pcircle = interp_point(p1, p2, 0.5 - (wwf - 4.0) / dn);
            start = interp_point(p1, p2, 0.5 - (wwf - 8.0) / dn);
            end = 0.5 + (wwf + 2.0) / dn;
        } else {
            self.pcircle = interp_point(p1, p2, 0.5 + (wwf - 1.0) / dn);
            start = self.base.lead1;
            end = 0.5 + (wwf - 5.0) / dn;
        }
        if EURO_GATES.with(|e| e.get()) {
            let l2 = interp_point(p1, p2, 0.5 + (wwf - 5.0) / dn);
            let (a, b) = interp_point2(self.base.lead1, l2, 0.0, f64::from(hs));
            let (d, c) = interp_point2(self.base.lead1, l2, 1.0, f64::from(hs));
            self.gate_poly = Polygon::from_points(&[a, b, c, d]);
            self.center = interp_point(self.base.lead1, l2, 0.5);
        } else {
            let (a, b) = interp_point2(start, self.base.lead2, 0.0, f64::from(hs));
            self.gate_poly = Polygon::from_points(&[a, b, interp_point(p1, p2, end)]);
        }
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        if EURO_GATES.with(|e| e.replace(c.iec_gates)) != c.iec_gates {
            self.set_points();
        }
        draw_posts(g, &self.base, self, c);
        draw_2_leads(g, &self.base, c);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        draw_thick_polygon(g, &self.gate_poly);
        if c.iec_gates {
            let (cx, cy) = (self.center.x, self.center.y - 6);
            draw_centered_text(g, &mut self.base, "1", cx, cy, true);
        }
        draw_thick_circle(g, self.pcircle.x, self.pcircle.y, 3);
        self.base.update_dot_count(c);
        draw_dots(g, c, self.base.lead2, self.base.point2, self.base.curcount);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.base.volt_source {
            s.stamp_voltage_source_var(NodeId::GROUND, self.base.nodes[1], vs);
        }
    }
    fn start_iteration(&mut self, _s: &Stamper) {
        self.last_output_voltage = self.base.volts[1];
    }
    fn do_step(&mut self, s: &mut Stamper) {
        let mut out =
            if self.base.volts[0] > self.high_voltage * 0.5 { 0.0 } else { self.high_voltage };
        let max_step = self.slew_rate * s.time_step * 1e9;
        out =
            (self.last_output_voltage + max_step).min(out).max(self.last_output_voltage - max_step);
        if let Some(vs) = self.base.volt_source {
            s.update_voltage_source(vs, out);
        }
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("inverter".into());
        arr.push(format!("Vi = {}", voltage_text(self.base.volts[0])));
        arr.push(format!("Vo = {}", voltage_text(self.base.volts[1])));
    }
    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Slew Rate (V/ns)", self.slew_rate, 0.0, 0.0)),
            1 => Some(
                EditInfo::new("High Logic Voltage", self.high_voltage, 1.0, 10.0).set_unit_step(),
            ),
            2 => Some(EditInfo::checkbox("DeMorgan's Symbol", self.demorgan())),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.slew_rate = ei.value,
            1 => {
                self.high_voltage = ei.value;
                set_last_high_voltage(ei.value);
            }
            2 => {
                self.base.flags = ei.change_flag(self.base.flags, FLAG_DEMORGAN);
                self.set_points();
            }
            _ => {}
        }
    }
    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        validate_rail_node(v, idx, self.base.nodes[1])
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n1: usize) -> bool {
        n1 == 1
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 1 { self.base.current } else { 0.0 }
    }
}

// ---------------------------------------------------------------------------------------------
// Schmitt triggers
// ---------------------------------------------------------------------------------------------

/// A Schmitt trigger buffer (`inverting` false) or inverter (`inverting` true).
#[derive(Clone, Debug)]
pub struct SchmittElm {
    base: Base,
    inverting: bool,
    /// Output slew rate in volts per nanosecond.
    pub slew_rate: f64,
    /// Input voltage below which the state flips low.
    pub lower_trigger: f64,
    /// Input voltage above which the state flips high.
    pub upper_trigger: f64,
    state: bool,
    /// Output voltage for logic one.
    pub logic_on_level: f64,
    /// Output voltage for logic zero.
    pub logic_off_level: f64,
    last_output_voltage: f64,
    gate_poly: Polygon,
    symbol_poly: Polygon,
    pcircle: Point,
    dlt: f64,
    dut: f64,
}

impl SchmittElm {
    fn blank(inverting: bool, base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        SchmittElm {
            base,
            inverting,
            slew_rate: 0.5,
            lower_trigger: 1.66,
            upper_trigger: 3.33,
            state: false,
            logic_on_level: 5.0,
            logic_off_level: 0.0,
            last_output_voltage: 0.0,
            gate_poly: Polygon::new(),
            symbol_poly: Polygon::new(),
            pcircle: Point::default(),
            dlt: 1.66,
            dut: 3.33,
        }
    }

    /// A Schmitt trigger being placed at (`xx`, `yy`).
    pub fn new(inverting: bool, xx: i32, yy: i32) -> Self {
        Self::blank(inverting, Base::new(xx, yy))
    }

    /// A Schmitt trigger read from a file.
    pub fn from_dump(
        inverting: bool,
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut e = Self::blank(inverting, Base::from_dump(xa, ya, xb, yb, f));
        let fields = (|| {
            Ok::<_, crate::format::tokenizer::TokenError>((
                st.next_f64()?,
                st.next_f64()?,
                st.next_f64()?,
                st.next_f64()?,
                st.next_f64()?,
            ))
        })();
        if let Ok((sr, lt, ut, on, off)) = fields {
            e.slew_rate = sr;
            e.lower_trigger = lt;
            e.upper_trigger = ut;
            e.logic_on_level = on;
            e.logic_off_level = off;
        }
        e.dlt = e.lower_trigger;
        e.dut = e.upper_trigger;
        e
    }
}

impl Element for SchmittElm {
    fn base(&self) -> &Base {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Base {
        &mut self.base
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn class_name(&self) -> &'static str {
        if self.inverting { "InvertingSchmittElm" } else { "SchmittElm" }
    }
    fn dump_type(&self) -> i32 {
        if self.inverting { 183 } else { 182 }
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.slew_rate),
            js_number(self.lower_trigger),
            js_number(self.upper_trigger),
            js_number(self.logic_on_level),
            js_number(self.logic_off_level)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.slew_rate = xml.double_attr("slr", self.slew_rate);
        self.lower_trigger = xml.double_attr("lt", self.lower_trigger);
        self.upper_trigger = xml.double_attr("ut", self.upper_trigger);
        self.logic_on_level = xml.double_attr("lon", self.logic_on_level);
        self.logic_off_level = xml.double_attr("loff", self.logic_off_level);
        self.dlt = self.lower_trigger;
        self.dut = self.upper_trigger;
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("slr", self.slew_rate);
        out.double_attr("lt", self.lower_trigger);
        out.double_attr("ut", self.upper_trigger);
        out.double_attr("lon", self.logic_on_level);
        out.double_attr("loff", self.logic_off_level);
    }
    fn voltage_source_count(&self) -> usize {
        1
    }

    fn set_points(&mut self) {
        self.base.no_diagonal = true;
        self.base.set_points();
        let hs = 16;
        let mut ww = 16;
        if f64::from(ww) > self.base.dn / 2.0 {
            ww = (self.base.dn / 2.0) as i32;
        }
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let wwf = f64::from(ww);
        self.base.lead1 = interp_point(p1, p2, 0.5 - wwf / dn);
        let tip = if self.inverting { wwf + 2.0 } else { wwf - 3.0 };
        self.base.lead2 = interp_point(p1, p2, 0.5 + tip / dn);
        if self.inverting {
            self.pcircle = interp_point(p1, p2, 0.5 + (wwf - 2.0) / dn);
        }
        let (a, b) = interp_point2(self.base.lead1, self.base.lead2, 0.0, f64::from(hs));
        self.gate_poly =
            Polygon::from_points(&[a, b, interp_point(p1, p2, 0.5 + (wwf - 5.0) / dn)]);
        self.symbol_poly = schmitt_polygon(&self.base, 1.0, 0.3);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        draw_posts(g, &self.base, self, c);
        draw_2_leads(g, &self.base, c);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        draw_thick_polygon(g, &self.gate_poly);
        g.set_line_width(2.0);
        draw_polygon(g, &self.symbol_poly);
        g.set_line_width(1.0);
        if self.inverting {
            draw_thick_circle(g, self.pcircle.x, self.pcircle.y, 3);
        }
        self.base.update_dot_count(c);
        draw_dots(g, c, self.base.lead2, self.base.point2, self.base.curcount);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.base.volt_source {
            s.stamp_voltage_source_var(NodeId::GROUND, self.base.nodes[1], vs);
        }
    }
    fn start_iteration(&mut self, _s: &Stamper) {
        self.last_output_voltage = self.base.volts[1];
    }
    fn do_step(&mut self, s: &mut Stamper) {
        let v_in = self.base.volts[0];
        // the inverting version slews from the present output, the other from the output at the
        // start of the iteration
        let from = if self.inverting { self.base.volts[1] } else { self.last_output_voltage };
        let (on, off) = (self.logic_on_level, self.logic_off_level);
        let out;
        if self.state {
            if v_in > self.upper_trigger {
                self.state = false;
                out = if self.inverting { off } else { on };
            } else {
                out = if self.inverting { on } else { off };
            }
        } else if v_in < self.lower_trigger {
            self.state = true;
            out = if self.inverting { on } else { off };
        } else {
            out = if self.inverting { off } else { on };
        }
        let max_step = self.slew_rate * s.time_step * 1e9;
        let out = (from + max_step).min(out).max(from - max_step);
        if let Some(vs) = self.base.volt_source {
            s.update_voltage_source(vs, out);
        }
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        if self.inverting {
            arr.push("inverting Schmitt trigger".into());
            arr.push(format!("Vi = {}", voltage_text(self.base.volts[0])));
            arr.push(format!("Vo = {}", voltage_text(self.base.volts[1])));
        } else {
            arr.push("Schmitt Trigger~".into());
        }
    }
    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Lower threshold (V)", self.lower_trigger, 0.01, 5.0)),
            1 => Some(EditInfo::new("Upper threshold (V)", self.upper_trigger, 0.01, 5.0)),
            2 => Some(EditInfo::new("Slew Rate (V/ns)", self.slew_rate, 0.0, 0.0)),
            3 => Some(
                EditInfo::new("High Logic Voltage", self.logic_on_level, 0.0, 0.0).set_unit_step(),
            ),
            4 => Some(
                EditInfo::new("Low Voltage (V)", self.logic_off_level, 0.0, 0.0).set_unit_step(),
            ),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.dlt = ei.value,
            1 => self.dut = ei.value,
            2 => self.slew_rate = ei.value,
            3 => self.logic_on_level = ei.value,
            4 => self.logic_off_level = ei.value,
            _ => {}
        }
        if self.dlt > self.dut {
            self.upper_trigger = self.dlt;
            self.lower_trigger = self.dut;
        } else {
            self.upper_trigger = self.dut;
            self.lower_trigger = self.dlt;
        }
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn has_ground_connection(&self, n1: usize) -> bool {
        n1 == 1
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 1 { self.base.current } else { 0.0 }
    }
}

// ---------------------------------------------------------------------------------------------
// Logic input and output
// ---------------------------------------------------------------------------------------------

const LI_FLAG_TERNARY: i32 = 1;
const LI_FLAG_NUMERIC: i32 = 2;

/// A logic input: a switch that drives a node high or low.
#[derive(Clone, Debug)]
pub struct LogicInputElm {
    inner: SwitchElm,
    /// Voltage of logic one.
    pub hi_v: f64,
    /// Voltage of logic zero.
    pub lo_v: f64,
}

impl LogicInputElm {
    /// A logic input being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        LogicInputElm { inner: SwitchElm::new(xx, yy), hi_v: 5.0, lo_v: 0.0 }
    }

    /// A logic input read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, crate::format::tokenizer::TokenError> {
        let mut inner = SwitchElm::from_dump_as(xa, ya, xb, yb, f, st, true)?;
        let (hi_v, lo_v) = match (st.next_f64(), st.next_f64()) {
            (Ok(h), Ok(l)) => (h, l),
            _ => (5.0, 0.0),
        };
        if f & LI_FLAG_TERNARY != 0 {
            inner.pos_count = 3;
        }
        Ok(LogicInputElm { inner, hi_v, lo_v })
    }

    fn is_ternary(&self) -> bool {
        self.inner.base().flags & LI_FLAG_TERNARY != 0
    }
    fn is_numeric(&self) -> bool {
        self.inner.base().flags & (LI_FLAG_TERNARY | LI_FLAG_NUMERIC) != 0
    }
}

impl Element for LogicInputElm {
    fn base(&self) -> &Base {
        self.inner.base()
    }
    fn base_mut(&mut self) -> &mut Base {
        self.inner.base_mut()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn class_name(&self) -> &'static str {
        "LogicInputElm"
    }
    fn dump_type(&self) -> i32 {
        i32::from(b'L')
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {} {}",
            self.base().dump_prefix(self.dump_type()),
            self.inner.dump_fields(),
            js_number(self.hi_v),
            js_number(self.lo_v)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.inner.undump_xml(xml);
        self.hi_v = xml.double_attr("hi", self.hi_v);
        self.lo_v = xml.double_attr("lo", self.lo_v);
        self.inner.pos_count = if self.is_ternary() { 3 } else { 2 };
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        self.inner.dump_xml(out);
        if self.hi_v != 5.0 {
            out.double_attr("hi", self.hi_v);
        }
        if self.lo_v != 0.0 {
            out.double_attr("lo", self.lo_v);
        }
    }
    fn post_count(&self) -> usize {
        1
    }
    fn voltage_source_count(&self) -> usize {
        1
    }

    fn set_points(&mut self) {
        let b = self.inner.base_mut();
        b.set_points();
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - 12.0 / b.dn);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        g.set_font_size(20);
        g.set_color(if self.base().needs_highlight() { c.select_color } else { c.white });
        let pos = self.inner.position;
        let s = if self.is_numeric() {
            pos.to_string()
        } else if pos == 0 {
            "L".to_string()
        } else {
            "H".to_string()
        };
        let (p1, l1) = (self.base().point1, self.base().lead1);
        self.base_mut().set_bbox_pts(p1, l1, 0.0);
        let (x2, y2) = (self.base().x2, self.base().y2);
        draw_centered_text(g, self.base_mut(), &s, x2, y2, true);
        set_voltage_color(g, self.base(), c, self.base().volts[0]);
        draw_thick_line(g, p1, l1);
        self.base_mut().update_dot_count(c);
        let cc = -self.base().curcount;
        draw_dots(g, c, p1, l1, cc);
        draw_posts(g, self.base(), self, c);
        g.restore();
    }

    fn click_rect(&self) -> Option<Rect> {
        let b = self.base();
        Some(Rect::new(b.x2 - 10, b.y2 - 10, 20, 20))
    }
    fn simple_toggle(&mut self) {
        self.inner.simple_toggle();
    }
    fn link_label(&self) -> Option<&str> {
        None
    }
    fn is_momentary(&self) -> bool {
        self.inner.momentary
    }
    fn toggle_key(&self) -> Option<&str> {
        self.inner.key_shortcut.as_deref()
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.base().volt_source {
            s.stamp_voltage_source_var(NodeId::GROUND, self.base().nodes[0], vs);
        }
    }
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base_mut().volt_source = Some(v);
    }
    fn do_step(&mut self, s: &mut Stamper) {
        let pos = self.inner.position;
        let mut v = if pos == 0 { self.lo_v } else { self.hi_v };
        if self.is_ternary() {
            v = self.lo_v + f64::from(pos) * (self.hi_v - self.lo_v) * 0.5;
        }
        if let Some(vs) = self.base().volt_source {
            s.update_voltage_source(vs, v);
        }
    }
    fn calculate_current(&mut self) {}
    fn is_wire_equivalent(&self) -> bool {
        false
    }
    fn is_removable_wire(&self) -> bool {
        false
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base().volts[0]
    }
    fn elm_type(&self) -> Option<String> {
        Some("logic input".into())
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("logic input".into());
        let mut state =
            if self.inner.position == 0 { "low".to_string() } else { "high".to_string() };
        if self.is_numeric() {
            state = self.inner.position.to_string();
        }
        arr.push(format!("{state} ({})", voltage_text(self.base().volts[0])));
        arr.push(format!("I = {}", current_text(self.get_current())));
    }
    fn has_ground_connection(&self, _n1: usize) -> bool {
        true
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn get_current_into_node(&self, _n: usize) -> f64 {
        self.base().current
    }
    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        validate_rail_node(v, idx, self.base().nodes[0])
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::checkbox("Momentary Switch", self.inner.momentary)),
            1 => Some(EditInfo::new("High Logic Voltage", self.hi_v, 10.0, -10.0).set_unit_step()),
            2 => Some(EditInfo::new("Low Voltage", self.lo_v, 10.0, -10.0).set_unit_step()),
            3 => Some(EditInfo::checkbox("Numeric", self.is_numeric())),
            4 => Some(EditInfo::checkbox("Ternary", self.is_ternary())),
            5 => Some(EditInfo::text(
                "Keyboard Shortcut",
                self.inner.key_shortcut.as_deref().unwrap_or(""),
            )),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.inner.momentary = ei.checkbox.unwrap_or(false),
            1 => self.hi_v = ei.value,
            2 => self.lo_v = ei.value,
            3 => {
                let b = self.base_mut();
                b.flags = ei.change_flag(b.flags, LI_FLAG_NUMERIC);
            }
            4 => {
                let b = self.base_mut();
                b.flags = ei.change_flag(b.flags, LI_FLAG_TERNARY);
                self.inner.pos_count = if self.is_ternary() { 3 } else { 2 };
            }
            5 => {
                let t = ei.text.clone().unwrap_or_default();
                self.inner.key_shortcut =
                    t.trim().chars().next().map(|c| c.to_lowercase().to_string());
            }
            _ => {}
        }
    }
    fn shortcut(&self) -> i32 {
        i32::from(b'i')
    }
}

const LO_FLAG_TERNARY: i32 = 1;
const LO_FLAG_NUMERIC: i32 = 2;
const LO_FLAG_PULLDOWN: i32 = 4;

/// A logic output: shows `L`/`H` (or a digit) for the voltage at its terminal.
#[derive(Clone, Debug)]
pub struct LogicOutputElm {
    base: Base,
    /// Voltage separating low from high.
    pub threshold: f64,
    value: String,
}

impl LogicOutputElm {
    /// A logic output being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        LogicOutputElm { base: Base::new(xx, yy), threshold: 2.5, value: String::new() }
    }

    /// A logic output read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        LogicOutputElm {
            base: Base::from_dump(xa, ya, xb, yb, f),
            threshold: st.next_f64().unwrap_or(2.5),
            value: String::new(),
        }
    }

    fn is_ternary(&self) -> bool {
        self.base.flags & LO_FLAG_TERNARY != 0
    }
    fn is_numeric(&self) -> bool {
        self.base.flags & (LO_FLAG_TERNARY | LO_FLAG_NUMERIC) != 0
    }
    fn needs_pull_down(&self) -> bool {
        self.base.flags & LO_FLAG_PULLDOWN != 0
    }
}

impl Element for LogicOutputElm {
    impl_element_base!("LogicOutputElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'M')
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!("{} {}", self.base.dump_prefix(self.dump_type()), js_number(self.threshold))
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.threshold = xml.double_attr("th", self.threshold);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        if self.threshold != 2.5 {
            out.double_attr("th", self.threshold);
        }
    }
    fn post_count(&self) -> usize {
        1
    }
    fn set_points(&mut self) {
        self.base.set_points();
        self.base.lead1 =
            interp_point(self.base.point1, self.base.point2, 1.0 - 12.0 / self.base.dn);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        g.set_font_size(20);
        g.set_color(c.light_gray);
        let v = self.base.volts[0];
        let mut s = if v < self.threshold { "L" } else { "H" }.to_string();
        if self.is_ternary() {
            s = if v > self.threshold * 1.5 {
                "2"
            } else if v > self.threshold * 0.5 {
                "1"
            } else {
                "0"
            }
            .to_string();
        } else if self.is_numeric() {
            s = if v < self.threshold { "0" } else { "1" }.to_string();
        }
        self.value = s.clone();
        let (p1, l1) = (self.base.point1, self.base.lead1);
        self.base.set_bbox_pts(p1, l1, 0.0);
        let (x2, y2) = (self.base.x2, self.base.y2);
        g.set_text_baseline(TextBaseline::Middle);
        draw_centered_text(g, &mut self.base, &s, x2, y2, true);
        set_voltage_color(g, &self.base, c, v);
        draw_thick_line(g, p1, l1);
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if self.needs_pull_down() {
            s.stamp_resistor(self.base.nodes[0], NodeId::GROUND, 1e6);
        }
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn elm_type(&self) -> Option<String> {
        Some("logic output".into())
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("logic output".into());
        let mut state =
            if self.base.volts[0] < self.threshold { "low" } else { "high" }.to_string();
        if self.is_numeric() {
            state = self.value.clone();
        }
        arr.push(state);
        arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
    }
    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Threshold", self.threshold, 10.0, -10.0)),
            1 => Some(EditInfo::checkbox("Current Required", self.needs_pull_down())),
            2 => Some(EditInfo::checkbox("Numeric", self.is_numeric())),
            3 => Some(EditInfo::checkbox("Ternary", self.is_ternary())),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.threshold = ei.value,
            1 => self.base.flags = ei.change_flag(self.base.flags, LO_FLAG_PULLDOWN),
            2 => self.base.flags = ei.change_flag(self.base.flags, LO_FLAG_NUMERIC),
            3 => self.base.flags = ei.change_flag(self.base.flags, LO_FLAG_TERNARY),
            _ => {}
        }
    }
    fn shortcut(&self) -> i32 {
        i32::from(b'o')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::registry::Registry;
    use crate::format::parser::parse;
    use crate::solver::sim::Simulation;

    fn run(text: &str, steps: usize) -> Simulation {
        let r = parse(text, &Registry::standard());
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        sim.analyze().unwrap();
        sim.run_steps(steps).unwrap();
        sim
    }

    /// Two logic inputs (`a`, `b` = 0 low, 1 high) into a gate of type code `code`, with a
    /// pull-down resistor on the output; returns the output voltage.
    fn gate_output(code: i32, a: i32, b: i32) -> f64 {
        let hdr = "$ 1 0.000005 10 50 5 50 5e-11";
        // a two-input gate from (64, 8) to (160, 8) has its inputs at (64, -8) and (64, 24)
        let text = format!(
            "{hdr}\nL 64 -8 32 -8 0 {a} false 5 0\nL 64 24 32 24 0 {b} false 5 0\n{code} 64 8 160 8 0 2 0 5\nr 160 8 160 64 0 10000\ng 160 64 160 96 0 0\n"
        );
        let sim = run(&text, 50);
        let gate = sim.elements.iter().find(|e| e.dump_type() == code).unwrap();
        gate.base().volts[gate.post_count() - 1]
    }

    #[test]
    fn gate_truth_tables() {
        for (code, table) in [
            (150, [0.0, 0.0, 0.0, 5.0]),
            (151, [5.0, 5.0, 5.0, 0.0]),
            (152, [0.0, 5.0, 5.0, 5.0]),
            (153, [5.0, 0.0, 0.0, 0.0]),
            (154, [0.0, 5.0, 5.0, 0.0]),
            (431, [5.0, 0.0, 0.0, 5.0]),
        ] {
            for (k, want) in table.iter().enumerate() {
                let (a, b) = ((k >> 1) as i32 & 1, k as i32 & 1);
                let got = gate_output(code, a, b);
                assert!((got - want).abs() < 0.05, "gate {code} inputs {a}{b}: {got} vs {want}");
            }
        }
    }

    #[test]
    fn gate_dump_round_trips() {
        let mut st = Tokenizer::new("3 5 5");
        let g = GateElm::from_dump(GateKind::Nand, 0, 0, 64, 0, 0, &mut st).unwrap();
        assert_eq!(g.input_count, 3);
        assert_eq!(g.dump(), "151 0 0 64 0 0 3 0 5");
        assert_eq!(g.post_count(), 4);
    }

    #[test]
    fn inverter_inverts_with_a_limited_slew_rate() {
        let hdr = "$ 1 0.000005 10 50 5 50 5e-11";
        let text = format!(
            "{hdr}\nL 16 0 -16 0 0 0 false 5 0\nI 64 0 128 0 0 0.5 5\nw 16 0 64 0 0\nr 128 0 128 64 0 10000\ng 128 64 128 96 0 0\n"
        );
        let early = run(&text, 20);
        let inv = early.elements.iter().find(|e| e.dump_type() == i32::from(b'I')).unwrap();
        // 0.5 V/ns is far faster than one 5 us step, so the output is already high
        assert!((inv.base().volts[1] - 5.0).abs() < 0.05, "{}", inv.base().volts[1]);
    }

    #[test]
    fn logic_output_shows_numeric_and_ternary_digits() {
        let mut o = LogicOutputElm::new(0, 0);
        o.base.volts = vec![4.0];
        o.base.flags = LO_FLAG_NUMERIC;
        let mut g = crate::render::svg::SvgPainter::new(100.0, 100.0);
        o.set_points();
        o.draw(&mut g, &DrawCtx::default());
        assert_eq!(o.value, "1");
        o.base.flags = LO_FLAG_TERNARY;
        o.base.volts = vec![4.0];
        o.draw(&mut g, &DrawCtx::default());
        assert_eq!(o.value, "2");
    }

    #[test]
    fn logic_input_reads_true_false_reversed_and_cycles_ternary_positions() {
        let mut st = Tokenizer::new("true false 5 0");
        let li = LogicInputElm::from_dump(0, 0, 64, 0, 0, &mut st).unwrap();
        assert_eq!(li.inner.position, 0);
        let mut st = Tokenizer::new("0 false 5 0");
        let mut t = LogicInputElm::from_dump(0, 0, 64, 0, LI_FLAG_TERNARY, &mut st).unwrap();
        t.simple_toggle();
        t.simple_toggle();
        assert_eq!(t.inner.position, 2);
        t.simple_toggle();
        assert_eq!(t.inner.position, 0);
    }

    #[test]
    fn schmitt_trigger_has_hysteresis() {
        let mut s = SchmittElm::new(true, 0, 0);
        s.base.volts = vec![0.0, 0.0];
        // slew is not the point here
        let mut st = Stamper { time_step: 1.0, ..Stamper::default() };
        // no voltage source is attached, so only the state machine is exercised
        s.base.volts[0] = 1.0;
        s.do_step(&mut st);
        assert!(s.state);
        s.base.volts[0] = 2.5; // between the thresholds: stays high
        s.do_step(&mut st);
        assert!(s.state);
        s.base.volts[0] = 4.0;
        s.do_step(&mut st);
        assert!(!s.state);
    }
}
