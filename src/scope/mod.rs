// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Scope).
//! Scope data: sample buffers and plots, read from circuit files and filled while the
//! simulation runs. Triggers, XY plots and the FFT are not implemented.

pub mod fft;

use crate::elements::element::{Element, ScopeSource};
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};

/// Value shown by a scope plot: power.
pub const VAL_POWER: i32 = 7;
/// Old numbering of power in files.
pub const VAL_POWER_OLD: i32 = 1;
/// Value shown by a scope plot: voltage.
pub const VAL_VOLTAGE: i32 = 0;
/// Value shown by a scope plot: current.
pub const VAL_CURRENT: i32 = 3;
/// Transistor base current.
pub const VAL_IB: i32 = 1;
/// Transistor collector current.
pub const VAL_IC: i32 = 2;
/// Transistor emitter current.
pub const VAL_IE: i32 = 3;
/// Transistor base-emitter voltage.
pub const VAL_VBE: i32 = 4;
/// Transistor base-collector voltage.
pub const VAL_VBC: i32 = 5;
/// Transistor collector-emitter voltage.
pub const VAL_VCE: i32 = 6;
/// Resistance.
pub const VAL_R: i32 = 2;
/// Charge.
pub const VAL_CHARGE: i32 = 8;

/// Units of a plotted value: volts.
pub const UNITS_V: i32 = 0;
/// Units: amps.
pub const UNITS_A: i32 = 1;
/// Units: watts.
pub const UNITS_W: i32 = 2;
/// Units: ohms.
pub const UNITS_OHMS: i32 = 3;
/// Units: coulombs.
pub const UNITS_C: i32 = 4;
/// Number of unit kinds.
pub const UNITS_COUNT: i32 = 5;

/// Samples kept per plot: enough for one sample per pixel of a wide scope, which is what the
/// original draws (it keeps the next power of two above the width).
pub const POINTS: usize = 2048;

/// The full-scale values never shrink.
pub const FLAG_MAX_SCALE: i32 = 8192;
/// Do not show the largest value (it is shown unless this bit is set).
pub const FLAG_NO_MAX: i32 = 4;
/// Show the frequency of the first plot.
pub const FLAG_FREQ: i32 = 8;
/// Show the smallest value.
pub const FLAG_MIN: i32 = 256;
/// Show the scale per division.
pub const FLAG_SCALE: i32 = 512;
/// Show the RMS value.
pub const FLAG_RMS: i32 = 16384;
/// Show the duty cycle.
pub const FLAG_DUTY: i32 = 32768;
/// Show the average.
pub const FLAG_AVERAGE: i32 = 1 << 17;
/// Show what the element would say about itself.
pub const FLAG_ELM_INFO: i32 = 1 << 20;
/// Show the peak-to-peak value.
pub const FLAG_P2P: i32 = 1 << 22;
/// Per-plot flag: the plot is AC coupled.
/// Flag: the plots are drawn against each other instead of against time (X/Y or V-I).
pub const FLAG_PLOT2D: i32 = 64;
/// Flag: with `FLAG_PLOT2D`, a free choice of axes (X/Y); without it, voltage against current.
pub const FLAG_PLOT_XY: i32 = 128;
/// Flag: the frequency spectrum is shown.
pub const FLAG_FFT: i32 = 1024;
/// Flag: the spectrum is in decibels.
pub const FLAG_LOG_SPECTRUM: i32 = 65536;
/// Flag: the phase angle between voltage and current is shown.
pub const FLAG_PHASE_ANGLE: i32 = 1 << 23;
const PLOT_FLAG_AC: i32 = 1;

/// The first scope flag: show the current plot of a default scope.
pub const FLAG_SHOW_I: i32 = 1;
/// Show the voltage plot of a default scope.
pub const FLAG_SHOW_V: i32 = 2;
/// The scale was set by hand.
pub const FLAG_MAN_SCALE: i32 = 16;
/// The line lists plots (new-style dump).
pub const FLAG_PLOTS: i32 = 4096;
/// The line holds a number of divisions.
pub const FLAG_DIVISIONS: i32 = 1 << 21;
/// The line holds per-plot flags.
pub const FLAG_PERPLOTFLAGS: i32 = 1 << 18;
/// The line holds per-plot manual scales.
pub const FLAG_PERPLOT_MAN_SCALE: i32 = 1 << 19;
/// The line holds trigger settings.
pub const FLAG_TRIGGER: i32 = 1 << 24;

/// Trigger mode: the display just scrolls.
pub const TRIGGER_FREERUN: i32 = 0;
/// Trigger mode: the display moves only when the trigger fires.
pub const TRIGGER_NORMAL: i32 = 1;
/// Trigger mode: like normal, but scrolls anyway when nothing triggers for a while.
pub const TRIGGER_AUTO: i32 = 2;
/// Trigger on the signal rising through the level.
pub const TRIGGER_EDGE_RISING: i32 = 0;
/// Trigger on the signal falling through the level.
pub const TRIGGER_EDGE_FALLING: i32 = 1;
/// The smallest manual scale.
const MIN_MAN_SCALE: f64 = 1e-9;
/// The number of divisions of a manual scale unless the file says otherwise.
pub const DEFAULT_DIVISIONS: i32 = 8;
/// Steps of the vertical position of a plot in manual mode (from the bottom to the top).
pub const V_POSITION_STEPS: i32 = 200;
/// Samples after a trigger during which the next one is ignored (the original uses the width
/// of the scope; this one does not know it).
const TRIGGER_HOLDOFF: u32 = 1024;
/// Samples without a trigger after which the auto mode runs free.
const TRIGGER_AUTO_TIMEOUT: u32 = 2 * 1024;

/// What the trigger is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerState {
    /// Waiting for the signal to cross the level.
    Armed,
    /// Fired: the display is held around the trigger sample.
    Triggered,
    /// Auto mode gave up waiting and the display scrolls.
    AutoRun,
}

/// The trigger of a scope: it holds the display so that a repeating signal stands still.
#[derive(Clone, Debug)]
pub struct Trigger {
    /// `TRIGGER_FREERUN`, `TRIGGER_NORMAL` or `TRIGGER_AUTO`.
    pub mode: i32,
    /// `TRIGGER_EDGE_RISING` or `TRIGGER_EDGE_FALLING`.
    pub edge: i32,
    /// Level the first plot has to cross.
    pub level: f64,
    state: TriggerState,
    prev_value: f64,
    holdoff: u32,
    waiting: bool,
    fired: bool,
    last_total: Option<u64>,
    at_total: u64,
    time: f64,
}

impl Default for Trigger {
    fn default() -> Self {
        Trigger {
            mode: TRIGGER_FREERUN,
            edge: TRIGGER_EDGE_RISING,
            level: 0.0,
            state: TriggerState::Armed,
            prev_value: 0.0,
            holdoff: 0,
            waiting: false,
            fired: false,
            last_total: None,
            at_total: 0,
            time: 0.0,
        }
    }
}

impl Trigger {
    /// True unless the scope runs free.
    pub fn is_active(&self) -> bool {
        self.mode != TRIGGER_FREERUN
    }

    /// True while the display is held around a trigger sample.
    pub fn is_triggered(&self) -> bool {
        self.is_active() && self.fired && self.state != TriggerState::AutoRun
    }

    fn reset(&mut self) {
        self.state = TriggerState::Armed;
        self.holdoff = 0;
        self.waiting = false;
        self.fired = false;
        self.last_total = None;
    }

    /// Looks at the newest sample of the first plot, once per new sample.
    fn check(&mut self, plot: &ScopePlot, t: f64) {
        if !self.is_active() || self.last_total == Some(plot.total) {
            return;
        }
        self.last_total = Some(plot.total);
        let val = 0.5 * (plot.max[plot.ptr] + plot.min[plot.ptr]);
        let crossing = if self.edge == TRIGGER_EDGE_RISING {
            self.prev_value < self.level && val >= self.level
        } else {
            self.prev_value > self.level && val <= self.level
        };
        let fire = |s: &mut Trigger| {
            s.state = TriggerState::Triggered;
            s.at_total = plot.total;
            s.time = t;
            s.holdoff = 0;
            s.fired = true;
        };
        match self.state {
            TriggerState::Armed => {
                if crossing {
                    fire(self);
                    self.waiting = false;
                } else {
                    self.waiting = true;
                    if self.mode == TRIGGER_AUTO {
                        self.holdoff += 1;
                        if self.holdoff >= TRIGGER_AUTO_TIMEOUT {
                            self.state = TriggerState::AutoRun;
                            self.waiting = false;
                        }
                    }
                }
            }
            TriggerState::Triggered => {
                self.holdoff += 1;
                if self.holdoff >= TRIGGER_HOLDOFF {
                    self.state = TriggerState::Armed;
                    self.holdoff = 0;
                }
            }
            TriggerState::AutoRun => {
                if crossing {
                    fire(self);
                }
            }
        }
        self.prev_value = val;
    }
}

/// One plotted quantity with its history of minimum and maximum values.
#[derive(Clone, Debug)]
pub struct ScopePlot {
    /// Index of the element plotted.
    pub elm: usize,
    /// Which value of the element (`VAL_*`).
    pub value: i32,
    /// Units of the value (`UNITS_*`).
    pub units: i32,
    min: Vec<f64>,
    max: Vec<f64>,
    ptr: usize,
    last_update: f64,
    /// Samples recorded since the history was emptied (the one being filled included).
    filled: usize,
    /// Samples started since the history was emptied; it keeps counting past the buffer.
    total: u64,
    /// Units per division in manual mode (meaningful once `man_set`).
    pub man_scale: f64,
    /// Vertical position in manual mode, in `V_POSITION_STEPS` of the height.
    pub man_pos: i32,
    /// The manual scale was chosen (by the user or the file) and not just suggested.
    pub man_set: bool,
    /// The plot shows its voltage with the DC level removed.
    pub ac_coupled: bool,
    /// State of the high-pass filter behind AC coupling: its last output and last input.
    ac_last_out: f64,
    ac_last_in: f64,
}

impl ScopePlot {
    fn new(elm: usize, value: i32, units: i32) -> Self {
        ScopePlot {
            elm,
            value,
            units,
            min: vec![0.0; POINTS],
            max: vec![0.0; POINTS],
            ptr: 0,
            last_update: 0.0,
            filled: 0,
            total: 0,
            man_scale: 0.0,
            man_pos: 0,
            man_set: false,
            ac_coupled: false,
            ac_last_out: 0.0,
            ac_last_in: 0.0,
        }
    }

    /// AC coupling only makes sense for voltages.
    pub fn can_ac_couple(&self) -> bool {
        self.units == UNITS_V
    }

    fn clear(&mut self, t: f64) {
        self.min.fill(0.0);
        self.max.fill(0.0);
        self.ptr = 0;
        self.last_update = t;
        self.filled = 0;
        self.total = 0;
    }
}

/// A way a scope can draw its plots besides against time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeMode {
    /// One plot against another, chosen freely.
    Xy,
    /// Voltage against current.
    VoltageCurrent,
    /// The frequency spectrum of the first plot.
    Spectrum,
}

/// What an X/Y (or V-I) plot draws: which plots are the axes and which ones modulate the
/// brightness and the colour of the trace. Plots are numbered from 0; -1 means none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XyAxes {
    /// Plot on the horizontal axis.
    pub x: i32,
    /// Plot on the vertical axis.
    pub y: i32,
    /// Plot that sets the brightness of the trace.
    pub brightness: i32,
    /// Plot that sets the red part of the colour.
    pub red: i32,
    /// Plot that sets the green part of the colour.
    pub green: i32,
    /// Plot that sets the blue part of the colour.
    pub blue: i32,
    /// Trail persistence, in time steps (0: the trace stays until it scrolls away).
    pub trail: i32,
}

impl Default for XyAxes {
    fn default() -> Self {
        XyAxes { x: 0, y: 1, brightness: -1, red: -1, green: -1, blue: -1, trail: 0 }
    }
}

impl XyAxes {
    /// The plot an axis index points at, falling back to `default` when it points nowhere.
    pub fn valid(index: i32, default: usize, plots: usize) -> usize {
        match usize::try_from(index) {
            Ok(i) if i < plots => i,
            _ => default.min(plots.saturating_sub(1)),
        }
    }

    /// A modulating plot, if the index points at one.
    pub fn modulator(index: i32, plots: usize) -> Option<usize> {
        usize::try_from(index).ok().filter(|i| *i < plots)
    }
}

/// A scope: one or more plots drawn together.
#[derive(Clone, Debug)]
pub struct Scope {
    /// Time steps per sample.
    pub speed: i32,
    /// Flags of the file entry.
    pub flags: i32,
    /// Order among the scopes.
    pub position: i32,
    /// Full-scale value for each units (`UNITS_*`).
    pub scale: [f64; UNITS_COUNT as usize],
    /// Text shown on the scope.
    pub text: Option<String>,
    /// The plots.
    pub plots: Vec<ScopePlot>,
    /// Plot against each other instead of against time (X/Y when `plot_xy`, else V-I).
    pub xy: bool,
    /// The plot against each other has a free choice of axes (X/Y).
    pub plot_xy: bool,
    /// What the X/Y plot draws.
    pub axes: XyAxes,
    /// Show the frequency spectrum of the first plot.
    pub fft: bool,
    /// Number of vertical divisions in manual mode.
    pub man_divisions: i32,
    /// The trigger.
    pub trigger: Trigger,
}

/// The first value of the 1-2-5 series above `d`.
pub fn next_highest_scale(d: f64) -> f64 {
    const MULT: [f64; 3] = [2.0, 2.5, 2.0];
    let d = d * 1.001;
    let mut s = MIN_MAN_SCALE;
    let mut i = 0;
    while s < d {
        s *= MULT[i % 3];
        i += 1;
    }
    s
}

fn import_dec_or_hex(s: &str) -> Option<i32> {
    match s.strip_prefix('x') {
        Some(h) => i32::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    }
}

fn element_units(elements: &[Box<dyn Element>], elm: usize, value: i32) -> Option<i32> {
    elements.get(elm).map(|e| e.scope_units(value))
}

impl Scope {
    fn blank(speed: i32, flags: i32, position: i32) -> Self {
        Scope {
            speed: speed.max(1),
            flags,
            position,
            scale: [0.5; UNITS_COUNT as usize],
            text: None,
            plots: Vec::new(),
            xy: flags & FLAG_PLOT2D != 0,
            plot_xy: flags & FLAG_PLOT_XY != 0,
            axes: XyAxes::default(),
            fft: flags & FLAG_FFT != 0,
            man_divisions: DEFAULT_DIVISIONS,
            trigger: Trigger::default(),
        }
    }

    /// Whether the scope is drawn in `mode`.
    pub fn has_mode(&self, mode: ScopeMode) -> bool {
        match mode {
            ScopeMode::Xy => self.xy && self.plot_xy,
            ScopeMode::VoltageCurrent => self.xy && !self.plot_xy,
            ScopeMode::Spectrum => self.fft,
        }
    }

    /// Turns `mode` on or off. An X/Y or V-I plot starts with the first plot on the horizontal
    /// axis and the second on the vertical one, and no modulation.
    pub fn set_mode(&mut self, mode: ScopeMode, on: bool) {
        match mode {
            ScopeMode::Spectrum => self.fft = on,
            ScopeMode::Xy | ScopeMode::VoltageCurrent => {
                self.xy = on;
                self.plot_xy = mode == ScopeMode::Xy;
                if on {
                    self.axes = XyAxes {
                        y: 1.min(self.plots.len().saturating_sub(1)) as i32,
                        trail: self.axes.trail,
                        ..Default::default()
                    };
                }
            }
        }
    }

    /// The flags of the file entry: `flags` with the modes this scope is in.
    pub fn file_flags(&self) -> i32 {
        let modes =
            [(self.xy, FLAG_PLOT2D), (self.xy && self.plot_xy, FLAG_PLOT_XY), (self.fft, FLAG_FFT)];
        let mut f = self.flags & !(FLAG_PLOT2D | FLAG_PLOT_XY | FLAG_FFT);
        for (on, bit) in modes {
            if on {
                f |= bit;
            }
        }
        f
    }

    /// True when the plots have their own scales (manual mode).
    pub fn is_manual(&self) -> bool {
        self.flags & FLAG_MAN_SCALE != 0
    }

    /// The scale per division the full-scale value of `units` suggests. A rounded-up value
    /// is what the user gets on switching to manual mode; an exact one keeps the look of an
    /// old file.
    pub fn man_scale_from_max(&self, units: i32, round_up: bool) -> f64 {
        let mut s = self.scale[units as usize];
        if units > UNITS_A {
            s *= 0.5;
        }
        let d = 2.0 * s / f64::from(self.man_divisions.max(1));
        if round_up { next_highest_scale(d) } else { d }
    }

    /// Switches manual mode on or off; the plots that have no scale of their own get one.
    pub fn set_manual(&mut self, on: bool) {
        if on {
            self.flags |= FLAG_MAN_SCALE;
            let marks: Vec<f64> =
                self.plots.iter().map(|p| self.man_scale_from_max(p.units, true)).collect();
            for (p, m) in self.plots.iter_mut().zip(marks) {
                if !p.man_set {
                    p.man_scale = m;
                    p.man_pos = 0;
                    p.man_set = true;
                }
            }
        } else {
            self.flags &= !FLAG_MAN_SCALE;
        }
    }

    /// Reads a scope from an `o` line of the text format (after the `o`).
    pub fn from_line(st: &mut Tokenizer<'_>, elements: &[Box<dyn Element>]) -> Option<Scope> {
        let elm = st.next_i32().ok()?;
        if elm < 0 || elm as usize >= elements.len() {
            return None;
        }
        let elm = elm as usize;
        let speed = st.next_i32().ok()?;
        let mut value = st.next_i32().ok()?;
        let flags = import_dec_or_hex(st.next_token()?)?;
        let is_transistor = elements[elm].class_name() == "TransistorElm";
        if !is_transistor && value == VAL_POWER_OLD {
            value = VAL_POWER;
        }
        let scale_v = st.next_f64().ok()?;
        let scale_a = st.next_f64().ok()?;
        let mut sc = Scope::blank(speed, flags, 0);
        let v = if scale_v == 0.0 { 0.5 } else { scale_v };
        let a = if scale_a == 0.0 { 1.0 } else { scale_a };
        sc.scale = [v, a, v, v, v];
        if flags & FLAG_PLOTS != 0 {
            sc.position = st.next_i32().ok()?;
            let count = st.next_i32().ok()?;
            if flags & FLAG_DIVISIONS != 0 {
                sc.man_divisions = st.next_i32().ok()?;
            }
            let u = element_units(elements, elm, value)?;
            if u > UNITS_A {
                sc.scale[u as usize] = st.next_f64().ok()?;
            }
            let has_plot_flags = flags & FLAG_PERPLOTFLAGS != 0;
            let mut plot_flags = 0;
            for i in 0..count {
                if has_plot_flags {
                    plot_flags = i32::from_str_radix(st.next_token()?, 16).ok()?;
                }
                let (e, val) = if i == 0 {
                    (elm, value)
                } else {
                    let ne = st.next_i32().ok()?;
                    let val = st.next_i32().ok()?;
                    (usize::try_from(ne).ok()?, val)
                };
                let u = element_units(elements, e, val)?;
                if i != 0 && u > UNITS_A {
                    sc.scale[u as usize] = st.next_f64().ok()?;
                }
                let mut plot = ScopePlot::new(e, val, u);
                plot.ac_coupled = plot_flags & PLOT_FLAG_AC != 0 && plot.can_ac_couple();
                if flags & FLAG_PERPLOT_MAN_SCALE != 0 {
                    plot.man_scale = st.next_f64().ok()?;
                    plot.man_pos = st.next_i32().ok()?;
                    plot.man_set = true;
                }
                sc.plots.push(plot);
            }
            let mut text = Vec::new();
            while let Some(t) = st.next_token() {
                text.push(t.to_string());
            }
            if !text.is_empty() {
                sc.text = Some(text.join(" "));
            }
        } else {
            // old-style entries: the value, plus the current when both are shown
            sc.position = st.next_i32().unwrap_or(0);
            let u = element_units(elements, elm, value)?;
            sc.plots.push(ScopePlot::new(elm, value, u));
            if value == VAL_VOLTAGE && flags & FLAG_SHOW_I != 0 {
                let cu = element_units(elements, elm, VAL_CURRENT)?;
                sc.plots.push(ScopePlot::new(elm, VAL_CURRENT, cu));
            }
        }
        Some(sc)
    }

    /// Reads a scope from an `o` tag of the XML format.
    pub fn from_xml(node: &XmlNode, elements: &[Box<dyn Element>]) -> Option<Scope> {
        let elm = usize::try_from(node.int_attr("en", -1)).ok().filter(|e| *e < elements.len())?;
        let flags = import_dec_or_hex(&node.string_attr("f", "0")).unwrap_or(0);
        let mut sc = Scope::blank(node.int_attr("sp", 64), flags, node.int_attr("p", 0));
        sc.text = node.attr("x").map(str::to_string);
        // older files of this program kept the modes in attributes of their own
        if node.int_attr("xy", 0) != 0 {
            sc.xy = true;
            sc.plot_xy = true;
        }
        sc.fft |= node.int_attr("fft", 0) != 0;
        sc.axes = XyAxes {
            x: node.int_attr("xy2x", 0),
            y: node.int_attr("xy2y", 1),
            brightness: node.int_attr("xy2br", -1),
            red: node.int_attr("xy2r", -1),
            green: node.int_attr("xy2g", -1),
            blue: node.int_attr("xy2b", -1),
            trail: node.int_attr("tp", 0),
        };
        sc.man_divisions = node.int_attr("md", DEFAULT_DIVISIONS);
        sc.trigger.mode = node.int_attr("triggerMode", TRIGGER_FREERUN);
        sc.trigger.edge = node.int_attr("triggerEdge", TRIGGER_EDGE_RISING);
        sc.trigger.level = node.double_attr("triggerLevel", 0.0);
        for p in node.children.iter().filter(|c| c.tag == "p") {
            let e = usize::try_from(p.int_attr("e", elm as i32))
                .ok()
                .filter(|e| *e < elements.len())?;
            let value = p.int_attr("v", 0);
            let u = element_units(elements, e, value)?;
            sc.scale[u as usize] = p.double_attr("sc", sc.scale[u as usize]);
            let mut plot = ScopePlot::new(e, value, u);
            let pf = i32::from_str_radix(&p.string_attr("f", "0"), 16).unwrap_or(0);
            plot.ac_coupled = pf & PLOT_FLAG_AC != 0 && plot.can_ac_couple();
            let ms = p.double_attr("ms", -1.0);
            if ms >= 0.0 {
                plot.man_scale = ms;
                plot.man_pos = p.int_attr("mp", 0);
                plot.man_set = true;
            }
            sc.plots.push(plot);
        }
        (!sc.plots.is_empty()).then_some(sc)
    }

    /// A scope showing element `elm`: its voltage, and its current when it has two posts.
    /// None when the element cannot be viewed in a scope.
    pub fn for_element(elm: usize, elements: &[Box<dyn Element>], position: i32) -> Option<Scope> {
        let e = elements.get(elm)?;
        if !e.can_view_in_scope() {
            return None;
        }
        let mut sc = Scope::blank(64, FLAG_SHOW_I | FLAG_SHOW_V, position);
        let first = e.default_scope_value();
        sc.plots.push(ScopePlot::new(elm, first, e.scope_units(first)));
        if first == VAL_VOLTAGE && e.post_count() == 2 {
            sc.plots.push(ScopePlot::new(elm, VAL_CURRENT, e.scope_units(VAL_CURRENT)));
        }
        Some(sc)
    }

    /// The values the scope's element offers, with whether each is plotted now: voltage,
    /// current and power where they make sense, then the element's own values.
    pub fn menu_items(&self, elements: &[Box<dyn Element>]) -> Vec<(i32, &'static str, bool)> {
        let Some(e) = self.plots.first().and_then(|p| elements.get(p.elm)) else {
            return Vec::new();
        };
        let extras = e.scope_values();
        let mut values = Vec::new();
        if e.default_scope_value() == VAL_VOLTAGE {
            values.push(VAL_VOLTAGE);
        }
        if e.post_count() == 2 {
            values.push(VAL_CURRENT);
            values.push(VAL_POWER);
        }
        for v in extras {
            if !values.contains(&v) {
                values.push(v);
            }
        }
        let elm = self.plots[0].elm;
        // the numbers of a transistor's values overlap those of current and resistance
        let transistor = e.default_scope_value() == VAL_VCE;
        values
            .into_iter()
            .map(|v| {
                (
                    v,
                    value_menu_key(v, transistor),
                    self.plots.iter().any(|p| p.elm == elm && p.value == v),
                )
            })
            .collect()
    }

    /// Plots (or stops plotting) `value` of the scope's element. The last plot stays.
    pub fn set_value(&mut self, value: i32, on: bool, elements: &[Box<dyn Element>]) {
        let Some(elm) = self.plots.first().map(|p| p.elm) else { return };
        let has = self.plots.iter().any(|p| p.elm == elm && p.value == value);
        if on && !has {
            if let Some(e) = elements.get(elm) {
                self.plots.push(ScopePlot::new(elm, value, e.scope_units(value)));
            }
        } else if !on && has && self.plots.len() > 1 {
            self.plots.retain(|p| !(p.elm == elm && p.value == value));
        }
    }

    /// The scope as the `o` tag of the XML format.
    pub fn to_xml(&self) -> XmlOut {
        let first = self.plots.first().map_or(0, |p| p.elm);
        let mut o = XmlOut::new("o");
        o.int_attr("en", first as i32);
        o.int_attr("sp", self.speed);
        let f = self.file_flags()
            & !(FLAG_PERPLOTFLAGS | FLAG_PERPLOT_MAN_SCALE | FLAG_PLOTS | FLAG_TRIGGER);
        o.str_attr("f", &if f >= 0 { format!("x{f:x}") } else { f.to_string() });
        o.int_attr("p", self.position);
        if let Some(t) = &self.text {
            o.str_attr("x", t);
        }
        if self.xy && self.plot_xy {
            let d = XyAxes::default();
            let a = &self.axes;
            for (name, v, default) in [
                ("xy2x", a.x, d.x),
                ("xy2y", a.y, d.y),
                ("xy2br", a.brightness, -1),
                ("xy2r", a.red, -1),
                ("xy2g", a.green, -1),
                ("xy2b", a.blue, -1),
            ] {
                if v != default {
                    o.int_attr(name, v);
                }
            }
        }
        if self.axes.trail != 0 {
            o.int_attr("tp", self.axes.trail);
        }
        if self.man_divisions != DEFAULT_DIVISIONS {
            o.int_attr("md", self.man_divisions);
        }
        if self.trigger.is_active() {
            o.int_attr("triggerMode", self.trigger.mode);
            o.int_attr("triggerEdge", self.trigger.edge);
            o.double_attr("triggerLevel", self.trigger.level);
        }
        for p in &self.plots {
            let mut c = XmlOut::new("p");
            if p.elm != first {
                c.int_attr("e", p.elm as i32);
            }
            if p.ac_coupled {
                c.str_attr("f", &format!("{PLOT_FLAG_AC:x}"));
            }
            c.int_attr("v", p.value);
            c.double_attr("sc", self.scale[p.units as usize]);
            if self.is_manual() {
                let ms =
                    if p.man_set { p.man_scale } else { self.man_scale_from_max(p.units, false) };
                c.double_attr("ms", ms);
                c.int_attr("mp", p.man_pos);
            }
            o.child(c);
        }
        o
    }

    /// Adds the plots of `other` to this scope (the scales follow the larger of the two).
    pub fn combine(&mut self, other: Scope) {
        for (a, b) in self.scale.iter_mut().zip(other.scale) {
            *a = a.max(b);
        }
        self.plots.extend(other.plots);
    }

    /// One scope per plot, each with the speed, flags and scales of this one. The text stays
    /// with the first.
    pub fn separate(self) -> Vec<Scope> {
        let Scope { speed, position, scale, text, plots, man_divisions, trigger, .. } = self;
        let flags = self.flags & !(FLAG_PLOT2D | FLAG_PLOT_XY | FLAG_FFT);
        plots
            .into_iter()
            .enumerate()
            .map(|(i, p)| Scope {
                speed,
                flags,
                position,
                scale,
                text: if i == 0 { text.clone() } else { None },
                plots: vec![p],
                xy: false,
                plot_xy: false,
                axes: XyAxes::default(),
                fft: false,
                man_divisions,
                trigger: Trigger {
                    mode: trigger.mode,
                    edge: trigger.edge,
                    level: trigger.level,
                    ..Trigger::default()
                },
            })
            .collect()
    }

    /// Forgets the elements at the sorted indices `removed` and renumbers the others. Returns
    /// false when nothing is left to plot.
    pub fn drop_elements(&mut self, removed: &[usize]) -> bool {
        self.plots.retain(|p| !removed.contains(&p.elm));
        for p in &mut self.plots {
            p.elm -= removed.iter().filter(|&&r| r < p.elm).count();
        }
        !self.plots.is_empty()
    }

    /// Stops plotting plot `plot`; false when that was the last one (the scope is then empty).
    pub fn remove_plot(&mut self, plot: usize) -> bool {
        if plot < self.plots.len() {
            self.plots.remove(plot);
        }
        !self.plots.is_empty()
    }

    /// True when `other` plots the same values of the same elements at the same speed.
    pub fn same_plots(&self, other: &Scope) -> bool {
        self.speed == other.speed
            && self.plots.len() == other.plots.len()
            && self
                .plots
                .iter()
                .zip(&other.plots)
                .all(|(a, b)| a.elm == b.elm && a.value == b.value)
    }

    /// Takes everything but the recorded history from `new`: a scope that keeps its samples
    /// across an edit still follows the settings of the edited circuit.
    pub fn take_settings_from(&mut self, new: &Scope) {
        self.flags = new.flags;
        self.position = new.position;
        self.scale = new.scale;
        self.text.clone_from(&new.text);
        self.xy = new.xy;
        self.plot_xy = new.plot_xy;
        self.axes = new.axes;
        self.fft = new.fft;
        self.man_divisions = new.man_divisions;
        self.trigger.mode = new.trigger.mode;
        self.trigger.edge = new.trigger.edge;
        self.trigger.level = new.trigger.level;
        for (mine, theirs) in self.plots.iter_mut().zip(&new.plots) {
            mine.man_scale = theirs.man_scale;
            mine.man_pos = theirs.man_pos;
            mine.man_set = theirs.man_set;
            mine.ac_coupled = theirs.ac_coupled;
        }
    }

    /// Empties the history.
    pub fn reset(&mut self, t: f64) {
        for p in &mut self.plots {
            p.clear(t);
        }
        self.trigger.reset();
    }

    /// Records the values of the elements at time `t`.
    pub fn time_step(&mut self, elements: &[Box<dyn Element>], t: f64, max_time_step: f64) {
        let period = max_time_step * f64::from(self.speed);
        // the time constant is about the width of the scope, as in the original
        let alpha = 1.0 - 1.0 / (1.15 * f64::from(self.speed) * POINTS as f64);
        for p in &mut self.plots {
            let Some(e) = elements.get(p.elm) else { continue };
            let mut v = e.scope_value(p.value);
            // first-order high-pass filter, run all the time so that it is primed when the
            // plot is switched to AC coupling
            let out = alpha * (p.ac_last_out + v - p.ac_last_in);
            p.ac_last_in = v;
            p.ac_last_out = out;
            if p.ac_coupled && p.can_ac_couple() {
                v = out;
            }
            if p.filled == 0 {
                // the first sample after the history was emptied starts from this value, not
                // from the zero the buffer holds
                p.min[p.ptr] = v;
                p.max[p.ptr] = v;
                p.filled = 1;
                p.total = 1;
                p.last_update = t;
            }
            if v < p.min[p.ptr] {
                p.min[p.ptr] = v;
            }
            if v > p.max[p.ptr] {
                p.max[p.ptr] = v;
            }
            if t - p.last_update >= period {
                p.ptr = (p.ptr + 1) % POINTS;
                p.min[p.ptr] = v;
                p.max[p.ptr] = v;
                p.filled = (p.filled + 1).min(POINTS);
                p.total += 1;
                p.last_update += period;
                // when still behind (the time step changed, or the scope was just made) do not
                // catch up one step at a time: that would fill several samples with values a
                // few steps apart
                if t - p.last_update >= period {
                    p.last_update = t;
                }
            }
        }
        if let Some(first) = self.plots.first() {
            self.trigger.check(first, t);
        }
    }

    /// What the interface needs to draw the scope.
    pub fn view(&self, elements: &[Box<dyn Element>], max_time_step: f64, time: f64) -> ScopeView {
        let plots = self
            .plots
            .iter()
            .map(|p| {
                let (min, max): (Vec<f64>, Vec<f64>) = (1..=POINTS)
                    .map(|i| {
                        let k = (p.ptr + i) % POINTS;
                        (p.min[k], p.max[k])
                    })
                    .unzip();
                let title = elements.get(p.elm).and_then(|e| e.elm_type()).unwrap_or_default();
                // a plot without a manual scale of its own shows what its units suggest
                let man_scale =
                    if p.man_set { p.man_scale } else { self.man_scale_from_max(p.units, false) };
                PlotView {
                    units: p.units,
                    value: p.value,
                    title,
                    min,
                    max,
                    valid: p.filled,
                    man_scale,
                    man_pos: p.man_pos,
                }
            })
            .collect();
        let trigger = self.trigger.is_active().then(|| TriggerView {
            mode: self.trigger.mode,
            edge: self.trigger.edge,
            level: self.trigger.level,
            state: self.trigger.state,
            waiting: self.trigger.waiting,
            triggered: self.trigger.is_triggered(),
            back: self
                .plots
                .first()
                .map_or(0, |p| p.total.saturating_sub(self.trigger.at_total))
                .min(POINTS as u64 - 1) as usize,
            time: self.trigger.time,
        });
        ScopeView {
            position: self.position,
            text: self.text.clone(),
            scale: self.scale,
            speed: self.speed,
            time_per_sample: max_time_step * f64::from(self.speed),
            time,
            plots,
            xy: self.xy,
            plot_xy: self.plot_xy,
            axes: self.axes,
            fft: self.fft,
            flags: self.flags,
            elm_info: if self.flags & FLAG_ELM_INFO != 0 {
                self.plots
                    .first()
                    .and_then(|p| elements.get(p.elm))
                    .map(|e| {
                        let mut lines = Vec::new();
                        e.get_info(&mut lines);
                        lines
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            },
            manual: self.is_manual(),
            man_divisions: self.man_divisions,
            max_scale: self.flags & FLAG_MAX_SCALE != 0,
            trigger,
        }
    }
}

/// The trigger as the interface draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct TriggerView {
    /// `TRIGGER_NORMAL` or `TRIGGER_AUTO` (a free-running scope has no trigger view).
    pub mode: i32,
    /// `TRIGGER_EDGE_RISING` or `TRIGGER_EDGE_FALLING`.
    pub edge: i32,
    /// Level of the first plot that fires the trigger.
    pub level: f64,
    /// What the trigger is doing.
    pub state: TriggerState,
    /// Armed and waiting for a crossing.
    pub waiting: bool,
    /// The display is held around the trigger sample.
    pub triggered: bool,
    /// How many samples ago the trigger sample was recorded (0 for the newest).
    pub back: usize,
    /// Simulated time of the trigger.
    pub time: f64,
}

/// A plot as the interface draws it: samples from oldest to newest.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotView {
    /// Units of the value.
    pub units: i32,
    /// Which value of the element is plotted.
    pub value: i32,
    /// Name of the plotted element.
    pub title: String,
    /// Smallest value in each sample.
    pub min: Vec<f64>,
    /// Largest value in each sample.
    pub max: Vec<f64>,
    /// How many of the newest samples hold recorded data; the older ones are empty.
    pub valid: usize,
    /// Units per division in manual mode.
    pub man_scale: f64,
    /// Vertical position in manual mode, in `V_POSITION_STEPS`.
    pub man_pos: i32,
}

/// A scope as the interface draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct ScopeView {
    /// Order among the scopes.
    pub position: i32,
    /// Text shown on the scope.
    pub text: Option<String>,
    /// Full-scale value for each units, as the file had it.
    pub scale: [f64; UNITS_COUNT as usize],
    /// Time steps per sample.
    pub speed: i32,
    /// Simulated seconds between samples.
    pub time_per_sample: f64,
    /// Simulated time of the newest sample.
    pub time: f64,
    /// The plots.
    pub plots: Vec<PlotView>,
    /// The plots are drawn against each other.
    pub xy: bool,
    /// The axes are chosen freely (X/Y) instead of being voltage and current.
    pub plot_xy: bool,
    /// What the X/Y plot draws.
    pub axes: XyAxes,
    /// Frequency spectrum of the first plot.
    pub fft: bool,
    /// The flags of the file entry: which measurements are shown.
    pub flags: i32,
    /// What the first plot's element says about itself (when the flag asks for it).
    pub elm_info: Vec<String>,
    /// Manual scale: each plot has its own scale and position.
    pub manual: bool,
    /// Vertical divisions in manual mode.
    pub man_divisions: i32,
    /// The full-scale values never shrink.
    pub max_scale: bool,
    /// The trigger, when the scope has one.
    pub trigger: Option<TriggerView>,
}

/// Builds the scopes of a circuit from its file entries.
pub fn scopes_from_circuit(
    scope_lines: &[String],
    scope_xml: &[XmlNode],
    elements: &[Box<dyn Element>],
) -> Vec<Scope> {
    let mut out = Vec::new();
    for xml in scope_xml {
        if let Some(s) = Scope::from_xml(xml, elements) {
            out.push(s);
        }
    }
    for line in scope_lines {
        let mut st = Tokenizer::new(line);
        st.next_token(); // the "o"
        if let Some(s) = Scope::from_line(&mut st, elements) {
            out.push(s);
        }
    }
    out.sort_by_key(|s| s.position);
    out
}

/// The scopes placed in the circuit as elements, with the index of the element holding each.
pub fn embedded_scopes(elements: &[Box<dyn Element>]) -> Vec<(usize, Scope)> {
    let mut out = Vec::new();
    for (i, e) in elements.iter().enumerate() {
        let scope = match e.scope_source() {
            Some(ScopeSource::Line(l)) => {
                let text = l.replace('_', " ");
                Scope::from_line(&mut Tokenizer::new(&text), elements)
            }
            Some(ScopeSource::Xml(node)) => Scope::from_xml(&node, elements),
            None => None,
        };
        if let Some(s) = scope {
            out.push((i, s));
        }
    }
    out
}

/// The menu entry (an English key, translated by the interface) for plotting `value`.
fn value_menu_key(value: i32, transistor: bool) -> &'static str {
    if transistor {
        return match value {
            VAL_IB => "Show Ib",
            VAL_IC => "Show Ic",
            VAL_IE => "Show Ie",
            VAL_VBE => "Show Vbe",
            VAL_VBC => "Show Vbc",
            VAL_VCE => "Show Vce",
            VAL_POWER => "Show Power Consumed",
            _ => "Show Value",
        };
    }
    match value {
        VAL_VOLTAGE => "Show Voltage",
        VAL_CURRENT => "Show Current",
        VAL_POWER => "Show Power Consumed",
        VAL_R => "Show Resistance",
        VAL_CHARGE => "Show Charge",
        _ => "Show Value",
    }
}
