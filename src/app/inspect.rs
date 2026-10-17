// SPDX-License-Identifier: GPL-2.0-or-later
//! Looking inside a subcircuit instance: the canvas shows its parts, with the voltages and
//! currents they have in the running circuit, and nothing can be edited.

use super::session::Session;

impl Session {
    /// Whether the canvas shows the inside of an instance.
    pub fn inspecting(&self) -> bool {
        !self.inspect.is_empty()
    }

    /// Whether the parts of element `elm` can be shown: it is a subcircuit instance with parts.
    pub fn can_view_components(&self, elm: usize) -> bool {
        self.sim.elements.get(elm).is_some_and(|e| {
            e.class_name() == "CustomCompositeElm" && !e.child_elements().is_empty()
        })
    }

    /// Shows the parts of instance `elm` (or, when already inside one, of its part `elm`).
    pub fn view_components(&mut self, elm: usize) -> Result<(), String> {
        let list = self.display_elements();
        let ok = list.get(elm).is_some_and(|e| {
            e.class_name() == "CustomCompositeElm" && !e.child_elements().is_empty()
        });
        if !ok {
            return Err("Can't edit this model.".into());
        }
        self.inspect_views.push(self.view);
        self.inspect.push(elm);
        self.fit_pending = true;
        Ok(())
    }

    /// Goes back out of the instance, to the circuit or the instance around it.
    pub fn leave_components(&mut self) {
        if self.inspect.pop().is_some() {
            if let Some(v) = self.inspect_views.pop() {
                self.view = v;
            }
            self.fit_pending = false;
        }
    }

    /// The names of the instances on the way in, outermost first.
    pub fn inspect_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut list: &[Box<dyn crate::elements::element::Element>] = &self.sim.elements;
        for &i in &self.inspect {
            let Some(e) = list.get(i) else { break };
            names.push(e.subcircuit_model_name().unwrap_or_else(|| "subcircuit".to_string()));
            list = e.child_elements();
        }
        names
    }

    /// The elements the canvas shows: the circuit's, or the parts of the instance being looked
    /// into. A way in that no longer leads anywhere shows the circuit.
    pub fn display_elements(&self) -> &[Box<dyn crate::elements::element::Element>] {
        let mut list: &[Box<dyn crate::elements::element::Element>] = &self.sim.elements;
        for &i in &self.inspect {
            match list.get(i) {
                Some(e) => list = e.child_elements(),
                None => return &self.sim.elements,
            }
        }
        list
    }

    /// [`Session::display_elements`], to draw them.
    pub fn display_elements_mut(&mut self) -> &mut [Box<dyn crate::elements::element::Element>] {
        let valid = {
            let mut list: &[Box<dyn crate::elements::element::Element>] = &self.sim.elements;
            self.inspect.iter().all(|&i| match list.get(i) {
                Some(e) => {
                    list = e.child_elements();
                    true
                }
                None => false,
            })
        };
        let mut list: &mut [Box<dyn crate::elements::element::Element>] = &mut self.sim.elements;
        if valid {
            for &i in &self.inspect {
                list = list[i].child_elements_mut();
            }
        }
        list
    }
}
