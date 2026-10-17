// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (SimulationManager, CircuitMatrix,
// CircuitNode, VoltageSource).
//! Matrix storage and the stamping interface elements use to describe themselves to the solver.

use super::sparse_lu::SparseLu;

/// Index of a circuit node. Node 0 is ground.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// Index of a voltage source (an extra matrix row owned by an element).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct VsId(pub usize);

impl NodeId {
    /// The ground node.
    pub const GROUND: NodeId = NodeId(0);
}

/// A circuit node and the element posts attached to it.
#[derive(Debug, Default, Clone)]
pub struct Node {
    /// Position in the node list.
    pub index: usize,
    /// True for nodes internal to an element.
    pub internal: bool,
    /// `(element index, post number)` pairs attached to this node.
    pub links: Vec<(usize, usize)>,
    /// Matrix this node belongs to (none for ground).
    pub matrix: Option<usize>,
    /// 1-based row in the matrix; 0 for ground.
    pub row: usize,
}

/// A voltage source: one extra row and column in a matrix.
#[derive(Debug, Default, Clone)]
pub struct VoltageSource {
    /// Position in the voltage source list.
    pub index: usize,
    /// Index of the owning element.
    pub elm: usize,
    /// Matrix this source belongs to.
    pub matrix: Option<usize>,
    /// 1-based row in the matrix (after the node rows).
    pub row: usize,
    /// Nodes the source is stamped between, used to pick the matrix.
    pub n1: Option<NodeId>,
    /// Second node.
    pub n2: Option<NodeId>,
}

/// One independent system of equations (a connected group of nodes).
#[derive(Default)]
pub struct CircuitMatrix {
    /// The coefficient matrix (or its LU factors).
    pub matrix: Vec<Vec<f64>>,
    /// Right-hand side, replaced by the solution after solving.
    pub right_side: Vec<f64>,
    /// Linear part of the matrix, restored every nonlinear iteration.
    pub orig_matrix: Vec<Vec<f64>>,
    /// Linear part of the right-hand side.
    pub orig_right_side: Vec<f64>,
    /// Pivot rows from the dense factorization.
    pub permute: Vec<usize>,
    /// Matrix dimension (node rows plus voltage source rows).
    pub size: usize,
    /// Number of node rows.
    pub node_count: usize,
    /// True when elements restamp the matrix every iteration.
    pub non_linear: bool,
    /// Solved node voltages.
    pub node_voltages: Vec<f64>,
    /// Node voltages at the end of the previous time step.
    pub last_node_voltages: Vec<f64>,
    /// Sparse factorization, when the sparse solver is in use.
    pub sparse_lu: Option<SparseLu>,
    /// Nodes in this matrix.
    pub node_list: Vec<NodeId>,
    /// Voltage sources in this matrix.
    pub voltage_source_list: Vec<VsId>,
}

/// Something that occupies a matrix row: a node or a voltage source.
pub trait Row: Copy {
    /// `(matrix index, 1-based row)` when the row exists; none for ground.
    fn locate(self, st: &Stamper) -> Option<(usize, usize)>;
}

impl Row for NodeId {
    fn locate(self, st: &Stamper) -> Option<(usize, usize)> {
        let n = &st.nodes[self.0];
        match n.matrix {
            Some(m) if n.row > 0 => Some((m, n.row)),
            _ => None,
        }
    }
}

impl Row for VsId {
    fn locate(self, st: &Stamper) -> Option<(usize, usize)> {
        let v = &st.voltage_sources[self.0];
        v.matrix.map(|m| (m, v.row))
    }
}

/// A request from an element to stop the simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct StopRequest {
    /// Reason shown to the user.
    pub message: String,
    /// Index of the element that asked, when known.
    pub element: Option<usize>,
}

/// Everything elements need while stamping and stepping: the matrices plus simulation state.
#[derive(Default)]
pub struct Stamper {
    /// All nodes; index 0 is ground.
    pub nodes: Vec<Node>,
    /// All voltage sources.
    pub voltage_sources: Vec<VoltageSource>,
    /// One matrix per group of connected nodes.
    pub matrices: Vec<CircuitMatrix>,
    /// Simulated time in seconds.
    pub t: f64,
    /// Number of time steps taken since the last reset.
    pub time_step_count: u64,
    /// Current time step.
    pub time_step: f64,
    /// Largest time step (the nominal one).
    pub max_time_step: f64,
    /// Smallest time step used when convergence fails.
    pub min_time_step: f64,
    /// Set false by elements that have not converged this iteration.
    pub converged: bool,
    /// Iteration number inside the current time step.
    pub sub_iterations: usize,
    /// True during the DC operating point analysis.
    pub dc_analysis: bool,
    /// Set when an element asks the simulation to stop.
    pub stop: Option<StopRequest>,
    /// Set by an element that wants time to stop without it being an error (a stop trigger).
    pub pause_requested: bool,
    pub(crate) rng_state: u64,
}

impl Stamper {
    /// Adds `x` to the matrix entry at row `i`, column `j` (ignored when either is ground).
    pub fn stamp_matrix(&mut self, i: impl Row, j: impl Row, x: f64) {
        if let (Some((mi, ri)), Some((_, rj))) = (i.locate(self), j.locate(self)) {
            self.matrices[mi].matrix[ri - 1][rj - 1] += x;
        }
    }

    /// Adds `x` to the right-hand side of row `n` (an independent current source into it).
    pub fn stamp_right_side(&mut self, n: impl Row, x: f64) {
        if let Some((m, r)) = n.locate(self) {
            self.matrices[m].right_side[r - 1] += x;
        }
    }

    /// Voltage of node `n` as of the last solution.
    pub fn node_voltage(&self, n: NodeId) -> f64 {
        match n.locate(self) {
            Some((m, r)) => self.matrices[m].node_voltages[r - 1],
            None => 0.0,
        }
    }

    /// Records the nodes a voltage source is stamped between so it can pick its matrix.
    pub fn set_vs_nodes(&mut self, vs: VsId, n1: NodeId, n2: NodeId) {
        let v = &mut self.voltage_sources[vs.0];
        v.n1 = Some(n1);
        v.n2 = Some(n2);
    }

    /// Stamps a resistor of `r` ohms between `n1` and `n2`.
    pub fn stamp_resistor(&mut self, n1: NodeId, n2: NodeId, r: f64) {
        self.stamp_conductance(n1, n2, 1.0 / r);
    }

    /// Stamps a conductance of `g` siemens between `n1` and `n2`.
    pub fn stamp_conductance(&mut self, n1: NodeId, n2: NodeId, g: f64) {
        self.stamp_matrix(n1, n1, g);
        self.stamp_matrix(n2, n2, g);
        self.stamp_matrix(n1, n2, -g);
        self.stamp_matrix(n2, n1, -g);
    }

    /// Makes voltage source `vs` follow `coef` times the voltage from `n1` to `n2`.
    pub fn stamp_vcvs(&mut self, n1: NodeId, n2: NodeId, coef: f64, vs: VsId) {
        self.stamp_matrix(vs, n1, coef);
        self.stamp_matrix(vs, n2, -coef);
    }

    /// Stamps independent voltage source `vs` of `v` volts from `n1` to `n2`.
    pub fn stamp_voltage_source(&mut self, n1: NodeId, n2: NodeId, vs: VsId, v: f64) {
        self.stamp_matrix(vs, n1, -1.0);
        self.stamp_matrix(vs, n2, 1.0);
        self.stamp_right_side(vs, v);
        self.stamp_matrix(n1, vs, 1.0);
        self.stamp_matrix(n2, vs, -1.0);
    }

    /// Stamps a voltage source whose value is set later by [`Stamper::update_voltage_source`].
    pub fn stamp_voltage_source_var(&mut self, n1: NodeId, n2: NodeId, vs: VsId) {
        self.stamp_matrix(vs, n1, -1.0);
        self.stamp_matrix(vs, n2, 1.0);
        self.stamp_matrix(n1, vs, 1.0);
        self.stamp_matrix(n2, vs, -1.0);
    }

    /// Stamps a voltage source using the nodes saved with [`Stamper::set_vs_nodes`].
    pub fn stamp_voltage_source_saved(&mut self, vs: VsId, v: f64) {
        let (n1, n2) = self.vs_nodes(vs);
        self.stamp_voltage_source(n1, n2, vs, v);
    }

    /// Updates the value of a voltage source during `do_step`.
    pub fn update_voltage_source(&mut self, vs: VsId, v: f64) {
        self.stamp_right_side(vs, v);
    }

    /// Nodes saved for a voltage source (ground when unset).
    pub fn vs_nodes(&self, vs: VsId) -> (NodeId, NodeId) {
        let v = &self.voltage_sources[vs.0];
        (v.n1.unwrap_or(NodeId::GROUND), v.n2.unwrap_or(NodeId::GROUND))
    }

    /// Current from `cn1` to `cn2` equal to the voltage from `vn1` to `vn2` times `g`.
    pub fn stamp_vccs(&mut self, cn1: NodeId, cn2: NodeId, vn1: NodeId, vn2: NodeId, g: f64) {
        self.stamp_matrix(cn1, vn1, g);
        self.stamp_matrix(cn2, vn2, g);
        self.stamp_matrix(cn1, vn2, -g);
        self.stamp_matrix(cn2, vn1, -g);
    }

    /// Stamps an independent current source of `i` amps from `n1` to `n2`.
    pub fn stamp_current_source(&mut self, n1: NodeId, n2: NodeId, i: f64) {
        self.stamp_right_side(n1, -i);
        self.stamp_right_side(n2, i);
    }

    /// Current source from `n1` to `n2` proportional to the current through `vs`.
    pub fn stamp_cccs(&mut self, n1: NodeId, n2: NodeId, vs: VsId, gain: f64) {
        self.stamp_matrix(n1, vs, gain);
        self.stamp_matrix(n2, vs, -gain);
    }

    /// Marks that an element changes row `n`'s right side every iteration (no-op, documents intent).
    pub fn stamp_right_side_var(&mut self, _n: impl Row) {}

    /// Marks that an element changes row `n`'s left side every iteration (no-op, documents intent).
    pub fn stamp_non_linear(&mut self, _n: impl Row) {}

    /// Next pseudo-random number in [0, 1); deterministic for a fixed seed (see
    /// [`Stamper::seed`]).
    pub fn random(&mut self) -> f64 {
        if self.rng_state == 0 {
            self.rng_state = 0x9E37_79B9_7F4A_7C15;
        }
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Seeds the random number generator (tests use this for reproducible noise).
    pub fn seed(&mut self, seed: u64) {
        self.rng_state = seed.max(1);
    }

    /// Asks the simulation to stop with a message.
    pub fn stop(&mut self, message: &str, element: Option<usize>) {
        if self.stop.is_none() {
            self.stop = Some(StopRequest { message: message.to_string(), element });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_node_stamper() -> Stamper {
        let nodes = vec![
            Node { index: 0, ..Node::default() },
            Node { index: 1, matrix: Some(0), row: 1, ..Node::default() },
            Node { index: 2, matrix: Some(0), row: 2, ..Node::default() },
        ];
        let matrix = CircuitMatrix {
            matrix: vec![vec![0.0; 2]; 2],
            right_side: vec![0.0; 2],
            size: 2,
            node_count: 2,
            ..CircuitMatrix::default()
        };
        Stamper { nodes, matrices: vec![matrix], ..Stamper::default() }
    }

    #[test]
    fn resistor_to_ground_stamps_only_the_live_node() {
        let mut st = two_node_stamper();
        st.stamp_resistor(NodeId(1), NodeId::GROUND, 100.0);
        assert!((st.matrices[0].matrix[0][0] - 0.01).abs() < 1e-15);
        assert_eq!(st.matrices[0].matrix[1][1], 0.0);
    }

    #[test]
    fn resistor_between_nodes_stamps_the_full_pattern() {
        let mut st = two_node_stamper();
        st.stamp_resistor(NodeId(1), NodeId(2), 10.0);
        let m = &st.matrices[0].matrix;
        assert_eq!((m[0][0], m[1][1], m[0][1], m[1][0]), (0.1, 0.1, -0.1, -0.1));
    }

    #[test]
    fn current_source_goes_to_the_right_side() {
        let mut st = two_node_stamper();
        st.stamp_current_source(NodeId(1), NodeId(2), 2.0);
        assert_eq!(st.matrices[0].right_side, vec![-2.0, 2.0]);
    }
}
