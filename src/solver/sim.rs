// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (SimulationManager).
//! Circuit analysis and time stepping.
//!
//! [`Simulation::analyze`] turns a list of elements into matrices (one per group of connected
//! nodes); [`Simulation::step`] advances the circuit by one time step, iterating until nonlinear
//! elements converge.

use super::dense_lu;
use super::path::{PathGraph, Validator};
use super::snapshot::{FrameSnapshot, SnapshotError, Status};
use super::sparse_lu::{Csc, SparseLu};
use super::stamper::{CircuitMatrix, Node, NodeId, Stamper, StopRequest, VoltageSource, VsId};
use crate::elements::element::{Element, Endpoint, LabelRegistry};
use crate::elements::geometry::Point;
use crate::error::{AnalysisError, StepError};
use std::cell::Cell;
use std::collections::HashMap;

/// Matrices with at least this many rows use the sparse solver when the solver type is auto.
pub const SPARSE_THRESHOLD: usize = 150;

thread_local! {
    static DC_ANALYSIS: Cell<bool> = const { Cell::new(false) };
    static CLOCK: Cell<(f64, f64)> = const { Cell::new((0.0, 5e-6)) };
}

/// Simulated time and nominal time step as last published by the simulation on this thread.
/// Elements read these when edited (for example to keep a waveform continuous).
pub fn sim_clock() -> (f64, f64) {
    CLOCK.with(Cell::get)
}

fn publish_clock(t: f64, max_time_step: f64) {
    CLOCK.with(|c| c.set((t, max_time_step)));
}

/// True while the simulation computes the DC operating point on this thread.
pub fn dc_analysis() -> bool {
    DC_ANALYSIS.with(Cell::get)
}

fn set_dc_analysis(on: bool) {
    DC_ANALYSIS.with(|c| c.set(on));
}

/// Which linear solver to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SolverType {
    /// Dense for small matrices, sparse for large ones.
    #[default]
    Auto,
    /// Always dense.
    Dense,
    /// Always sparse.
    Sparse,
}

/// Simulation settings that come with a circuit file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimSettings {
    /// Nominal (and maximum) time step in seconds.
    pub max_time_step: f64,
    /// Smallest time step used when convergence fails.
    pub min_time_step: f64,
    /// Reduce the time step automatically when convergence fails.
    pub adjust_time_step: bool,
    /// Linear solver choice.
    pub solver: SolverType,
}

impl Default for SimSettings {
    fn default() -> Self {
        SimSettings {
            max_time_step: 5e-6,
            min_time_step: 50e-12,
            adjust_time_step: false,
            solver: SolverType::Auto,
        }
    }
}

/// One entry of the wire current bookkeeping: wires are removed from the matrix, so their
/// currents are derived from the currents of the elements connected to them.
#[derive(Debug, Clone)]
struct WireSegment {
    wire: usize,
    bit: usize,
    endpoint0: Endpoint,
    endpoint1: Endpoint,
    neighbors: Vec<usize>,
    label_neighbors: Vec<usize>,
    post: usize,
    current: f64,
}

/// A running circuit simulation.
pub struct Simulation {
    /// The elements being simulated.
    pub elements: Vec<Box<dyn Element>>,
    /// Matrices, nodes and simulation state shared with the elements.
    pub st: Stamper,
    settings: SimSettings,
    labels: LabelRegistry,
    wire_info: Vec<WireSegment>,
    using_sparse: bool,
    circuit_non_linear: bool,
    needs_stamp: bool,
    time_step_accum: f64,
    time_step_count: u64,
    good_iterations: u32,
    unconnected_nodes: Vec<usize>,
    with_ground_connection: Vec<usize>,
    path_graph: PathGraph,
    delay_wire_processing: bool,
    analyzed: bool,
}

/// Union-find over grid points, used to merge points joined by wires into one node.
struct PointClasses {
    index: HashMap<Point, usize>,
    parent: Vec<usize>,
    node: Vec<Option<NodeId>>,
}

impl PointClasses {
    fn new() -> Self {
        PointClasses { index: HashMap::new(), parent: Vec::new(), node: Vec::new() }
    }

    fn new_class(&mut self) -> usize {
        self.parent.push(self.parent.len());
        self.node.push(None);
        self.parent.len() - 1
    }

    fn find(&mut self, mut c: usize) -> usize {
        while self.parent[c] != c {
            self.parent[c] = self.parent[self.parent[c]];
            c = self.parent[c];
        }
        c
    }

    fn class_of(&mut self, p: Point) -> Option<usize> {
        let c = *self.index.get(&p)?;
        Some(self.find(c))
    }

    fn insert(&mut self, p: Point, class: usize) {
        self.index.insert(p, class);
    }

    fn merge(&mut self, keep: usize, drop: usize) {
        let (a, b) = (self.find(keep), self.find(drop));
        if a != b {
            self.parent[b] = a;
            if self.node[a].is_none() {
                self.node[a] = self.node[b];
            }
        }
    }

    fn node_of(&mut self, p: Point) -> Option<NodeId> {
        let c = self.class_of(p)?;
        self.node[c]
    }
}

impl Simulation {
    /// Creates a simulation of `elements` (call [`Simulation::analyze`] before stepping).
    pub fn new(elements: Vec<Box<dyn Element>>, settings: SimSettings) -> Self {
        let st = Stamper {
            max_time_step: settings.max_time_step,
            min_time_step: settings.min_time_step,
            time_step: settings.max_time_step,
            ..Stamper::default()
        };
        Simulation {
            elements,
            st,
            settings,
            labels: LabelRegistry::default(),
            wire_info: Vec::new(),
            using_sparse: false,
            circuit_non_linear: false,
            needs_stamp: true,
            time_step_accum: 0.0,
            time_step_count: 0,
            good_iterations: 100,
            unconnected_nodes: Vec::new(),
            with_ground_connection: Vec::new(),
            path_graph: PathGraph::default(),
            delay_wire_processing: false,
            analyzed: false,
        }
    }

    /// Sets the simulated time, for reloading a circuit without restarting the clock.
    pub fn set_time(&mut self, t: f64) {
        self.st.t = t;
    }

    /// Current simulated time in seconds.
    pub fn time(&self) -> f64 {
        self.st.t
    }

    /// Current time step.
    pub fn time_step(&self) -> f64 {
        self.st.time_step
    }

    /// Number of nominal time steps completed.
    pub fn time_step_count(&self) -> u64 {
        self.time_step_count
    }

    /// Settings in effect.
    pub fn settings(&self) -> &SimSettings {
        &self.settings
    }

    /// True once [`Simulation::analyze`] succeeded and no stop was requested.
    pub fn is_ready(&self) -> bool {
        self.analyzed && self.st.stop.is_none()
    }

    /// The reason the simulation stopped, if it did.
    pub fn stop_request(&self) -> Option<&StopRequest> {
        self.st.stop.as_ref()
    }

    /// Replaces the solver settings; analyse again afterwards.
    pub fn set_settings(&mut self, settings: SimSettings) {
        self.settings = settings;
        self.st.max_time_step = settings.max_time_step;
        self.st.min_time_step = settings.min_time_step;
        if self.st.time_step > settings.max_time_step || self.st.time_step < settings.min_time_step
        {
            self.st.time_step = settings.max_time_step;
        }
    }

    /// When true, wire currents are not recomputed after every step; call
    /// [`Simulation::calc_wire_currents`] once per frame instead.
    pub fn set_delay_wire_processing(&mut self, delay: bool) {
        self.delay_wire_processing = delay;
    }

    /// Applies an edit-dialog value to element `idx` with the simulation clock published, so the
    /// element can adjust time-dependent state. Re-analyse afterwards.
    pub fn apply_edit(
        &mut self,
        idx: usize,
        n: usize,
        ei: &mut crate::elements::element::EditInfo,
    ) {
        publish_clock(self.st.t, self.st.max_time_step);
        if let Some(e) = self.elements.get_mut(idx) {
            e.set_edit_value(n, ei);
        }
    }

    /// Resets time and element state (the reset button).
    pub fn reset(&mut self) {
        self.st.t = 0.0;
        self.time_step_accum = 0.0;
        self.time_step_count = 0;
        self.st.time_step_count = 0;
        self.good_iterations = 100;
        for e in &mut self.elements {
            e.reset();
        }
        if self.analyzed {
            self.st.stop = None;
            let _ = self.analyze();
        }
    }

    /// Analyses the circuit and stamps the matrices; must run after any change to the circuit.
    pub fn analyze(&mut self) -> Result<(), AnalysisError> {
        self.st.stop = None;
        self.analyzed = false;
        if self.elements.is_empty() {
            return Ok(());
        }
        // an element's validation may change the circuit; analyse again (at most 10 times)
        let mut ok = false;
        for _ in 0..10 {
            if self.pre_stamp_circuit() {
                ok = true;
                break;
            }
            if self.st.stop.is_some() {
                break;
            }
        }
        if let Some(stop) = &self.st.stop {
            return Err(AnalysisError::Invalid {
                message: stop.message.clone(),
                element: stop.element,
            });
        }
        if !ok {
            return Err(AnalysisError::Invalid {
                message: "failed to stamp circuit".into(),
                element: None,
            });
        }
        self.stamp_circuit();
        if let Some(stop) = &self.st.stop {
            return Err(AnalysisError::Invalid {
                message: stop.message.clone(),
                element: stop.element,
            });
        }
        self.analyzed = true;
        Ok(())
    }

    fn calculate_wire_closure(&mut self) -> PointClasses {
        self.labels.clear();
        self.wire_info.clear();
        let mut classes = PointClasses::new();
        for i in 0..self.elements.len() {
            if !self.elements[i].is_removable_wire() {
                continue;
            }
            let bw = self.elements[i].bus_width();
            for bit in 0..bw {
                let (ep0, ep1) = self.elements[i].wire_endpoints(bit, &mut self.labels);
                self.wire_info.push(WireSegment {
                    wire: i,
                    bit,
                    endpoint0: ep0,
                    endpoint1: ep1,
                    neighbors: Vec::new(),
                    label_neighbors: Vec::new(),
                    post: 0,
                    current: 0.0,
                });
            }
            for bit in 0..bw {
                let Some(p0) = self.elements[i].get_post(bit) else { continue };
                let cn = classes.class_of(p0);
                let p1 = self.elements[i].connected_post(bit, &mut self.labels);
                let Some(p1) = p1 else {
                    // no connected post (first labelled node or first ground)
                    if cn.is_none() {
                        let c = classes.new_class();
                        classes.insert(p0, c);
                    }
                    continue;
                };
                let cn2 = classes.class_of(p1);
                match (cn, cn2) {
                    (Some(a), Some(b)) => classes.merge(a, b),
                    (Some(a), None) => classes.insert(p1, a),
                    (None, Some(b)) => classes.insert(p0, b),
                    (None, None) => {
                        let c = classes.new_class();
                        classes.insert(p0, c);
                        classes.insert(p1, c);
                    }
                }
            }
        }
        classes
    }

    fn set_ground_node(&mut self, classes: &mut PointClasses) {
        self.st.nodes.push(Node { index: 0, ..Node::default() });
        let mut got_ground = false;
        let mut got_rail = false;
        let mut volt: Option<usize> = None;
        let mut battery: Option<usize> = None;
        for (i, ce) in self.elements.iter().enumerate() {
            if ce.is_ground() {
                got_ground = true;
                if let Some(p) = ce.get_post(0) {
                    if let Some(c) = classes.class_of(p) {
                        classes.node[c] = Some(NodeId::GROUND);
                    }
                }
                break;
            }
            if ce.is_rail() {
                got_rail = true;
            }
            if volt.is_none() && ce.is_voltage_elm() {
                volt = Some(i);
            }
            if battery.is_none() && ce.is_battery() {
                battery = Some(i);
            }
        }
        // no ground and no rails: the first voltage source's first terminal is ground
        if !got_ground && !got_rail {
            if let Some(i) = volt.or(battery) {
                if let Some(pt) = self.elements[i].get_post(0) {
                    match classes.class_of(pt) {
                        Some(c) => classes.node[c] = Some(NodeId::GROUND),
                        None => {
                            let c = classes.new_class();
                            classes.node[c] = Some(NodeId::GROUND);
                            classes.insert(pt, c);
                        }
                    }
                }
            }
        }
    }

    fn make_node_list(&mut self, classes: &mut PointClasses) -> usize {
        for e in &mut self.elements {
            e.pre_stamp();
            e.alloc_nodes();
        }
        let mut vscount = 0;
        for i in 0..self.elements.len() {
            let inodes = self.elements[i].internal_node_count();
            let ivs = self.elements[i].voltage_source_count();
            let posts = self.elements[i].post_count();
            for j in 0..posts {
                let Some(pt) = self.elements[i].get_post(j) else { continue };
                let existing = classes.node_of(pt);
                match existing {
                    None => {
                        let id = NodeId(self.st.nodes.len());
                        self.st.nodes.push(Node {
                            index: id.0,
                            links: vec![(i, j)],
                            ..Node::default()
                        });
                        self.elements[i].set_node(j, id);
                        match classes.class_of(pt) {
                            Some(c) => classes.node[c] = Some(id),
                            None => {
                                let c = classes.new_class();
                                classes.node[c] = Some(id);
                                classes.insert(pt, c);
                            }
                        }
                    }
                    Some(id) => {
                        self.st.nodes[id.0].links.push((i, j));
                        self.elements[i].set_node(j, id);
                        if id == NodeId::GROUND {
                            self.elements[i].set_node_voltage(j, 0.0);
                        }
                    }
                }
            }
            for j in 0..inodes {
                let id = NodeId(self.st.nodes.len());
                self.st.nodes.push(Node {
                    index: id.0,
                    internal: true,
                    links: vec![(i, j + posts)],
                    ..Node::default()
                });
                self.elements[i].set_node(j + posts, id);
            }
            vscount += ivs;
        }
        vscount
    }

    fn calc_wire_info(&mut self) -> bool {
        let n = self.wire_info.len();
        let mut resolved: HashMap<(usize, usize), bool> = HashMap::new();
        let mut label_map: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, ws) in self.wire_info.iter().enumerate() {
            if let Endpoint::Label(name) = &ws.endpoint1 {
                label_map.entry(name.clone()).or_default().push(i);
            }
        }
        let mut order: Vec<usize> = (0..n).collect();
        let mut i = 0;
        let mut moved = 0usize;
        while i < order.len() {
            let wi = order[i];
            let (wire, bit) = (self.wire_info[wi].wire, self.wire_info[wi].bit);
            let cn1 = self.elements[wire].base().nodes.get(bit).copied();
            let Some(cn1) = cn1 else {
                self.wire_info[wi].neighbors.clear();
                self.wire_info[wi].label_neighbors.clear();
                resolved.insert((wire, bit), true);
                i += 1;
                continue;
            };
            if self.st.nodes[cn1.0].links.is_empty() {
                self.wire_info[wi].neighbors.clear();
                self.wire_info[wi].label_neighbors.clear();
                resolved.insert((wire, bit), true);
                i += 1;
                continue;
            }
            let mut neighbors0 = Vec::new();
            let mut neighbors1 = Vec::new();
            let mut label_neighbors = Vec::new();
            let mut is_ready0 = true;
            let mut is_ready1 = !self.elements[wire].is_ground();
            let ep0 = self.wire_info[wi].endpoint0.clone();
            let ep1 = self.wire_info[wi].endpoint1.clone();
            for &(ce, num) in &self.st.nodes[cn1.0].links {
                if ce == wire {
                    continue;
                }
                if num >= self.elements[ce].post_count() {
                    continue;
                }
                let Some(pt) = self.elements[ce].get_post(num) else { continue };
                let neighbor_bit = num % self.elements[ce].bus_width().max(1);
                let not_ready = self.elements[ce].is_removable_wire()
                    && !resolved.contains_key(&(ce, neighbor_bit));
                if ep0 == Endpoint::Point(pt) {
                    neighbors0.push(ce);
                    if not_ready {
                        is_ready0 = false;
                    }
                } else if matches!(ep1, Endpoint::Point(p) if p == pt) {
                    neighbors1.push(ce);
                    if not_ready {
                        is_ready1 = false;
                    }
                }
            }
            if let Endpoint::Label(name) = &ep1 {
                if let Some(peers) = label_map.get(name) {
                    for &other in peers {
                        if other == wi {
                            continue;
                        }
                        let ow = &self.wire_info[other];
                        let not_ready = !resolved.contains_key(&(ow.wire, ow.bit));
                        label_neighbors.push(other);
                        if not_ready {
                            is_ready1 = false;
                        }
                    }
                }
            }
            let is_ground = self.elements[wire].is_ground();
            if is_ready0 {
                self.wire_info[wi].neighbors = neighbors0;
                self.wire_info[wi].post = 0;
                resolved.insert((wire, bit), true);
                moved = 0;
                i += 1;
            } else if is_ready1 && (ep1 != Endpoint::None || !is_ground) {
                self.wire_info[wi].neighbors = neighbors1;
                self.wire_info[wi].label_neighbors = label_neighbors;
                self.wire_info[wi].post = 1;
                resolved.insert((wire, bit), true);
                moved = 0;
                i += 1;
            } else {
                // not ready: move to the end of the list and try again later
                let item = order.remove(i);
                order.push(item);
                moved += 1;
                if moved > order.len() * 2 {
                    self.st.stop("wire loop detected", Some(wire));
                    return false;
                }
            }
        }
        // keep the processing order for calc_wire_currents
        let infos: Vec<WireSegment> = order.iter().map(|&k| self.wire_info[k].clone()).collect();
        // label neighbour indices refer to positions in the original list; remap them
        let mut position = vec![0usize; n];
        for (newpos, &old) in order.iter().enumerate() {
            position[old] = newpos;
        }
        self.wire_info = infos
            .into_iter()
            .map(|mut ws| {
                ws.label_neighbors = ws.label_neighbors.iter().map(|&o| position[o]).collect();
                ws
            })
            .collect();
        true
    }

    fn find_unconnected_nodes(&mut self) {
        let total = self.st.nodes.len();
        let mut closure = vec![false; total];
        self.unconnected_nodes.clear();
        self.with_ground_connection.clear();
        closure[0] = true;
        for (i, ce) in self.elements.iter().enumerate() {
            let mut has_ground = false;
            for j in 0..ce.post_count() {
                if ce.has_ground_connection(j) {
                    has_ground = true;
                    closure[ce.base().nodes[j].0] = true;
                }
            }
            if has_ground {
                self.with_ground_connection.push(i);
            }
        }
        let mut queue: Vec<usize> = (0..total).filter(|&i| closure[i]).collect();
        let mut head = 0;
        let mut scan_from = 1;
        loop {
            if head < queue.len() {
                let n = queue[head];
                head += 1;
                for &(ei, post1) in &self.st.nodes[n].links {
                    let ce = &self.elements[ei];
                    for k in 0..ce.post_count() {
                        if k == post1 {
                            continue;
                        }
                        let kn = ce.base().nodes[k].0;
                        if !closure[kn] && ce.get_connection(post1, k) {
                            closure[kn] = true;
                            queue.push(kn);
                        }
                    }
                }
            } else {
                let mut found = false;
                while scan_from < total {
                    if !closure[scan_from] && !self.st.nodes[scan_from].internal {
                        self.unconnected_nodes.push(scan_from);
                        closure[scan_from] = true;
                        queue.push(scan_from);
                        scan_from += 1;
                        found = true;
                        break;
                    }
                    scan_from += 1;
                }
                if !found {
                    break;
                }
            }
        }
    }

    fn calculate_closures(&mut self) {
        let total = self.st.nodes.len();
        let mut closure_index = vec![-1i64; total];
        let mut closure_count = 0usize;
        for i in 1..total {
            if closure_index[i] >= 0 {
                continue;
            }
            let mut stack = vec![i];
            closure_index[i] = closure_count as i64;
            while let Some(n) = stack.pop() {
                for &(ei, post1) in &self.st.nodes[n].links {
                    let ce = &self.elements[ei];
                    for k in 0..ce.node_count() {
                        if k == post1 || !ce.get_matrix_connection(post1, k) {
                            continue;
                        }
                        let kn = ce.base().nodes[k].0;
                        if kn == 0 {
                            continue; // don't flood through ground
                        }
                        if closure_index[kn] < 0 {
                            closure_index[kn] = closure_count as i64;
                            stack.push(kn);
                        }
                    }
                }
            }
            closure_count += 1;
        }
        let closure_count = closure_count.max(1);
        self.st.matrices = (0..closure_count).map(|_| CircuitMatrix::default()).collect();
        for i in 1..total {
            let ci = closure_index[i];
            if ci < 0 {
                continue;
            }
            let m = &mut self.st.matrices[ci as usize];
            m.node_count += 1;
            self.st.nodes[i].row = m.node_count;
            self.st.nodes[i].matrix = Some(ci as usize);
            m.node_list.push(NodeId(i));
        }
        self.st.nodes[0].row = 0;
        self.st.nodes[0].matrix = None;
    }

    fn pre_stamp_circuit(&mut self) -> bool {
        self.st.nodes.clear();
        self.st.voltage_sources.clear();
        let mut classes = self.calculate_wire_closure();
        self.set_ground_node(&mut classes);
        let vscount = self.make_node_list(&mut classes);
        if !self.calc_wire_info() {
            return false;
        }
        // voltage sources
        self.circuit_non_linear = false;
        let mut count = 0;
        for i in 0..self.elements.len() {
            if self.elements[i].non_linear() {
                self.circuit_non_linear = true;
            }
            let ivs = self.elements[i].voltage_source_count();
            for j in 0..ivs {
                self.st.voltage_sources.push(VoltageSource {
                    index: count,
                    elm: i,
                    ..VoltageSource::default()
                });
                self.elements[i].set_voltage_source(j, VsId(count));
                count += 1;
            }
        }
        debug_assert_eq!(count, vscount);
        self.find_unconnected_nodes();
        self.calculate_closures();
        // validate with a snapshot of the connectivity
        self.path_graph =
            PathGraph::build(&self.elements, &self.st, self.with_ground_connection.clone());
        for i in 0..self.elements.len() {
            let mut v = Validator { graph: &self.path_graph, st: &mut self.st };
            if !self.elements[i].validate(i, &mut v) {
                return false;
            }
        }
        if self.st.stop.is_some() {
            return false;
        }
        // assign voltage sources to matrices, then give them rows after the node rows
        for vs in 0..self.st.voltage_sources.len() {
            self.assign_matrix(vs);
        }
        let mut per_matrix = vec![0usize; self.st.matrices.len()];
        for vs in 0..self.st.voltage_sources.len() {
            let Some(mi) = self.st.voltage_sources[vs].matrix else { continue };
            per_matrix[mi] += 1;
            self.st.voltage_sources[vs].row = self.st.matrices[mi].node_count + per_matrix[mi];
            self.st.matrices[mi].voltage_source_list.push(VsId(vs));
        }
        for (mi, m) in self.st.matrices.iter_mut().enumerate() {
            m.size = m.node_count + per_matrix[mi];
        }
        self.st.time_step = self.st.max_time_step;
        self.needs_stamp = true;
        true
    }

    fn assign_matrix(&mut self, vs: usize) {
        let v = &self.st.voltage_sources[vs];
        let pick = |n: Option<NodeId>| {
            n.filter(|n| *n != NodeId::GROUND).and_then(|n| self.st.nodes[n.0].matrix)
        };
        // an element whose source moves between nodes says where it connects right now
        let j = self.st.voltage_sources[..vs].iter().rev().take_while(|o| o.elm == v.elm).count();
        let declared = self.elements[v.elm].voltage_source_nodes_at(j);
        let (n1, n2) = match declared {
            Some((a, b)) if v.n1.is_none() => (Some(a), Some(b)),
            _ => (v.n1, v.n2),
        };
        let matrix = pick(n1).or_else(|| pick(n2)).or_else(|| {
            let elm = &self.elements[v.elm];
            elm.base()
                .nodes
                .get(elm.post_count().saturating_sub(1))
                .and_then(|n| self.st.nodes[n.0].matrix)
        });
        self.st.voltage_sources[vs].matrix = matrix;
    }

    /// Stamps all linear elements and factors linear matrices.
    pub fn stamp_circuit(&mut self) {
        for m in &mut self.st.matrices {
            let sz = m.size;
            m.matrix = vec![vec![0.0; sz]; sz];
            m.right_side = vec![0.0; sz];
            m.orig_matrix = vec![vec![0.0; sz]; sz];
            m.orig_right_side = vec![0.0; sz];
            m.permute = vec![0; sz];
            m.node_voltages = vec![0.0; m.node_count];
            if m.last_node_voltages.len() != m.node_count {
                m.last_node_voltages = vec![0.0; m.node_count];
            }
            m.non_linear = self.circuit_non_linear;
        }
        // connect unconnected nodes to ground with a big resistor
        for i in 0..self.unconnected_nodes.len() {
            let n = self.unconnected_nodes[i];
            self.st.stamp_resistor(NodeId::GROUND, NodeId(n), 1e8);
        }
        for i in 0..self.elements.len() {
            self.elements[i].stamp(&mut self.st);
        }
        if self.st.stop.is_some() {
            return;
        }
        let mut max_size = 0;
        for m in &mut self.st.matrices {
            max_size = max_size.max(m.size);
            m.orig_right_side.clone_from(&m.right_side);
            m.orig_matrix.clone_from(&m.matrix);
        }
        self.using_sparse = match self.settings.solver {
            SolverType::Sparse => true,
            SolverType::Dense => false,
            SolverType::Auto => max_size >= SPARSE_THRESHOLD,
        };
        for mi in 0..self.st.matrices.len() {
            if !self.st.matrices[mi].non_linear && !self.lu_factor(mi) {
                self.st.stop("Singular matrix!", None);
                return;
            }
        }
        self.needs_stamp = false;
    }

    fn lu_factor(&mut self, mi: usize) -> bool {
        let m = &mut self.st.matrices[mi];
        if self.using_sparse {
            let csc = Csc::from_dense(&m.matrix, f64::EPSILON);
            m.sparse_lu.get_or_insert_with(SparseLu::new).set_a(&csc)
        } else {
            let size = m.size;
            dense_lu::lu_factor(&mut m.matrix, size, &mut m.permute)
        }
    }

    fn lu_solve(&mut self, mi: usize) {
        let m = &mut self.st.matrices[mi];
        if self.using_sparse {
            if let Some(lu) = m.sparse_lu.as_mut() {
                lu.solve(&mut m.right_side);
            }
        } else {
            dense_lu::lu_solve(&m.matrix, m.size, &m.permute, &mut m.right_side);
        }
    }

    fn apply_solved_right_side(&mut self, mi: usize) {
        let (size, node_count) = (self.st.matrices[mi].size, self.st.matrices[mi].node_count);
        for j in 0..size {
            let res = self.st.matrices[mi].right_side[j];
            if res.is_nan() {
                self.st.converged = false;
                break;
            }
            if j < node_count {
                self.st.matrices[mi].node_voltages[j] = res;
            }
        }
        for k in 0..self.st.matrices[mi].voltage_source_list.len() {
            let vs = self.st.matrices[mi].voltage_source_list[k];
            let row = self.st.voltage_sources[vs.0].row;
            let res = self.st.matrices[mi].right_side[row - 1];
            if !res.is_nan() {
                let elm = self.st.voltage_sources[vs.0].elm;
                self.elements[elm].set_current(vs, res);
            }
        }
        self.set_node_voltages(mi, false);
    }

    fn set_node_voltages(&mut self, mi: usize, use_last: bool) {
        for j in 0..self.st.matrices[mi].node_list.len() {
            let cn = self.st.matrices[mi].node_list[j];
            let row = self.st.nodes[cn.0].row;
            let res = if use_last {
                self.st.matrices[mi].last_node_voltages[row - 1]
            } else {
                self.st.matrices[mi].node_voltages[row - 1]
            };
            for k in 0..self.st.nodes[cn.0].links.len() {
                let (e, p) = self.st.nodes[cn.0].links[k];
                self.elements[e].set_node_voltage(p, res);
            }
        }
    }

    /// Advances the simulation by one time step, iterating until nonlinear elements converge.
    /// When convergence fails and time step adjustment is on, the step is retried with a
    /// smaller time step.
    pub fn step(&mut self) -> Result<(), StepError> {
        if !self.is_ready() {
            return Err(StepError::Stopped {
                message: self
                    .st
                    .stop
                    .as_ref()
                    .map_or_else(|| "circuit not analysed".to_string(), |s| s.message.clone()),
                element: self.st.stop.as_ref().and_then(|s| s.element),
            });
        }
        publish_clock(self.st.t, self.st.max_time_step);
        loop {
            if self.good_iterations >= 3 && self.st.time_step < self.st.max_time_step {
                // things are going well, double the time step
                self.st.time_step = (self.st.time_step * 2.0).min(self.st.max_time_step);
                self.stamp_circuit();
                self.good_iterations = 0;
                self.check_stop()?;
            }
            for i in 0..self.elements.len() {
                self.elements[i].start_iteration(&self.st);
            }
            let subiter_count = if self.settings.adjust_time_step
                && self.st.time_step / 2.0 > self.st.min_time_step
            {
                100
            } else {
                5000
            };
            let mut subiter = 0;
            while subiter != subiter_count {
                self.st.converged = true;
                self.st.sub_iterations = subiter;
                for m in &mut self.st.matrices {
                    m.right_side.clone_from(&m.orig_right_side);
                    if m.non_linear {
                        m.matrix.clone_from(&m.orig_matrix);
                    }
                }
                for i in 0..self.elements.len() {
                    self.elements[i].do_step(&mut self.st);
                }
                self.check_stop()?;
                for mi in 0..self.st.matrices.len() {
                    let m = &self.st.matrices[mi];
                    if m.size < 8 && m.matrix.iter().any(|row| row.iter().any(|x| !x.is_finite())) {
                        self.st.stop("nan/infinite matrix!", None);
                        return Err(StepError::NotFinite);
                    }
                    if m.non_linear {
                        if self.st.converged && subiter > 0 {
                            continue;
                        }
                        if !self.lu_factor(mi) {
                            self.st.stop("Singular matrix!", None);
                            return Err(StepError::Singular);
                        }
                    }
                    self.lu_solve(mi);
                    self.apply_solved_right_side(mi);
                }
                if !self.circuit_non_linear || (self.st.converged && subiter > 0) {
                    break;
                }
                subiter += 1;
            }
            if subiter == subiter_count {
                // convergence failed
                self.good_iterations = 0;
                if self.settings.adjust_time_step {
                    self.st.time_step /= 2.0;
                }
                if self.st.time_step < self.st.min_time_step || !self.settings.adjust_time_step {
                    self.st.stop("Convergence failed!", None);
                    return Err(StepError::NotConverged);
                }
                // reset circuit state to the way it was at the start of the iteration
                for mi in 0..self.st.matrices.len() {
                    self.set_node_voltages(mi, true);
                }
                self.stamp_circuit();
                self.check_stop()?;
                continue;
            }
            if subiter < 3 {
                self.good_iterations += 1;
            } else {
                self.good_iterations = 0;
            }
            self.st.t += self.st.time_step;
            self.time_step_accum += self.st.time_step;
            if self.time_step_accum >= self.st.max_time_step {
                self.time_step_accum -= self.st.max_time_step;
                self.time_step_count += 1;
                self.st.time_step_count = self.time_step_count;
            }
            for i in 0..self.elements.len() {
                self.elements[i].step_finished(&mut self.st);
            }
            self.check_stop()?;
            if !self.delay_wire_processing {
                self.calc_wire_currents();
            }
            for m in &mut self.st.matrices {
                let n = m.node_count;
                m.last_node_voltages[..n].copy_from_slice(&m.node_voltages[..n]);
            }
            return Ok(());
        }
    }

    fn check_stop(&mut self) -> Result<(), StepError> {
        match &self.st.stop {
            Some(s) => Err(StepError::Stopped { message: s.message.clone(), element: s.element }),
            None => Ok(()),
        }
    }

    /// Runs steps until the simulated time reaches `t_end` (or a step fails).
    pub fn run_until(&mut self, t_end: f64) -> Result<(), StepError> {
        while self.st.t < t_end - self.st.time_step * 1e-9 {
            self.step()?;
        }
        Ok(())
    }

    /// Runs `n` time steps.
    pub fn run_steps(&mut self, n: usize) -> Result<(), StepError> {
        for _ in 0..n {
            self.step()?;
        }
        Ok(())
    }

    /// Wires are removed from the matrices, so their currents come from the elements
    /// connected to them. Call once per displayed frame.
    pub fn calc_wire_currents(&mut self) {
        for i in 0..self.wire_info.len() {
            let mut cur = 0.0;
            let ws = &self.wire_info[i];
            let wire = ws.wire;
            let bit = ws.bit;
            if ws.post == 0 {
                if let Some(p) = self.elements[wire].get_post(bit) {
                    for &ce in &ws.neighbors {
                        let e = &self.elements[ce];
                        cur += e.get_current_into_node(e.node_at_point(p));
                    }
                }
            } else if matches!(ws.endpoint1, Endpoint::Label(_)) {
                for &other in &ws.label_neighbors {
                    cur += self.wire_info[other].current;
                }
            } else {
                if let Some(p) = self.elements[wire].connected_post(bit, &mut self.labels) {
                    for &ce in &ws.neighbors {
                        let e = &self.elements[ce];
                        cur += e.get_current_into_node(e.node_at_point(p));
                    }
                }
            }
            if self.wire_info[i].post != 0 {
                cur = -cur;
            }
            self.elements[wire].set_wire_current(bit, cur);
            self.wire_info[i].current = cur;
        }
    }

    /// An immutable copy of what the UI draws: time, node voltages and element currents.
    /// `running` says whether the user has time running; an error recorded by the simulation
    /// takes precedence.
    pub fn snapshot(&self, sequence: u64, running: bool) -> FrameSnapshot {
        let error = self
            .st
            .stop
            .as_ref()
            .map(|s| SnapshotError { message: s.message.clone(), element: s.element });
        let status = match &error {
            Some(e) => Status::Errored { reason: e.message.clone(), element: e.element },
            None if running => Status::Running,
            None => Status::Stopped,
        };
        FrameSnapshot {
            sequence,
            time: self.st.t,
            time_step: self.st.time_step,
            status,
            node_voltages: (0..self.st.nodes.len()).map(|n| self.node_voltage(NodeId(n))).collect(),
            element_volts: self.elements.iter().map(|e| e.base().volts.clone()).collect(),
            element_currents: self.elements.iter().map(|e| e.get_current()).collect(),
            scope_samples: Vec::new(),
            error,
        }
    }

    /// Voltage of node `n` as of the last step.
    pub fn node_voltage(&self, n: NodeId) -> f64 {
        self.st.node_voltage(n)
    }

    /// Computes the DC operating point: capacitors become high resistances and inductors
    /// become shorts, the circuit is solved until it settles, then normal analysis resumes.
    pub fn find_dc_operating_point(&mut self) -> Result<(), StepError> {
        set_dc_analysis(true);
        self.st.dc_analysis = true;
        let result = self
            .analyze()
            .map_err(|e| StepError::Stopped { message: e.to_string(), element: None });
        let result = result.and_then(|()| {
            let saved_t = self.st.t;
            let r = self.run_steps(1);
            self.st.t = saved_t;
            r
        });
        set_dc_analysis(false);
        self.st.dc_analysis = false;
        let again = self.analyze();
        result?;
        again.map_err(|e| StepError::Stopped { message: e.to_string(), element: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_copies_time_voltages_and_currents() {
        use crate::elements::registry::Registry;
        use crate::format::parser::parse;
        let text = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 128 0 0 0 0 40 10 0 0 0.5\nr 0 0 64 0 0 1000\nr 64 0 64 128 0 3000\nw 64 128 0 128 0\n";
        let r = parse(text, &Registry::standard());
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        sim.analyze().unwrap();
        sim.run_steps(3).unwrap();
        let snap = sim.snapshot(7, true);
        assert_eq!(snap.sequence, 7);
        assert_eq!(snap.status, Status::Running);
        assert_eq!(snap.time, sim.time());
        assert_eq!(snap.element_currents.len(), 4);
        assert!((snap.element_currents[1] - 10.0 / 4000.0).abs() < 1e-9);
        assert!(snap.node_voltages.iter().any(|v| (v - 7.5).abs() < 1e-9));
        assert_eq!(snap.node_voltages[0], 0.0);
        assert_eq!(sim.snapshot(8, false).status, Status::Stopped);
        // the snapshot is a copy: stepping on does not change it
        sim.run_steps(5).unwrap();
        assert!(sim.time() > snap.time);
    }

    #[test]
    fn point_classes_merge_and_share_nodes() {
        let mut pc = PointClasses::new();
        let a = pc.new_class();
        pc.insert(Point::new(0, 0), a);
        let b = pc.new_class();
        pc.insert(Point::new(1, 0), b);
        pc.node[b] = Some(NodeId(3));
        pc.merge(a, b);
        assert_eq!(pc.class_of(Point::new(0, 0)), pc.class_of(Point::new(1, 0)));
        assert_eq!(pc.class_of(Point::new(9, 9)), None);
    }
}
