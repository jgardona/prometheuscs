// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (VCVSElm, VCCSElm, CCVSElm,
// CCCSElm, ControlledSourceElm, ControlledVoltageElm, ControlledCurrentElm).
//! Controlled sources whose output is an expression of the inputs: voltage-controlled
//! voltage and current sources and current-controlled voltage and current sources, plus the
//! two-terminal controlled voltage and current sources (the diamond symbol), whose expression
//! refers to the rest of the circuit by name (`v(label)`, `i(meter)`, see [`crate::expr`]).
//!
//! The expression is linearised around the previous iteration, so a source can implement any
//! function a circuit needs (multipliers, limiters, differentiators).
//!
//! Current-controlled sources come in two styles. The native one adds a 0 V source across each
//! input pair. The spice style (flag 2) senses the current of a voltage source element of the
//! same circuit that sits across the pair; inside a subcircuit the composite hands it over.

use super::chip::*;
use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::{current_text, voltage_text};
use crate::expr::refs::{ExprEngine, OwnerNodes, RefResolution, StampInput};
use crate::expr::{ExprRef, ExprState};
use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::Painter;
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
    r.add(ElementDescriptor::new(
        "ControlledVoltageElm",
        FAMILY,
        "Add Controlled Voltage Source",
        CAT,
        |x, y| Box::new(ControlledSource::new(Output::Voltage, x, y)),
    ));
    r.add(ElementDescriptor::new(
        "ControlledCurrentElm",
        FAMILY,
        "Add Controlled Current Source",
        CAT,
        |x, y| Box::new(ControlledSource::new(Output::Current, x, y)),
    ));
}

/// What a two-terminal controlled source puts out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Output {
    /// A voltage (post 2 is +), tag `cv`.
    Voltage,
    /// A current from post 1 to post 2, tag `ci`.
    Current,
}

/// Half the long diagonal of the diamond.
const DIAMOND_SIZE: i32 = 18;

/// A two-terminal controlled source: a diamond whose output is an expression that refers to
/// the rest of the circuit by name. It has no input pins; every input is a reference.
pub struct ControlledSource {
    base: Base,
    output: Output,
    engine: ExprEngine,
    broken: bool,
    center: Point,
    side1: Point,
    side2: Point,
    ashaft: [Point; 2],
    arrow: Polygon,
}

impl ControlledSource {
    /// A source being placed, with the default expression of the original.
    pub fn new(output: Output, x: i32, y: i32) -> Self {
        let text = match output {
            Output::Voltage => "v(in)",
            Output::Current => ".001*v(in)",
        };
        ControlledSource {
            base: Base::new(x, y),
            output,
            engine: ExprEngine::new(text),
            broken: false,
            center: Point::default(),
            side1: Point::default(),
            side2: Point::default(),
            ashaft: [Point::default(); 2],
            arrow: Polygon::default(),
        }
    }

    /// The expression as typed.
    pub fn expr_string(&self) -> &str {
        &self.engine.expr_string
    }

    fn label(&self) -> &'static str {
        match self.output {
            Output::Voltage => "Output Voltage",
            Output::Current => "Output Current",
        }
    }

    /// Splits the borrow: the engine, and the node data it reads.
    fn engine_and_nodes(&mut self) -> (&mut ExprEngine, OwnerNodes<'_>) {
        let ref_base = 2;
        let o = OwnerNodes { volts: &self.base.volts, nodes: &self.base.nodes, ref_base };
        (&mut self.engine, o)
    }
}

impl Element for ControlledSource {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        match self.output {
            Output::Voltage => "ControlledVoltageElm",
            Output::Current => "ControlledCurrentElm",
        }
    }
    fn xml_dump_type(&self) -> String {
        match self.output {
            Output::Voltage => "cv".into(),
            Output::Current => "ci".into(),
        }
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.str_attr("ex", &self.engine.expr_string);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        let text = xml.string_attr("ex", &self.engine.expr_string);
        let _ = self.engine.set_text(&text);
    }

    fn expr_refs(&self) -> &[ExprRef] {
        self.engine.refs()
    }
    fn set_ref_resolution(&mut self, k: usize, r: RefResolution) {
        self.engine.set_resolution(k, r);
    }
    fn set_ref_current(&mut self, k: usize, current: f64) {
        self.engine.set_ref_current(k, current);
    }

    fn voltage_source_count(&self) -> usize {
        usize::from(self.output == Output::Voltage)
    }
    fn voltage_source_nodes(&self) -> Option<(NodeId, NodeId)> {
        (self.output == Output::Voltage).then(|| (self.base.nodes[0], self.base.nodes[1]))
    }
    fn non_linear(&self) -> bool {
        true
    }
    // a reference node is only read, but it has to share the matrix with the posts
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 < 2 && n2 < 2
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        match n {
            0 => -self.base.current,
            1 => self.base.current,
            _ => 0.0,
        }
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        // a current source with nowhere for its current to go is a matrix error
        self.broken = self.output == Output::Current
            && !v.find_path(PathKind::Inductor, idx, self.base.nodes[1], self.base.nodes[0]);
        true
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = &self.base.nodes;
        match self.output {
            Output::Voltage => {
                if let Some(vs) = self.base.volt_source {
                    s.set_vs_nodes(vs, n[0], n[1]);
                    s.stamp_voltage_source_var(n[0], n[1], vs);
                }
            }
            Output::Current => {
                if self.broken {
                    s.stamp_resistor(n[0], n[1], 1e8);
                }
            }
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        if !self.engine.is_valid() || (self.output == Output::Current && self.broken) {
            if self.output == Output::Current {
                self.base.current = 0.0;
            }
            return;
        }
        let limit = crate::expr::refs::converge_limit(s);
        let (t, dt) = (s.t, s.time_step);
        let output = self.output;
        let (n0, n1) = (self.base.nodes[0], self.base.nodes[1]);
        let vs = self.base.volt_source;
        let out_v = self.base.volts[1] - self.base.volts[0];
        let (engine, o) = self.engine_and_nodes();
        if !engine.check_convergence(&o, limit) {
            s.converged = false;
        }
        let rs = engine.eval_and_stamp(false, &o, t, dt, |input, dx| match (output, input) {
            (Output::Voltage, StampInput::Voltage(pos, neg)) => {
                if let Some(vs) = vs {
                    s.stamp_matrix(vs, pos, -dx);
                    s.stamp_matrix(vs, neg, dx);
                }
            }
            (Output::Voltage, StampInput::Current(ivs)) => {
                if let Some(vs) = vs {
                    s.stamp_matrix(vs, ivs, -dx);
                }
            }
            (Output::Current, StampInput::Voltage(pos, neg)) => {
                s.stamp_vccs(n0, n1, pos, neg, dx);
            }
            (Output::Current, StampInput::Current(ivs)) => s.stamp_cccs(n0, n1, ivs, dx),
        });
        let v0 = engine.value;
        engine.save_inputs(&o);
        match output {
            Output::Voltage => {
                // the output has to settle too, not just the inputs
                if (out_v - v0).abs() > v0.abs() * 0.01 && s.sub_iterations < 100 {
                    s.converged = false;
                }
                if let Some(vs) = vs {
                    s.stamp_right_side(vs, rs);
                }
            }
            Output::Current => {
                s.stamp_current_source(n0, n1, rs);
                self.base.current = v0;
            }
        }
    }

    fn step_finished(&mut self, _s: &mut Stamper) {
        // the output as solved, which lastoutput reports
        let out = match self.output {
            Output::Voltage => self.get_voltage_diff(),
            Output::Current => self.base.current,
        };
        self.engine.step_finished(out);
    }
    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
        self.base.current = 0.0;
        self.engine.reset();
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(DIAMOND_SIZE * 2);
        let b = &self.base;
        let d = f64::from(DIAMOND_SIZE);
        self.center = interp_point(b.point1, b.point2, 0.5);
        self.side1 = interp_point_g(b.point1, b.point2, 0.5, d);
        self.side2 = interp_point_g(b.point1, b.point2, 0.5, -d);
        // arrow along the long axis, pointing from post 1 to post 2
        self.ashaft = [interp_point(b.lead1, b.lead2, 0.3), interp_point(b.lead1, b.lead2, 0.6)];
        let tip = interp_point(b.lead1, b.lead2, 0.75);
        self.arrow = calc_arrow(self.center, tip, 7.0, 6.0);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        draw_2_leads(g, &self.base, c);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        set_power_color_w(g, &self.base, c, self.get_power());
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        draw_thick_line(g, l1, self.side1);
        draw_thick_line(g, self.side1, l2);
        draw_thick_line(g, l2, self.side2);
        draw_thick_line(g, self.side2, l1);
        match self.output {
            Output::Voltage => {
                // post 2 is the + terminal; the signs sit along the diamond, which keeps its
                // size when the element is stretched
                g.set_font_size(12);
                let pm = interp_point(l1, l2, 0.3);
                let pp = interp_point(l1, l2, 0.7);
                draw_centered_text(g, &mut self.base, "\u{2212}", pm.x, pm.y, true);
                draw_centered_text(g, &mut self.base, "+", pp.x, pp.y, true);
            }
            Output::Current => {
                draw_thick_line(g, self.ashaft[0], self.ashaft[1]);
                fill_polygon(g, &self.arrow);
            }
        }
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(DIAMOND_SIZE));
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[1] - self.base.volts[0]
    }
    fn get_power(&self) -> f64 {
        -self.get_voltage_diff() * self.base.current
    }
    fn set_current(&mut self, _vs: VsId, c: f64) {
        self.base.current = c;
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(
            match self.output {
                Output::Voltage => "Controlled Voltage Source",
                Output::Current => "Controlled Current Source",
            }
            .into(),
        );
        arr.push(format!("{} = {}", self.label(), self.engine.expr_string));
        arr.push(format!("V = {}", voltage_text(self.get_voltage_diff())));
        arr.push(format!("I = {}", current_text(self.base.current)));
        arr.extend(self.engine.unresolved_info());
        if self.broken {
            arr.push("(no current path)".into());
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| EditInfo::text(self.label(), &self.engine.expr_string).disallow_sliders())
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if n != 0 {
            return;
        }
        let Some(text) = ei.text.clone() else { return };
        let old = self.engine.expr_string.clone();
        if let Err(err) = self.engine.set_text(&text) {
            // keep the expression that worked
            let _ = self.engine.set_text(&old);
            ei.error = Some(err);
        }
    }
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
    /// The expression, its `v()`/`i()` references and the input history.
    engine: ExprEngine,
    last_currents: Vec<f64>,
    last_output: f64,
    broken: bool,
}

fn sign(a: f64, b: f64) -> f64 {
    if a > 0.0 { b } else { -b }
}

use crate::expr::refs::converge_limit;

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
            engine: ExprEngine::new(expr_string),
            last_currents: Vec::new(),
            last_output: 0.0,
            broken: false,
        };
        s.update_pin_inputs();
        s
    }

    /// The expression as typed.
    pub fn expr_string(&self) -> &str {
        &self.engine.expr_string
    }

    /// Parses the expression again; the message is the edit dialog's error.
    fn parse_expr(&mut self) -> Option<String> {
        let err = self.engine.parse().err();
        self.update_pin_inputs();
        err
    }

    /// The voltage-controlled sources read their input pins as `a`, `b`, ...; the
    /// current-controlled ones load their sensed currents themselves.
    fn update_pin_inputs(&mut self) {
        let pins = if self.current_input() { 0 } else { self.input_count };
        self.engine.set_pin_inputs(pins);
    }

    fn owner<'a>(&self, c: &'a ChipCore) -> OwnerNodes<'a> {
        OwnerNodes { volts: &c.base.volts, nodes: &c.base.nodes, ref_base: c.pins.len() }
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
            self.engine.state.values[8] = cur;
        }
        self.engine.state.values[n] = cur;
    }

    fn eval(&self) -> f64 {
        self.engine.eval_state()
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
            self.engine.state.values = ExprState::new().values;
            self.engine.reset();
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
            self.engine.state.values = ExprState::new().values;
            self.engine.reset();
        }
        c.pins = pins;
        self.update_pin_inputs();
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
            self.engine.expr_string = unescape(t);
        }
        self.parse_expr();
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len());
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {} {}", self.input_count, escape(&self.engine.expr_string))
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.input_count = xml.int_attr("ic", self.input_count as i32).clamp(0, 8) as usize;
        self.engine.expr_string = xml.string_attr("ex", &self.engine.expr_string);
        self.parse_expr();
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("ic", self.input_count as i32);
        out.str_attr("ex", &self.engine.expr_string);
    }
    fn on_reset(&mut self, _c: &mut ChipCore) {
        self.engine.reset();
        self.last_currents.fill(0.0);
        self.last_output = 0.0;
    }
    fn expr_refs(&self) -> &[ExprRef] {
        self.engine.refs()
    }
    fn set_ref_resolution(&mut self, k: usize, r: RefResolution) {
        self.engine.set_resolution(k, r);
    }
    fn set_ref_current(&mut self, k: usize, current: f64) {
        self.engine.set_ref_current(k, current);
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
            Kind::Vccs | Kind::Cccs => self.engine.step_finished(c.pins[ic].current),
            Kind::Vcvs => self.engine.step_finished(c.base.volts[ic] - c.base.volts[ic + 1]),
            Kind::Ccvs => {
                self.engine.step_finished(c.base.volts[ic] - c.base.volts[ic + 1]);
                for i in 0..self.pairs() {
                    self.engine.state.last_values[i] = c.pins[i * 2 + 1].current;
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
        arr.extend(self.engine.unresolved_info());
    }

    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::text("Output Function", &self.engine.expr_string)),
            1 => Some(
                EditInfo::new("# of Inputs", self.input_count as f64, 1.0, 8.0).set_dimensionless(),
            ),
            _ => None,
        }
    }

    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                let old = self.engine.expr_string.clone();
                if let Some(t) = &ei.text {
                    self.engine.expr_string.clone_from(t);
                }
                if let Some(err) = self.parse_expr() {
                    // keep the expression that worked
                    self.engine.expr_string = old;
                    self.parse_expr();
                    ei.error = Some(err);
                }
            }
            1 => {
                let v = ei.value;
                let odd = self.current_input() && (v as i64) % 2 == 1;
                if (0.0..=8.0).contains(&v) && !odd {
                    self.input_count = v as usize;
                    self.update_pin_inputs();
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
        let (t, dt) = (s.t, s.time_step);
        let o = OwnerNodes { volts: &c.base.volts, nodes: &c.base.nodes, ref_base: c.pins.len() };
        if !self.engine.check_convergence(&o, limit) {
            s.converged = false;
        }
        // the output is a current from the + pin to the - pin of minus the expression
        let rs = self.engine.eval_and_stamp(true, &o, t, dt, |input, dx| match input {
            StampInput::Voltage(pos, neg) => s.stamp_vccs(n[ic], n[ic + 1], pos, neg, dx),
            StampInput::Current(vs) => s.stamp_cccs(n[ic], n[ic + 1], vs, dx),
        });
        self.engine.save_inputs(&o);
        s.stamp_current_source(n[ic], n[ic + 1], rs);
        let v0 = self.engine.value;
        c.pins[ic].current = -v0;
        c.pins[ic + 1].current = v0;
    }

    fn step_vcvs(&mut self, c: &mut ChipCore, s: &mut Stamper) {
        let ic = self.input_count;
        let limit = converge_limit(s);
        let (t, dt) = (s.t, s.time_step);
        let o = OwnerNodes { volts: &c.base.volts, nodes: &c.base.nodes, ref_base: c.pins.len() };
        if !self.engine.check_convergence(&o, limit) {
            s.converged = false;
        }
        let Some(vn) = c.pins[ic].volt_source else { return };
        let rs = self.engine.eval_and_stamp(false, &o, t, dt, |input, dx| match input {
            StampInput::Voltage(pos, neg) => {
                s.stamp_matrix(vn, pos, -dx);
                s.stamp_matrix(vn, neg, dx);
            }
            StampInput::Current(ivs) => s.stamp_matrix(vn, ivs, -dx),
        });
        let v0 = self.engine.value;
        let volts = &c.base.volts;
        if (volts[ic] - volts[ic + 1] - v0).abs() > v0.abs() * 0.01 && s.sub_iterations < 100 {
            s.converged = false;
        }
        s.stamp_right_side(vn, rs);
        self.engine.save_inputs(&o);
    }

    /// The `v()`/`i()` part of a current-controlled source's linearisation: the sensed
    /// currents were differentiated by the caller, so this only adds the references (it
    /// re-derives the same value, so subtracting `v0` leaves just their residual).
    fn stamp_refs(
        &mut self,
        c: &ChipCore,
        s: &mut Stamper,
        v0: f64,
        mut stamp: impl FnMut(&mut Stamper, StampInput, f64),
    ) -> f64 {
        if self.engine.refs().is_empty() {
            return 0.0;
        }
        let (t, dt) = (s.t, s.time_step);
        let o = self.owner(c);
        self.engine.eval_and_stamp(false, &o, t, dt, |input, dx| stamp(s, input, dx)) - v0
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
        if !self.engine.check_convergence(&self.owner(c), converge_limit(s)) {
            s.converged = false;
        }
        if let Some(vno) = c.pins[ic].volt_source {
            self.engine.state.t = s.t;
            self.engine.state.time_step = s.time_step;
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
            rs += self.stamp_refs(c, s, v0, |s, input, dx| match input {
                StampInput::Voltage(pos, neg) => {
                    s.stamp_matrix(vno, pos, -dx);
                    s.stamp_matrix(vno, neg, dx);
                }
                StampInput::Current(ivs) => s.stamp_matrix(vno, ivs, -dx),
            });
            s.stamp_right_side(vno, rs);
        }
        self.engine.save_inputs(&self.owner(c));
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
        if !self.engine.check_convergence(&self.owner(c), converge_limit(s)) {
            s.converged = false;
        }
        self.engine.state.t = s.t;
        self.engine.state.time_step = s.time_step;
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
        // the output is a current from O- to O+
        rs += self.stamp_refs(c, s, v0, |s, input, dx| match input {
            StampInput::Voltage(pos, neg) => s.stamp_vccs(n[ic + 1], n[ic], pos, neg, dx),
            StampInput::Current(vs) => s.stamp_cccs(n[ic + 1], n[ic], vs, dx),
        });
        s.stamp_current_source(n[ic + 1], n[ic], rs);
        self.engine.save_inputs(&self.owner(c));
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
        assert_eq!(back.logic.expr_string(), "a/2+b/2");
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
