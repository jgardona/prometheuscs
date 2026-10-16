// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CirSim, CircuitLoader, Adjustable).
//! The in-memory circuit: elements plus the settings stored in a circuit file.

use crate::elements::element::Element;
use crate::format::xml::XmlNode;
use crate::solver::sim::{SimSettings, SolverType};

/// Settings stored in the header of a circuit file.
#[derive(Clone, Debug, PartialEq)]
pub struct CircuitOptions {
    /// Draw moving current dots.
    pub show_dots: bool,
    /// Use the small grid.
    pub small_grid: bool,
    /// Colour wires by voltage.
    pub show_volts: bool,
    /// Colour elements by power.
    pub show_power: bool,
    /// Show component values.
    pub show_values: bool,
    /// Reduce the time step when convergence fails.
    pub adjust_time_step: bool,
    /// Find the DC operating point whenever the circuit is reset.
    pub auto_dc_on_reset: bool,
    /// Nominal time step in seconds.
    pub max_time_step: f64,
    /// Smallest time step in seconds.
    pub min_time_step: f64,
    /// Simulation speed as the original's iteration count factor.
    pub iter_count: f64,
    /// Current-speed slider position (0 to 100).
    pub current_bar: i32,
    /// Voltage at the ends of the colour scale.
    pub voltage_range: f64,
    /// Power-brightness slider position (0 to 100).
    pub power_bar: i32,
    /// Linear solver choice.
    pub solver: SolverType,
}

impl Default for CircuitOptions {
    fn default() -> Self {
        CircuitOptions {
            show_dots: false,
            small_grid: false,
            show_volts: true,
            show_power: false,
            show_values: true,
            adjust_time_step: false,
            auto_dc_on_reset: false,
            max_time_step: 5e-6,
            min_time_step: 50e-12,
            iter_count: 0.1 * ((117.0 - 61.0) / 24.0f64).exp(),
            current_bar: 50,
            voltage_range: 5.0,
            power_bar: 50,
            solver: SolverType::Auto,
        }
    }
}

impl CircuitOptions {
    /// The option bits of the file header.
    pub fn flag_bits(&self) -> i32 {
        i32::from(self.show_dots)
            | (i32::from(self.small_grid) << 1)
            | (i32::from(!self.show_volts) << 2)
            | (i32::from(self.show_power) << 3)
            | (i32::from(!self.show_values) << 4)
            | (i32::from(self.adjust_time_step) << 6)
            | (i32::from(self.auto_dc_on_reset) << 7)
    }

    /// Applies the option bits of a file header.
    pub fn set_flag_bits(&mut self, f: i32) {
        self.show_dots = f & 1 != 0;
        self.small_grid = f & 2 != 0;
        self.show_volts = f & 4 == 0;
        self.show_power = f & 8 == 8;
        self.show_values = f & 16 == 0;
        self.adjust_time_step = f & 64 != 0;
        self.auto_dc_on_reset = f & 128 != 0;
    }

    /// Speed slider position corresponding to `iter_count`.
    pub fn speed_bar(&self) -> i32 {
        ((10.0 * self.iter_count).ln() * 24.0 + 61.5) as i32
    }

    /// Solver settings derived from these options.
    pub fn sim_settings(&self) -> SimSettings {
        SimSettings {
            max_time_step: self.max_time_step,
            min_time_step: self.min_time_step,
            adjust_time_step: self.adjust_time_step,
            solver: self.solver,
        }
    }
}

/// A slider the user attached to one editable field of an element.
#[derive(Clone, Debug, PartialEq)]
pub struct Adjustable {
    /// Index of the element in the circuit.
    pub elm: usize,
    /// Index of the field in the element's edit list.
    pub edit_item: usize,
    /// Value at the left end.
    pub min_value: f64,
    /// Value at the right end.
    pub max_value: f64,
    /// Label above the slider.
    pub slider_text: String,
    /// Step size (0 = continuous).
    pub slider_step: f64,
    /// Slider is logarithmic.
    pub logarithmic: bool,
    /// Index of another adjustable whose slider this one shares.
    pub shared_slider: Option<usize>,
}

impl Adjustable {
    /// Converts a value to a slider position from 0 to 100.
    pub fn value_to_position(&self, value: f64) -> i32 {
        if self.logarithmic && self.min_value > 0.0 {
            let (lo, hi) = (self.min_value.ln(), self.max_value.ln());
            return ((value.ln() - lo) / (hi - lo) * 100.0) as i32;
        }
        ((value - self.min_value) * 100.0 / (self.max_value - self.min_value)) as i32
    }

    /// Converts a slider position (0 to 100) to a value, honouring the step size.
    pub fn position_to_value(&self, pos: f64) -> f64 {
        let mut result = if self.logarithmic && self.min_value > 0.0 {
            let (lo, hi) = (self.min_value.ln(), self.max_value.ln());
            (lo + (hi - lo) * pos / 100.0).exp()
        } else {
            self.min_value + (self.max_value - self.min_value) * pos / 100.0
        };
        if self.slider_step > 0.0 {
            result = self.min_value
                + ((result - self.min_value) / self.slider_step).round() * self.slider_step;
        }
        result
    }
}

/// A circuit: its elements and settings.
#[derive(Default)]
pub struct Circuit {
    /// The elements, in file order.
    pub elements: Vec<Box<dyn Element>>,
    /// Header settings.
    pub options: CircuitOptions,
    /// Sliders attached to element fields.
    pub adjustables: Vec<Adjustable>,
    /// Hint shown with example circuits: `(type, item1, item2)`.
    pub hint: Option<(i32, i32, i32)>,
    /// Scope entries kept verbatim until scopes are interpreted: text lines of the old format.
    pub scope_lines: Vec<String>,
    /// Scope entries kept verbatim: XML tags.
    pub scope_xml: Vec<XmlNode>,
    /// Model definitions kept verbatim (diode, transistor, custom logic, subcircuit models).
    pub model_xml: Vec<XmlNode>,
    /// Model definitions of the old text format, kept verbatim.
    pub model_lines: Vec<String>,
}

impl Circuit {
    /// An empty circuit with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}
