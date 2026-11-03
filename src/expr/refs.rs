// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (ExprEngine).
//! The expression machinery shared by the controlled sources: parsing, the `v(name)` /
//! `i(name)` references and their inputs, and the numeric linearisation that stamps an
//! expression into the matrix.
//!
//! Expression inputs come in two groups. First the owner's own input pins, whose voltages are
//! the letters `a`..`i`; then the references. Each reference occupies two nodes of the owning
//! element, after its posts and internal nodes: the positive and negative end of the
//! referenced quantity. The simulator fills them in when it analyses the circuit
//! (`Simulation::analyze`), and because they are real nodes of the owner, the referenced node
//! ends up in the owner's matrix, which is what makes it legal to stamp a derivative against
//! it. A current reference reads the current of a labeled ammeter and stamps against that
//! ammeter's voltage source row instead.

use super::{Expr, ExprRef, ExprState};
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// What a reference was resolved to, as the simulator found it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefResolution {
    /// No node or meter has that name; the reference reads 0.
    #[default]
    Unresolved,
    /// A labeled node, or a voltmeter whose two nodes the reference reads.
    Voltage,
    /// A labeled ammeter: the reference reads its current and stamps against its source.
    Current(VsId),
    /// `i(name)` names something that is not a current meter; the reference reads 0.
    NotCurrentMeter,
}

/// Where the linearisation of one input goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StampInput {
    /// The input is the voltage from the first node to the second.
    Voltage(NodeId, NodeId),
    /// The input is the current through a voltage source.
    Current(VsId),
}

/// The owner's node voltages and node numbers, which inputs are read from.
#[derive(Clone, Copy, Debug)]
pub struct OwnerNodes<'a> {
    /// Voltage of each node of the owner (posts, internal nodes, reference nodes).
    pub volts: &'a [f64],
    /// Solver node of each node of the owner.
    pub nodes: &'a [NodeId],
    /// Index of the first reference node.
    pub ref_base: usize,
}

/// Convergence window for expression inputs, widening as sub-iterations go on so a hard
/// expression still settles (the schedule of the original).
pub fn converge_limit(s: &Stamper) -> f64 {
    if s.sub_iterations < 10 {
        0.001
    } else if s.sub_iterations < 200 {
        0.01
    } else {
        0.1
    }
}

/// Parsing, reference bookkeeping and linearisation of one expression.
#[derive(Clone, Debug)]
pub struct ExprEngine {
    /// The expression as typed.
    pub expr_string: String,
    expr: Option<Expr>,
    refs: Vec<ExprRef>,
    /// The values the expression is evaluated with.
    pub state: ExprState,
    pin_inputs: usize,
    resolution: Vec<RefResolution>,
    ref_currents: Vec<f64>,
    last_inputs: Vec<f64>,
    /// Value of the expression at the last [`ExprEngine::eval_and_stamp`].
    pub value: f64,
}

impl ExprEngine {
    /// An engine for `text`; a parse error leaves the expression empty (it evaluates to 0).
    pub fn new(text: &str) -> Self {
        let mut e = ExprEngine {
            expr_string: text.to_string(),
            expr: None,
            refs: Vec::new(),
            state: ExprState::new(),
            pin_inputs: 0,
            resolution: Vec::new(),
            ref_currents: Vec::new(),
            last_inputs: Vec::new(),
            value: 0.0,
        };
        let _ = e.parse();
        e
    }

    /// Parses [`ExprEngine::expr_string`] again. On error the message is the one the edit
    /// dialog shows, and the expression evaluates to 0. The number of references may change,
    /// and with it the owner's node count, so the circuit must be analysed again.
    pub fn parse(&mut self) -> Result<(), String> {
        let result = Expr::parse_with_refs(&self.expr_string);
        let err = match result {
            Ok((e, refs)) => {
                self.expr = Some(e);
                self.refs = refs;
                None
            }
            Err(err) => {
                self.expr = None;
                self.refs.clear();
                Some(format!("Parse error in expression: {}: {err}", self.expr_string))
            }
        };
        self.alloc();
        err.map_or(Ok(()), Err)
    }

    /// Sets the text and parses it (see [`ExprEngine::parse`]).
    pub fn set_text(&mut self, text: &str) -> Result<(), String> {
        self.expr_string = text.to_string();
        self.parse()
    }

    /// True when the expression parsed.
    pub fn is_valid(&self) -> bool {
        self.expr.is_some()
    }

    /// Sets how many inputs come from the owner's pins (the letters `a`, `b`, ...).
    pub fn set_pin_inputs(&mut self, n: usize) {
        self.pin_inputs = n;
        self.alloc();
    }

    fn alloc(&mut self) {
        let n = self.refs.len();
        self.state.alloc_refs(n);
        self.resolution = vec![RefResolution::Unresolved; n];
        self.ref_currents = vec![0.0; n];
        self.last_inputs = vec![0.0; self.input_count()];
    }

    /// The quantities the expression refers to by name.
    pub fn refs(&self) -> &[ExprRef] {
        &self.refs
    }

    /// Number of nodes the references need: two per reference.
    pub fn ref_node_count(&self) -> usize {
        2 * self.refs.len()
    }

    /// Number of inputs the expression is linearised against: pins, then references.
    pub fn input_count(&self) -> usize {
        self.pin_inputs + self.refs.len()
    }

    /// Records what reference `k` was resolved to.
    pub fn set_resolution(&mut self, k: usize, r: RefResolution) {
        if let Some(slot) = self.resolution.get_mut(k) {
            *slot = r;
        }
    }

    /// Gives current reference `k` the present current of its ammeter.
    pub fn set_ref_current(&mut self, k: usize, current: f64) {
        if let Some(slot) = self.ref_currents.get_mut(k) {
            *slot = current;
        }
    }

    /// Value of input `i`: a pin voltage, a referenced voltage or a referenced current.
    pub fn input_value(&self, i: usize, o: &OwnerNodes<'_>) -> f64 {
        if i < self.pin_inputs {
            return o.volts.get(i).copied().unwrap_or(0.0);
        }
        let k = i - self.pin_inputs;
        match self.resolution.get(k).copied().unwrap_or_default() {
            RefResolution::Current(_) => self.ref_currents[k],
            RefResolution::Unresolved | RefResolution::NotCurrentMeter if self.refs[k].current => {
                0.0
            }
            _ => {
                let ix = o.ref_base + 2 * k;
                let v = |j: usize| o.volts.get(j).copied().unwrap_or(0.0);
                v(ix) - v(ix + 1)
            }
        }
    }

    /// Where input `i` stamps its derivative; `None` for an unresolved current reference.
    pub fn input_target(&self, i: usize, o: &OwnerNodes<'_>) -> Option<StampInput> {
        if i < self.pin_inputs {
            return Some(StampInput::Voltage(o.nodes.get(i).copied()?, NodeId::GROUND));
        }
        let k = i - self.pin_inputs;
        if let RefResolution::Current(vs) = self.resolution.get(k).copied().unwrap_or_default() {
            return Some(StampInput::Current(vs));
        }
        if self.refs[k].current {
            return None;
        }
        let ix = o.ref_base + 2 * k;
        Some(StampInput::Voltage(
            o.nodes.get(ix).copied().unwrap_or(NodeId::GROUND),
            o.nodes.get(ix + 1).copied().unwrap_or(NodeId::GROUND),
        ))
    }

    fn set_input(&mut self, i: usize, v: f64) {
        if i < self.pin_inputs {
            if let Some(slot) = self.state.values.get_mut(i) {
                *slot = v;
            }
        } else if let Some(slot) = self.state.ref_values.get_mut(i - self.pin_inputs) {
            *slot = v;
        }
    }

    /// True when every input moved less than `limit` since [`ExprEngine::save_inputs`]
    /// (current inputs use a window ten times tighter).
    pub fn check_convergence(&self, o: &OwnerNodes<'_>, limit: f64) -> bool {
        (0..self.input_count()).all(|i| {
            let current = matches!(self.input_target(i, o), Some(StampInput::Current(_)));
            let lim = if current { limit * 0.1 } else { limit };
            (self.input_value(i, o) - self.last_inputs[i]).abs() <= lim
        })
    }

    /// Remembers the present inputs for the next derivative and convergence check.
    pub fn save_inputs(&mut self, o: &OwnerNodes<'_>) {
        for i in 0..self.input_count() {
            self.last_inputs[i] = self.input_value(i, o);
        }
    }

    /// Evaluates the expression with the state exactly as it is (for owners that set some
    /// inputs themselves, such as the sensed currents of a current-controlled source).
    pub fn eval_state(&self) -> f64 {
        self.expr.as_ref().map_or(0.0, |e| e.eval(&self.state))
    }

    /// Evaluates the expression with the present inputs (without stamping).
    pub fn eval(&mut self, o: &OwnerNodes<'_>) -> f64 {
        for i in 0..self.input_count() {
            let v = self.input_value(i, o);
            self.set_input(i, v);
        }
        self.expr.as_ref().map_or(0.0, |e| e.eval(&self.state))
    }

    /// Evaluates the expression and hands `stamp` the linearisation of each input's
    /// contribution (its target and d(output)/d(input)). `negate` is for owners whose output
    /// is the negated expression. Leaves the value in [`ExprEngine::value`] and returns the
    /// right-side residual for the owner to stamp.
    pub fn eval_and_stamp(
        &mut self,
        negate: bool,
        o: &OwnerNodes<'_>,
        t: f64,
        time_step: f64,
        mut stamp: impl FnMut(StampInput, f64),
    ) -> f64 {
        let sgn = if negate { -1.0 } else { 1.0 };
        self.state.t = t;
        self.state.time_step = time_step;
        let Some(e) = self.expr.clone() else {
            self.value = 0.0;
            return 0.0;
        };
        let n = self.input_count();
        for i in 0..n {
            let v = self.input_value(i, o);
            self.set_input(i, v);
        }
        let v0 = sgn * e.eval(&self.state);
        self.value = v0;
        let mut rs = v0;
        for i in 0..n {
            let x0 = self.input_value(i, o);
            let mut dv = x0 - self.last_inputs[i];
            if dv.abs() < 1e-6 {
                dv = 1e-6;
            }
            self.set_input(i, x0);
            let v = sgn * e.eval(&self.state);
            self.set_input(i, x0 - dv);
            let v2 = sgn * e.eval(&self.state);
            let mut dx = (v - v2) / dv;
            if dx.abs() < 1e-6 {
                dx = if dx > 0.0 { 1e-6 } else { -1e-6 };
            }
            if let Some(target) = self.input_target(i, o) {
                stamp(target, dx);
            }
            rs -= dx * x0;
            self.set_input(i, x0);
        }
        rs
    }

    /// Rolls the present values into the previous-step slots that `lasta`..`lasti`,
    /// `lastoutput`, `dvdt()` and `didt()` read; called once per time step.
    pub fn step_finished(&mut self, output: f64) {
        self.state.update_last_values(output);
    }

    /// Forgets the previous values (the reset button).
    pub fn reset(&mut self) {
        self.state.reset();
        self.last_inputs.fill(0.0);
    }

    /// Lines for the info box describing references that could not be used.
    pub fn unresolved_info(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (k, r) in self.refs.iter().enumerate() {
            let func = if r.current { "i" } else { "v" };
            match self.resolution[k] {
                RefResolution::Unresolved => out.push(format!("unknown name: {func}({})", r.name)),
                RefResolution::NotCurrentMeter => {
                    out.push(format!("not a current meter: {}", r.name));
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner<'a>(volts: &'a [f64], nodes: &'a [NodeId], ref_base: usize) -> OwnerNodes<'a> {
        OwnerNodes { volts, nodes, ref_base }
    }

    #[test]
    fn inputs_are_pins_then_references() {
        let mut e = ExprEngine::new("a+v(x)+i(m)");
        e.set_pin_inputs(1);
        assert_eq!(e.input_count(), 3);
        assert_eq!(e.ref_node_count(), 4);
        // posts 0..2, references from 2: v(x) on nodes 2,3, i(m) on nodes 4,5
        let volts = [1.0, 0.0, 5.0, 1.0, 0.0, 0.0];
        let nodes: Vec<NodeId> = (0..6).map(NodeId).collect();
        e.set_resolution(0, RefResolution::Voltage);
        e.set_resolution(1, RefResolution::Current(VsId(3)));
        e.set_ref_current(1, 0.25);
        let o = owner(&volts, &nodes, 2);
        assert_eq!(e.input_value(0, &o), 1.0);
        assert_eq!(e.input_value(1, &o), 4.0);
        assert_eq!(e.input_value(2, &o), 0.25);
        assert_eq!(e.eval(&o), 5.25);
        assert_eq!(e.input_target(0, &o), Some(StampInput::Voltage(NodeId(0), NodeId::GROUND)));
        assert_eq!(e.input_target(1, &o), Some(StampInput::Voltage(NodeId(2), NodeId(3))));
        assert_eq!(e.input_target(2, &o), Some(StampInput::Current(VsId(3))));
    }

    #[test]
    fn unresolved_references_read_zero_and_are_reported() {
        let mut e = ExprEngine::new("v(nothere)+i(x)");
        e.set_resolution(1, RefResolution::NotCurrentMeter);
        let volts = [0.0; 6];
        let nodes = [NodeId::GROUND; 6];
        let o = owner(&volts, &nodes, 2);
        assert_eq!(e.eval(&o), 0.0);
        assert_eq!(e.input_target(1, &o), None);
        assert_eq!(
            e.unresolved_info(),
            vec!["unknown name: v(nothere)".to_string(), "not a current meter: x".to_string()]
        );
    }

    #[test]
    fn derivatives_follow_the_previous_step() {
        let mut e = ExprEngine::new("dvdt(x)");
        e.set_resolution(0, RefResolution::Voltage);
        let nodes: Vec<NodeId> = (0..4).map(NodeId).collect();
        let o1 = [0.0, 0.0, 1.0, 0.0];
        e.eval(&owner(&o1, &nodes, 2));
        e.step_finished(0.0);
        let o2 = [0.0, 0.0, 1.5, 0.0];
        let rs = e.eval_and_stamp(false, &owner(&o2, &nodes, 2), 0.0, 0.25, |_, _| {});
        assert!((e.value - 2.0).abs() < 1e-12, "{}", e.value);
        assert!(rs.is_finite());
        e.reset();
        assert_eq!(e.state.last_ref_values, vec![0.0]);
    }

    #[test]
    fn linearisation_uses_the_original_steps_and_floors() {
        // linear in its input: the derivative is exact and the residual is zero
        let mut e = ExprEngine::new("2*v(x)");
        e.set_resolution(0, RefResolution::Voltage);
        let nodes: Vec<NodeId> = (0..4).map(NodeId).collect();
        let volts = [0.0, 0.0, 3.0, 0.0];
        let mut got = Vec::new();
        let rs = e.eval_and_stamp(false, &owner(&volts, &nodes, 2), 0.0, 1e-6, |t, dx| {
            got.push((t, dx));
        });
        assert_eq!(got.len(), 1);
        assert!((got[0].1 - 2.0).abs() < 1e-9);
        assert!(rs.abs() < 1e-9, "{rs}");
        assert_eq!(e.value, 6.0);
        // a flat expression stamps the smallest allowed derivative, with the sign of 0 (-)
        let mut e = ExprEngine::new("0*v(x)+1");
        e.set_resolution(0, RefResolution::Voltage);
        let mut dx = 0.0;
        e.eval_and_stamp(true, &owner(&volts, &nodes, 2), 0.0, 1e-6, |_, d| dx = d);
        assert_eq!(dx, -1e-6);
    }

    #[test]
    fn convergence_window_is_tighter_for_currents() {
        let mut e = ExprEngine::new("v(x)+i(m)");
        e.set_resolution(0, RefResolution::Voltage);
        e.set_resolution(1, RefResolution::Current(VsId(0)));
        let nodes: Vec<NodeId> = (0..6).map(NodeId).collect();
        let volts = [0.0; 6];
        let o = owner(&volts, &nodes, 2);
        e.save_inputs(&o);
        assert!(e.check_convergence(&o, 0.001));
        // a current change of 0.0005 is within 0.001 for voltages but not for currents
        e.set_ref_current(1, 0.0005);
        assert!(!e.check_convergence(&o, 0.001));
        e.save_inputs(&o);
        assert!(e.check_convergence(&o, 0.001));
    }

    #[test]
    fn parse_errors_keep_the_text_and_clear_the_references() {
        let mut e = ExprEngine::new("v(a)");
        assert_eq!(e.refs().len(), 1);
        let err = e.set_text("2*(").unwrap_err();
        assert!(err.starts_with("Parse error in expression: 2*(: "), "{err}");
        assert!(!e.is_valid());
        assert_eq!(e.ref_node_count(), 0);
    }
}
