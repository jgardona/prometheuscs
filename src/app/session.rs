// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CirSim run loop, UIManager).
//! A loaded circuit being simulated: run loop pacing, display settings and sliders.
//!
//! The session knows nothing about GTK, so the pacing and slider logic is unit tested.

use super::edit::{Drag, Tool};
use super::settings::AppSettings;
use super::worker::{Command, Worker};
use crate::elements::element::DrawCtx;
use crate::elements::registry::Registry;
use crate::error::{AnalysisError, StepError};
use crate::format::circuit::{Adjustable, Circuit, CircuitOptions};
use crate::format::parser::{ParseResult, parse};
use crate::format::xml::XmlNode;
use crate::render::painter::Color;
use crate::render::scene::View;
use crate::solver::sim::Simulation;
use std::path::PathBuf;
use std::time::Duration;

/// Largest wall-clock gap one update accounts for; longer gaps (window hidden, debugger) are
/// clipped so the simulation does not try to catch up.
pub(crate) const MAX_FRAME_GAP: Duration = Duration::from_millis(100);

/// A circuit loaded for simulation.
pub struct Session {
    /// The running simulation (owns the elements).
    pub sim: Simulation,
    /// The circuit's display and speed settings.
    pub options: CircuitOptions,
    /// Sliders attached to element fields.
    pub adjustables: Vec<Adjustable>,
    /// Current value of each slider, as a position from 0 to 100.
    pub slider_positions: Vec<f64>,
    /// Whether time advances.
    pub running: bool,
    /// Drawing settings shared by all elements.
    pub draw_ctx: DrawCtx,
    /// Messages about parts of the file that could not be used.
    pub load_warnings: Vec<String>,
    /// The pauses by elements the worker had reported when the newest snapshot was applied.
    pub(crate) pause_seen: u64,
    /// The same problems with their details, for the dialog that lists them.
    pub load_errors: Vec<crate::error::LoadError>,
    /// Why the simulation stopped, if it did, naming the offending element when known.
    pub stop_message: Option<String>,
    /// Index of the element that caused the stop, highlighted on the canvas.
    pub stop_element: Option<usize>,
    /// The reason of the stop without the element's name, as the solver gave it.
    pub(crate) stop_reason: Option<String>,
    /// The simulation thread, when the interface runs the solver off the UI thread.
    pub(crate) worker: Option<Worker>,
    /// What the scopes showed in the newest snapshot (with a worker).
    pub(crate) scope_views: Vec<crate::scope::ScopeView>,
    /// Full-scale values of the scopes, following the data.
    pub scope_scales: crate::render::scopes::ScopeScales,
    pub(crate) snapshot_time: f64,
    /// Mapping of the last drawn frame, used to turn mouse positions into circuit coordinates.
    pub view: View,
    /// Size of the canvas in pixels, as of the last drawn frame.
    pub canvas_size: (f64, f64),
    /// The next drawn frame must fit the circuit into the canvas.
    pub fit_pending: bool,
    /// Application-wide settings (the Options menu).
    pub settings: AppSettings,
    /// Where `settings` are saved after a change, when persistence is wanted.
    pub settings_path: Option<PathBuf>,
    /// All known element types.
    pub registry: Registry,
    /// Circuit title shown in the side panel.
    pub title: String,
    /// File the circuit was loaded from or last saved to.
    pub path: Option<PathBuf>,
    /// The circuit changed since it was loaded or saved.
    pub dirty: bool,
    /// What a mouse press on the canvas does.
    pub tool: Tool,
    /// Text of the last cut or copied elements.
    pub clipboard: Option<String>,
    /// Mouse position in circuit coordinates, snapped to the grid, while the pointer is over
    /// the canvas.
    pub pointer: Option<(i32, i32)>,
    pub(crate) drag: Drag,
    pub(crate) undo: Vec<String>,
    pub(crate) redo: Vec<String>,
    pub(crate) extras: Extras,
    /// Circuits left aside while a subcircuit model is edited, outermost first.
    #[cfg(feature = "subcircuits")]
    pub(crate) contexts: Vec<super::contexts::EditContext>,
    /// The way down to the subcircuit instance whose parts are being shown: the element, then
    /// the part of it, and so on. Empty when the circuit itself is shown.
    pub(crate) inspect: Vec<usize>,
    /// The view of each circuit left to look inside an instance.
    pub(crate) inspect_views: Vec<View>,
    pub(crate) held: Option<usize>,
    pub(crate) hover: Option<usize>,
    step_debt: f64,
}

/// Parts of a circuit file the session keeps only to write them back unchanged.
#[derive(Default)]
pub(crate) struct Extras {
    pub hint: Option<(i32, i32, i32)>,
    pub scope_lines: Vec<String>,
    pub scope_xml: Vec<XmlNode>,
    pub model_xml: Vec<XmlNode>,
    pub model_lines: Vec<String>,
}

/// Errors from loading a circuit into a session.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The circuit could not be analysed.
    #[error("{0}")]
    Analysis(#[from] AnalysisError),
}

impl Session {
    /// Parses `text` and prepares a running simulation.
    pub fn load(text: &str, registry: &Registry) -> Result<Session, SessionError> {
        Self::load_with(text, registry, AppSettings::default())
    }

    /// Like [`Session::load`], with the application settings in force.
    pub fn load_with(
        text: &str,
        registry: &Registry,
        settings: AppSettings,
    ) -> Result<Session, SessionError> {
        let mut session = Session::empty_with(registry, settings);
        session.running = true;
        session.install(parse(text, registry))?;
        Ok(session)
    }

    /// A session with no elements.
    pub fn empty() -> Session {
        Self::empty_with(&Registry::standard(), AppSettings::default())
    }

    /// An empty session using `registry` and `settings`.
    pub fn empty_with(registry: &Registry, settings: AppSettings) -> Session {
        let options = Self::default_options(&settings);
        let mut session = Session {
            sim: Simulation::new(Vec::new(), options.sim_settings()),
            options,
            adjustables: Vec::new(),
            slider_positions: Vec::new(),
            running: false,
            draw_ctx: DrawCtx::default(),
            load_warnings: Vec::new(),
            pause_seen: 0,
            load_errors: Vec::new(),
            stop_message: None,
            stop_element: None,
            stop_reason: None,
            worker: None,
            scope_views: Vec::new(),
            scope_scales: crate::render::scopes::ScopeScales::default(),
            snapshot_time: 0.0,
            view: View::default(),
            canvas_size: (0.0, 0.0),
            fit_pending: true,
            settings,
            settings_path: None,
            registry: registry.clone(),
            title: String::new(),
            path: None,
            dirty: false,
            tool: Tool::Select,
            clipboard: None,
            pointer: None,
            drag: Drag::None,
            undo: Vec::new(),
            redo: Vec::new(),
            extras: Extras::default(),
            #[cfg(feature = "subcircuits")]
            contexts: Vec::new(),
            inspect: Vec::new(),
            inspect_views: Vec::new(),
            held: None,
            hover: None,
            step_debt: 0.0,
        };
        session.refresh_draw_ctx(Duration::ZERO);
        session
    }

    /// Display options of a new circuit, taken from the saved settings.
    pub(crate) fn default_options(settings: &AppSettings) -> CircuitOptions {
        CircuitOptions {
            show_dots: settings.show_dots,
            show_volts: settings.show_volts,
            show_power: settings.show_power,
            show_values: settings.show_values,
            small_grid: settings.small_grid,
            ..CircuitOptions::default()
        }
    }

    /// Replaces the circuit with a parsed one, resetting time and the editing history.
    pub(crate) fn install(&mut self, parsed: ParseResult) -> Result<(), SessionError> {
        self.inspect.clear();
        self.inspect_views.clear();
        let c = parsed.circuit;
        self.options = c.options;
        crate::elements::geometry::set_grid_size(self.grid_size());
        self.adjustables = c.adjustables;
        self.extras = Extras {
            hint: c.hint,
            scope_lines: c.scope_lines,
            scope_xml: c.scope_xml,
            model_xml: c.model_xml,
            model_lines: c.model_lines,
        };
        self.load_warnings = parsed.errors.iter().map(ToString::to_string).collect();
        self.load_errors = parsed.errors.clone();
        self.pause_seen = 0;
        self.sim = Simulation::new(c.elements, self.options.sim_settings());
        self.sim.scopes = crate::scope::scopes_from_circuit(
            &self.extras.scope_lines,
            &self.extras.scope_xml,
            &self.sim.elements,
        );
        self.store_scopes();
        self.scope_views.clear();
        self.scope_scales = crate::render::scopes::ScopeScales::default();
        self.undo.clear();
        self.redo.clear();
        self.drag = Drag::None;
        self.held = None;
        self.hover = None;
        self.clear_stop();
        self.step_debt = 0.0;
        self.dirty = false;
        self.fit_pending = true;
        // with a worker the analysis happens there and its error arrives in a snapshot
        let analysis = if self.worker.is_some() {
            Ok(())
        } else {
            self.sim.analyze().map_err(SessionError::from)
        };
        self.refresh_slider_positions();
        self.refresh_draw_ctx(Duration::ZERO);
        self.snapshot_time = 0.0;
        self.sync_worker(false);
        analysis
    }

    /// Recomputes every slider position from the value of its element field.
    pub fn refresh_slider_positions(&mut self) {
        self.slider_positions = self
            .adjustables
            .iter()
            .map(|a| {
                self.sim
                    .elements
                    .get(a.elm)
                    .and_then(|e| e.get_edit_info(a.edit_item))
                    .map_or(0.0, |ei| f64::from(a.value_to_position(ei.value)))
            })
            .collect();
    }

    /// The circuit as the XML text the original writes.
    pub fn circuit_xml(&mut self) -> String {
        self.pull_state();
        self.document_xml()
    }

    /// The document as XML without first fetching the live state from the worker, for
    /// sending the document itself to the worker.
    pub(crate) fn document_xml(&mut self) -> String {
        let elements = std::mem::take(&mut self.sim.elements);
        let circuit = Circuit {
            elements,
            options: self.options.clone(),
            adjustables: self.adjustables.clone(),
            hint: self.extras.hint,
            scope_lines: self.extras.scope_lines.clone(),
            scope_xml: self.extras.scope_xml.clone(),
            model_xml: self.extras.model_xml.clone(),
            model_lines: self.extras.model_lines.clone(),
        };
        let text = crate::format::writer::write_xml(&circuit);
        self.sim.elements = circuit.elements;
        text
    }

    /// Time steps per second of wall time at the circuit's speed setting.
    pub fn step_rate(&self) -> f64 {
        160.0 * self.options.iter_count
    }

    /// Refreshes the drawing settings without advancing the current-dot speed.
    pub(crate) fn refresh_draw_ctx_now(&mut self) {
        self.refresh_draw_ctx(Duration::ZERO);
    }

    pub(crate) fn refresh_draw_ctx(&mut self, elapsed: Duration) {
        let sim_time = self.time();
        let o = &self.options;
        let c = &mut self.draw_ctx;
        c.show_volts = o.show_volts;
        c.show_values = o.show_values;
        c.show_power = o.show_power;
        c.show_dots = o.show_dots;
        c.small_grid = o.small_grid;
        c.voltage_range = o.voltage_range;
        c.sim_running = self.running;
        c.power_mult = (f64::from(o.power_bar) / 4.762 - 7.0).exp();
        let st = &self.settings;
        c.euro_resistors = st.euro_resistors;
        c.show_ohm = st.show_ohm;
        c.iec_gates = st.iec_gates;
        c.printable = st.white_background;
        c.white = if st.white_background { Color::BLACK } else { Color::WHITE };
        c.light_gray = if st.white_background { Color::BLACK } else { Color::LIGHT_GRAY };
        c.select_color = Color::CYAN;
        if self.running {
            let ms = elapsed.as_secs_f64() * 1000.0;
            let mult = 1.7 * ms * (f64::from(o.current_bar) / 3.5 - 14.2).exp();
            c.current_mult = if st.conventional_current { mult } else { -mult };
        }
        c.sim_time = sim_time;
    }

    /// Advances the simulation by the wall time `elapsed` since the last call and refreshes
    /// the drawing settings. Does nothing to time while stopped.
    pub fn update(&mut self, elapsed: Duration) -> Result<(), StepError> {
        if self.worker.is_some() {
            return self.pump(elapsed);
        }
        let elapsed = elapsed.min(MAX_FRAME_GAP);
        if self.running {
            self.step_debt += self.step_rate() * elapsed.as_secs_f64();
            let steps = self.step_debt.floor();
            self.step_debt -= steps;
            if steps >= 1.0 {
                self.sim.set_delay_wire_processing(true);
                let before = self.sim.pause_count;
                let result = self.sim.run_steps(steps as usize);
                self.sim.calc_wire_currents();
                if self.sim.pause_count != before {
                    // a stop trigger: time stops, with no error
                    self.set_running(false);
                }
                if let Err(e) = result {
                    let element = match &e {
                        StepError::Stopped { element, .. } => *element,
                        _ => None,
                    };
                    self.record_stop(&e.to_string(), element);
                    self.refresh_draw_ctx(elapsed);
                    return Err(e);
                }
            }
            if let Some(stop) = self.sim.stop_request() {
                let (message, element) = (stop.message.clone(), stop.element);
                self.record_stop(&message, element);
            }
        }
        self.highlight_stop_element();
        self.refresh_draw_ctx(elapsed);
        Ok(())
    }

    /// Resets time and element state, as the Reset button does.
    pub fn reset(&mut self) {
        tracing::info!("simulation reset");
        self.sim.reset();
        if let Some(w) = &self.worker {
            w.send(Command::Reset);
        }
        self.clear_stop();
        self.step_debt = 0.0;
        self.running = true;
        self.refresh_draw_ctx(Duration::ZERO);
    }

    /// Sets the simulation speed from the slider position (0 to 260, as in the original).
    pub fn set_speed_bar(&mut self, bar: f64) {
        self.options.iter_count = 0.1 * ((bar - 61.0) / 24.0).exp();
        if let Some(w) = &self.worker {
            w.send(Command::SetSpeed(self.options.iter_count));
        }
    }

    /// Sets the current-dot speed slider position (0 to 100).
    pub fn set_current_bar(&mut self, bar: f64) {
        self.options.current_bar = bar as i32;
    }

    /// Re-analyses after the circuit changed; a failure stops the simulation with a message.
    pub(crate) fn reanalyze(&mut self) {
        if self.worker.is_some() {
            self.sync_worker(true);
            return;
        }
        match self.sim.analyze() {
            Ok(()) => self.clear_stop(),
            Err(AnalysisError::Invalid { message, element }) => self.record_stop(&message, element),
        }
    }

    /// Stops the simulation because of `message`, naming and highlighting the element at
    /// index `element` when it is known (contract C5).
    pub fn record_stop(&mut self, message: &str, element: Option<usize>) {
        let text = match element.filter(|i| *i < self.sim.elements.len()) {
            Some(i) => {
                let b = self.sim.elements[i].base();
                format!("{message} ({} em {}, {})", self.element_title(i), b.x, b.y)
            }
            None => message.to_string(),
        };
        tracing::warn!(element = ?element, "simulation stopped: {text}");
        self.stop_message = Some(text);
        self.stop_reason = Some(message.to_string());
        self.stop_element = element;
        self.running = false;
        self.highlight_stop_element();
    }

    /// Forgets the error and removes its highlight.
    pub fn clear_stop(&mut self) {
        if let Some(e) = self.stop_element.take().and_then(|i| self.sim.elements.get_mut(i)) {
            e.base_mut().highlight = false;
        }
        self.stop_message = None;
        self.stop_reason = None;
    }

    pub(crate) fn highlight_stop_element(&mut self) {
        if let Some(e) = self.stop_element.and_then(|i| self.sim.elements.get_mut(i)) {
            e.base_mut().highlight = true;
        }
    }

    /// Starts or stops time.
    pub fn set_running(&mut self, running: bool) {
        tracing::info!(running, "run state changed");
        self.running = running;
        if let Some(w) = &self.worker {
            w.send(Command::SetRunning(running));
        }
        if running {
            self.clear_stop();
        }
    }

    /// Moves slider `i` to `pos` (0 to 100) and applies the value to its element. Returns the
    /// new value.
    pub fn set_slider(&mut self, i: usize, pos: f64) -> Option<f64> {
        let a = self.adjustables.get(i)?.clone();
        let value = a.position_to_value(pos);
        let mut ei = self.sim.elements.get(a.elm)?.get_edit_info(a.edit_item)?;
        ei.value = value;
        self.sim.apply_edit(a.elm, a.edit_item, &mut ei);
        self.sim.elements[a.elm].set_points();
        self.param_changed(a.elm, a.edit_item, value);
        if self.stop_element.is_some() && self.worker.is_none() {
            return None;
        }
        if let Some(p) = self.slider_positions.get_mut(i) {
            *p = pos;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n38 2 0 1e-7 1e-5 Capacitance\n";

    fn session() -> Session {
        Session::load(RC, &Registry::standard()).unwrap()
    }

    #[test]
    fn speed_bar_covers_the_original_range() {
        let mut s = session();
        // the file's speed lies beyond position 100, so the slider needs the original 0 to 260
        let bar = f64::from(s.options.speed_bar());
        assert!(bar > 100.0, "{bar}");
        s.set_speed_bar(bar);
        assert!((s.options.iter_count - 10.0).abs() < 0.3, "{}", s.options.iter_count);
        s.set_speed_bar(260.0);
        assert!(s.step_rate() > 5e4);
        s.set_speed_bar(0.0);
        assert!(s.step_rate() < 2.0);
    }

    #[test]
    fn step_rate_follows_the_speed_bar() {
        let s = session();
        // header iteration count 10
        assert!((s.step_rate() - 1600.0).abs() < 1e-9);
    }

    #[test]
    fn update_advances_time_in_proportion_to_wall_time() {
        let mut s = session();
        s.update(Duration::from_millis(16)).unwrap();
        let t1 = s.sim.time();
        assert!(t1 > 0.0);
        s.update(Duration::from_millis(16)).unwrap();
        assert!(s.sim.time() > t1);
        s.set_running(false);
        let t = s.sim.time();
        s.update(Duration::from_millis(16)).unwrap();
        assert_eq!(s.sim.time(), t);
    }

    #[test]
    fn long_gaps_are_clipped() {
        let mut a = session();
        let mut b = session();
        a.update(Duration::from_secs(10)).unwrap();
        b.update(MAX_FRAME_GAP).unwrap();
        assert_eq!(a.sim.time(), b.sim.time());
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "reactive")]
    #[cfg(feature = "sources")]
    #[test]
    fn slider_changes_the_element_value() {
        let mut s = session();
        assert_eq!(s.adjustables.len(), 1);
        let v = s.set_slider(0, 100.0).unwrap();
        assert!((v - 1e-5).abs() < 1e-12);
        let info = s.sim.elements[2].get_edit_info(0).unwrap();
        assert!((info.value - 1e-5).abs() < 1e-12);
    }

    fn lrc() -> Session {
        let text = std::fs::read_to_string("assets/circuits/lrc.txt").unwrap();
        Session::load(&text, &Registry::standard()).unwrap()
    }

    #[cfg(feature = "switches")]
    fn open_switch(s: &Session) -> i32 {
        s.sim.elements[1]
            .as_any()
            .downcast_ref::<crate::elements::switches::SwitchElm>()
            .unwrap()
            .position
    }

    #[cfg(feature = "passive")]
    #[test]
    #[cfg(feature = "switches")]
    fn clicking_the_switch_closes_it_and_keeps_the_circuit_state() {
        let mut s = lrc();
        s.update(Duration::from_millis(30)).unwrap();
        assert_eq!(open_switch(&s), 1);
        let vc = s.sim.elements[3].get_voltage_diff();
        // identity view: surface pixels are circuit coordinates; the blade sits above the lead
        assert!(s.press_at(416.0, 80.0));
        assert_eq!(open_switch(&s), 0);
        assert!(s.sim.elements[1].is_wire_equivalent());
        assert_eq!(s.sim.elements[3].get_voltage_diff(), vc, "analysis must not reset state");
        s.update(Duration::from_millis(30)).unwrap();
        assert!(s.stop_message.is_none());
        // clicking away from every element does nothing
        assert!(!s.press_at(300.0, 200.0));
        assert!(s.press_at(416.0, 80.0));
        assert_eq!(open_switch(&s), 1);
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "switches")]
    #[test]
    fn hovering_highlights_the_element_under_the_mouse() {
        let mut s = lrc();
        s.hover_at(Some((416.0, 80.0)));
        assert!(s.sim.elements[1].base().highlight);
        s.hover_at(None);
        assert!(!s.sim.elements[1].base().highlight);
    }

    #[test]
    fn reset_returns_time_to_zero() {
        let mut s = session();
        s.update(Duration::from_millis(50)).unwrap();
        s.reset();
        assert_eq!(s.sim.time(), 0.0);
    }
}
