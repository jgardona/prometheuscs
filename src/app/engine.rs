// SPDX-License-Identifier: GPL-2.0-or-later
//! How a [`Session`] talks to the simulation worker.
//!
//! With a worker attached, the session's own simulation is only the editable document: the
//! worker steps its own copy and the session copies voltages and currents from the newest
//! snapshot into the elements so they can be drawn. Without a worker (tests, headless use)
//! the session steps its simulation itself.

use super::session::Session;
use super::worker::{Command, Worker};
use crate::error::StepError;
use crate::format::xml;
use crate::solver::snapshot::{FrameSnapshot, SnapshotError, Status};
use std::time::Duration;

impl Session {
    /// Moves the solver to `worker` and sends it the circuit.
    pub fn attach_worker(&mut self, worker: Worker) {
        self.worker = Some(worker);
        self.sync_worker(false);
    }

    /// The worker, when one is attached.
    pub fn worker_handle(&self) -> Option<&Worker> {
        self.worker.as_ref()
    }

    /// True when the solver runs on its own thread.
    pub fn has_worker(&self) -> bool {
        self.worker.is_some()
    }

    /// Simulated time: the newest snapshot's with a worker, the simulation's otherwise.
    pub fn time(&self) -> f64 {
        if self.worker.is_some() { self.snapshot_time } else { self.sim.time() }
    }

    /// Sends the whole circuit, with its running state, to the worker.
    pub(crate) fn sync_worker(&mut self, keep_time: bool) {
        if self.worker.is_none() {
            return;
        }
        let xml = self.document_xml();
        if let Some(w) = &self.worker {
            w.send(Command::Load { xml, keep_time });
            w.send(Command::SetSpeed(self.options.iter_count));
            w.send(Command::SetRunning(self.running));
        }
    }

    /// Copies the running state of the elements (capacitor voltages and so on) from the worker
    /// into the document, so saving, undo and reloading see the live values.
    pub(crate) fn pull_state(&mut self) {
        let Some(dump) = self.worker.as_ref().and_then(Worker::dump) else { return };
        let Ok(root) = xml::parse(&dump) else { return };
        let n = self.sim.elements.len();
        if root.children.len() < n {
            return;
        }
        for (e, node) in self.sim.elements.iter_mut().zip(&root.children) {
            if node.tag == e.xml_dump_type() {
                e.undump_xml(node);
                // reading a node may rebuild what an element keeps about its shape (a chip
                // makes its pins again, with no coordinates); as when a file is read, the
                // points are computed after it
                e.set_points();
            }
        }
    }

    /// Number of scopes in the circuit.
    pub fn scope_count(&self) -> usize {
        self.sim.scopes.len()
    }

    /// How big the area under the canvas has to be for the docked scopes, and whether it shows at
    /// all: 130 pixels for each row of the tallest column, at most three rows, and hidden when
    /// there are no scopes.
    pub fn scope_area_state(&self) -> (i32, bool) {
        let n = crate::render::scopes::rows(&self.scope_views());
        ((n as i32 * 130).min(390), n > 0)
    }

    /// What every scope shows now.
    pub fn scope_views(&self) -> Vec<crate::scope::ScopeView> {
        if self.worker.is_some() {
            self.scope_views.clone()
        } else {
            let dt = self.sim.st.max_time_step;
            self.sim.scopes.iter().map(|s| s.view(&self.sim.elements, dt, self.sim.st.t)).collect()
        }
    }

    /// Takes the worker's newest snapshot, if any, and refreshes the drawing settings.
    pub(crate) fn pump(&mut self, elapsed: Duration) -> Result<(), StepError> {
        let snapshot = self.worker.as_ref().and_then(Worker::latest);
        if let Some(snap) = snapshot {
            self.apply_snapshot(&snap);
        }
        if self.worker.is_none() {
            self.sim.push_embedded_views();
        }
        // every frame, with or without a worker: the flag is not part of what the worker sends
        self.mark_placed_scopes();
        self.highlight_stop_element();
        self.refresh_draw_ctx(elapsed.min(self.max_frame_gap()));
        Ok(())
    }

    fn apply_snapshot(&mut self, snap: &FrameSnapshot) {
        self.snapshot_time = snap.time;
        self.scope_views.clone_from(&snap.scopes);
        if snap.pause_count != self.pause_seen {
            // an element (a stop trigger) paused the worker's simulation
            if snap.pause_count > self.pause_seen {
                self.running = false;
            }
            self.pause_seen = snap.pause_count;
        }
        for (i, v) in &snap.embedded {
            if let Some(e) = self.sim.elements.get_mut(*i) {
                e.set_scope_view(v.clone());
            }
        }
        // a snapshot of a circuit that has since been edited does not fit the elements
        if snap.element_currents.len() == self.sim.elements.len() {
            for ((e, volts), current) in
                self.sim.elements.iter_mut().zip(&snap.element_volts).zip(&snap.element_currents)
            {
                let b = e.base_mut();
                if b.volts.len() == volts.len() {
                    b.volts.copy_from_slice(volts);
                }
                b.current = *current;
            }
            for (e, state) in self.sim.elements.iter_mut().zip(&snap.element_states) {
                if !state.is_empty() {
                    e.set_visual_state(state);
                }
            }
        }
        if let Some(err) = &snap.error {
            if self.stop_reason.as_deref() != Some(err.message.as_str()) {
                self.record_stop(&err.message, err.element);
            }
        }
    }

    /// A snapshot of this session's simulation, with the session's own error if it stopped.
    pub(crate) fn snapshot(&self, sequence: u64) -> FrameSnapshot {
        let mut snap = self.sim.snapshot(sequence, self.running);
        if let Some(reason) = &self.stop_reason {
            snap.status = Status::Errored { reason: reason.clone(), element: self.stop_element };
            snap.error =
                Some(SnapshotError { message: reason.clone(), element: self.stop_element });
        }
        snap
    }

    /// Sets field `n` of element `elm` to `value` and analyses the circuit again.
    pub(crate) fn edit_value(&mut self, elm: usize, n: usize, value: f64) {
        let Some(mut ei) = self.sim.elements.get(elm).and_then(|e| e.get_edit_info(n)) else {
            return;
        };
        ei.value = value;
        self.sim.apply_edit(elm, n, &mut ei);
        self.sim.elements[elm].set_points();
        self.reanalyze();
    }

    /// Moves the slider element `elm` owns and analyses the circuit again.
    pub(crate) fn apply_element_slider(&mut self, elm: usize, value: i32) {
        let Some(e) = self.sim.elements.get_mut(elm) else {
            return;
        };
        e.set_slider_value(value);
        e.set_points();
        self.reanalyze();
    }

    /// Tells the worker about a parameter that was just changed in the document: a small
    /// message instead of rebuilding the circuit. Without a worker the circuit is analysed.
    pub(crate) fn param_changed(&mut self, elm: usize, n: usize, value: f64) {
        match &self.worker {
            Some(w) => w.send(Command::Edit { elm, n, value }),
            None => self.reanalyze(),
        }
    }

    /// Advances `n` steps right now (headless runs), recording a failure.
    pub(crate) fn run_steps_now(&mut self, n: usize) {
        self.sim.set_delay_wire_processing(true);
        let result = self.sim.run_steps(n);
        self.sim.calc_wire_currents();
        if let Err(e) = result {
            let element = match &e {
                StepError::Stopped { element, .. } => *element,
                _ => None,
            };
            self.record_stop(&e.to_string(), element);
        }
    }
}
