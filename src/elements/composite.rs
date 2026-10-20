// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CompositeElm, DarlingtonElm, OTAElm).
//! Elements made of other elements: the shared machinery ([`ChildNet`]) and the composite
//! elements that have a fixed internal circuit (the Darlington pair and the OTA).
//!
//! A [`ChildNet`] owns private copies of the inner elements and maps the nodes of the outer
//! element onto their posts. The solver sees one element; everything it asks is forwarded.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::current_text;
use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Color, Painter};
use crate::solver::path::Validator;
use crate::solver::stamper::{NodeId, Stamper, VsId};
use std::cell::OnceCell;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Family name used for the feature switch.
pub const FAMILY: &str = "subcircuits";

/// Flag of composite elements: children are dumped with escaped spaces.
pub const FLAG_ESCAPE: i32 = 1;

thread_local! {
    static REGISTRY: OnceCell<Registry> = const { OnceCell::new() };
}

/// Runs `f` with the registry of every element type (built once per thread).
pub fn with_registry<R>(f: impl FnOnce(&Registry) -> R) -> R {
    REGISTRY.with(|c| f(c.get_or_init(Registry::standard)))
}

/// True for the element the registry creates at the origin just to learn its tag: building
/// the children would need the registry, which is still being built.
fn probe(base: &Base, st: &Option<&mut Tokenizer<'_>>) -> bool {
    st.is_none() && base.x == 0 && base.y == 0
}

/// Changes some parameters of an element by handing it an XML tag with those attributes (its
/// flags are kept). Used to set up the parts of composite elements.
pub fn patch_child(child: &mut dyn Element, attrs: &[(&str, String)]) {
    let mut node = XmlNode { tag: child.xml_dump_type(), ..XmlNode::default() };
    node.attrs.push(("f".into(), child.base().flags.to_string()));
    for (k, v) in attrs {
        node.attrs.push(((*k).to_string(), v.clone()));
    }
    child.undump_xml(&node);
}

/// Reads the node numbers of an `nn` attribute.
pub fn parse_nodes(s: &str) -> Vec<i32> {
    s.split(|c: char| c.is_whitespace() || c == '+')
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn is_model_tag(tag: &str) -> bool {
    matches!(tag, "dm" | "rlm" | "tm" | "mm" | "clm" | "ccm")
}

/// Creates the elements described by XML tags with an `nn` attribute (their node numbers).
/// Wires, labels and other display-only elements are left out: the node numbers already say
/// how everything connects.
pub fn children_from_xml(elms: &[XmlNode]) -> (Vec<Box<dyn Element>>, Vec<Vec<i32>>) {
    let mut children: Vec<Box<dyn Element>> = Vec::new();
    let mut nodes = Vec::new();
    with_registry(|reg| {
        for node in elms {
            if is_model_tag(&node.tag) {
                continue;
            }
            let Some(d) = reg.get_by_xml_tag(&node.tag) else { continue };
            if matches!(d.id, "WireElm" | "RoutedWireElm" | "LabeledNodeElm" | "ScopeElm") {
                continue;
            }
            if d.id == "GroundElm" && node.attr("x").is_some() {
                continue;
            }
            let Some(mut e) = reg.create(d.id, 0, 0) else { continue };
            if e.is_graphic() {
                continue;
            }
            e.undump_xml(node);
            let pos: Vec<i32> = node
                .attr("x")
                .map(|x| x.split_whitespace().filter_map(|p| p.parse().ok()).collect())
                .unwrap_or_default();
            if let [x1, y1, x2, y2] = pos[..] {
                e.base_mut().set_position(x1, y1, x2, y2);
                // the element's own geometry (leads and the like) comes from its set_points
                e.set_points();
            } else {
                e.set_points();
            }
            e.alloc_nodes();
            nodes.push(node.attr("nn").map(parse_nodes).unwrap_or_default());
            children.push(e);
        }
    });
    (children, nodes)
}

/// Union-find over the nodes of a composite.
#[derive(Clone, Debug, Default)]
struct Classes(Vec<usize>);

impl Classes {
    fn new(n: usize) -> Self {
        Classes((0..n).collect())
    }
    fn find(&mut self, mut a: usize) -> usize {
        while self.0[a] != a {
            self.0[a] = self.0[self.0[a]];
            a = self.0[a];
        }
        a
    }
    fn root(&self, mut a: usize) -> usize {
        while self.0[a] != a {
            a = self.0[a];
        }
        a
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        self.0[ra] = rb;
    }
}

/// The inner elements of a composite and how they attach to its nodes.
#[derive(Default)]
pub struct ChildNet {
    /// The inner elements.
    pub children: Vec<Box<dyn Element>>,
    /// For each child, the model node number of each of its posts (0 is ground).
    pub child_nodes: Vec<Vec<i32>>,
    /// Model node number of each post of the composite.
    pub ext_nodes: Vec<i32>,
    /// Per node of the composite (posts, then internal), the child posts attached to it.
    node_links: Vec<Vec<(usize, usize)>>,
    /// Child that owns each voltage source handed to the composite.
    vs_owner: Vec<(VsId, usize)>,
    connected: Classes,
    matrix_connected: Classes,
    grounded: Vec<bool>,
    /// Children that sense currents of siblings, with the sibling for each input pair.
    spice_links: Vec<(usize, Vec<Option<usize>>)>,
}

impl ChildNet {
    /// Builds the children of a model written as lines `ClassName n1 n2 ...`. When `st` is
    /// given it holds the state of each child, one token per child, as the text format keeps
    /// them (`escaped` tells how the tokens are delimited).
    pub fn from_model_string(
        model: &str,
        ext_nodes: &[i32],
        mut st: Option<&mut Tokenizer<'_>>,
        escaped: bool,
    ) -> ChildNet {
        Self::build(model, ext_nodes, |_| {
            st.as_deref_mut()
                .and_then(|st| st.next_token())
                .map(|tok| if escaped { unescape(tok) } else { tok.replace('_', " ") })
        })
    }

    /// Like [`ChildNet::from_model_string`] with the state of each child given as one escaped
    /// text per child.
    pub fn from_model_tokens(model: &str, ext_nodes: &[i32], tokens: &[&str]) -> ChildNet {
        Self::build(model, ext_nodes, |i| tokens.get(i).map(|t| unescape(t)))
    }

    fn build(
        model: &str,
        ext_nodes: &[i32],
        mut state: impl FnMut(usize) -> Option<String>,
    ) -> ChildNet {
        let mut net = ChildNet { ext_nodes: ext_nodes.to_vec(), ..ChildNet::default() };
        if model.is_empty() {
            return net;
        }
        with_registry(|reg| {
            for (index, line) in model.lines().enumerate() {
                let mut parts = line.split_whitespace();
                let Some(class) = parts.next() else { continue };
                let nodes: Vec<i32> = parts.filter_map(|t| t.parse().ok()).collect();
                // the original names some classes by their parent
                let class = match class {
                    "TransistorElm" => "NTransistorElm",
                    "DarlingtonElm" => "NDarlingtonElm",
                    "MosfetElm" => "NMosfetElm",
                    "JfetElm" => "NJfetElm",
                    other => other,
                };
                let Some(mut elm) = reg.create(class, 0, 0) else { continue };
                if let Some(text) = state(index) {
                    let mut ct = Tokenizer::new(&text);
                    if let Ok(flags) = ct.next_i32() {
                        let code = elm.dump_type();
                        if let Some(Ok(e)) = reg.create_from_dump(code, 0, 0, 0, 0, flags, &mut ct)
                        {
                            elm = e;
                        }
                    }
                }
                elm.set_points();
                elm.alloc_nodes();
                net.children.push(elm);
                net.child_nodes.push(nodes);
            }
        });
        net.build_links();
        net
    }

    /// Builds the children from XML tags (see [`children_from_xml`]).
    pub fn from_xml(xml: &str, ext_nodes: &[i32]) -> ChildNet {
        let mut net = ChildNet { ext_nodes: ext_nodes.to_vec(), ..ChildNet::default() };
        if let Ok(root) = crate::format::xml::parse(xml) {
            let (children, nodes) = children_from_xml(&root.children);
            net.children = children;
            net.child_nodes = nodes;
        }
        net.build_links();
        net
    }

    /// The state of the children as the text format writes it (each escaped, with a leading
    /// space).
    pub fn dump_children(&self) -> String {
        let mut s = String::new();
        for c in &self.children {
            let d = c.dump();
            let rest: Vec<&str> = d.split(' ').skip(5).collect();
            s.push(' ');
            s += &escape(&rest.join(" "));
        }
        s
    }

    /// Writes the children that have state as `ix`-tagged tags.
    pub fn dump_state(&self, out: &mut XmlOut) {
        for (i, c) in self.children.iter().enumerate() {
            let mut t = XmlOut::new(&c.xml_dump_type());
            c.dump_xml_state(&mut t);
            if t.attrs.is_empty() && t.children.is_empty() {
                continue;
            }
            t.int_attr("ix", i as i32);
            out.child(t);
        }
    }

    /// Restores the state written by [`ChildNet::dump_state`].
    pub fn undump_state(&mut self, xml: &XmlNode) {
        for t in &xml.children {
            let ix = t.int_attr("ix", -1);
            if let Some(c) = usize::try_from(ix).ok().and_then(|i| self.children.get_mut(i)) {
                c.undump_xml(t);
            }
        }
    }

    /// Number of posts of the composite.
    pub fn post_total(&self) -> usize {
        self.ext_nodes.len()
    }

    /// Number of internal nodes (those that are not posts).
    pub fn internal_nodes(&self) -> usize {
        self.node_links.len().saturating_sub(self.post_total())
    }

    /// Sets the high logic voltage of every child.
    pub fn set_high_voltage(&mut self, hv: f64) {
        if hv == 0.0 {
            return;
        }
        for c in &mut self.children {
            c.set_high_voltage(hv);
        }
    }

    /// Works out which child posts sit on which node of the composite.
    pub fn build_links(&mut self) {
        // a ground part makes its node the real ground
        let grounds: Vec<i32> = self
            .children
            .iter()
            .zip(&self.child_nodes)
            .filter(|(c, _)| c.class_name() == "GroundElm")
            .filter_map(|(_, n)| n.first().copied())
            .filter(|n| *n != 0)
            .collect();
        for nodes in &mut self.child_nodes {
            for n in nodes.iter_mut() {
                if grounds.contains(n) {
                    *n = 0;
                }
            }
        }
        let mut by_node: BTreeMap<i32, Vec<(usize, usize)>> = BTreeMap::new();
        let mut grounded_posts = Vec::new();
        for (i, child) in self.children.iter_mut().enumerate() {
            child.pre_stamp();
            child.alloc_nodes();
            for post in 0..child.post_count() {
                let Some(&n) = self.child_nodes[i].get(post) else { continue };
                if n == 0 {
                    child.set_node(post, NodeId::GROUND);
                    child.set_node_voltage(post, 0.0);
                    grounded_posts.push((i, post));
                } else {
                    by_node.entry(n).or_default().push((i, post));
                }
            }
        }
        let mut links: Vec<Vec<(usize, usize)>> = Vec::new();
        let mut used = HashSet::new();
        for n in &self.ext_nodes {
            links.push(by_node.get(n).cloned().unwrap_or_default());
            used.insert(*n);
        }
        for (n, l) in &by_node {
            if !used.contains(n) {
                links.push(l.clone());
            }
        }
        for (i, child) in self.children.iter().enumerate() {
            for j in 0..child.internal_node_count() {
                links.push(vec![(i, child.post_count() + j)]);
            }
        }
        self.node_links = links;
        self.classify(&grounded_posts);
    }

    /// Computes which nodes of the composite are joined through its children.
    fn classify(&mut self, grounded_posts: &[(usize, usize)]) {
        let n = self.node_links.len();
        let ground = n;
        let mut node_of: HashMap<(usize, usize), usize> = HashMap::new();
        for (idx, links) in self.node_links.iter().enumerate() {
            for &l in links {
                node_of.insert(l, idx);
            }
        }
        for &g in grounded_posts {
            node_of.insert(g, ground);
        }
        let mut conn = Classes::new(n + 1);
        let mut mconn = Classes::new(n + 1);
        let mut anchors: Vec<usize> = Vec::new();
        for (i, child) in self.children.iter().enumerate() {
            let posts = child.post_count();
            for a in 0..posts {
                let Some(&na) = node_of.get(&(i, a)) else { continue };
                if child.has_ground_connection(a) {
                    anchors.push(na);
                }
                for b in 0..posts {
                    if a != b && child.get_connection(a, b) {
                        if let Some(&nb) = node_of.get(&(i, b)) {
                            conn.union(na, nb);
                        }
                    }
                }
            }
            for a in 0..child.node_count() {
                let Some(&na) = node_of.get(&(i, a)) else { continue };
                for b in 0..child.node_count() {
                    if a != b && child.get_matrix_connection(a, b) {
                        if let Some(&nb) = node_of.get(&(i, b)) {
                            mconn.union(na, nb);
                        }
                    }
                }
            }
        }
        let mut roots: HashSet<usize> = anchors.iter().map(|&a| conn.find(a)).collect();
        roots.insert(conn.find(ground));
        self.grounded = (0..n).map(|k| roots.contains(&conn.find(k))).collect();
        self.connected = conn;
        self.matrix_connected = mconn;
    }

    fn owner_of_source(&self, j: usize) -> Option<(usize, usize)> {
        let mut rest = j;
        for (i, c) in self.children.iter().enumerate() {
            let n = c.voltage_source_count();
            if rest < n {
                return Some((i, rest));
            }
            rest -= n;
        }
        None
    }

    /// Number of voltage sources of all children.
    pub fn voltage_source_count(&self) -> usize {
        self.children.iter().map(|c| c.voltage_source_count()).sum()
    }
    /// True if any child is nonlinear.
    pub fn non_linear(&self) -> bool {
        self.children.iter().any(|c| c.non_linear())
    }
    /// Tells the children on node `p` which solver node it is.
    pub fn set_node(&mut self, p: usize, n: NodeId) {
        if let Some(links) = self.node_links.get(p) {
            for &(c, cp) in links {
                self.children[c].set_node(cp, n);
            }
        }
    }
    /// Passes the solved voltage of node `n` to the children on it.
    pub fn set_node_voltage(&mut self, n: usize, v: f64) {
        if let Some(links) = self.node_links.get(n) {
            for &(c, cp) in links {
                self.children[c].set_node_voltage(cp, v);
            }
        }
    }
    /// Hands the `n`th voltage source of the composite to the child that owns it.
    pub fn set_voltage_source(&mut self, n: usize, v: VsId) {
        if n == 0 {
            self.vs_owner.clear();
        }
        if let Some((c, local)) = self.owner_of_source(n) {
            self.children[c].set_voltage_source(local, v);
            self.vs_owner.push((v, c));
        }
    }
    /// Nodes the `j`th source of the composite connects.
    pub fn voltage_source_nodes_at(&self, j: usize) -> Option<(NodeId, NodeId)> {
        let (c, local) = self.owner_of_source(j)?;
        let child = &self.children[c];
        if let Some(declared) = child.voltage_source_nodes_at(local) {
            return Some(declared);
        }
        // like the solver does for a top-level element: the child's last post, then the
        // others, then its internal nodes decide which matrix the source lives in
        let nodes = &child.base().nodes;
        let posts = child.post_count().min(nodes.len());
        let mut it = nodes[..posts]
            .iter()
            .rev()
            .chain(nodes[posts..].iter())
            .copied()
            .filter(|n| *n != NodeId::GROUND);
        let first = it.next()?;
        Some((first, it.next().unwrap_or(NodeId::GROUND)))
    }
    /// Passes the current of voltage source `vs` to its owner.
    pub fn set_current(&mut self, vs: VsId, c: f64) {
        if let Some(&(_, child)) = self.vs_owner.iter().find(|(v, _)| *v == vs) {
            self.children[child].set_current(vs, c);
        }
    }
    /// Current into node `n` of the composite.
    pub fn current_into_node(&self, n: usize) -> f64 {
        self.node_links.get(n).map_or(0.0, |links| {
            links.iter().map(|&(c, cp)| self.children[c].get_current_into_node(cp)).sum()
        })
    }
    /// Power of all children.
    pub fn power(&self) -> f64 {
        self.children.iter().map(|c| c.get_power()).sum()
    }
    /// Are nodes `n1` and `n2` connected through the children?
    pub fn connection(&self, n1: usize, n2: usize) -> bool {
        let n = self.node_links.len();
        n1 < n && n2 < n && self.connected.root(n1) == self.connected.root(n2)
    }
    /// Are nodes `n1` and `n2` in the same matrix?
    pub fn matrix_connection(&self, n1: usize, n2: usize) -> bool {
        let n = self.node_links.len();
        n1 < n && n2 < n && self.matrix_connected.root(n1) == self.matrix_connected.root(n2)
    }
    /// Is node `n1` tied to ground through the children?
    pub fn ground_connection(&self, n1: usize) -> bool {
        self.grounded.get(n1).copied().unwrap_or(false)
    }
    /// Validates every child.
    pub fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        let mut ok = true;
        for (k, c) in self.children.iter_mut().enumerate() {
            let part = v.part_index(idx, k);
            ok &= c.validate(part, v);
        }
        ok
    }
    /// Finds, for every child that senses currents "spice style", the voltage source elements
    /// across its input pairs.
    fn resolve_spice(&mut self) {
        self.spice_links.clear();
        for i in 0..self.children.len() {
            let pairs = self.children[i].spice_pairs();
            let mut links = Vec::new();
            for k in 0..pairs {
                let nodes = &self.children[i].base().nodes;
                let (a, b) = (nodes[k * 2], nodes[k * 2 + 1]);
                let found = self.children.iter().position(|o| {
                    o.is_voltage_elm()
                        && o.base().nodes.len() >= 2
                        && o.base().nodes[0] == a
                        && o.base().nodes[1] == b
                });
                links.push(found);
            }
            for (k, f) in links.iter().enumerate() {
                if let Some(vs) = f.and_then(|j| self.children[j].base().volt_source) {
                    self.children[i].set_spice_source(k, vs);
                }
            }
            if pairs > 0 {
                self.spice_links.push((i, links));
            }
        }
    }

    /// Stamps every child.
    pub fn stamp(&mut self, s: &mut Stamper) {
        self.resolve_spice();
        for c in &mut self.children {
            c.stamp(s);
        }
    }
    /// Starts an iteration in every child.
    pub fn start_iteration(&mut self, s: &Stamper) {
        for c in &mut self.children {
            c.start_iteration(s);
        }
    }
    /// Runs one nonlinear step in every child.
    pub fn do_step(&mut self, s: &mut Stamper) {
        for (i, links) in &self.spice_links {
            for (k, f) in links.iter().enumerate() {
                if let Some(j) = f {
                    let cur = self.children[*j].get_current();
                    self.children[*i].set_spice_current(k, cur);
                }
            }
        }
        for c in &mut self.children {
            c.do_step(s);
        }
    }
    /// Finishes the time step in every child.
    pub fn step_finished(&mut self, s: &mut Stamper) {
        for c in &mut self.children {
            c.step_finished(s);
        }
    }
    /// The drawing state of the children, each preceded by its length.
    pub fn visual_state(&self) -> Vec<f64> {
        let mut out = Vec::new();
        for c in &self.children {
            let st = c.visual_state();
            out.push(st.len() as f64);
            out.extend(st);
        }
        out
    }

    /// Hands the state written by [`ChildNet::visual_state`] back to the children.
    pub fn set_visual_state(&mut self, state: &[f64]) {
        let mut rest = state;
        for c in &mut self.children {
            let Some((&len, tail)) = rest.split_first() else { return };
            let len = len as usize;
            if tail.len() < len {
                return;
            }
            let (mine, more) = tail.split_at(len);
            if !mine.is_empty() {
                c.set_visual_state(mine);
            }
            rest = more;
        }
    }

    /// Resets every child.
    pub fn reset(&mut self) {
        for c in &mut self.children {
            c.reset();
        }
    }
}

/// Defines the [`Element`] methods that only forward to the field `net` (a [`ChildNet`]).
/// The element must have fields `base` and `net`.
#[macro_export]
macro_rules! forward_to_net {
    // `forward_to_net!(record_current)` also keeps the current into post 1 as the element's
    // current, taken once per step before the parts finish it (the original lists the parts
    // after the element)
    ($($rec:ident)?) => {
        fn internal_node_count(&self) -> usize {
            self.net.internal_nodes()
        }
        fn child_elements(&self) -> &[Box<dyn $crate::elements::element::Element>] {
            &self.net.children
        }
        fn child_elements_mut(&mut self) -> &mut [Box<dyn $crate::elements::element::Element>] {
            &mut self.net.children
        }
        fn voltage_source_count(&self) -> usize {
            self.net.voltage_source_count()
        }
        fn non_linear(&self) -> bool {
            self.net.non_linear()
        }
        fn pre_stamp(&mut self) {
            self.net.build_links();
        }
        fn set_node(&mut self, p: usize, n: $crate::solver::stamper::NodeId) {
            self.base.nodes[p] = n;
            self.net.set_node(p, n);
        }
        fn set_node_voltage(&mut self, n: usize, v: f64) {
            self.base.volts[n] = v;
            self.net.set_node_voltage(n, v);
        }
        fn set_voltage_source(&mut self, n: usize, v: $crate::solver::stamper::VsId) {
            self.net.set_voltage_source(n, v);
        }
        fn voltage_source_nodes_at(
            &self,
            j: usize,
        ) -> Option<($crate::solver::stamper::NodeId, $crate::solver::stamper::NodeId)> {
            self.net.voltage_source_nodes_at(j)
        }
        fn set_current(&mut self, vs: $crate::solver::stamper::VsId, c: f64) {
            self.net.set_current(vs, c);
        }
        fn get_current_into_node(&self, n: usize) -> f64 {
            self.net.current_into_node(n)
        }
        fn get_power(&self) -> f64 {
            self.net.power()
        }
        fn get_connection(&self, n1: usize, n2: usize) -> bool {
            self.net.connection(n1, n2)
        }
        fn get_matrix_connection(&self, n1: usize, n2: usize) -> bool {
            self.net.matrix_connection(n1, n2)
        }
        fn has_ground_connection(&self, n1: usize) -> bool {
            self.net.ground_connection(n1)
        }
        fn validate(&mut self, idx: usize, v: &mut $crate::solver::path::Validator<'_>) -> bool {
            self.net.validate(idx, v)
        }
        fn stamp(&mut self, s: &mut $crate::solver::stamper::Stamper) {
            self.net.stamp(s);
        }
        fn start_iteration(&mut self, s: &$crate::solver::stamper::Stamper) {
            self.net.start_iteration(s);
        }
        fn do_step(&mut self, s: &mut $crate::solver::stamper::Stamper) {
            self.net.do_step(s);
        }
        fn visual_state(&self) -> Vec<f64> {
            self.net.visual_state()
        }
        fn set_visual_state(&mut self, state: &[f64]) {
            // the interface copy gets the node voltages in one piece: give them to the parts
            // so that the parts that are drawn show them
            for n in 0..self.base.volts.len() {
                let v = self.base.volts[n];
                self.net.set_node_voltage(n, v);
            }
            self.net.set_visual_state(state);
        }
        fn step_finished(&mut self, s: &mut $crate::solver::stamper::Stamper) {
            $(
                let _ = stringify!($rec);
                self.base.current = self.net.current_into_node(1);
            )?
            self.net.step_finished(s);
        }
    };
}

// ---------------------------------------------------------------------------------------
// Darlington pair
// ---------------------------------------------------------------------------------------

const DARLINGTON_MODEL: &str = "NTransistorElm 1 2 4\nNTransistorElm 4 2 3";
const DARLINGTON_NODES: [i32; 3] = [1, 2, 3];

/// A Darlington pair: two transistors in one package.
pub struct Darlington {
    base: Base,
    net: ChildNet,
    /// `1` for NPN, `-1` for PNP.
    pnp: i32,
    coll: [Point; 2],
    coll2: [Point; 2],
    emit: [Point; 2],
    base_pt: Point,
    rect_poly: Polygon,
    arrow_poly: Polygon,
    curcount_c: f64,
    curcount_e: f64,
    curcount_b: f64,
}

impl Darlington {
    fn build(base: Base, pnp: i32, st: Option<&mut Tokenizer<'_>>) -> Self {
        let escaped = base.flags & FLAG_ESCAPE != 0;
        let model = if probe(&base, &st) { "" } else { DARLINGTON_MODEL };
        let net = ChildNet::from_model_string(model, &DARLINGTON_NODES, st, escaped);
        let mut d = Darlington {
            base,
            net,
            pnp,
            coll: [Point::default(); 2],
            coll2: [Point::default(); 2],
            emit: [Point::default(); 2],
            base_pt: Point::default(),
            rect_poly: Polygon::default(),
            arrow_poly: Polygon::default(),
            curcount_c: 0.0,
            curcount_e: 0.0,
            curcount_b: 0.0,
        };
        d.base.flags |= FLAG_ESCAPE;
        d.base.no_diagonal = true;
        d.apply_polarity();
        d
    }

    /// A pair being placed (`pnp` selects the PNP version).
    pub fn new(pnp: bool, x: i32, y: i32) -> Self {
        Self::build(Base::new(x, y), if pnp { -1 } else { 1 }, None)
    }

    /// A pair read from a text line.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        // the children come first in the line and the polarity last, so read the children
        // into a scratch token list and then the polarity
        let mut tokens: Vec<&str> = Vec::new();
        while let Some(t) = st.next_token() {
            tokens.push(t);
        }
        let pnp = tokens.pop().and_then(|t| t.parse().ok()).unwrap_or(1);
        let joined = tokens.join(" ");
        let mut inner = Tokenizer::new(&joined);
        Self::build(Base::from_dump(xa, ya, xb, yb, f), pnp, Some(&mut inner))
    }

    fn apply_polarity(&mut self) {
        #[cfg(feature = "transistors")]
        for c in &mut self.net.children {
            if let Some(t) = c.as_any_mut().downcast_mut::<super::transistors::TransistorElm>() {
                t.set_pnp(self.pnp);
            }
        }
    }
}

impl Element for Darlington {
    impl_element_base!("DarlingtonElm");
    forward_to_net!();

    fn dump_type(&self) -> i32 {
        400
    }
    fn xml_dump_type(&self) -> String {
        "dar".into()
    }
    fn dump(&self) -> String {
        format!("{}{} {}", self.base.dump_prefix(400), self.net.dump_children(), self.pnp)
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("pnp", self.pnp);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        self.net.dump_state(out);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.pnp = xml.int_attr("pnp", self.pnp);
        if self.net.children.is_empty() {
            self.net = ChildNet::from_model_string(DARLINGTON_MODEL, &DARLINGTON_NODES, None, true);
        }
        self.apply_polarity();
        self.net.undump_state(xml);
        self.set_points();
    }

    fn post_count(&self) -> usize {
        3
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.base.point1,
            1 => self.coll[0],
            _ => self.emit[0],
        })
    }
    fn can_flip_y(&self) -> bool {
        false
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let hs = 16;
        let hs2 = hs * self.base.dsign * self.pnp;
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let ds = self.base.dsign * self.pnp;
        let (c0, e0) = interp_point2(p1, p2, 1.0, f64::from(hs2));
        self.coll[0] = c0;
        self.emit[0] = e0;
        self.coll2[0] = interp_point_g(p1, p2, 1.0, f64::from(hs2 - 5 * ds));
        let (r0, r1) = interp_point2(p1, p2, 1.0 - 16.0 / dn, f64::from(hs));
        let (r2, r3) = interp_point2(p1, p2, 1.0 - 13.0 / dn, f64::from(hs));
        let (c1, e1) = interp_point2(p1, p2, 1.0 - 13.0 / dn, f64::from(6 * ds));
        self.coll[1] = c1;
        self.emit[1] = e1;
        self.coll2[1] = interp_point_g(p1, p2, 1.0 - 13.0 / dn, f64::from(ds));
        self.base_pt = interp_point(p1, p2, 1.0 - 16.0 / dn);
        self.rect_poly = Polygon::from_points(&[r0, r2, r3, r1]);
        self.arrow_poly = if self.pnp == 1 {
            calc_arrow(self.emit[1], self.emit[0], 8.0, 4.0)
        } else {
            let pt = interp_point_g(p1, p2, 1.0 - 11.0 / dn, f64::from(-5 * ds));
            calc_arrow(self.emit[0], pt, 8.0, 4.0)
        };
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
        self.curcount_c = 0.0;
        self.curcount_e = 0.0;
        self.curcount_b = 0.0;
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, 16.0);
        let power = self.get_power();
        let v = self.base.volts.clone();
        set_power_color_w(g, &self.base, c, power);
        set_voltage_color(g, &self.base, c, v[1]);
        draw_thick_line(g, self.coll[0], self.coll[1]);
        draw_thick_line(g, self.coll2[0], self.coll2[1]);
        draw_thick_line(g, self.coll[0], self.coll2[0]);
        set_voltage_color(g, &self.base, c, v[2]);
        draw_thick_line(g, self.emit[0], self.emit[1]);
        g.set_color(c.light_gray);
        fill_polygon(g, &self.arrow_poly);
        set_voltage_color(g, &self.base, c, v[0]);
        if c.show_power {
            g.set_color(Color::GRAY);
        }
        draw_thick_line(g, p1, self.base_pt);
        let ib = self.get_current_into_node(0);
        self.curcount_b = self.base.update_dot_count_for(ib, self.curcount_b, c);
        draw_dots(g, c, self.base_pt, p1, self.curcount_b);
        let ic = self.get_current_into_node(1);
        self.curcount_c = self.base.update_dot_count_for(ic, self.curcount_c, c);
        draw_dots(g, c, self.coll[1], self.coll[0], self.curcount_c);
        let ie = self.get_current_into_node(2);
        self.curcount_e = self.base.update_dot_count_for(ie, self.curcount_e, c);
        draw_dots(g, c, self.emit[1], self.emit[0], self.curcount_e);
        set_voltage_color(g, &self.base, c, v[0]);
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.rect_poly);
        if (self.base.needs_highlight() || self.base.creating) && self.base.dy == 0 {
            g.set_color(c.white);
            let ds = sign(self.base.dx);
            g.draw_string("B", f64::from(self.base_pt.x - 10 * ds), f64::from(self.base_pt.y - 5));
            g.draw_string(
                "C",
                f64::from(self.coll[0].x - 3 + 9 * ds),
                f64::from(self.coll[0].y + 4),
            );
            g.draw_string(
                "E",
                f64::from(self.emit[0].x - 3 + 9 * ds),
                f64::from(self.emit[0].y + 4),
            );
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let v = &self.base.volts;
        arr.push(format!("darlington pair ({})", if self.pnp == -1 { "PNP" } else { "NPN" }));
        arr.push(format!("Ic = {}", current_text(-self.get_current_into_node(1))));
        arr.push(format!("Ib = {}", current_text(-self.get_current_into_node(0))));
        arr.push(format!("Vbe = {}", super::units::voltage_text(v[0] - v[2])));
        arr.push(format!("Vbc = {}", super::units::voltage_text(v[0] - v[1])));
        arr.push(format!("Vce = {}", super::units::voltage_text(v[1] - v[2])));
    }
}

// ---------------------------------------------------------------------------------------
// OTA (LM13700 style)
// ---------------------------------------------------------------------------------------

const OTA_MODEL: &str = "RailElm 4\nRailElm 10\nNTransistorElm 1 2 3\nNTransistorElm 3 1 4\n\
NTransistorElm 3 3 4\nNTransistorElm 5 6 2\nNTransistorElm 7 8 2\nPTransistorElm 9 6 10\n\
PTransistorElm 9 9 10\nPTransistorElm 6 12 9\nPTransistorElm 11 8 10\nPTransistorElm 11 11 10\n\
PTransistorElm 8 13 11\nNTransistorElm 14 14 4\nNTransistorElm 14 12 4\n\
NTransistorElm 12 13 14\nNTransistorElm 15 15 5\nNTransistorElm 15 15 7";
const OTA_NODES: [i32; 5] = [7, 5, 15, 1, 13];
const OP_HEIGHT: i32 = 32;
const OP_WIDTH: i32 = 32;
const CIRC_DIAM: i32 = 19;
const CIRC_OVERLAP: i32 = 8;

/// An operational transconductance amplifier (LM13700 style).
pub struct Ota {
    base: Base,
    net: ChildNet,
    pos_volt: f64,
    neg_volt: f64,
    in_lines: [[Point; 2]; 4],
    text_p: [Point; 2],
    bar1: [Point; 2],
    bar2: [Point; 2],
    circ_cent: [Point; 2],
    point2_bis: Point,
    triangle: Polygon,
    arrow1: Polygon,
    arrow2: Polygon,
    curcount: [f64; 4],
}

impl Ota {
    fn build(base: Base, st: Option<&mut Tokenizer<'_>>) -> Self {
        let escaped = base.flags & FLAG_ESCAPE != 0;
        let model = if probe(&base, &st) { "" } else { OTA_MODEL };
        let net = ChildNet::from_model_string(model, &OTA_NODES, st, escaped);
        let mut o = Ota {
            base,
            net,
            pos_volt: 9.0,
            neg_volt: -9.0,
            in_lines: [[Point::default(); 2]; 4],
            text_p: [Point::default(); 2],
            bar1: [Point::default(); 2],
            bar2: [Point::default(); 2],
            circ_cent: [Point::default(); 2],
            point2_bis: Point::default(),
            triangle: Polygon::default(),
            arrow1: Polygon::default(),
            arrow2: Polygon::default(),
            curcount: [0.0; 4],
        };
        o.base.flags |= FLAG_ESCAPE;
        o.base.no_diagonal = true;
        o
    }

    /// An OTA being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut o = Self::build(Base::new(x, y), None);
        o.init_supplies();
        o
    }

    /// An OTA read from a text line; the supplies come from the rails inside.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        // only the sources family gives the OTA rails to read its supplies from
        #[cfg_attr(not(feature = "sources"), allow(unused_mut))]
        let mut o = Self::build(Base::from_dump(xa, ya, xb, yb, f), Some(st));
        #[cfg(feature = "sources")]
        {
            let volts = |net: &ChildNet, i: usize| {
                net.children
                    .get(i)
                    .and_then(|c| c.as_any().downcast_ref::<super::sources::RailElm>())
                    .map(|r| r.wave.max_voltage)
            };
            if let Some(v) = volts(&o.net, 0) {
                o.neg_volt = v;
            }
            if let Some(v) = volts(&o.net, 1) {
                o.pos_volt = v;
            }
        }
        o
    }

    fn init_supplies(&mut self) {
        #[cfg(feature = "sources")]
        for (i, v) in [self.neg_volt, self.pos_volt].into_iter().enumerate() {
            if let Some(r) = self
                .net
                .children
                .get_mut(i)
                .and_then(|c| c.as_any_mut().downcast_mut::<super::sources::RailElm>())
            {
                r.wave.max_voltage = v;
            }
        }
    }
}

impl Element for Ota {
    impl_element_base!("OTAElm");
    forward_to_net!();

    fn dump_type(&self) -> i32 {
        402
    }
    fn xml_dump_type(&self) -> String {
        "OTA".into()
    }
    fn dump(&self) -> String {
        format!("{}{}", self.base.dump_prefix(402), self.net.dump_children())
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("pv", self.pos_volt);
        out.double_attr("nv", self.neg_volt);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        self.net.dump_state(out);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.pos_volt = xml.double_attr("pv", self.pos_volt);
        self.neg_volt = xml.double_attr("nv", self.neg_volt);
        if self.net.children.is_empty() {
            self.net = ChildNet::from_model_string(OTA_MODEL, &OTA_NODES, None, true);
        }
        self.init_supplies();
        self.net.undump_state(xml);
    }

    fn post_count(&self) -> usize {
        5
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0..=3 => self.in_lines[n][0],
            _ => self.point2_bis,
        })
    }
    fn can_flip_x(&self) -> bool {
        false
    }
    fn can_flip_y(&self) -> bool {
        false
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &self.base;
        let (p1, p2, dn, dsign) = (b.point1, b.point2, b.dn, b.dsign);
        let ww = OP_WIDTH;
        let wtot = f64::from(ww * 2 + 2 * CIRC_DIAM - CIRC_OVERLAP);
        let (lead1, lead2);
        if dn > wtot {
            lead1 = interp_point(p1, p2, 1.0 - wtot / dn);
            lead2 = p2;
            self.point2_bis = p2;
        } else {
            lead1 = p1;
            lead2 = interp_point(p1, p2, wtot / dn);
            self.point2_bis = lead2;
        }
        self.base.lead1 = lead1;
        self.base.lead2 = lead2;
        let hs = f64::from(OP_HEIGHT * dsign);
        let (a0, b0) = interp_point2(p1, self.point2_bis, 0.0, hs);
        let (a1, b1) = interp_point2(lead1, lead2, 0.0, hs);
        let (t0, t1) = interp_point2(lead1, lead2, 0.1, hs);
        self.in_lines[0] = [a0, a1];
        self.in_lines[1] = [b0, b1];
        self.in_lines[2] = [p1, lead1];
        self.in_lines[3] = [
            interp_point_g(lead1, lead2, 1.0 - 16.0 / wtot, 32.0),
            interp_point_g(lead1, lead2, 1.0 - 16.0 / wtot, 8.0),
        ];
        self.text_p = [t0, t1];
        let (tr0, tr1) = interp_point2(lead1, lead2, 0.0, 3.0 * hs / 2.0);
        let tr2 = interp_point(lead1, lead2, f64::from(2 * ww) / wtot);
        self.triangle = Polygon::from_points(&[tr0, tr1, tr2]);
        self.circ_cent = [
            interp_point(lead1, lead2, 1.0 - f64::from(CIRC_DIAM) / (2.0 * wtot)),
            interp_point(
                lead1,
                lead2,
                1.0 - (1.5 * f64::from(CIRC_DIAM) - f64::from(CIRC_OVERLAP)) / wtot,
            ),
        ];
        let arrow = |target: Point| {
            let d1 = interp_point(lead1, target, 0.3333);
            let d2 = interp_point(lead1, target, 0.6666);
            let (x, y) = interp_point2(d1, d2, 1.0, 4.0);
            (calc_arrow(d1, d2, 8.0, 4.0), [x, y])
        };
        let (a1p, bar1) = arrow(self.in_lines[0][1]);
        let (a2p, bar2) = arrow(self.in_lines[1][1]);
        self.arrow1 = a1p;
        self.arrow2 = a2p;
        self.bar1 = bar1;
        self.bar2 = bar2;
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
        self.curcount = [0.0; 4];
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(3 * OP_HEIGHT / 2));
        let v = self.base.volts.clone();
        for i in 0..4 {
            set_voltage_color(g, &self.base, c, v[i]);
            draw_thick_line(g, self.in_lines[i][0], self.in_lines[i][1]);
        }
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        let power = self.get_power();
        set_power_color_w(g, &self.base, c, power);
        draw_thick_polygon(g, &self.triangle);
        fill_polygon(g, &self.arrow1);
        fill_polygon(g, &self.arrow2);
        draw_thick_line(g, self.bar1[0], self.bar1[1]);
        draw_thick_line(g, self.bar2[0], self.bar2[1]);
        for cc in &self.circ_cent {
            draw_thick_circle(g, cc.x, cc.y, CIRC_DIAM / 2);
        }
        g.set_font_size(14);
        let (tp0, tp1) = (self.text_p[0], self.text_p[1]);
        draw_centered_text(g, &mut self.base, "+", tp0.x, tp0.y - 2, true);
        draw_centered_text(g, &mut self.base, "-", tp1.x, tp1.y, true);
        for i in 0..4 {
            let cur = -self.get_current_into_node(i);
            self.curcount[i] = self.base.update_dot_count_for(cur, self.curcount[i], c);
            draw_dots(g, c, self.in_lines[i][0], self.in_lines[i][1], self.curcount[i]);
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("OTA (LM13700 style)".into());
        arr.push(format!("Iabc = {}", current_text(-self.get_current_into_node(3))));
        let v = &self.base.volts;
        arr.push(format!("V+ - V- = {}", super::units::voltage_text(v[0] - v[1])));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("Positive Supply Voltage (5-20V)", self.pos_volt, 5.0, 20.0)
                    .set_unit_step(),
            ),
            1 => Some(
                EditInfo::new("Negative Supply Voltage (V)", self.neg_volt, -20.0, -5.0)
                    .set_unit_step(),
            ),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.pos_volt = ei.value,
            1 => self.neg_volt = ei.value,
            _ => {}
        }
        self.init_supplies();
    }
}

/// Registers the composite elements with a fixed internal circuit.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "CrystalElm",
            FAMILY,
            "Add Crystal",
            "Passive Components",
            |x, y| Box::new(Crystal::new(x, y)),
        )
        .dump(412, |a, b, c, d, f, st| Ok(Box::new(Crystal::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new(
            "UnijunctionElm",
            FAMILY,
            "Add Unijunction Transistor",
            "Active Components",
            |x, y| Box::new(Unijunction::new(x, y)),
        )
        .dump(417, |a, b, c, d, f, _| Ok(Box::new(Unijunction::from_dump(a, b, c, d, f)))),
    );
    r.add(
        ElementDescriptor::new(
            "ComparatorElm",
            FAMILY,
            "Add Comparator (Hi-Z/GND output)",
            "Active Components",
            |x, y| Box::new(Comparator::new(x, y)),
        )
        .dump(401, |a, b, c, d, f, st| Ok(Box::new(Comparator::from_dump(a, b, c, d, f, st)))),
    );
    r.add(ElementDescriptor::new(
        "NortonAmpElm",
        FAMILY,
        "Add Norton Amplifier",
        "Active Components",
        |x, y| Box::new(NortonAmp::new(x, y)),
    ));
    const CAT: &str = "Active Components";
    r.add(
        ElementDescriptor::new(
            "NDarlingtonElm",
            FAMILY,
            "Add Darlington Pair (NPN)",
            CAT,
            |x, y| Box::new(Darlington::new(false, x, y)),
        )
        .dump(400, |a, b, c, d, f, st| Ok(Box::new(Darlington::from_dump(a, b, c, d, f, st)))),
    );
    r.add(ElementDescriptor::new(
        "PDarlingtonElm",
        FAMILY,
        "Add Darlington Pair (PNP)",
        CAT,
        |x, y| Box::new(Darlington::new(true, x, y)),
    ));
    r.add(
        ElementDescriptor::new("OTAElm", FAMILY, "Add OTA (LM13700 style)", CAT, |x, y| {
            Box::new(Ota::new(x, y))
        })
        .dump(402, |a, b, c, d, f, st| Ok(Box::new(Ota::from_dump(a, b, c, d, f, st)))),
    );
}

// ---------------------------------------------------------------------------------------
// Crystal
// ---------------------------------------------------------------------------------------

const CRYSTAL_MODEL: &str = "CapacitorElm 1 2\nCapacitorElm 1 3\nInductorElm 3 4\nResistorElm 4 2";
const CRYSTAL_NODES: [i32; 2] = [1, 2];
const FLAG_SHOW_FREQ: i32 = 2;

/// A quartz crystal: a series RLC branch in parallel with a capacitor.
pub struct Crystal {
    base: Base,
    net: ChildNet,
    series_capacitance: f64,
    parallel_capacitance: f64,
    inductance: f64,
    resistance: f64,
    plate1: [Point; 2],
    plate2: [Point; 2],
    sandwich: [Point; 4],
}

impl Crystal {
    fn build(base: Base, st: Option<&mut Tokenizer<'_>>) -> Self {
        let escaped = base.flags & FLAG_ESCAPE != 0;
        let model = if probe(&base, &st) { "" } else { CRYSTAL_MODEL };
        let net = ChildNet::from_model_string(model, &CRYSTAL_NODES, st, escaped);
        let mut c = Crystal {
            base,
            net,
            series_capacitance: 0.1e-12,
            parallel_capacitance: 28.7e-12,
            inductance: 2.5e-3,
            resistance: 6.4,
            plate1: [Point::default(); 2],
            plate2: [Point::default(); 2],
            sandwich: [Point::default(); 4],
        };
        c.base.flags |= FLAG_ESCAPE;
        c
    }

    /// A crystal being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut c = Self::build(Base::new(x, y), None);
        c.base.flags = FLAG_SHOW_FREQ | FLAG_ESCAPE;
        c.push_values();
        c
    }

    /// A crystal read from a text line; the values come from the children.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut c = Self::build(Base::from_dump(xa, ya, xb, yb, f), Some(st));
        c.pull_values();
        c
    }

    #[cfg(all(feature = "reactive", feature = "passive"))]
    fn push_values(&mut self) {
        use super::passive::ResistorElm;
        use super::reactive::{CapacitorElm, InductorElm};
        let ch = &mut self.net.children;
        if let Some(c) = ch.get_mut(0).and_then(|c| c.as_any_mut().downcast_mut::<CapacitorElm>()) {
            c.capacitance = self.parallel_capacitance;
        }
        if let Some(c) = ch.get_mut(1).and_then(|c| c.as_any_mut().downcast_mut::<CapacitorElm>()) {
            c.capacitance = self.series_capacitance;
        }
        if let Some(l) = ch.get_mut(2).and_then(|c| c.as_any_mut().downcast_mut::<InductorElm>()) {
            l.inductance = self.inductance;
        }
        if let Some(r) = ch.get_mut(3).and_then(|c| c.as_any_mut().downcast_mut::<ResistorElm>()) {
            r.resistance = self.resistance;
        }
    }
    #[cfg(not(all(feature = "reactive", feature = "passive")))]
    fn push_values(&mut self) {}

    #[cfg(all(feature = "reactive", feature = "passive"))]
    fn pull_values(&mut self) {
        use super::passive::ResistorElm;
        use super::reactive::{CapacitorElm, InductorElm};
        let ch = &self.net.children;
        if let Some(c) = ch.first().and_then(|c| c.as_any().downcast_ref::<CapacitorElm>()) {
            self.parallel_capacitance = c.capacitance;
        }
        if let Some(c) = ch.get(1).and_then(|c| c.as_any().downcast_ref::<CapacitorElm>()) {
            self.series_capacitance = c.capacitance;
        }
        if let Some(l) = ch.get(2).and_then(|c| c.as_any().downcast_ref::<InductorElm>()) {
            self.inductance = l.inductance;
        }
        if let Some(r) = ch.get(3).and_then(|c| c.as_any().downcast_ref::<ResistorElm>()) {
            self.resistance = r.resistance;
        }
    }
    #[cfg(not(all(feature = "reactive", feature = "passive")))]
    fn pull_values(&mut self) {}

    fn series_frequency(&self) -> f64 {
        1.0 / ((self.inductance * self.series_capacitance).sqrt() * std::f64::consts::TAU)
    }
}

impl Element for Crystal {
    impl_element_base!("CrystalElm");
    forward_to_net!(record_current);

    fn dump_type(&self) -> i32 {
        412
    }
    fn xml_dump_type(&self) -> String {
        "cr".into()
    }
    fn dump(&self) -> String {
        format!("{}{}", self.base.dump_prefix(412), self.net.dump_children())
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("pc", self.parallel_capacitance);
        out.double_attr("sc", self.series_capacitance);
        out.double_attr("in", self.inductance);
        out.double_attr("r", self.resistance);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        self.net.dump_state(out);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.resistance = xml.double_attr("r", self.resistance);
        self.inductance = xml.double_attr("in", self.inductance);
        self.parallel_capacitance = xml.double_attr("pc", self.parallel_capacitance);
        self.series_capacitance = xml.double_attr("sc", self.series_capacitance);
        if self.net.children.is_empty() {
            self.net = ChildNet::from_model_string(CRYSTAL_MODEL, &CRYSTAL_NODES, None, true);
        }
        self.push_values();
        self.net.undump_state(xml);
    }

    fn post_count(&self) -> usize {
        2
    }
    fn can_view_in_scope(&self) -> bool {
        true
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let f = (dn / 2.0 - 10.0) / dn;
        self.base.lead1 = interp_point(p1, p2, f);
        self.base.lead2 = interp_point(p1, p2, 1.0 - f);
        let (a, b) = interp_point2(p1, p2, f, 8.0);
        self.plate1 = [a, b];
        let (a, b) = interp_point2(p1, p2, 1.0 - f, 8.0);
        self.plate2 = [a, b];
        let f2 = (dn / 2.0 - 5.0) / dn;
        let (s0, s1) = interp_point2(p1, p2, f2, 10.0);
        let (s3, s2) = interp_point2(p1, p2, 1.0 - f2, 10.0);
        self.sandwich = [s0, s1, s2, s3];
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
        self.base.curcount = 0.0;
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let hs = 12.0;
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, hs);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let v = self.base.volts.clone();
        let power = self.net.power();
        set_voltage_color(g, &self.base, c, v[0]);
        draw_thick_line(g, p1, l1);
        set_power_color_w(g, &self.base, c, power);
        draw_thick_line(g, self.plate1[0], self.plate1[1]);
        if c.show_power {
            g.set_color(Color::GRAY);
        }
        set_voltage_color(g, &self.base, c, v[1]);
        draw_thick_line(g, p2, l2);
        set_power_color_w(g, &self.base, c, power);
        draw_thick_line(g, self.plate2[0], self.plate2[1]);
        set_voltage_color(g, &self.base, c, 0.5 * (v[0] + v[1]));
        for i in 0..4 {
            draw_thick_line(g, self.sandwich[i], self.sandwich[(i + 1) % 4]);
        }
        self.base.current = self.net.current_into_node(1);
        self.base.update_dot_count(c);
        if !self.base.creating {
            draw_dots(g, c, p1, l1, self.base.curcount);
            draw_dots(g, c, p2, l2, -self.base.curcount);
        }
        draw_posts(g, &self.base, self, c);
        if self.base.flags & FLAG_SHOW_FREQ != 0 {
            let s = super::units::short_unit_text(self.series_frequency(), "Hz");
            draw_values(g, &self.base, c, Some(&s), hs, false, false);
        }
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        use super::units::unit_text;
        arr.push("crystal".into());
        basic_info(self, arr);
        let fs = self.series_frequency();
        let c_ser = self.parallel_capacitance * self.series_capacitance
            / (self.parallel_capacitance + self.series_capacitance);
        let fp = 1.0 / ((self.inductance * c_ser).sqrt() * std::f64::consts::TAU);
        let q = std::f64::consts::TAU * fs * self.inductance / self.resistance;
        arr.push(format!("fs = {}", unit_text(fs, "Hz")));
        arr.push(format!("fp = {}", unit_text(fp, "Hz")));
        arr.push(format!("Q = {}", unit_text(q, "")));
        arr.push(format!("P = {}", unit_text(self.net.power(), "W")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::number("Parallel Capacitance", self.parallel_capacitance).set_positive(),
            ),
            1 => Some(
                EditInfo::number("Series Capacitance (F)", self.series_capacitance).set_positive(),
            ),
            2 => Some(EditInfo::new("Inductance (H)", self.inductance, 0.0, 0.0).set_positive()),
            3 => Some(
                EditInfo::new(
                    &format!("Resistance ({})", crate::locale::OHM),
                    self.resistance,
                    0.0,
                    0.0,
                )
                .set_positive(),
            ),
            4 => Some(EditInfo::checkbox("Show Frequency", self.base.flags & FLAG_SHOW_FREQ != 0)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 if ei.value > 0.0 => self.parallel_capacitance = ei.value,
            1 if ei.value > 0.0 => self.series_capacitance = ei.value,
            2 if ei.value > 0.0 => self.inductance = ei.value,
            3 if ei.value > 0.0 => self.resistance = ei.value,
            4 => self.base.flags = ei.change_flag(self.base.flags, FLAG_SHOW_FREQ),
            _ => {}
        }
        self.push_values();
    }
}

// ---------------------------------------------------------------------------------------
// Norton amplifier
// ---------------------------------------------------------------------------------------

const NORTON_MODEL: &str = r#"<elms><d nn="1 0" f="2" mo="default"/><d nn="2 0" f="2" mo="default"/><CCCS nn="3 2 1 0" f="0" ic="2" ex="-a"/><a nn="1 0 4" f="8" ma="12" mi="0" ga="100000"/></elms>"#;
const NORTON_NODES: [i32; 3] = [3, 1, 4];
const NORTON_SWAP: i32 = 1;
const NORTON_SMALL: i32 = 2;

/// A Norton amplifier: an op amp whose inputs are current mirrors.
pub struct NortonAmp {
    base: Base,
    net: ChildNet,
    size: i32,
    in1: [Point; 2],
    in2: [Point; 2],
    text_p: [Point; 2],
    triangle: Polygon,
    center: Point,
    radius: i32,
    inner: Polygon,
    curcount: [f64; 3],
}

impl NortonAmp {
    /// A Norton amplifier being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let net = if x == 0 && y == 0 {
            ChildNet::default()
        } else {
            ChildNet::from_xml(NORTON_MODEL, &NORTON_NODES)
        };
        let mut a = NortonAmp {
            base: Base::new(x, y),
            net,
            size: 2,
            in1: [Point::default(); 2],
            in2: [Point::default(); 2],
            text_p: [Point::default(); 2],
            triangle: Polygon::default(),
            center: Point::default(),
            radius: 0,
            inner: Polygon::default(),
            curcount: [0.0; 3],
        };
        a.base.flags |= FLAG_ESCAPE;
        a.base.no_diagonal = true;
        a
    }

    fn set_size(&mut self, s: i32) {
        self.size = s;
        self.base.flags &= !NORTON_SMALL;
        if s == 1 {
            self.base.flags |= NORTON_SMALL;
        }
    }
}

impl Element for NortonAmp {
    impl_element_base!("NortonAmpElm");
    forward_to_net!();

    fn xml_dump_type(&self) -> String {
        "nor".into()
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        self.net.dump_state(out);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        if self.net.children.is_empty() {
            self.net = ChildNet::from_xml(NORTON_MODEL, &NORTON_NODES);
        }
        self.net.undump_state(xml);
        self.set_size(if self.base.flags & NORTON_SMALL != 0 { 1 } else { 2 });
    }

    fn post_count(&self) -> usize {
        3
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.in1[0],
            1 => self.in2[0],
            _ => self.base.point2,
        })
    }

    fn set_points(&mut self) {
        self.base.set_points();
        if self.base.dn > 150.0 && self.base.creating {
            self.set_size(2);
        }
        let (opheight, opwidth) = (8 * self.size, 13 * self.size);
        let mut ww = opwidth;
        if f64::from(ww) > self.base.dn / 2.0 {
            ww = (self.base.dn / 2.0) as i32;
        }
        self.base.calc_leads(ww * 2);
        let mut hs = opheight * self.base.dsign;
        if self.base.flags & NORTON_SWAP != 0 {
            hs = -hs;
        }
        let (p1, p2, l1, l2) =
            (self.base.point1, self.base.point2, self.base.lead1, self.base.lead2);
        let (a, b) = interp_point2(p1, p2, 0.0, f64::from(hs));
        let (c, d) = interp_point2(l1, l2, 0.0, f64::from(hs));
        self.in1 = [a, c];
        self.in2 = [b, d];
        let (t0, t1) = interp_point2(l1, l2, 0.2, f64::from(hs));
        self.text_p = [t0, t1];
        let (tr0, tr1) = interp_point2(l1, l2, 0.0, f64::from(hs * 2));
        self.triangle = Polygon::from_points(&[tr0, tr1, l2]);
        self.center = l1;
        self.radius = (f64::from(opheight) * 0.5) as i32;
        let tip = interp_point(self.in1[1], self.in2[1], 1.2 / 3.0);
        let (b0, b1) =
            interp_point2(self.in1[1], self.in2[1], 1.8 / 3.0, f64::from(opheight) * 0.3);
        self.inner = Polygon::from_points(&[tip, b0, b1]);
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
        self.curcount = [0.0; 3];
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(8 * self.size * 2));
        let v = self.base.volts.clone();
        set_voltage_color(g, &self.base, c, v[0]);
        draw_thick_line(g, self.in1[0], self.in1[1]);
        set_voltage_color(g, &self.base, c, v[1]);
        draw_thick_line(g, self.in2[0], self.in2[1]);
        set_voltage_color(g, &self.base, c, v[2]);
        draw_thick_line(g, self.base.lead2, p2);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        let power = self.net.power();
        set_power_color_w(g, &self.base, c, power);
        draw_thick_polygon(g, &self.triangle);
        g.set_font_size(if self.size == 2 { 14 } else { 10 });
        let (t0, t1) = (self.text_p[0], self.text_p[1]);
        draw_centered_text(g, &mut self.base, "+", t0.x, t0.y - 2, true);
        draw_centered_text(g, &mut self.base, "-", t1.x, t1.y, true);
        draw_thick_circle(g, self.center.x, self.center.y, self.radius);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        fill_polygon(g, &self.inner);
        let ends = [(self.in1[0], self.in1[1]), (self.in2[0], self.in2[1]), (p2, self.base.lead2)];
        for (i, (a, b)) in ends.into_iter().enumerate() {
            let cur = -self.net.current_into_node(i);
            self.curcount[i] = self.base.update_dot_count_for(cur, self.curcount[i], c);
            draw_dots(g, c, a, b, self.curcount[i]);
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        use super::units::{current_text, voltage_text};
        let v = &self.base.volts;
        arr.push("norton amp".into());
        arr.push(format!("V+ = {}", voltage_text(v[0])));
        arr.push(format!("V- = {}", voltage_text(v[1])));
        arr.push(format!("Vout = {}", voltage_text(v[2])));
        arr.push(format!("Iout = {}", current_text(-self.net.current_into_node(2))));
    }

    fn flip_x(&mut self, center2: i32, count: usize) {
        if self.base.dx == 0 {
            self.base.flags ^= NORTON_SWAP;
        }
        self.base.flip_x(center2);
        let _ = count;
    }
    fn flip_y(&mut self, center2: i32, count: usize) {
        if self.base.dy == 0 {
            self.base.flags ^= NORTON_SWAP;
        }
        self.base.flip_y(center2);
        let _ = count;
    }
    fn flip_xy(&mut self, xmy: i32, count: usize) {
        self.base.flags ^= NORTON_SWAP;
        self.base.flip_xy(xmy);
        let _ = count;
    }
}

// ---------------------------------------------------------------------------------------
// Comparator
// ---------------------------------------------------------------------------------------

const COMP_MODEL: &str = "OpAmpElm 1 2 3\nAnalogSwitchElm 4 5 3\nGroundElm 5";
const COMP_NODES: [i32; 3] = [2, 1, 4];
const COMP_SMALL: i32 = 2;
const COMP_SWAP: i32 = 4;

/// A comparator: an op amp driving an analog switch that pulls the output to ground.
pub struct Comparator {
    base: Base,
    net: ChildNet,
    size: i32,
    in1: [Point; 2],
    in2: [Point; 2],
    text_p: [Point; 3],
    triangle: Polygon,
    curcount: [f64; 3],
}

impl Comparator {
    fn build(base: Base, st: Option<&mut Tokenizer<'_>>) -> Self {
        let escaped = base.flags & FLAG_ESCAPE != 0;
        let model = if probe(&base, &st) { "" } else { COMP_MODEL };
        let net = ChildNet::from_model_string(model, &COMP_NODES, st, escaped);
        let small = base.flags & COMP_SMALL != 0;
        let mut a = Comparator {
            base,
            net,
            size: if small { 1 } else { 2 },
            in1: [Point::default(); 2],
            in2: [Point::default(); 2],
            text_p: [Point::default(); 3],
            triangle: Polygon::default(),
            curcount: [0.0; 3],
        };
        a.base.flags |= FLAG_ESCAPE;
        a.base.no_diagonal = true;
        a
    }

    /// A comparator being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::build(Base::new(x, y), None)
    }

    /// A comparator read from a text line.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        Self::build(Base::from_dump(xa, ya, xb, yb, f), Some(st))
    }

    fn set_size(&mut self, s: i32) {
        self.size = s;
        self.base.flags &= !COMP_SMALL;
        if s == 1 {
            self.base.flags |= COMP_SMALL;
        }
    }
}

impl Element for Comparator {
    impl_element_base!("ComparatorElm");
    forward_to_net!();

    fn dump_type(&self) -> i32 {
        401
    }
    fn dump(&self) -> String {
        format!("{}{}", self.base.dump_prefix(401), self.net.dump_children())
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        self.net.dump_state(out);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        if self.net.children.is_empty() {
            self.net = ChildNet::from_model_string(COMP_MODEL, &COMP_NODES, None, true);
        }
        self.net.undump_state(xml);
        self.set_size(if self.base.flags & COMP_SMALL != 0 { 1 } else { 2 });
    }

    fn post_count(&self) -> usize {
        3
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.in1[0],
            1 => self.in2[0],
            _ => self.base.point2,
        })
    }

    fn set_points(&mut self) {
        self.base.set_points();
        if self.base.dn > 150.0 && self.base.creating {
            self.set_size(2);
        }
        let (opheight, opwidth) = (8 * self.size, 13 * self.size);
        let mut ww = opwidth;
        if f64::from(ww) > self.base.dn / 2.0 {
            ww = (self.base.dn / 2.0) as i32;
        }
        self.base.calc_leads(ww * 2);
        let hs = opheight * self.base.dsign;
        let sgn = if self.base.flags & COMP_SWAP != 0 { -1 } else { 1 };
        let (p1, p2, l1, l2) =
            (self.base.point1, self.base.point2, self.base.lead1, self.base.lead2);
        let (a, b) = interp_point2(p1, p2, 0.0, f64::from(hs * sgn));
        let (c, d) = interp_point2(l1, l2, 0.0, f64::from(hs * sgn));
        self.in1 = [a, c];
        self.in2 = [b, d];
        let (t0, t1) = interp_point2(l1, l2, 0.2, f64::from(hs * sgn));
        self.text_p = [t0, t1, interp_point(l1, l2, 0.5)];
        let (tr0, tr1) = interp_point2(l1, l2, 0.0, f64::from(hs * 2));
        self.triangle = Polygon::from_points(&[tr0, tr1, l2]);
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
        self.curcount = [0.0; 3];
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(8 * self.size * 2));
        let v = self.base.volts.clone();
        set_voltage_color(g, &self.base, c, v[0]);
        draw_thick_line(g, self.in1[0], self.in1[1]);
        set_voltage_color(g, &self.base, c, v[1]);
        draw_thick_line(g, self.in2[0], self.in2[1]);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        let power = self.net.power();
        set_power_color_w(g, &self.base, c, power);
        draw_thick_polygon(g, &self.triangle);
        g.set_font_size(if self.size == 2 { 14 } else { 10 });
        let t = self.text_p;
        draw_centered_text(g, &mut self.base, "-", t[0].x, t[0].y - 2, true);
        draw_centered_text(g, &mut self.base, "+", t[1].x, t[1].y, true);
        draw_centered_text(g, &mut self.base, "\u{2265}?", t[2].x, t[2].y, true);
        set_voltage_color(g, &self.base, c, v[2]);
        draw_thick_line(g, self.base.lead2, p2);
        let cur = -self.net.current_into_node(2);
        self.curcount[2] = self.base.update_dot_count_for(cur, self.curcount[2], c);
        draw_dots(g, c, p2, self.base.lead2, self.curcount[2]);
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        use super::units::voltage_text;
        arr.push("Comparator".into());
        arr.push(format!("V+ = {}", voltage_text(self.base.volts[1])));
        arr.push(format!("V- = {}", voltage_text(self.base.volts[0])));
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        if self.base.dx == 0 {
            self.base.flags ^= COMP_SWAP;
        }
        self.base.flip_x(center2);
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        if self.base.dy == 0 {
            self.base.flags ^= COMP_SWAP;
        }
        self.base.flip_y(center2);
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.base.flags ^= COMP_SWAP;
        self.base.flip_xy(xmy);
    }
}

// ---------------------------------------------------------------------------------------
// Unijunction transistor
// ---------------------------------------------------------------------------------------

const UJT_MODEL: &str = "DiodeElm 1 4\nVoltageElm 4 5\nCCVSElm 4 5 6 0\nResistorElm 0 6\n\
VCCSElm 5 7 5 7 6 7 5\nCapacitorElm 5 7\nResistorElm 7 2\nResistorElm 3 5";
const UJT_NODES: [i32; 3] = [1, 2, 3];
const UJT_STATE: &str = "2 x2n2646-emitter/0 0 0 0 0 0 0/2 2 1000*a/0 1000000/0 5 0.00028*(a-b)\\p0.00575*(c-d)*e/2 3.5e-11 0 0/0 38.15/0 2518";
const UJT_FLIP: i32 = 2;

/// A unijunction transistor (emitter, base 1, base 2) built from a circuit model.
pub struct Unijunction {
    base: Base,
    net: ChildNet,
    b1: [Point; 3],
    b2: [Point; 3],
    emitter: [Point; 3],
    emitter_poly: Polygon,
    arrow_poly: Polygon,
    curcount: [f64; 3],
}

impl Unijunction {
    fn build(base: Base, probe: bool) -> Self {
        let net = if probe {
            ChildNet::default()
        } else {
            let tokens: Vec<&str> = UJT_STATE.split('/').collect();
            ChildNet::from_model_tokens(UJT_MODEL, &UJT_NODES, &tokens)
        };
        let mut u = Unijunction {
            base,
            net,
            b1: [Point::default(); 3],
            b2: [Point::default(); 3],
            emitter: [Point::default(); 3],
            emitter_poly: Polygon::default(),
            arrow_poly: Polygon::default(),
            curcount: [0.0; 3],
        };
        u.base.flags |= FLAG_ESCAPE;
        u.base.no_diagonal = true;
        u
    }

    /// A transistor being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::build(Base::new(x, y), x == 0 && y == 0)
    }

    /// A transistor read from a text line; its inner state is not written, so the line ends
    /// with the common fields.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32) -> Self {
        Self::build(Base::from_dump(xa, ya, xb, yb, f), false)
    }
}

impl Element for Unijunction {
    impl_element_base!("UnijunctionElm");
    forward_to_net!();

    fn dump_type(&self) -> i32 {
        417
    }
    fn xml_dump_type(&self) -> String {
        "Unijunction".into()
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        self.net.dump_state(out);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        if self.net.children.is_empty() {
            let tokens: Vec<&str> = UJT_STATE.split('/').collect();
            self.net = ChildNet::from_model_tokens(UJT_MODEL, &UJT_NODES, &tokens);
        }
        self.net.undump_state(xml);
    }

    fn post_count(&self) -> usize {
        3
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.emitter[0],
            1 => self.b1[0],
            _ => self.b2[0],
        })
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let hs = 16;
        let flip = if self.base.flags & UJT_FLIP != 0 { -1 } else { 1 };
        let hs2 = f64::from(hs * self.base.dsign * flip);
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let q1 = interp_point_g(p1, p2, 0.0, -hs2);
        let q2 = interp_point_g(p1, p2, 1.0, -hs2);
        let (a, b) = interp_point2(q1, q2, 1.0, -hs2);
        self.b1[0] = a;
        self.b2[0] = b;
        let (a, b) = interp_point2(q1, q2, 1.0, -hs2 / 2.0);
        self.b1[1] = a;
        self.b2[1] = b;
        let (a, b) = interp_point2(q1, q2, 1.0 - 10.0 / dn, -hs2 / 2.0);
        self.b1[2] = a;
        self.b2[2] = b;
        self.emitter[0] = interp_point_g(q1, q2, 0.0, hs2);
        self.emitter[1] = interp_point_g(q1, q2, 1.0 - 28.0 / dn, hs2);
        self.emitter[2] = interp_point(q1, q2, 1.0 - 14.0 / dn);
        let (r0, r1) = interp_point2(q1, q2, 1.0 - 13.0 / dn, f64::from(hs));
        let (r2, r3) = interp_point2(q1, q2, 1.0 - 10.0 / dn, f64::from(hs));
        self.emitter_poly = Polygon::from_points(&[r0, r1, r3, r2]);
        self.arrow_poly = calc_arrow(self.emitter[1], self.emitter[2], 8.0, 3.0);
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
        self.curcount = [0.0; 3];
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, b1_0) = (self.base.point1, self.b1[0]);
        self.base.set_bbox_pts(p1, b1_0, 0.0);
        let v = self.base.volts.clone();
        set_voltage_color(g, &self.base, c, v[1]);
        draw_thick_line(g, self.b1[0], self.b1[1]);
        draw_thick_line(g, self.b1[1], self.b1[2]);
        set_voltage_color(g, &self.base, c, v[2]);
        draw_thick_line(g, self.b2[0], self.b2[1]);
        draw_thick_line(g, self.b2[1], self.b2[2]);
        set_voltage_color(g, &self.base, c, v[0]);
        draw_thick_line(g, self.emitter[0], self.emitter[1]);
        draw_thick_line(g, self.emitter[1], self.emitter[2]);
        fill_polygon(g, &self.arrow_poly);
        let power = self.net.power();
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.emitter_poly);
        let ib2 = -self.net.current_into_node(2);
        let ib1 = -self.net.current_into_node(1);
        let cur = [ib1, ib2, -ib1 - ib2];
        for i in 0..3 {
            self.curcount[i] = self.base.update_dot_count_for(cur[i], self.curcount[i], c);
        }
        if self.curcount[0] != 0.0 || self.curcount[1] != 0.0 {
            let (cb1, cb2, ce) = (self.curcount[0], self.curcount[1], self.curcount[2]);
            draw_dots(g, c, self.b1[0], self.b1[1], cb1);
            draw_dots(g, c, self.b1[1], self.b1[2], Base::add_cur_count(cb1, 8.0));
            draw_dots(g, c, self.b2[0], self.b2[1], cb2);
            draw_dots(g, c, self.b2[1], self.b2[2], Base::add_cur_count(cb2, 8.0));
            draw_dots(g, c, self.emitter[0], self.emitter[1], ce);
            draw_dots(g, c, self.emitter[1], self.emitter[2], ce);
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        use super::units::{current_text, unit_text, voltage_text};
        let v = &self.base.volts;
        arr.push("unijunction transistor".into());
        arr.push(format!("Ie = {}", current_text(-self.net.current_into_node(0))));
        arr.push(format!("Ib2 = {}", current_text(-self.net.current_into_node(2))));
        arr.push(format!("Veb1 = {}", voltage_text(v[0] - v[1])));
        arr.push(format!("Vb2b1 = {}", voltage_text(v[2] - v[1])));
        arr.push(format!("P = {}", unit_text(self.net.power(), "W")));
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        if self.base.dx == 0 {
            self.base.flags ^= UJT_FLIP;
        }
        self.base.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        if self.base.dy == 0 {
            self.base.flags ^= UJT_FLIP;
        }
        self.base.flip_y(center2);
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.base.flags ^= UJT_FLIP;
        self.base.flip_xy(xmy);
        self.set_points();
    }
}
