// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (EditCompositeModelDialog,
// SubcircuitDialog and SimulationManager.getCircuitAsComposite).
//! Making subcircuits: turning the selected part of the circuit (or all of it) into a model,
//! keeping models for this circuit, this session or for good, and the subcircuit manager.
//!
//! The pins of the new subcircuit are the labeled nodes of the selection that are not marked
//! internal; the model records how every element connects to nodes numbered like the solver
//! numbers them, so that an instance can rebuild the same circuit.

use super::edit::Tool;
use super::session::Session;
use super::settings::config_dir;
use crate::elements::element::Element;
use crate::elements::subcircuit::{self, CompositeModel, ExtPin};
use crate::format::xml::{XmlNode, XmlOut, parse};
use std::path::PathBuf;

/// Where a new subcircuit is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelScope {
    /// In the circuit file only.
    Circuit,
    /// Until the program closes.
    Session,
    /// In the settings folder, for every circuit.
    Saved,
}

/// North, south, west and east, as the pins number the sides.
const SIDE_N: i32 = 0;
const SIDE_S: i32 = 1;
const SIDE_W: i32 = 2;
const SIDE_E: i32 = 3;
const LABEL_INTERNAL: i32 = 1;
const MODEL_SHOW_LABEL: i32 = 1;

fn saved_dir() -> PathBuf {
    config_dir().join(super::consts::SUBCIRCUITS_DIR)
}

fn saved_path(name: &str) -> PathBuf {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || "-_.".contains(c) { c } else { '_' })
        .collect();
    saved_dir().join(format!("{safe}.xml"))
}

/// Reads the subcircuits kept across sessions and defines them.
pub fn load_saved() {
    let Ok(dir) = std::fs::read_dir(saved_dir()) else { return };
    for entry in dir.flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        if let Ok(node) = parse(&text) {
            subcircuit::define_from_xml(&node);
        }
    }
}

fn out_to_node(o: &XmlOut) -> XmlNode {
    XmlNode {
        tag: o.tag.clone(),
        attrs: o.attrs.clone(),
        children: o.children.iter().map(out_to_node).collect(),
        text: o.text.clone().unwrap_or_default(),
    }
}

/// Elements that only connect or annotate: they go after the real parts in a model.
fn is_plumbing(e: &dyn Element) -> bool {
    matches!(
        e.class_name(),
        "WireElm" | "RoutedWireElm" | "LabeledNodeElm" | "ScopeElm" | "GroundElm"
    ) || e.is_graphic()
}

impl Session {
    /// The pins and parts of the selected elements (all of them when nothing is selected) as a
    /// subcircuit model with no name yet, with the pins laid out on the chip.
    pub fn start_subcircuit(&mut self) -> Result<CompositeModel, String> {
        let selected = self.has_selection();
        // closed switches would merge their nodes; the model has to see them open
        let closed = self.open_closed_switches();
        let numbers = self.sim.model_node_numbers();
        self.restore_switches(&closed);
        let model = self.model_from_numbers(selected, &numbers);
        // the numbering above replaced the analysis of the circuit
        self.reanalyze();
        model
    }

    #[cfg(feature = "switches")]
    fn open_closed_switches(&mut self) -> Vec<usize> {
        use crate::elements::switches::SwitchElm;
        let mut closed = Vec::new();
        for (i, e) in self.sim.elements.iter_mut().enumerate() {
            if let Some(s) = e.as_any_mut().downcast_mut::<SwitchElm>() {
                if s.position == 0 {
                    s.position = 1;
                    closed.push(i);
                }
            }
        }
        closed
    }

    #[cfg(not(feature = "switches"))]
    fn open_closed_switches(&mut self) -> Vec<usize> {
        Vec::new()
    }

    #[cfg(feature = "switches")]
    fn restore_switches(&mut self, closed: &[usize]) {
        use crate::elements::switches::SwitchElm;
        for &i in closed {
            if let Some(s) = self.sim.elements[i].as_any_mut().downcast_mut::<SwitchElm>() {
                s.position = 0;
            }
        }
    }

    #[cfg(not(feature = "switches"))]
    fn restore_switches(&mut self, _closed: &[usize]) {}

    fn model_from_numbers(
        &self,
        selected: bool,
        numbers: &[Vec<usize>],
    ) -> Result<CompositeModel, String> {
        let included = |i: usize| !selected || self.sim.elements[i].base().selected;
        // the pins: labeled nodes that are not internal, by side
        let mut sides: [Vec<(usize, &dyn Element)>; 4] = Default::default();
        let mut external = std::collections::HashSet::new();
        for (i, e) in self.sim.elements.iter().enumerate() {
            if !included(i) || e.class_name() != "LabeledNodeElm" {
                continue;
            }
            if e.base().flags & LABEL_INTERNAL != 0 {
                continue;
            }
            let first = numbers[i].first().copied().unwrap_or(0);
            if external.contains(&first) {
                continue;
            }
            let (dx, dy) = (e.base().dx, e.base().dy);
            let mut side = SIDE_W;
            if dx.abs() >= dy.abs() && dx > 0 {
                side = SIDE_E;
            }
            if dx.abs() <= dy.abs() && dy < 0 {
                side = SIDE_N;
            }
            if dx.abs() <= dy.abs() && dy > 0 {
                side = SIDE_S;
            }
            for &n in &numbers[i] {
                external.insert(n);
                if n == 0 {
                    return Err(format!(
                        "Node \"{}\" can't be connected to ground",
                        label_text(e.as_ref())
                    ));
                }
            }
            sides[side as usize].push((i, e.as_ref()));
        }
        // along each side by position
        for (side, list) in sides.iter_mut().enumerate() {
            if side as i32 == SIDE_N || side as i32 == SIDE_S {
                list.sort_by_key(|(_, e)| e.base().x);
            } else {
                list.sort_by_key(|(_, e)| e.base().y);
            }
        }
        let mut ext: Vec<ExtPin> = Vec::new();
        for (side, list) in sides.iter().enumerate() {
            for (pos, (i, e)) in list.iter().enumerate() {
                let width = e.bus_width().max(1);
                for (j, node) in numbers[*i].iter().enumerate().take(width) {
                    ext.push(ExtPin {
                        name: label_text(*e),
                        node: *node as i32,
                        pos: pos as i32,
                        side: side as i32,
                        bus_width: width as i32,
                        bus_z: j as i32,
                    });
                }
            }
        }
        // the parts first, then the wires, labels and the like
        let mut elms = Vec::new();
        let mut extra = Vec::new();
        let mut used = std::collections::HashSet::new();
        for (i, e) in self.sim.elements.iter().enumerate() {
            if !included(i) {
                continue;
            }
            let mut tag = XmlOut::new(&e.xml_dump_type());
            e.dump_xml(&mut tag);
            let nn: Vec<String> = numbers[i].iter().map(ToString::to_string).collect();
            tag.str_attr("nn", &nn.join(" "));
            used.extend(numbers[i].iter().copied());
            let node = out_to_node(&tag);
            if is_plumbing(e.as_ref()) {
                extra.push(node);
            } else {
                elms.push(node);
            }
        }
        elms.extend(extra);
        if ext.is_empty() {
            return Err("Device has no external inputs/outputs!".into());
        }
        ext.sort_by_key(|p| p.name.to_lowercase());
        let mut seen = std::collections::HashSet::new();
        for p in &ext {
            if !seen.insert(p.node) {
                return Err("Can't have two input/output nodes connected!".into());
            }
        }
        for p in &ext {
            if !used.contains(&(p.node as usize)) {
                return Err(format!("Node \"{}\" is not used!", p.name));
            }
        }
        // the chip: a column of pins on each side that has any
        let mut counts = [0; 4];
        for p in ext.iter().filter(|p| p.bus_z == 0) {
            counts[p.side as usize] += 1;
        }
        let left = i32::from(counts[SIDE_W as usize] > 0);
        let right = i32::from(counts[SIDE_E as usize] > 0);
        for p in &mut ext {
            if p.side == SIDE_N || p.side == SIDE_S {
                p.pos += left;
            }
        }
        let min_height =
            if counts[SIDE_N as usize] > 0 && counts[SIDE_S as usize] > 0 { 2 } else { 1 };
        let ns = counts[SIDE_N as usize].max(counts[SIDE_S as usize]);
        let we = counts[SIDE_W as usize].max(counts[SIDE_E as usize]);
        Ok(CompositeModel {
            name: String::new(),
            flags: MODEL_SHOW_LABEL,
            size_x: 2.max(ns + left + right),
            size_y: min_height.max(we),
            ext,
            elms,
        })
    }

    /// Checks the pin layout of `model` and the name it has, then keeps it where `scope` says
    /// and makes it the model the next instance placed uses.
    pub fn finish_subcircuit(
        &mut self,
        model: CompositeModel,
        scope: ModelScope,
    ) -> Result<(), String> {
        Self::check_model(&model)?;
        self.push_undo();
        self.store_model(model, scope)?;
        self.set_tool(Tool::Place("CustomCompositeElm".into()));
        Ok(())
    }

    /// Checks the name and the pin layout of a model.
    pub(crate) fn check_model(model: &CompositeModel) -> Result<(), String> {
        if model.name.trim().is_empty() {
            return Err("Please enter a model name.".into());
        }
        if model.name == "default" {
            return Err(
                "This model cannot be modified.  Change the model name to allow customization."
                    .into(),
            );
        }
        let mut taken = std::collections::HashSet::new();
        for p in model.ext.iter().filter(|p| p.bus_z == 0) {
            let limit =
                if p.side == SIDE_N || p.side == SIDE_S { model.size_x } else { model.size_y };
            if p.pos < 0 || p.pos >= limit {
                return Err(format!("Pin \"{}\" is outside the chip", p.name));
            }
            if !taken.insert((p.side, p.pos)) {
                return Err(format!("Pin \"{}\" shares its place with another pin", p.name));
            }
        }
        Ok(())
    }

    /// Where the model called `name` is kept now.
    pub(crate) fn model_scope_of(&self, name: &str) -> ModelScope {
        if saved_path(name).exists() {
            ModelScope::Saved
        } else if self
            .extras
            .model_xml
            .iter()
            .any(|n| n.tag == "ccm" && n.string_attr("nm", "") == name)
        {
            ModelScope::Circuit
        } else {
            ModelScope::Session
        }
    }

    /// Keeps a model where `scope` says and makes every instance look it up again.
    pub(crate) fn store_model(
        &mut self,
        model: CompositeModel,
        scope: ModelScope,
    ) -> Result<(), String> {
        // leave the other places the model could have been kept in
        self.extras
            .model_xml
            .retain(|n| !(n.tag == "ccm" && n.string_attr("nm", "") == model.name));
        let _ = std::fs::remove_file(saved_path(&model.name));
        match scope {
            ModelScope::Circuit => self.extras.model_xml.push(out_to_node(&model.to_xml())),
            ModelScope::Session => {}
            ModelScope::Saved => {
                let _ = std::fs::create_dir_all(saved_dir());
                std::fs::write(saved_path(&model.name), model.to_xml().to_pretty_string())
                    .map_err(|e| format!("{}: {e}", saved_path(&model.name).display()))?;
            }
        }
        let name = model.name.clone();
        subcircuit::define(model);
        subcircuit::set_last_model(&name);
        for e in &mut self.sim.elements {
            e.update_models();
        }
        self.dirty = true;
        self.reanalyze();
        Ok(())
    }

    /// The subcircuits that can be placed, by name.
    pub fn subcircuit_names(&self) -> Vec<String> {
        subcircuit::model_names().into_iter().filter(|n| n != "default").collect()
    }

    /// The model called `name`, to change its pin layout.
    pub fn subcircuit_model(&self, name: &str) -> Option<CompositeModel> {
        subcircuit::lookup(name)
    }

    /// Forgets subcircuit `name` everywhere; instances of it fall back to the default model.
    pub fn delete_subcircuit(&mut self, name: &str) {
        self.push_undo();
        self.extras.model_xml.retain(|n| !(n.tag == "ccm" && n.string_attr("nm", "") == name));
        let _ = std::fs::remove_file(saved_path(name));
        subcircuit::remove(name);
        for e in &mut self.sim.elements {
            e.update_models();
        }
        self.dirty = true;
        self.reanalyze();
    }

    /// Makes the next element placed an instance of subcircuit `name`.
    pub fn place_subcircuit(&mut self, name: &str) {
        subcircuit::set_last_model(name);
        self.set_tool(Tool::Place("CustomCompositeElm".into()));
    }
}

fn label_text(e: &dyn Element) -> String {
    match e.bus_role() {
        crate::elements::element::BusRole::Label(t) => t,
        _ => String::new(),
    }
}
