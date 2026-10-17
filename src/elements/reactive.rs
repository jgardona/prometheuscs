// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CapacitorElm, PolarCapacitorElm,
// InductorElm, Inductor).
//! Capacitors and inductors.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::{TokenError, Tokenizer};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale;
use crate::render::painter::{Color, Painter};
use crate::scope;
use crate::solver::path::{PathKind, Validator};
use crate::solver::sim::dc_analysis;
use crate::solver::stamper::{NodeId, Stamper};

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "reactive";

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "CapacitorElm",
            FAMILY,
            "Add Capacitor",
            "Passive Components",
            |x, y| Box::new(CapacitorElm::new(x, y)),
        )
        .dump(i32::from(b'c'), |a, b, c, d, f, st| {
            Ok(Box::new(CapacitorElm::from_dump(a, b, c, d, f, st)?))
        })
        .icon("capacitor")
        .shortcut('c')
        .quick(),
    );
    r.add(
        ElementDescriptor::new(
            "PolarCapacitorElm",
            FAMILY,
            "Add Capacitor (polarized)",
            "Passive Components",
            |x, y| Box::new(CapacitorElm::new_polar(x, y)),
        )
        .dump(209, |a, b, c, d, f, st| {
            Ok(Box::new(CapacitorElm::from_dump_polar(a, b, c, d, f, st)?))
        })
        .shortcut('C'),
    );
    r.add(
        ElementDescriptor::new(
            "InductorElm",
            FAMILY,
            "Add Inductor",
            "Passive Components",
            |x, y| Box::new(InductorElm::new(x, y)),
        )
        .dump(i32::from(b'l'), |a, b, c, d, f, st| {
            Ok(Box::new(InductorElm::from_dump(a, b, c, d, f, st)?))
        })
        .icon("induct")
        .shortcut('L')
        .quick(),
    );
}

/// Capacitor flag: use backward Euler integration instead of trapezoidal.
pub const FLAG_BACK_EULER: i32 = 2;
/// Capacitor flag: the dump contains a series resistance.
pub const FLAG_RESISTANCE: i32 = 4;

/// State that only polarized capacitors have.
#[derive(Clone, Debug)]
struct Polar {
    max_negative_voltage: f64,
    plate_points: Vec<Point>,
    plus_point: Point,
}

/// A capacitor, optionally polarized.
#[derive(Clone, Debug)]
pub struct CapacitorElm {
    base: Base,
    /// Capacitance in farads.
    pub capacitance: f64,
    comp_resistance: f64,
    voltdiff: f64,
    /// Equivalent series resistance in ohms (0 for an ideal capacitor).
    pub series_resistance: f64,
    /// Voltage on the capacitor after a reset.
    pub initial_voltage: f64,
    cap_node2: usize,
    plate1: [Point; 2],
    plate2: [Point; 2],
    cur_source_value: f64,
    polar: Option<Polar>,
}

impl CapacitorElm {
    /// A capacitor being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        CapacitorElm {
            base: Base::new(xx, yy),
            capacitance: 1e-5,
            comp_resistance: 0.0,
            voltdiff: 0.0,
            series_resistance: 0.0,
            initial_voltage: 1e-3,
            cap_node2: 1,
            plate1: [Point::default(); 2],
            plate2: [Point::default(); 2],
            cur_source_value: 0.0,
            polar: None,
        }
    }

    /// A polarized capacitor being placed at (`xx`, `yy`).
    pub fn new_polar(xx: i32, yy: i32) -> Self {
        let mut c = Self::new(xx, yy);
        c.polar = Some(Polar {
            max_negative_voltage: 1.0,
            plate_points: Vec::new(),
            plus_point: Point::default(),
        });
        c
    }

    /// A capacitor read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut c = CapacitorElm::new(xa, ya);
        c.base = Base::from_dump(xa, ya, xb, yb, f);
        c.capacitance = st.next_f64()?;
        c.voltdiff = st.next_f64()?;
        c.initial_voltage = 1e-3;
        // the remaining fields are optional in older files
        if let Ok(v) = st.next_f64() {
            c.initial_voltage = v;
            if c.base.flags & FLAG_RESISTANCE != 0 {
                if let Ok(r) = st.next_f64() {
                    c.series_resistance = r;
                }
            }
        }
        Ok(c)
    }

    /// A polarized capacitor read from a file.
    pub fn from_dump_polar(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut c = Self::from_dump(xa, ya, xb, yb, f, st)?;
        c.polar = Some(Polar {
            max_negative_voltage: st.next_f64()?,
            plate_points: Vec::new(),
            plus_point: Point::default(),
        });
        Ok(c)
    }

    fn is_trapezoidal(&self) -> bool {
        self.base.flags & FLAG_BACK_EULER == 0
    }

    fn shorted(&mut self) {
        Element::reset(self);
        self.voltdiff = 0.0;
        self.base.current = 0.0;
        self.base.curcount = 0.0;
        self.cur_source_value = 0.0;
    }

    /// Sets the series resistance (used when validation finds a capacitor loop).
    pub fn set_series_resistance(&mut self, r: f64) {
        self.series_resistance = r;
    }

    /// Charge on the capacitor in coulombs.
    pub fn charge(&self) -> f64 {
        self.capacitance * self.voltdiff
    }
}

impl Element for CapacitorElm {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        if self.polar.is_some() { "PolarCapacitorElm" } else { "CapacitorElm" }
    }
    fn dump_type(&self) -> i32 {
        if self.polar.is_some() { 209 } else { i32::from(b'c') }
    }
    fn xml_dump_type(&self) -> String {
        if self.polar.is_some() { "pc".into() } else { "c".into() }
    }

    fn dump(&self) -> String {
        // the original sets FLAG_RESISTANCE while dumping so the series resistance is read back
        let mut b = self.base.clone();
        b.flags |= FLAG_RESISTANCE;
        let mut s = format!(
            "{} {} {} {} {}",
            b.dump_prefix(self.dump_type()),
            js_number(self.capacitance),
            js_number(self.voltdiff),
            js_number(self.initial_voltage),
            js_number(self.series_resistance)
        );
        if let Some(p) = &self.polar {
            s.push(' ');
            s.push_str(&js_number(p.max_negative_voltage));
        }
        s
    }

    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.capacitance = xml.double_attr("c", self.capacitance);
        self.initial_voltage = xml.double_attr("iv", self.initial_voltage);
        self.series_resistance = xml.double_attr("sr", self.series_resistance);
        self.voltdiff = xml.double_attr("vd", self.voltdiff);
        if let Some(p) = &mut self.polar {
            p.max_negative_voltage = xml.double_attr("mv", p.max_negative_voltage);
        }
        self.alloc_nodes();
    }

    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("c", self.capacitance);
        out.double_attr("iv", self.initial_voltage);
        out.double_attr("sr", self.series_resistance);
        if let Some(p) = &self.polar {
            out.double_attr("mv", p.max_negative_voltage);
        }
    }

    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.double_attr("vd", self.voltdiff);
    }

    fn reset(&mut self) {
        let b = &mut self.base;
        b.volts.iter_mut().for_each(|v| *v = 0.0);
        b.curcount = 0.0;
        b.current = 0.0;
        self.cur_source_value = 0.0;
        // put a small charge on capacitors when reset to start oscillators
        self.voltdiff = self.initial_voltage;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &mut self.base;
        let f = (b.dn / 2.0 - 4.0) / b.dn;
        b.lead1 = interp_point(b.point1, b.point2, f);
        b.lead2 = interp_point(b.point1, b.point2, 1.0 - f);
        let (a0, a1) = interp_point2(b.point1, b.point2, f, 12.0);
        self.plate1 = [a0, a1];
        let (c0, c1) = interp_point2(b.point1, b.point2, 1.0 - f, 12.0);
        self.plate2 = [c0, c1];
        if let Some(p) = &mut self.polar {
            const COUNT: usize = 14;
            let max_i = (COUNT - 1) as f64;
            let mid_i = max_i / 2.0;
            p.plate_points = (0..COUNT)
                .map(|i| {
                    let q = (i as f64 - mid_i) * 0.9 / mid_i;
                    interp_point_g(
                        self.plate2[0],
                        self.plate2[1],
                        i as f64 / max_i,
                        5.0 * (1.0 - (1.0 - q * q).sqrt()),
                    )
                })
                .collect();
            let mut plus =
                interp_point_g(b.point1, b.point2, f - 8.0 / b.dn, f64::from(-10 * b.dsign));
            if b.y2 > b.y {
                plus.y += 4;
            }
            if b.y > b.y2 {
                plus.y += 3;
            }
            p.plus_point = plus;
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let hs = 12;
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
        let b = &self.base;
        // first lead and plate
        set_voltage_color(g, b, c, b.volts[0]);
        draw_thick_line(g, b.point1, b.lead1);
        set_power_color_w(g, b, c, self.get_power());
        draw_thick_line(g, self.plate1[0], self.plate1[1]);
        if c.show_power {
            g.set_color(Color::GRAY);
        }
        // second lead and plate
        set_voltage_color(g, b, c, b.volts[1]);
        draw_thick_line(g, b.point2, b.lead2);
        set_power_color_w(g, b, c, self.get_power());
        match &self.polar {
            Some(p) if !p.plate_points.is_empty() => {
                for w in p.plate_points.windows(2) {
                    draw_thick_line(g, w[0], w[1]);
                }
            }
            _ => draw_thick_line(g, self.plate2[0], self.plate2[1]),
        }
        self.base.update_dot_count(c);
        let b = &self.base;
        if !b.creating {
            draw_dots(g, c, b.point1, b.lead1, b.curcount);
            draw_dots(g, c, b.point2, b.lead2, -b.curcount);
        }
        draw_posts(g, &self.base, self, c);
        if c.show_values {
            let s = short_unit_text(self.capacitance, "F");
            draw_values(g, &self.base, c, Some(&s), f64::from(hs), false, false);
        }
        if let Some(p) = &self.polar {
            g.set_color(c.white);
            g.set_font_size(12);
            let w = g.measure_text("+") as i32;
            g.draw_string("+", f64::from(p.plus_point.x - w / 2), f64::from(p.plus_point.y));
        }
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = &self.base.nodes;
        if dc_analysis() {
            // when finding the DC operating point, replace the capacitor with a 100M resistor
            s.stamp_resistor(n[0], n[1], 1e8);
            self.cur_source_value = 0.0;
            self.cap_node2 = 1;
            return;
        }
        // the capacitor model is between nodes 0 and cap_node2; with a series resistance
        // cap_node2 is internal node 2 and a resistor joins nodes 2 and 1
        self.cap_node2 = if self.series_resistance > 0.0 { 2 } else { 1 };
        // trapezoidal companion model (a current source in parallel with a resistor)
        self.comp_resistance = if self.is_trapezoidal() {
            s.time_step / (2.0 * self.capacitance)
        } else {
            s.time_step / self.capacitance
        };
        s.stamp_resistor(n[0], n[self.cap_node2], self.comp_resistance);
        s.stamp_right_side_var(n[0]);
        s.stamp_right_side_var(n[self.cap_node2]);
        if self.series_resistance > 0.0 {
            s.stamp_resistor(n[1], n[2], self.series_resistance);
        }
    }

    fn start_iteration(&mut self, _s: &Stamper) {
        self.cur_source_value = if self.is_trapezoidal() {
            -self.voltdiff / self.comp_resistance - self.base.current
        } else {
            -self.voltdiff / self.comp_resistance
        };
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if let Some(p) = &self.polar {
            let vd = self.get_voltage_diff();
            if vd < 0.0 && vd < -p.max_negative_voltage {
                s.stop("capacitor exceeded max reverse voltage", None);
            }
        }
        self.voltdiff = self.base.volts[0] - self.base.volts[self.cap_node2];
        self.calculate_current();
    }

    // current is only calculated in step_finished(); otherwise it may be calculated while the
    // circuit is stamped, which would discharge the capacitor
    fn set_node_voltage(&mut self, n: usize, v: f64) {
        self.base.volts[n] = v;
    }

    fn calculate_current(&mut self) {
        let voltdiff = self.base.volts[0] - self.base.volts[self.cap_node2];
        if dc_analysis() {
            self.base.current = voltdiff / 1e8;
            return;
        }
        // comp_resistance may still be zero before stamp(), which would give infinite current
        if self.comp_resistance > 0.0 {
            self.base.current = voltdiff / self.comp_resistance + self.cur_source_value;
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        if dc_analysis() {
            return;
        }
        let n = &self.base.nodes;
        s.stamp_current_source(n[0], n[self.cap_node2], self.cur_source_value);
    }

    fn internal_node_count(&self) -> usize {
        usize::from(!dc_analysis() && self.series_resistance > 0.0)
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(if self.polar.is_some() { "capacitor (polarized)" } else { "capacitor" }.into());
        basic_info(self, arr);
        arr.push(format!("C = {}", unit_text(self.capacitance, "F")));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
        arr.push(format!("Q = {}", unit_text(self.capacitance * self.voltdiff, "C")));
    }

    fn scope_text(&self, _v: i32) -> Option<String> {
        Some(format!("{}, {}", locale::ls("capacitor"), unit_text(self.capacitance, "F")))
    }

    fn scope_value(&self, x: i32) -> f64 {
        if x == scope::VAL_CHARGE {
            return self.capacitance * self.voltdiff;
        }
        match x {
            scope::VAL_CURRENT => self.get_current(),
            scope::VAL_POWER => self.get_power(),
            _ => self.get_voltage_diff(),
        }
    }

    fn scope_values(&self) -> Vec<i32> {
        vec![scope::VAL_CHARGE]
    }

    fn scope_units(&self, x: i32) -> i32 {
        match x {
            scope::VAL_CHARGE => scope::UNITS_C,
            scope::VAL_CURRENT => scope::UNITS_A,
            scope::VAL_POWER => scope::UNITS_W,
            _ => scope::UNITS_V,
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Capacitance (F)", self.capacitance, 1e-6, 1e-3)),
            1 => Some(EditInfo::checkbox("Trapezoidal Approximation", self.is_trapezoidal())),
            2 => Some(EditInfo::number("Initial Voltage (on Reset)", self.initial_voltage)),
            3 => Some(EditInfo::number("Series Resistance", self.series_resistance)),
            4 => self
                .polar
                .as_ref()
                .map(|p| EditInfo::new("Max Reverse Voltage", p.max_negative_voltage, 0.0, 0.0)),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.capacitance = if ei.value > 0.0 { ei.value } else { 1e-12 },
            1 => {
                if ei.checkbox.unwrap_or(false) {
                    self.base.flags &= !FLAG_BACK_EULER;
                } else {
                    self.base.flags |= FLAG_BACK_EULER;
                }
            }
            2 => self.initial_voltage = ei.value,
            3 => {
                self.series_resistance = ei.value;
                self.alloc_nodes();
            }
            4 => {
                if let Some(p) = &mut self.polar {
                    if ei.value >= 0.0 {
                        p.max_negative_voltage = ei.value;
                    }
                }
            }
            _ => {}
        }
    }

    fn shortcut(&self) -> i32 {
        if self.polar.is_some() { i32::from(b'C') } else { i32::from(b'c') }
    }

    fn is_ideal_capacitor(&self) -> bool {
        self.series_resistance == 0.0
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        if self.is_ideal_capacitor() {
            let (n0, n1) = (self.base.nodes[0], self.base.nodes[1]);
            if v.find_path(PathKind::Short, idx, n1, n0) {
                self.shorted();
            } else if v.find_path(PathKind::CapVoltage, idx, n1, n0) {
                // loop of ideal capacitors: add a small series resistance to avoid oscillation
                self.set_series_resistance(0.1);
                return false;
            }
        }
        true
    }
}

/// Companion model of an inductor, shared by inductors, transformers and similar elements.
#[derive(Clone, Debug, Default)]
pub struct Inductor {
    nodes: [NodeId; 2],
    /// Flag bits (only [`FLAG_BACK_EULER`] matters).
    pub flags: i32,
    /// Inductance in henries.
    pub inductance: f64,
    comp_resistance: f64,
    /// Current through the inductor.
    pub current: f64,
    cur_source_value: f64,
    /// Saturation onset current (0 disables saturation).
    pub saturation_current: f64,
}

impl Inductor {
    /// Sets inductance, initial current and flags.
    pub fn setup(&mut self, ic: f64, cr: f64, f: i32) {
        self.inductance = ic;
        self.current = cr;
        self.flags = f;
    }

    /// Like [`Inductor::setup`] with a saturation current.
    pub fn setup_sat(&mut self, ic: f64, cr: f64, f: i32, isat: f64) {
        self.setup(ic, cr, f);
        self.saturation_current = isat;
    }

    /// True when trapezoidal integration is used.
    pub fn is_trapezoidal(&self) -> bool {
        self.flags & FLAG_BACK_EULER == 0
    }

    /// Resets the current to zero.
    pub fn reset(&mut self) {
        self.reset_to(0.0);
    }

    /// Resets the current to `c`.
    pub fn reset_to(&mut self, c: f64) {
        // set cur_source_value too in case one of the nodes is ground: current may be
        // calculated before start_iteration()
        self.cur_source_value = c;
        self.current = c;
    }

    /// Effective inductance with saturation: `L0 / (1 + (I/Isat)^2)`.
    pub fn effective_inductance(&self, i: f64) -> f64 {
        if self.saturation_current <= 0.0 {
            return self.inductance;
        }
        let ratio = i / self.saturation_current;
        self.inductance / (1.0 + ratio * ratio)
    }

    /// Stamps the companion model between `n0` and `n1`.
    pub fn stamp(&mut self, s: &mut Stamper, n0: NodeId, n1: NodeId) {
        self.nodes = [n0, n1];
        if self.saturation_current > 0.0 {
            // nonlinear: the conductance changes with current and is stamped in do_step()
            s.stamp_non_linear(n0);
            s.stamp_non_linear(n1);
        } else {
            self.comp_resistance = if self.is_trapezoidal() {
                2.0 * self.inductance / s.time_step
            } else {
                self.inductance / s.time_step
            };
            s.stamp_resistor(n0, n1, self.comp_resistance);
        }
        s.stamp_right_side_var(n0);
        s.stamp_right_side_var(n1);
    }

    /// True when the companion model changes with current.
    pub fn non_linear(&self) -> bool {
        self.saturation_current > 0.0
    }

    /// Updates the companion source at the start of a time step.
    pub fn start_iteration(&mut self, s: &Stamper, voltdiff: f64) {
        if self.saturation_current > 0.0 {
            let l_eff = self.effective_inductance(self.current);
            self.comp_resistance =
                if self.is_trapezoidal() { 2.0 * l_eff / s.time_step } else { l_eff / s.time_step };
        }
        self.cur_source_value = if self.is_trapezoidal() {
            voltdiff / self.comp_resistance + self.current
        } else {
            self.current
        };
    }

    /// Computes the current from the voltage across the inductor.
    pub fn calculate_current(&mut self, voltdiff: f64) -> f64 {
        // comp_resistance may still be zero before stamp(), which would give infinite current
        if self.comp_resistance > 0.0 {
            self.current = voltdiff / self.comp_resistance + self.cur_source_value;
        }
        self.current
    }

    /// Restamps the nonlinear part and the companion current source.
    pub fn do_step(&mut self, s: &mut Stamper) {
        if self.saturation_current > 0.0 {
            s.stamp_conductance(self.nodes[0], self.nodes[1], 1.0 / self.comp_resistance);
        }
        s.stamp_current_source(self.nodes[0], self.nodes[1], self.cur_source_value);
    }
}

/// An inductor.
#[derive(Clone, Debug)]
pub struct InductorElm {
    base: Base,
    ind: Inductor,
    /// Inductance in henries.
    pub inductance: f64,
    /// Current after a reset.
    pub initial_current: f64,
    /// Saturation onset current in amps (0 = linear).
    pub saturation_current: f64,
}

impl InductorElm {
    /// An inductor being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut e = InductorElm {
            base: Base::new(xx, yy),
            ind: Inductor::default(),
            inductance: 1.0,
            initial_current: 0.0,
            saturation_current: 0.0,
        };
        e.setup_ind();
        e
    }

    /// An inductor read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut e = InductorElm {
            base: Base::from_dump(xa, ya, xb, yb, f),
            ind: Inductor::default(),
            inductance: st.next_f64()?,
            initial_current: 0.0,
            saturation_current: 0.0,
        };
        e.base.current = st.next_f64()?;
        // like the original, keep what was read before the first missing field
        if let Ok(ic) = st.next_f64() {
            e.initial_current = ic;
            if let Ok(isat) = st.next_f64() {
                e.saturation_current = isat;
            }
        }
        e.setup_ind();
        Ok(e)
    }

    fn setup_ind(&mut self) {
        self.ind.setup_sat(
            self.inductance,
            self.base.current,
            self.base.flags,
            self.saturation_current,
        );
    }
}

impl Element for InductorElm {
    impl_element_base!("InductorElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'l')
    }

    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.inductance),
            js_number(self.base.current),
            js_number(self.initial_current),
            js_number(self.saturation_current)
        )
    }

    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.inductance = xml.double_attr("l", self.inductance);
        self.initial_current = xml.double_attr("ic", self.initial_current);
        self.base.current = xml.double_attr("i", self.base.current);
        self.saturation_current = xml.double_attr("isat", self.saturation_current);
        self.setup_ind();
    }

    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("l", self.inductance);
        out.double_attr("ic", self.initial_current);
        if self.saturation_current != 0.0 {
            out.double_attr("isat", self.saturation_current);
        }
    }

    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.double_attr("i", self.base.current);
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(32);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (v1, v2) = (self.base.volts[0], self.base.volts[1]);
        let hs = 8;
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
        draw_2_leads(g, &self.base, c);
        set_power_color_w(g, &self.base, c, self.get_power());
        draw_coil(g, &self.base, c, 8, self.base.lead1, self.base.lead2, v1, v2);
        if c.show_values {
            let s = short_unit_text(self.inductance, "H");
            draw_values(g, &self.base, c, Some(&s), f64::from(hs), false, false);
        }
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn reset(&mut self) {
        self.base.volts[0] = 0.0;
        self.base.volts[1] = 0.0;
        self.base.curcount = 0.0;
        self.base.current = self.initial_current;
        self.ind.reset_to(self.initial_current);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let (n0, n1) = (self.base.nodes[0], self.base.nodes[1]);
        self.ind.stamp(s, n0, n1);
    }

    fn start_iteration(&mut self, s: &Stamper) {
        let vd = self.base.volts[0] - self.base.volts[1];
        self.ind.start_iteration(s, vd);
    }

    fn non_linear(&self) -> bool {
        self.ind.non_linear()
    }

    fn calculate_current(&mut self) {
        let vd = self.base.volts[0] - self.base.volts[1];
        self.base.current = self.ind.calculate_current(vd);
    }

    fn do_step(&mut self, s: &mut Stamper) {
        self.ind.do_step(s);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(if self.saturation_current > 0.0 { "inductor (sat)" } else { "inductor" }.into());
        basic_info(self, arr);
        arr.push(format!("L = {}", unit_text(self.inductance, "H")));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
        if self.saturation_current > 0.0 {
            let l_eff = self.ind.effective_inductance(self.base.current);
            arr.push(format!("Leff = {}", unit_text(l_eff, "H")));
            arr.push(format!("Isat = {}", unit_text(self.saturation_current, "A")));
        }
    }

    fn scope_text(&self, _v: i32) -> Option<String> {
        Some(format!("{}, {}", locale::ls("inductor"), unit_text(self.inductance, "H")))
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Inductance (H)", self.inductance, 1e-2, 10.0).set_positive()),
            1 => Some(EditInfo::checkbox("Trapezoidal Approximation", self.ind.is_trapezoidal())),
            2 => Some(EditInfo::number("Initial Current (on Reset) (A)", self.initial_current)),
            3 => Some(EditInfo::number("Saturation Current (A) (0=none)", self.saturation_current)),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.inductance = ei.value,
            1 => {
                if ei.checkbox.unwrap_or(false) {
                    self.base.flags &= !FLAG_BACK_EULER;
                } else {
                    self.base.flags |= FLAG_BACK_EULER;
                }
            }
            2 => self.initial_current = ei.value,
            3 => {
                if ei.value >= 0.0 {
                    self.saturation_current = ei.value;
                }
            }
            _ => {}
        }
        self.setup_ind();
    }

    fn shortcut(&self) -> i32 {
        i32::from(b'L')
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        let (n0, n1) = (self.base.nodes[0], self.base.nodes[1]);
        if !v.find_path(PathKind::Inductor, idx, n1, n0) {
            Element::reset(self);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacitor_dump_always_includes_series_resistance_flag() {
        let mut st = Tokenizer::new("1e-5 0 0.001");
        let c = CapacitorElm::from_dump(0, 0, 64, 0, 0, &mut st).unwrap();
        assert_eq!(c.dump(), "c 0 0 64 0 4 0.00001 0 0.001 0");
    }

    #[test]
    fn capacitor_reads_series_resistance_when_flagged() {
        let mut st = Tokenizer::new("1e-5 0 0.001 25");
        let c = CapacitorElm::from_dump(0, 0, 64, 0, FLAG_RESISTANCE, &mut st).unwrap();
        assert_eq!(c.series_resistance, 25.0);
        assert_eq!(c.node_count(), 3);
    }

    #[test]
    fn polar_capacitor_round_trips_extra_field() {
        let mut st = Tokenizer::new("1e-5 0 0.001 0 2.5");
        let c = CapacitorElm::from_dump_polar(0, 0, 64, 0, FLAG_RESISTANCE, &mut st).unwrap();
        assert!(c.dump().starts_with("209 "));
        assert!(c.dump().ends_with(" 2.5"));
        assert_eq!(c.class_name(), "PolarCapacitorElm");
    }

    #[test]
    fn inductor_effective_inductance_rolls_off_with_current() {
        let mut ind = Inductor::default();
        ind.setup_sat(1.0, 0.0, 0, 2.0);
        assert!((ind.effective_inductance(0.0) - 1.0).abs() < 1e-12);
        assert!((ind.effective_inductance(2.0) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn inductor_dump_round_trips() {
        let mut st = Tokenizer::new("1 0 0 0");
        let l = InductorElm::from_dump(0, 0, 0, 64, 0, &mut st).unwrap();
        assert_eq!(l.dump(), "l 0 0 0 64 0 1 0 0 0");
    }
}
