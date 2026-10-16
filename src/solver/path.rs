// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (FindPathInfo).
//! Depth-first search for loops through the circuit, used by elements to validate themselves.

use super::stamper::{NodeId, Stamper};
use crate::elements::element::Element;

/// Kind of loop being searched for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathKind {
    /// A path of inductors (and anything else except current sources) with matching current.
    Inductor,
    /// A loop made only of voltage sources, wires and grounds.
    Voltage,
    /// A path made only of wires.
    Short,
    /// A loop made of wires, ideal capacitors and voltage sources.
    CapVoltage,
}

/// What an element is, as far as path searching cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathRole {
    /// Any other element.
    Other,
    /// A current source.
    Current,
    /// A voltage source.
    Voltage,
    /// A logic input (behaves as a voltage source).
    LogicInput,
    /// A ground element.
    Ground,
    /// An inductor.
    Inductor,
}

/// Connectivity facts about one element.
#[derive(Debug, Clone)]
pub struct PathElement {
    /// Node of each post.
    pub nodes: Vec<NodeId>,
    /// Whether each post is connected to ground.
    pub ground: Vec<bool>,
    /// `connection[j][k]`: are posts `j` and `k` connected through the element?
    pub connection: Vec<Vec<bool>>,
    /// True for wire-like elements.
    pub wire_equivalent: bool,
    /// True for ideal capacitors.
    pub ideal_capacitor: bool,
    /// Role in the search.
    pub role: PathRole,
    /// Current through the element (for matching inductor currents).
    pub current: f64,
}

/// What an element gets while validating itself: path searches and the ability to stop.
pub struct Validator<'a> {
    /// Connectivity snapshot.
    pub graph: &'a PathGraph,
    /// Simulation state, for stopping with a message.
    pub st: &'a mut Stamper,
}

impl Validator<'_> {
    /// Looks for a path of `kind` from `start` back to `dest`, ignoring element `first_elm`.
    pub fn find_path(&self, kind: PathKind, first_elm: usize, start: NodeId, dest: NodeId) -> bool {
        self.graph.find_path(kind, first_elm, start, dest)
    }

    /// Asks the simulation to stop with `message`, blaming element `elm`.
    pub fn stop(&mut self, message: &str, elm: usize) {
        self.st.stop(message, Some(elm));
    }
}

/// A snapshot of the circuit's connectivity that elements can search while validating.
#[derive(Debug, Default)]
pub struct PathGraph {
    /// One entry per element, in element order.
    pub elements: Vec<PathElement>,
    /// For each node, the `(element, post)` pairs attached to it.
    pub links: Vec<Vec<(usize, usize)>>,
    /// Elements that connect some post to ground.
    pub with_ground_connection: Vec<usize>,
}

impl PathGraph {
    /// Builds the snapshot from the analysed circuit.
    pub fn build(elements: &[Box<dyn Element>], st: &Stamper, with_ground: Vec<usize>) -> Self {
        let elms = elements
            .iter()
            .map(|e| {
                let pc = e.post_count();
                let role = match e.class_name() {
                    "CurrentElm" => PathRole::Current,
                    "LogicInputElm" => PathRole::LogicInput,
                    "InductorElm" => PathRole::Inductor,
                    _ if e.is_ground() => PathRole::Ground,
                    _ if e.is_voltage_elm() => PathRole::Voltage,
                    _ => PathRole::Other,
                };
                PathElement {
                    nodes: e.base().nodes.iter().take(pc).copied().collect(),
                    ground: (0..pc).map(|j| e.has_ground_connection(j)).collect(),
                    connection: (0..pc)
                        .map(|j| (0..pc).map(|k| e.get_connection(j, k)).collect())
                        .collect(),
                    wire_equivalent: e.is_wire_equivalent(),
                    ideal_capacitor: e.is_ideal_capacitor(),
                    role,
                    current: e.get_current(),
                }
            })
            .collect();
        let links = st.nodes.iter().map(|n| n.links.clone()).collect();
        PathGraph { elements: elms, links, with_ground_connection: with_ground }
    }

    /// Looks for a path of the given kind from `start` back to `dest`, ignoring element
    /// `first_elm` (the element doing the search).
    pub fn find_path(&self, kind: PathKind, first_elm: usize, start: NodeId, dest: NodeId) -> bool {
        let mut search = Search {
            graph: self,
            kind,
            first: first_elm,
            dest,
            visited: vec![false; self.links.len()],
            first_current: self.elements.get(first_elm).map_or(0.0, |e| e.current),
        };
        search.find(start)
    }
}

struct Search<'a> {
    graph: &'a PathGraph,
    kind: PathKind,
    first: usize,
    dest: NodeId,
    visited: Vec<bool>,
    first_current: f64,
}

impl Search<'_> {
    fn find(&mut self, n1: NodeId) -> bool {
        if n1 == self.dest {
            return true;
        }
        if self.visited[n1.0] {
            return false;
        }
        self.visited[n1.0] = true;
        for &(elm, _) in &self.graph.links[n1.0] {
            if self.check(n1, elm) {
                return true;
            }
        }
        if n1 == NodeId::GROUND {
            for &elm in &self.graph.with_ground_connection {
                if self.check(NodeId::GROUND, elm) {
                    return true;
                }
            }
        }
        false
    }

    fn check(&mut self, n1: NodeId, elm: usize) -> bool {
        if elm == self.first {
            return false;
        }
        let ce = &self.graph.elements[elm];
        match self.kind {
            PathKind::Inductor if ce.role == PathRole::Current => return false,
            PathKind::Voltage
                if !(ce.wire_equivalent
                    || matches!(
                        ce.role,
                        PathRole::Voltage | PathRole::LogicInput | PathRole::Ground
                    )) =>
            {
                return false;
            }
            PathKind::Short if !ce.wire_equivalent => return false,
            PathKind::CapVoltage
                if !(ce.wire_equivalent
                    || ce.ideal_capacitor
                    || matches!(ce.role, PathRole::Voltage | PathRole::LogicInput)) =>
            {
                return false;
            }
            _ => {}
        }
        let posts = ce.nodes.len();
        if n1 == NodeId::GROUND {
            for j in 0..posts {
                if ce.ground[j] && self.find(ce.nodes[j]) {
                    return true;
                }
            }
        }
        for j in 0..posts {
            if ce.nodes[j] != n1 {
                continue;
            }
            if ce.ground[j] && self.find(NodeId::GROUND) {
                return true;
            }
            if self.kind == PathKind::Inductor && ce.role == PathRole::Inductor {
                // inductors can use paths with other inductors of matching current
                let c = if j == 0 { -ce.current } else { ce.current };
                if (c - self.first_current).abs() > 1e-10 {
                    continue;
                }
            }
            for k in 0..posts {
                if j != k && ce.connection[j][k] && self.find(ce.nodes[k]) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(nodes: &[usize], wire: bool) -> PathElement {
        let n = nodes.len();
        PathElement {
            nodes: nodes.iter().map(|&i| NodeId(i)).collect(),
            ground: vec![false; n],
            connection: vec![vec![true; n]; n],
            wire_equivalent: wire,
            ideal_capacitor: false,
            role: PathRole::Other,
            current: 0.0,
        }
    }

    fn graph(elements: Vec<PathElement>, nodes: usize) -> PathGraph {
        let mut links = vec![Vec::new(); nodes];
        for (i, e) in elements.iter().enumerate() {
            for (j, n) in e.nodes.iter().enumerate() {
                links[n.0].push((i, j));
            }
        }
        PathGraph { elements, links, with_ground_connection: Vec::new() }
    }

    #[test]
    fn finds_wire_path_between_nodes() {
        // element 0 (searcher) between nodes 1 and 2; wires 1-3 and 3-2 form a short path
        let g =
            graph(vec![element(&[1, 2], false), element(&[1, 3], true), element(&[3, 2], true)], 4);
        assert!(g.find_path(PathKind::Short, 0, NodeId(1), NodeId(2)));
    }

    #[test]
    fn short_search_ignores_non_wires() {
        let g = graph(vec![element(&[1, 2], false), element(&[1, 2], false)], 3);
        assert!(!g.find_path(PathKind::Short, 0, NodeId(1), NodeId(2)));
    }

    #[test]
    fn voltage_loop_needs_sources_wires_or_ground() {
        let mut vs = element(&[1, 2], false);
        vs.role = PathRole::Voltage;
        let g = graph(vec![element(&[1, 2], false), vs], 3);
        assert!(g.find_path(PathKind::Voltage, 0, NodeId(1), NodeId(2)));
    }

    #[test]
    fn path_can_pass_through_ground() {
        let mut a = element(&[1, 0], true);
        a.ground = vec![false, true];
        let mut b = element(&[0, 2], true);
        b.ground = vec![true, false];
        let mut g = graph(vec![element(&[1, 2], false), a, b], 3);
        g.with_ground_connection = vec![1, 2];
        assert!(g.find_path(PathKind::Short, 0, NodeId(1), NodeId(2)));
    }
}
