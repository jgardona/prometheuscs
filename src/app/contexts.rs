// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CirSim.pushContext and popContext,
// SubcircuitBar).
//! Editing the circuit inside a subcircuit model.
//!
//! "Edit Model" sets the circuit aside and opens the model's own circuit in its place, as the
//! original does; "Save" (or "Save Copy") turns what is on the canvas back into a model, keeps
//! the pin layout the model had, and brings the first circuit back with the model replaced.

use super::session::Session;
use super::subcircuits::ModelScope;
use crate::elements::subcircuit::{self, CompositeModel};
use crate::format::xml::XmlOut;
use crate::render::scene::View;
use std::path::PathBuf;

/// A circuit set aside while one of its subcircuit models is edited.
#[derive(Clone, Debug)]
pub struct EditContext {
    /// The model being edited.
    pub model: String,
    outer_xml: String,
    outer_title: String,
    outer_path: Option<PathBuf>,
    outer_dirty: bool,
    outer_view: View,
    /// Where the model is kept.
    scope: ModelScope,
}

/// The pins that keep their place: a pin of `new` with the name of a pin of `old` goes where
/// that one was, and the chip keeps its size and flags.
fn preserve_pin_layout(new: &mut CompositeModel, old: &CompositeModel) {
    new.flags = old.flags;
    new.size_x = old.size_x;
    new.size_y = old.size_y;
    for pin in &mut new.ext {
        if let Some(o) = old.ext.iter().find(|o| o.name == pin.name && o.bus_z == pin.bus_z) {
            pin.side = o.side;
            pin.pos = o.pos;
        }
    }
}

impl Session {
    /// The name of the model being edited, if the circuit on the canvas is one.
    pub fn editing_model(&self) -> Option<&str> {
        self.contexts.last().map(|c| c.model.as_str())
    }

    /// Whether instance `elm` is of a model that can be edited.
    pub fn can_edit_model(&self, elm: usize) -> bool {
        self.sim.elements.get(elm).and_then(|e| e.subcircuit_model_name()).is_some()
    }

    /// Leaves the circuit aside and opens the circuit of the model of instance `elm`.
    pub fn edit_model(&mut self, elm: usize) -> Result<(), String> {
        let name = self
            .sim
            .elements
            .get(elm)
            .and_then(|e| e.subcircuit_model_name())
            .ok_or("Can't edit this model.")?;
        let model = subcircuit::lookup(&name).ok_or("Can't edit this model.")?;
        let context = EditContext {
            model: name.clone(),
            outer_xml: self.circuit_xml(),
            outer_title: self.title.clone(),
            outer_path: self.path.clone(),
            outer_dirty: self.dirty,
            outer_view: self.view,
            scope: self.model_scope_of(&name),
        };
        // the model's elements are written as tags of a circuit; the node numbers are not needed
        let mut root = XmlOut::new("cir");
        for e in &model.elms {
            let mut tag = e.to_out();
            tag.attrs.retain(|(k, _)| k != "nn");
            root.child(tag);
        }
        self.open_text(&root.to_pretty_string(), &format!("Modelo: {name}"), None)
            .map_err(|e| e.to_string())?;
        self.contexts.push(context);
        self.fit_pending = true;
        self.dirty = false;
        Ok(())
    }

    /// Puts the circuit that was set aside back, as it was.
    fn restore_context(&mut self, c: &EditContext) {
        let _ = self.open_text(&c.outer_xml, &c.outer_title, c.outer_path.clone());
        self.dirty = c.outer_dirty;
        self.view = c.outer_view;
        self.fit_pending = false;
    }

    /// Leaves the model without keeping what was done to it.
    pub fn leave_context(&mut self) {
        if let Some(c) = self.contexts.pop() {
            self.restore_context(&c);
        }
    }

    /// Keeps the circuit on the canvas as the model (or, with `copy`, as a new model of that
    /// name) and goes back to the circuit that was set aside, which uses the new model.
    pub fn save_context(&mut self, copy: Option<&str>) -> Result<(), String> {
        let Some(context) = self.contexts.last().cloned() else { return Ok(()) };
        // the model the canvas makes, with the pins where the old one had them
        // the whole circuit is the model, whatever is selected
        self.deselect_all();
        let mut model = self.start_subcircuit()?;
        if let Some(old) = subcircuit::lookup(&context.model) {
            preserve_pin_layout(&mut model, &old);
        }
        model.name = copy.map_or_else(|| context.model.clone(), |n| n.trim().to_string());
        Self::check_model(&model)?;
        self.contexts.pop();
        self.restore_context(&context);
        let scope = if copy.is_some() { ModelScope::Circuit } else { context.scope };
        self.push_undo();
        self.store_model(model, scope)
    }
}
