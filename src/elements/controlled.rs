// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (VCVSElm, VCCSElm, CCVSElm,
// CCCSElm).
//! Controlled sources whose output is an expression of the inputs: voltage-controlled
//! voltage and current sources and current-controlled voltage and current sources.
//!
//! The expression is linearised around the previous iteration, so a source can implement any
//! function a circuit needs (multipliers, limiters, differentiators).
//!
//! Current-controlled sources come in two styles. The native one adds a 0 V source across each
//! input pair. The spice style (flag 2) senses the current of a voltage source element of the
//! same circuit that sits across the pair; inside a subcircuit the composite hands it over.

use super::chip::*;
use super::element::*;
use super::registry::{ElementDescriptor, Registry};
use crate::expr::{Expr, ExprState};
use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::solver::path::{PathKind, Validator};
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name used for the feature switch.
pub const FAMILY: &str = "analog";
const CAT: &str = "Active Building Blocks";

/// Registers the controlled sources.
pub fn register(r: &mut Registry) {
    macro_rules! source {
        ($id:expr, $menu:expr, $code:expr, $kind:expr) => {
            r.add(
                ElementDescriptor::new($id, FAMILY, $menu, CAT, |x, y| {
                    Box::new(ChipElm::new(Controlled::new($kind), x, y))
                })
                .dump($code, |a, b, c, d, f, st| {
                    Ok(Box::new(ChipElm::from_dump(Controlled::new($kind), a, b, c, d, f, st)))
                }),
            );
        };
    }
    source!("VCVSElm", "Add VCVS", 212, Kind::Vcvs);
    source!("VCCSElm", "Add VCCS", 213, Kind::Vccs);
    source!("CCVSElm", "Add CCVS", 214, Kind::Ccvs);
    source!("CCCSElm", "Add CCCS", 215, Kind::Cccs);
}

/// Which controlled source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Voltage out, voltage in.
    Vcvs,
    /// Current out, voltage in.
    Vccs,
    /// Voltage out, current in.
    Ccvs,
    /// Current out, current in.
    Cccs,
}

/// A controlled source.
#[derive(Clone, Debug)]
pub struct Controlled {
    kind: Kind,
    input_count: usize,
    expr_string: String,
    expr: Option<Expr>,
    state: ExprState,
    last_volts: Vec<f64>,
    last_currents: Vec<f64>,
    last_output: f64,
    broken: bool,
}

fn sign(a: f64, b: f64) -> f64 {
    if a > 0.0 { b } else { -b }
}

fn converge_limit(s: &Stamper) -> f64 {
    if s.sub_iterations < 10 {
        0.001
    } else if s.sub_iterations < 200 {
        0.01
    } else {
        0.1
    }
}

impl Controlled {
    fn spice(c: &ChipCore) -> bool {
        c.base.flags & 2 != 0
    }

    fn new(kind: Kind) -> Self {
        let expr_string = match kind {
            Kind::Vcvs | Kind::Vccs => ".1*(a-b)",
            Kind::Ccvs | Kind::Cccs => "2*a",
        };
        let mut s = Controlled {
            kind,
            input_count: 2,
            expr_string: expr_string.into(),
            expr: None,
            state: ExprState::new(),
            last_volts: Vec::new(),
            last_currents: Vec::new(),
            last_output: 0.0,
            broken: false,
        };
        s.parse_expr();
        s
    }

    fn parse_expr(&mut self) -> Option<String> {
        match Expr::parse(&self.expr_string) {
            Ok(e) => {
                self.expr = Some(e);
                None
            }
            Err(err) => {
                self.expr = Expr::parse("").ok();
                Some(format!("Parse error in expression: {}: {err}", self.expr_string))
            }
        }
    }

    fn current_input(&self) -> bool {
        matches!(self.kind, Kind::Ccvs | Kind::Cccs)
    }

    fn pairs(&self) -> usize {
        self.input_count / 2
    }

    /// Sets variable `n` of the expression to a sensed current (variable `i` also follows the
    /// first current, as in the original).
    fn set_current_value(&mut self, n: usize, cur: f64) {
        if n == 0 && self.pairs() < 9 {
            self.state.values[8] = cur;
        }
        self.state.values[n] = cur;
    }

    fn eval(&self) -> f64 {
        self.expr.as_ref().map_or(0.0, |e| e.eval(&self.state))
    }
}

impl ChipLogic for Controlled {
    fn class_name(&self) -> &'static str {
        match self.kind {
            Kind::Vcvs => "VCVSElm",
            Kind::Vccs => "VCCSElm",
            Kind::Ccvs => "CCVSElm",
            Kind::Cccs => "CCCSElm",
        }
    }
    fn dump_type(&self) -> i32 {
        match self.kind {
            Kind::Vcvs => 212,
            Kind::Vccs => 213,
            Kind::Ccvs => 214,
            Kind::Cccs => 215,
        }
    }
    fn chip_name(&self) -> &'static str {
        match self.kind {
            Kind::Vcvs => "VCVS",
            Kind::Vccs => "VCCS",
            Kind::Ccvs => "CCVS",
            Kind::Cccs => "CCCS",
        }
    }
    fn is_digital(&self) -> bool {
        false
    }
    fn non_linear(&self) -> bool {
        true
    }

    fn setup_pins(&mut self, c: &mut ChipCore) {
        let ic = self.input_count;
        c.size_x = 2;
        c.size_y = ic.max(2) as i32;
        let mut pins = Vec::new();
        if self.current_input() {
            for i in 0..self.pairs() {
                let name = char::from(b'A' + i as u8);
                pins.push(Pin::new((i * 2) as i32, SIDE_W, &format!("{name}+")));
                pins.push(Pin::output((i * 2 + 1) as i32, SIDE_W, &format!("{name}-")));
            }
            let (plus, minus) = if self.kind == Kind::Ccvs { ("V+", "V-") } else { ("O+", "O-") };
            pins.push(Pin::output(0, SIDE_E, plus));
            pins.push(Pin::new(1, SIDE_E, minus));
            self.state = ExprState::new();
            self.last_currents = vec![0.0; self.pairs() + 1];
        } else {
            for i in 0..ic {
                pins.push(Pin::new(i as i32, SIDE_W, &char::from(b'A' + i as u8).to_string()));
            }
            if self.kind == Kind::Vcvs {
                pins.push(Pin::output(0, SIDE_E, "V+"));
                pins.push(Pin::new(1, SIDE_E, "V-"));
            } else {
                pins.push(Pin::new(0, SIDE_E, "C+"));
                pins.push(Pin::new(1, SIDE_E, "C-"));
            }
            self.last_volts = vec![0.0; ic];
            self.state = ExprState::new();
        }
        c.pins = pins;
    }

    fn voltage_source_count(&self, c: &ChipCore) -> usize {
        let spice = Self::spice(c);
        match self.kind {
            Kind::Vcvs => 1,
            Kind::Vccs => 0,
            Kind::Ccvs => 1 + if spice { 0 } else { self.pairs() },
            Kind::Cccs => {
                if spice {
                    0
                } else {
                    self.pairs()
                }
            }
        }
    }
    fn output_pin_for_source(&self, c: &ChipCore, j: usize) -> Option<usize> {
        (self.kind == Kind::Ccvs && Self::spice(c) && j == 0).then_some(self.input_count)
    }
    fn spice_pairs(&self, c: &ChipCore) -> usize {
        if self.current_input() && Self::spice(c) { self.pairs() } else { 0 }
    }
    fn set_spice_source(&mut self, c: &mut ChipCore, i: usize, vs: VsId) {
        if let Some(p) = c.pins.get_mut(i * 2 + 1) {
            p.volt_source = Some(vs);
        }
    }
    fn set_spice_current(&mut self, c: &mut ChipCore, i: usize, current: f64) {
        if let Some(p) = c.pins.get_mut(i * 2 + 1) {
            p.current = current;
        }
    }

    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Ok(n) = st.next_i32() {
            self.input_count = n.clamp(0, 8) as usize;
        }
        if let Some(t) = st.next_token() {
            self.expr_string = unescape(t);
        }
        self.parse_expr();
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len());
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {} {}", self.input_count, escape(&self.expr_string))
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.input_count = xml.int_attr("ic", self.input_count as i32).clamp(0, 8) as usize;
        self.expr_string = xml.string_attr("ex", &self.expr_string);
        self.parse_expr();
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("ic", self.input_count as i32);
        out.str_attr("ex", &self.expr_string);
    }
    fn on_reset(&mut self, _c: &mut ChipCore) {
        self.state.reset();
        self.last_volts.fill(0.0);
        self.last_currents.fill(0.0);
        self.last_output = 0.0;
    }

    fn connection(&self, n1: usize, n2: usize) -> bool {
        if self.current_input() {
            n1 / 2 == n2 / 2
        } else {
            let ic = self.input_count;
            (n1 == ic && n2 == ic + 1) || (n1 == ic + 1 && n2 == ic)
        }
    }
    fn matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn ground_connection(&self, _c: &ChipCore, _n: usize) -> bool {
        false
    }

    fn validate(&mut self, c: &mut ChipCore, idx: usize, v: &mut Validator<'_>) -> Option<bool> {
        if matches!(self.kind, Kind::Vccs | Kind::Cccs) {
            let ic = self.input_count;
            let n = &c.base.nodes;
            self.broken = !v.find_path(PathKind::Inductor, idx, n[ic + 1], n[ic]);
        } else {
            self.broken = false;
        }
        Some(true)
    }

    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let ic = self.input_count;
        let n = c.base.nodes.clone();
        match self.kind {
            Kind::Vccs => {}
            Kind::Vcvs => {
                if let Some(vs) = c.pins[ic].volt_source {
                    s.stamp_voltage_source_var(n[ic + 1], n[ic], vs);
                }
            }
            Kind::Ccvs | Kind::Cccs => {
                for i in 0..self.pairs() {
                    if Self::spice(c) {
                        break;
                    }
                    if let Some(vs) = c.pins[i * 2 + 1].volt_source {
                        s.stamp_voltage_source(n[i * 2], n[i * 2 + 1], vs, 0.0);
                    }
                }
                if self.kind == Kind::Ccvs {
                    if let Some(vs) = c.pins[ic].volt_source {
                        s.stamp_voltage_source_var(n[ic + 1], n[ic], vs);
                    }
                }
            }
        }
        true
    }

    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        match self.kind {
            Kind::Vccs => self.step_vccs(c, s),
            Kind::Vcvs => self.step_vcvs(c, s),
            Kind::Ccvs => self.step_ccvs(c, s),
            Kind::Cccs => self.step_cccs(c, s),
        }
        true
    }

    fn step_finished(&mut self, c: &mut ChipCore, _s: &mut Stamper) {
        let ic = self.input_count;
        match self.kind {
            Kind::Vccs | Kind::Cccs => self.state.update_last_values(c.pins[ic].current),
            Kind::Vcvs => self.state.update_last_values(c.base.volts[ic] - c.base.volts[ic + 1]),
            Kind::Ccvs => {
                self.state.update_last_values(c.base.volts[ic] - c.base.volts[ic + 1]);
                for i in 0..self.pairs() {
                    self.state.last_values[i] = c.pins[i * 2 + 1].current;
                }
            }
        }
    }

    fn calculate_current(&mut self, c: &mut ChipCore) {
        let ic = self.input_count;
        match self.kind {
            Kind::Vcvs => c.pins[ic + 1].current = -c.pins[ic].current,
            Kind::Ccvs => {
                for i in 0..self.pairs() {
                    c.pins[i * 2].current = -c.pins[i * 2 + 1].current;
                }
                c.pins[ic + 1].current = -c.pins[ic].current;
            }
            Kind::Cccs => {
                for i in 0..self.pairs() {
                    c.pins[i * 2].current = -c.pins[i * 2 + 1].current;
                }
            }
            Kind::Vccs => {}
        }
    }

    fn info_extra(&self, c: &ChipCore, arr: &mut Vec<String>) {
        use crate::elements::units::{current_text, voltage_text};
        let ic = self.input_count;
        if self.current_input() {
            for i in 0..self.pairs() {
                arr.push(format!(
                    "{} = {}",
                    c.pins[i * 2].text,
                    current_text(-c.pins[i * 2].current)
                ));
            }
            arr.push(format!(
                "{} = {}; {} = {}",
                c.pins[ic].text,
                voltage_text(c.base.volts[ic]),
                c.pins[ic + 1].text,
                voltage_text(c.base.volts[ic + 1])
            ));
        }
        arr.push(format!("I = {}", current_text(c.pins[ic].current)));
    }

    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::text("Output Function", &self.expr_string)),
            1 => Some(
                EditInfo::new("# of Inputs", self.input_count as f64, 1.0, 8.0).set_dimensionless(),
            ),
            _ => None,
        }
    }

    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                if let Some(t) = &ei.text {
                    self.expr_string.clone_from(t);
                }
                if let Some(err) = self.parse_expr() {
                    ei.error = Some(err);
                }
            }
            1 => {
                let v = ei.value;
                let odd = self.current_input() && (v as i64) % 2 == 1;
                if (0.0..=8.0).contains(&v) && !odd {
                    self.input_count = v as usize;
                }
            }
            _ => {}
        }
    }
}

impl Controlled {
    fn step_vccs(&mut self, c: &mut ChipCore, s: &mut Stamper) {
        let ic = self.input_count;
        let n = c.base.nodes.clone();
        if self.broken {
            c.pins[ic].current = 0.0;
            c.pins[ic + 1].current = 0.0;
            s.stamp_resistor(n[ic], n[ic + 1], 1e8);
            return;
        }
        let limit = converge_limit(s);
        let volts = c.base.volts.clone();
        for i in 0..ic {
            if (volts[i] - self.last_volts[i]).abs() > limit {
                s.converged = false;
            }
        }
        self.state.t = s.t;
        self.state.time_step = s.time_step;
        for (i, v) in volts.iter().take(ic).enumerate() {
            self.state.values[i] = *v;
        }
        let v0 = -self.eval();
        let mut rs = v0;
        for i in 0..ic {
            let mut dv = volts[i] - self.last_volts[i];
            if dv.abs() < 1e-6 {
                dv = 1e-6;
            }
            self.state.values[i] = volts[i];
            let v = -self.eval();
            self.state.values[i] = volts[i] - dv;
            let v2 = -self.eval();
            let mut dx = (v - v2) / dv;
            if dx.abs() < 1e-6 {
                dx = sign(dx, 1e-6);
            }
            s.stamp_vccs(n[ic], n[ic + 1], n[i], NodeId::GROUND, dx);
            rs -= dx * volts[i];
            self.state.values[i] = volts[i];
        }
        s.stamp_current_source(n[ic], n[ic + 1], rs);
        c.pins[ic].current = -v0;
        c.pins[ic + 1].current = v0;
        self.last_volts[..ic].copy_from_slice(&volts[..ic]);
    }

    fn step_vcvs(&mut self, c: &mut ChipCore, s: &mut Stamper) {
        let ic = self.input_count;
        let n = c.base.nodes.clone();
        let limit = converge_limit(s);
        let volts = c.base.volts.clone();
        for i in 0..ic {
            if (volts[i] - self.last_volts[i]).abs() > limit {
                s.converged = false;
            }
        }
        let Some(vn) = c.pins[ic].volt_source else { return };
        self.state.t = s.t;
        self.state.time_step = s.time_step;
        for (i, v) in volts.iter().take(ic).enumerate() {
            self.state.values[i] = *v;
        }
        let v0 = self.eval();
        if (volts[ic] - volts[ic + 1] - v0).abs() > v0.abs() * 0.01 && s.sub_iterations < 100 {
            s.converged = false;
        }
        let mut rs = v0;
        for i in 0..ic {
            let mut dv = volts[i] - self.last_volts[i];
            if dv.abs() < 1e-6 {
                dv = 1e-6;
            }
            self.state.values[i] = volts[i];
            let v = self.eval();
            self.state.values[i] = volts[i] - dv;
            let v2 = self.eval();
            let mut dx = (v - v2) / dv;
            if dx.abs() < 1e-6 {
                dx = sign(dx, 1e-6);
            }
            s.stamp_matrix(vn, n[i], -dx);
            rs -= dx * volts[i];
            self.state.values[i] = volts[i];
        }
        s.stamp_right_side(vn, rs);
        self.last_volts[..ic].copy_from_slice(&volts[..ic]);
    }

    fn step_ccvs(&mut self, c: &mut ChipCore, s: &mut Stamper) {
        let ic = self.input_count;
        let pairs = self.pairs();
        let limit_i = converge_limit(s) * 0.1;
        for i in 0..pairs {
            if (c.pins[i * 2 + 1].current - self.last_currents[i]).abs() > limit_i {
                s.converged = false;
            }
        }
        let out_v = c.base.volts[ic] - c.base.volts[ic + 1];
        if (out_v - self.last_output).abs() > converge_limit(s) {
            s.converged = false;
        }
        if let Some(vno) = c.pins[ic].volt_source {
            self.state.t = s.t;
            self.state.time_step = s.time_step;
            for i in 0..pairs {
                self.set_current_value(i, c.pins[i * 2 + 1].current);
            }
            let v0 = self.eval();
            let mut rs = v0;
            for i in 0..pairs {
                let cur = c.pins[i * 2 + 1].current;
                let dv = 1e-9;
                self.set_current_value(i, cur);
                let v = self.eval();
                self.set_current_value(i, cur - dv);
                let v2 = self.eval();
                let mut dx = (v - v2) / dv;
                if dx.abs() < 1e-6 {
                    dx = sign(dx, 1e-6);
                }
                if let Some(vni) = c.pins[i * 2 + 1].volt_source {
                    s.stamp_matrix(vno, vni, -dx);
                }
                rs -= dx * cur;
                self.set_current_value(i, cur);
            }
            s.stamp_right_side(vno, rs);
        }
        for i in 0..pairs {
            self.last_currents[i] = c.pins[i * 2 + 1].current;
        }
        self.last_output = out_v;
    }

    fn step_cccs(&mut self, c: &mut ChipCore, s: &mut Stamper) {
        let ic = self.input_count;
        let pairs = self.pairs();
        let n = c.base.nodes.clone();
        if self.broken {
            c.pins[ic].current = 0.0;
            c.pins[ic + 1].current = 0.0;
            s.stamp_resistor(n[ic], n[ic + 1], 1e8);
            return;
        }
        let limit = converge_limit(s) * 0.1;
        for i in 0..=pairs {
            if (c.pins[i * 2 + 1].current - self.last_currents[i]).abs() > limit {
                s.converged = false;
            }
        }
        for i in 0..=pairs {
            self.last_currents[i] = c.pins[i * 2 + 1].current;
        }
        self.state.t = s.t;
        self.state.time_step = s.time_step;
        for i in 0..pairs {
            self.set_current_value(i, c.pins[i * 2 + 1].current);
        }
        let v0 = self.eval();
        let mut rs = v0;
        c.pins[ic].current = v0;
        c.pins[ic + 1].current = -v0;
        for i in 0..pairs {
            let cur = c.pins[i * 2 + 1].current;
            // the original measures the change after having stored the current, so the step is
            // always the smallest allowed one
            let dv = 1e-6;
            self.set_current_value(i, cur);
            let v = self.eval();
            self.set_current_value(i, cur - dv);
            let v2 = self.eval();
            let mut dx = (v - v2) / dv;
            if dx.abs() < 1e-6 {
                dx = sign(dx, 1e-6);
            }
            if let Some(vs) = c.pins[i * 2 + 1].volt_source {
                s.stamp_cccs(n[ic + 1], n[ic], vs, dx);
            }
            rs -= dx * cur;
            self.set_current_value(i, cur);
        }
        s.stamp_current_source(n[ic + 1], n[ic], rs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_round_trips_with_escaped_expression() {
        let chip = ChipElm::new(Controlled::new(Kind::Vccs), 0, 0);
        let text = chip.dump();
        assert!(text.ends_with(" 2 .1*(a-b)"), "{text}");
        let mut st = Tokenizer::new("2 a/2\\pb/2");
        let back = ChipElm::from_dump(Controlled::new(Kind::Vcvs), 0, 0, 32, 0, 0, &mut st);
        assert_eq!(back.logic.expr_string, "a/2+b/2");
        assert_eq!(back.core.pins.len(), 4);
    }

    #[test]
    fn current_controlled_pins_come_in_pairs() {
        let chip = ChipElm::new(Controlled::new(Kind::Ccvs), 0, 0);
        assert_eq!(chip.core.pins.len(), 4);
        assert!(chip.core.pins[1].output && chip.core.pins[2].output);
        assert_eq!(chip.logic.voltage_source_count(&chip.core), 2);
    }
}
