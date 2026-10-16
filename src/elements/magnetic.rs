// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (TransformerElm, TransLineElm).
//! Coupled inductors and the lossless transmission line.

use super::element::*;
use super::geometry::*;
use super::reactive::FLAG_BACK_EULER;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::{TokenError, Tokenizer};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale;
use crate::render::painter::Painter;
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name used for the feature switch.
pub const FAMILY: &str = "reactive";
const PASSIVE: &str = "Passive Components";

/// Registers the elements of this module.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("TransformerElm", FAMILY, "Add Transformer", PASSIVE, |x, y| {
            Box::new(TransformerElm::new(x, y))
        })
        .dump(i32::from(b'T'), |a, b, c, d, f, st| {
            Ok(Box::new(TransformerElm::from_dump(a, b, c, d, f, st)?))
        })
        .shortcut('T'),
    );
    r.add(
        ElementDescriptor::new("TransLineElm", FAMILY, "Add Transmission Line", PASSIVE, |x, y| {
            Box::new(TransLineElm::new(x, y))
        })
        .dump(171, |a, b, c, d, f, st| Ok(Box::new(TransLineElm::from_dump(a, b, c, d, f, st)?))),
    );
}

// ---------------------------------------------------------------------------------------------
// Transformer
// ---------------------------------------------------------------------------------------------

const FLAG_REVERSE: i32 = 4;
const FLAG_VERTICAL: i32 = 8;
const FLAG_FLIP: i32 = 16;

/// Two coupled inductors, optionally with a saturating core.
#[derive(Clone, Debug)]
pub struct TransformerElm {
    base: Base,
    inductance: f64,
    ratio: f64,
    coupling: f64,
    saturation_current: f64,
    current: [f64; 2],
    curcount: [f64; 2],
    width: i32,
    polarity: i32,
    flip: i32,
    pt_ends: [Point; 4],
    pt_coil: [Point; 4],
    pt_core: [Point; 4],
    dots: Option<[Point; 2]>,
    a: [f64; 4],
    cur_source: [f64; 2],
}

impl TransformerElm {
    fn blank(base: Base) -> Self {
        TransformerElm {
            base,
            inductance: 4.0,
            ratio: 1.0,
            coupling: 0.999,
            saturation_current: 0.0,
            current: [0.0; 2],
            curcount: [0.0; 2],
            width: 32,
            polarity: 1,
            flip: 1,
            pt_ends: [Point::default(); 4],
            pt_coil: [Point::default(); 4],
            pt_core: [Point::default(); 4],
            dots: None,
            a: [0.0; 4],
            cur_source: [0.0; 2],
        }
    }

    /// A transformer being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut base = Base::new(xx, yy);
        base.no_diagonal = true;
        TransformerElm::blank(base)
    }

    /// A transformer read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut base = Base::from_dump(xa, ya, xb, yb, f);
        base.no_diagonal = true;
        let mut t = TransformerElm::blank(base);
        t.width =
            if f & FLAG_VERTICAL != 0 { -32.max((xb - xa).abs()) } else { 32.max((yb - ya).abs()) };
        t.inductance = st.next_f64()?;
        t.ratio = st.next_f64()?;
        t.current[0] = st.next_f64()?;
        t.current[1] = st.next_f64()?;
        if let (Ok(c), Ok(s)) = (st.next_f64(), st.next_f64()) {
            t.coupling = c;
            t.saturation_current = s;
        }
        t.polarity = if f & FLAG_REVERSE != 0 { -1 } else { 1 };
        Ok(t)
    }

    fn vertical(&self) -> bool {
        self.base.flags & FLAG_VERTICAL != 0
    }

    fn trapezoidal(&self) -> bool {
        self.base.flags & FLAG_BACK_EULER == 0
    }

    fn effective_inductance(l0: f64, i: f64, isat: f64) -> f64 {
        if isat <= 0.0 {
            return l0;
        }
        let r = i / isat;
        l0 / (1.0 + r * r)
    }

    fn compute_coefficients(&mut self, l1: f64, l2: f64, m: f64, time_step: f64) {
        let deti = 1.0 / (l1 * l2 - m * m);
        let ts = if self.trapezoidal() { time_step / 2.0 } else { time_step };
        self.a = [l2 * deti * ts, -m * deti * ts, -m * deti * ts, l1 * deti * ts];
    }
}

impl Element for TransformerElm {
    impl_element_base!("TransformerElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'T')
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.inductance),
            js_number(self.ratio),
            js_number(self.current[0]),
            js_number(self.current[1]),
            js_number(self.coupling),
            js_number(self.saturation_current)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        let (b, v) = (&self.base, self.vertical());
        self.width = if v { -32.max((b.x2 - b.x).abs()) } else { 32.max((b.y2 - b.y).abs()) };
        self.inductance = xml.double_attr("in", self.inductance);
        self.ratio = xml.double_attr("ra", self.ratio);
        self.coupling = xml.double_attr("co", self.coupling);
        self.width = xml.int_attr("wi", self.width);
        self.saturation_current = xml.double_attr("isat", self.saturation_current);
        self.current[0] = xml.double_attr("c0", 0.0);
        self.current[1] = xml.double_attr("c1", 0.0);
        self.polarity = if self.base.flags & FLAG_REVERSE != 0 { -1 } else { 1 };
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("in", self.inductance);
        out.double_attr("ra", self.ratio);
        out.double_attr("co", self.coupling);
        out.int_attr("wi", self.width);
        if self.saturation_current != 0.0 {
            out.double_attr("isat", self.saturation_current);
        }
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.double_attr("c0", self.current[0]);
        out.double_attr("c1", self.current[1]);
    }

    fn post_count(&self) -> usize {
        4
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.pt_ends.get(n).copied()
    }
    fn non_linear(&self) -> bool {
        self.saturation_current > 0.0
    }

    fn drag(&mut self, xx: i32, yy: i32) {
        let (xx, mut yy) = (snap_grid(xx), snap_grid(yy));
        let b = &mut self.base;
        if (xx - b.x).abs() > (yy - b.y).abs() {
            b.flags &= !FLAG_VERTICAL;
        } else {
            b.flags |= FLAG_VERTICAL;
        }
        self.width = if b.flags & FLAG_VERTICAL != 0 {
            -32.max((xx - b.x).abs())
        } else {
            32.max((yy - b.y).abs())
        };
        if xx == b.x {
            yy = b.y;
        }
        b.x2 = xx;
        b.y2 = yy;
        self.set_points();
    }

    fn set_points(&mut self) {
        self.base.set_points();
        if self.vertical() {
            self.base.point2.x = self.base.point1.x;
        } else {
            self.base.point2.y = self.base.point1.y;
        }
        let b = &self.base;
        let (p1, p2, dn, dsign) = (b.point1, b.point2, b.dn, b.dsign);
        self.flip = if b.flags & FLAG_FLIP != 0 { -1 } else { 1 };
        let off = f64::from(-dsign * self.width * self.flip);
        let mut ends = [p1, p2, interp_point_g(p1, p2, 0.0, off), interp_point_g(p1, p2, 1.0, off)];
        let ce = 0.5 - 12.0 / dn;
        let cd = 0.5 - 2.0 / dn;
        let mut coil = [Point::default(); 4];
        let mut core = [Point::default(); 4];
        for i in [0, 2] {
            coil[i] = interp_point(ends[i], ends[i + 1], ce);
            coil[i + 1] = interp_point(ends[i], ends[i + 1], 1.0 - ce);
            core[i] = interp_point(ends[i], ends[i + 1], cd);
            core[i + 1] = interp_point(ends[i], ends[i + 1], 1.0 - cd);
        }
        self.dots = None;
        if self.polarity == -1 {
            let vsign = if self.vertical() { -1 } else { 1 };
            let dotp = (7.0 / f64::from(self.width)).abs();
            let o = f64::from(-7 * dsign * vsign * self.flip);
            self.dots = Some([
                interp_point_g(coil[0], coil[2], dotp, o),
                interp_point_g(coil[3], coil[1], dotp, o),
            ]);
            ends.swap(1, 3);
            coil.swap(1, 3);
        }
        self.pt_ends = ends;
        self.pt_coil = coil;
        self.pt_core = core;
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let v = self.base.volts.clone();
        for i in 0..4 {
            set_voltage_color(g, &self.base, c, v[i]);
            draw_thick_line(g, self.pt_ends[i], self.pt_coil[i]);
        }
        for i in 0..2 {
            set_power_color_w(g, &self.base, c, self.current[i] * (v[i] - v[i + 2]));
            let mut csign =
                self.base.dsign * (if i == 1 { -6 * self.polarity } else { 6 }) * self.flip;
            if self.vertical() {
                csign = -csign;
            }
            draw_coil(
                g,
                &self.base,
                c,
                csign,
                self.pt_coil[i],
                self.pt_coil[i + 2],
                v[i],
                v[i + 2],
            );
        }
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        for i in 0..2 {
            draw_thick_line(g, self.pt_core[i], self.pt_core[i + 2]);
            if let Some(d) = &self.dots {
                g.fill_oval(d[i].x - 2, d[i].y - 2, 5, 5);
            }
            self.curcount[i] = self.base.update_dot_count_for(self.current[i], self.curcount[i], c);
        }
        for i in 0..2 {
            draw_dots(g, c, self.pt_ends[i], self.pt_coil[i], self.curcount[i]);
            draw_dots(g, c, self.pt_coil[i], self.pt_coil[i + 2], self.curcount[i]);
            draw_dots(g, c, self.pt_ends[i + 2], self.pt_coil[i + 2], -self.curcount[i]);
        }
        draw_posts(g, &self.base, self, c);
        let (a, b) = (self.pt_ends[0], self.pt_ends[if self.polarity == 1 { 3 } else { 1 }]);
        self.base.set_bbox_pts(a, b, 0.0);
    }

    fn reset(&mut self) {
        self.current = [0.0; 2];
        self.curcount = [0.0; 2];
        self.cur_source = [0.0; 2];
        self.base.volts.fill(0.0);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let l1 = self.inductance;
        let l2 = self.inductance * self.ratio * self.ratio;
        let m = self.coupling * (l1 * l2).sqrt();
        self.compute_coefficients(l1, l2, m, s.time_step);
        let n = &self.base.nodes;
        if self.saturation_current <= 0.0 {
            s.stamp_conductance(n[0], n[2], self.a[0]);
            s.stamp_vccs(n[0], n[2], n[1], n[3], self.a[1]);
            s.stamp_vccs(n[1], n[3], n[0], n[2], self.a[2]);
            s.stamp_conductance(n[1], n[3], self.a[3]);
        }
    }

    fn start_iteration(&mut self, s: &Stamper) {
        if self.saturation_current > 0.0 {
            let l1 = Self::effective_inductance(
                self.inductance,
                self.current[0],
                self.saturation_current,
            );
            let l2 = Self::effective_inductance(
                self.inductance * self.ratio * self.ratio,
                self.current[1],
                self.saturation_current * self.ratio,
            );
            let m = self.coupling * (l1 * l2).sqrt();
            self.compute_coefficients(l1, l2, m, s.time_step);
        }
        let v = &self.base.volts;
        let (d1, d2) = (v[0] - v[2], v[1] - v[3]);
        if self.trapezoidal() {
            self.cur_source = [
                d1 * self.a[0] + d2 * self.a[1] + self.current[0],
                d1 * self.a[2] + d2 * self.a[3] + self.current[1],
            ];
        } else {
            self.cur_source = self.current;
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let n = &self.base.nodes;
        if self.saturation_current > 0.0 {
            s.stamp_conductance(n[0], n[2], self.a[0]);
            s.stamp_vccs(n[0], n[2], n[1], n[3], self.a[1]);
            s.stamp_vccs(n[1], n[3], n[0], n[2], self.a[2]);
            s.stamp_conductance(n[1], n[3], self.a[3]);
        }
        s.stamp_current_source(n[0], n[2], self.cur_source[0]);
        s.stamp_current_source(n[1], n[3], self.cur_source[1]);
    }

    fn calculate_current(&mut self) {
        let v = &self.base.volts;
        let (d1, d2) = (v[0] - v[2], v[1] - v[3]);
        self.current[0] = d1 * self.a[0] + d2 * self.a[1] + self.cur_source[0];
        self.current[1] = d1 * self.a[2] + d2 * self.a[3] + self.cur_source[1];
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        if n < 2 { -self.current[n] } else { self.current[n - 2] }
    }

    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        matches!((n1.min(n2), n1.max(n2)), (0, 2) | (1, 3))
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let v = &self.base.volts;
        arr.push(
            if self.saturation_current > 0.0 { "transformer (sat)" } else { "transformer" }.into(),
        );
        arr.push(format!("L = {}", unit_text(self.inductance, "H")));
        arr.push(format!("Ratio = 1:{}", self.ratio));
        arr.push(format!("Vd1 = {}", voltage_text(v[0] - v[2])));
        arr.push(format!("Vd2 = {}", voltage_text(v[1] - v[3])));
        arr.push(format!("I1 = {}", current_text(self.current[0])));
        arr.push(format!("I2 = {}", current_text(self.current[1])));
        if self.saturation_current > 0.0 {
            let eff = Self::effective_inductance(
                self.inductance,
                self.current[0],
                self.saturation_current,
            );
            arr.push(format!("L1eff = {}", unit_text(eff, "H")));
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("Primary Inductance (H)", self.inductance, 0.01, 5.0).set_positive(),
            ),
            1 => Some(
                EditInfo::new("Ratio (N1/N2)", 1.0 / self.ratio, 1.0, 10.0)
                    .set_dimensionless()
                    .set_positive(),
            ),
            2 => Some(
                EditInfo::new("Coupling Coefficient", self.coupling, 0.0, 1.0)
                    .set_dimensionless()
                    .set_positive(),
            ),
            3 => Some(EditInfo::checkbox("Trapezoidal Approximation", self.trapezoidal())),
            4 => Some(EditInfo::checkbox("Swap Secondary Polarity", self.polarity == -1)),
            5 => Some(EditInfo::number("Saturation Current (A) (0=none)", self.saturation_current)),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 if ei.value > 0.0 => self.inductance = ei.value,
            1 if ei.value > 0.0 => self.ratio = 1.0 / ei.value,
            2 => {
                if ei.value > 0.0 && ei.value < 1.0 {
                    self.coupling = ei.value;
                } else {
                    ei.error = Some("must be > 0 and < 1".into());
                }
            }
            3 => {
                if on {
                    self.base.flags &= !FLAG_BACK_EULER;
                } else {
                    self.base.flags |= FLAG_BACK_EULER;
                }
            }
            4 => {
                self.polarity = if on { -1 } else { 1 };
                if on {
                    self.base.flags |= FLAG_REVERSE;
                } else {
                    self.base.flags &= !FLAG_REVERSE;
                }
                self.set_points();
            }
            5 => {
                if ei.value >= 0.0 {
                    self.saturation_current = ei.value;
                } else {
                    ei.error = Some("must be >= 0".into());
                }
            }
            _ => {}
        }
    }

    fn shortcut(&self) -> i32 {
        i32::from(b'T')
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        if self.vertical() {
            self.base.flags ^= FLAG_FLIP;
        }
        self.base.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        if !self.vertical() {
            self.base.flags ^= FLAG_FLIP;
        }
        self.base.flip_y(center2);
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.base.flags ^= FLAG_VERTICAL;
        self.width = -self.width;
        self.base.flip_xy(xmy);
        self.set_points();
    }
}

// ---------------------------------------------------------------------------------------------
// Transmission line
// ---------------------------------------------------------------------------------------------

/// Lossless transmission line: two matched ports linked by a delay.
#[derive(Clone, Debug)]
pub struct TransLineElm {
    base: Base,
    delay: f64,
    imped: f64,
    width: i32,
    voltage_l: Vec<f64>,
    voltage_r: Vec<f64>,
    too_long: bool,
    len_steps: usize,
    ptr: usize,
    last_step_count: u64,
    vs: [Option<VsId>; 2],
    current1: f64,
    current2: f64,
    curcount1: f64,
    curcount2: f64,
    posts: [Point; 4],
    inner: [Point; 4],
}

impl TransLineElm {
    fn blank(base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        TransLineElm {
            base,
            delay: 5e-3,
            imped: 75.0,
            width: 0,
            voltage_l: Vec::new(),
            voltage_r: Vec::new(),
            too_long: false,
            len_steps: 0,
            ptr: 0,
            last_step_count: 0,
            vs: [None; 2],
            current1: 0.0,
            current2: 0.0,
            curcount1: 0.0,
            curcount2: 0.0,
            posts: [Point::default(); 4],
            inner: [Point::default(); 4],
        }
    }

    /// A line being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        TransLineElm::blank(Base::new(xx, yy))
    }

    /// A line read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut t = TransLineElm::blank(Base::from_dump(xa, ya, xb, yb, f));
        t.delay = st.next_f64()?;
        t.imped = st.next_f64()?;
        t.width = st.next_i32()?;
        st.next_token();
        Ok(t)
    }

    /// Allocates the delay buffers for the current time step; false when the line is too long.
    fn allocate(&mut self, max_time_step: f64) -> bool {
        if max_time_step <= 0.0 {
            return false;
        }
        let len = (self.delay / max_time_step) as usize;
        if len > 100_000 {
            self.too_long = true;
            self.voltage_l.clear();
            self.voltage_r.clear();
            return false;
        }
        let len = len.max(1);
        if len != self.len_steps || self.voltage_l.len() != len {
            self.len_steps = len;
            self.voltage_l = vec![0.0; len];
            self.voltage_r = vec![0.0; len];
            self.ptr = 0;
        }
        self.too_long = false;
        true
    }
}

impl Element for TransLineElm {
    impl_element_base!("TransLineElm");

    fn dump_type(&self) -> i32 {
        171
    }
    fn xml_dump_type(&self) -> String {
        "tl".into()
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} 0",
            self.base.dump_prefix(171),
            js_number(self.delay),
            js_number(self.imped),
            self.width
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.delay = xml.double_attr("de", self.delay);
        self.imped = xml.double_attr("im", self.imped);
        self.width = xml.int_attr("wi", self.width);
        self.reset();
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("de", self.delay);
        out.double_attr("im", self.imped);
        out.int_attr("wi", self.width);
    }

    fn post_count(&self) -> usize {
        4
    }
    fn internal_node_count(&self) -> usize {
        2
    }
    fn voltage_source_count(&self) -> usize {
        2
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.posts.get(n).copied()
    }

    fn drag(&mut self, xx: i32, yy: i32) {
        let (mut xx, mut yy) = (snap_grid(xx), snap_grid(yy));
        let g = grid_size();
        let b = &mut self.base;
        let w1 = g.max((yy - b.y).abs());
        let w2 = g.max((xx - b.x).abs());
        if w1 > w2 {
            xx = b.x;
            self.width = w2;
        } else {
            yy = b.y;
            self.width = w1;
        }
        b.x2 = xx;
        b.y2 = yy;
        self.set_points();
    }

    fn reset(&mut self) {
        self.voltage_l.clear();
        self.voltage_r.clear();
        self.len_steps = 0;
        self.ptr = 0;
        self.last_step_count = 0;
        self.base.volts.fill(0.0);
        self.base.curcount = 0.0;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &self.base;
        let (p1, p2) = (b.point1, b.point2);
        let ds = if b.dy == 0 { sign(b.dx) } else { -sign(b.dy) };
        let w = self.width;
        let sep = grid_size() / 2;
        let off = |o: i32| f64::from(-o * ds);
        let p3 = interp_point_g(p1, p2, 0.0, off(w));
        let p4 = interp_point_g(p1, p2, 1.0, off(w));
        let p5 = interp_point_g(p1, p2, 0.0, off(w / 2 - sep));
        let p6 = interp_point_g(p1, p2, 1.0, off(w / 2 - sep));
        let p7 = interp_point_g(p1, p2, 0.0, off(w / 2 + sep));
        let p8 = interp_point_g(p1, p2, 1.0, off(w / 2 + sep));
        self.posts = [p3, p4, p1, p2];
        self.inner = [p7, p8, p5, p6];
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        self.base.set_bbox_pts(self.posts[0], self.posts[3], 0.0);
        let segments = (self.base.dn / 2.0) as usize;
        let len = self.len_steps;
        let ix0 = self.ptr + len;
        let segf = 1.0 / segments.max(1) as f64;
        g.set_color(crate::render::painter::Color::DARK_GRAY);
        let (i2, i1) = (self.inner[2], self.inner[1]);
        g.fill_rect(
            f64::from(i2.x),
            f64::from(i2.y),
            f64::from(i1.x - i2.x + 2),
            f64::from(i1.y - i2.y + 2),
        );
        for i in 0..4 {
            set_voltage_color(g, &self.base, c, self.base.volts[i]);
            draw_thick_line(g, self.posts[i], self.inner[i]);
        }
        if len > 0 && self.voltage_l.len() == len {
            for i in 0..segments {
                let ix1 = (ix0 - 1 - len * i / segments) % len;
                let ix2 = (ix0 - 1 - len * (segments - 1 - i) / segments) % len;
                let v = (self.voltage_l[ix1] + self.voltage_r[ix2]) / 2.0;
                set_voltage_color(g, &self.base, c, v);
                let a = interp_point(self.inner[0], self.inner[1], i as f64 * segf);
                let b = interp_point(self.inner[2], self.inner[3], i as f64 * segf);
                g.draw_line(a.x, a.y, b.x, b.y);
                let a2 = interp_point(self.inner[2], self.inner[3], (i + 1) as f64 * segf);
                draw_thick_line(g, a2, b);
            }
        }
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        draw_thick_line(g, self.inner[0], self.inner[1]);
        draw_posts(g, &self.base, self, c);
        self.curcount1 = self.base.update_dot_count_for(-self.current1, self.curcount1, c);
        self.curcount2 = self.base.update_dot_count_for(self.current2, self.curcount2, c);
        if !self.base.creating {
            draw_dots(g, c, self.posts[0], self.inner[0], self.curcount1);
            draw_dots(g, c, self.posts[2], self.inner[2], -self.curcount1);
            draw_dots(g, c, self.posts[1], self.inner[1], -self.curcount2);
            draw_dots(g, c, self.posts[3], self.inner[3], self.curcount2);
        }
    }

    fn set_voltage_source(&mut self, n: usize, v: VsId) {
        if n < 2 {
            self.vs[n] = Some(v);
        }
    }
    fn voltage_source_nodes_at(&self, j: usize) -> Option<(NodeId, NodeId)> {
        let n = &self.base.nodes;
        match j {
            0 => Some((n[4], n[0])),
            _ => Some((n[5], n[1])),
        }
    }
    fn set_current(&mut self, vs: VsId, c: f64) {
        if self.vs[0] == Some(vs) {
            self.current1 = c;
        } else {
            self.current2 = c;
        }
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        if let (Some(a), Some(b)) = (self.vs[0], self.vs[1]) {
            s.stamp_voltage_source_var(n[4], n[0], a);
            s.stamp_voltage_source_var(n[5], n[1], b);
        }
        s.stamp_resistor(n[2], n[4], self.imped);
        s.stamp_resistor(n[3], n[5], self.imped);
    }

    fn start_iteration(&mut self, s: &Stamper) {
        if (self.len_steps == 0 || self.voltage_l.is_empty()) && !self.allocate(s.max_time_step) {
            return;
        }
        let v = &self.base.volts;
        self.voltage_l[self.ptr] = v[2] - v[0] + v[2] - v[4];
        self.voltage_r[self.ptr] = v[3] - v[1] + v[3] - v[5];
    }

    fn do_step(&mut self, s: &mut Stamper) {
        if self.voltage_l.is_empty() {
            s.stop("Transmission line delay too large!", None);
            return;
        }
        let next = (self.ptr + 1) % self.len_steps;
        if let (Some(a), Some(b)) = (self.vs[0], self.vs[1]) {
            s.update_voltage_source(a, -self.voltage_r[next]);
            s.update_voltage_source(b, -self.voltage_l[next]);
        }
        if self.base.volts[0].abs() > 1e-5 || self.base.volts[1].abs() > 1e-5 {
            s.stop("Need to ground transmission line!", None);
        }
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if s.time_step_count == self.last_step_count || self.len_steps == 0 {
            return;
        }
        self.last_step_count = s.time_step_count;
        self.ptr = (self.ptr + 1) % self.len_steps;
    }

    fn has_ground_connection(&self, _n1: usize) -> bool {
        false
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn get_matrix_connection(&self, n1: usize, n2: usize) -> bool {
        n1 % 2 == n2 % 2
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        match n {
            0 => self.current1,
            2 => -self.current1,
            3 => -self.current2,
            _ => self.current2,
        }
    }

    fn can_flip_x(&self) -> bool {
        self.base.dy == 0
    }
    fn can_flip_y(&self) -> bool {
        self.base.dx == 0
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("transmission line".into());
        arr.push(unit_text(self.imped, locale::OHM));
        arr.push(format!("length = {}", unit_text(0.65 * 2.9979e8 * self.delay, "m")));
        arr.push(format!("delay = {}", unit_text(self.delay, "s")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Delay (s)", self.delay, 0.0, 0.0).set_positive()),
            1 => Some(EditInfo::new("Impedance (ohms)", self.imped, 0.0, 0.0).set_positive()),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if ei.value <= 0.0 {
            return;
        }
        match n {
            0 => self.delay = ei.value,
            1 => self.imped = ei.value,
            _ => return,
        }
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transformer_dump_round_trips() {
        let mut st = Tokenizer::new("4 1 0 0 0.999 0");
        let t = TransformerElm::from_dump(0, 0, 64, 32, 0, &mut st).unwrap();
        assert_eq!(t.dump(), "T 0 0 64 32 0 4 1 0 0 0.999 0");
    }

    #[test]
    fn transmission_line_dump_round_trips() {
        let mut st = Tokenizer::new("0.005 75 32 0");
        let t = TransLineElm::from_dump(0, 0, 64, 0, 0, &mut st).unwrap();
        assert_eq!(t.dump(), "171 0 0 64 0 0 0.005 75 32 0");
        assert_eq!(t.post_count(), 4);
    }
}
