// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (VoltageElm, RailElm, ACRailElm,
// SquareRailElm, ClockElm, VarRailElm, CurrentElm).
//! Voltage sources (two-terminal and rails) and the current source.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::{TokenError, Tokenizer};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale;
use crate::render::painter::{Color, Painter};
use crate::solver::path::{PathKind, Validator};
use crate::solver::sim::{dc_analysis, sim_clock};
use crate::solver::stamper::{NodeId, Stamper, VsId};
use std::f64::consts::PI;

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "sources";

const SOURCES: &str = "Inputs and Sources";

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("VoltageElm", FAMILY, "Add Voltage Source", SOURCES, |x, y| {
            Box::new(VoltageElm::new_dc(x, y))
        })
        .dump(i32::from(b'v'), |a, b, c, d, f, st| {
            Ok(Box::new(VoltageElm::from_dump(a, b, c, d, f, st)))
        })
        .hidden(),
    );
    r.add(
        ElementDescriptor::new(
            "DCVoltageElm",
            FAMILY,
            "Add Voltage Source (2-terminal)",
            SOURCES,
            |x, y| Box::new(VoltageElm::new_dc(x, y)),
        )
        .icon("voltage")
        .shortcut('v')
        .quick(),
    );
    // old files store noise generators with their own type code
    r.add(
        ElementDescriptor::new("NoiseElm", FAMILY, "Add Noise Generator", SOURCES, |x, y| {
            Box::new(RailElm::new(x, y, RailKind::Dc))
        })
        .dump(i32::from(b'n'), |a, b, c, d, f, st| {
            let mut rail = RailElm::from_dump(a, b, c, d, f, st);
            rail.wave.waveform = WF_NOISE;
            Ok(Box::new(rail))
        })
        .hidden(),
    );
    r.add(
        ElementDescriptor::new(
            "ACVoltageElm",
            FAMILY,
            "Add A/C Voltage Source (2-terminal)",
            SOURCES,
            |x, y| Box::new(VoltageElm::new_ac(x, y)),
        )
        .icon("acsource"),
    );
    r.add(
        ElementDescriptor::new(
            "RailElm",
            FAMILY,
            "Add Voltage Source (1-terminal)",
            SOURCES,
            |x, y| Box::new(RailElm::new(x, y, RailKind::Dc)),
        )
        .dump(i32::from(b'R'), |a, b, c, d, f, st| {
            Ok(Box::new(RailElm::from_dump(a, b, c, d, f, st)))
        })
        .icon("rail")
        .shortcut('V')
        .quick(),
    );
    r.add(ElementDescriptor::new(
        "ACRailElm",
        FAMILY,
        "Add A/C Voltage Source (1-terminal)",
        SOURCES,
        |x, y| Box::new(RailElm::new(x, y, RailKind::Ac)),
    ));
    r.add(ElementDescriptor::new(
        "SquareRailElm",
        FAMILY,
        "Add Square Wave Source (1-terminal)",
        SOURCES,
        |x, y| Box::new(RailElm::new(x, y, RailKind::Square)),
    ));
    r.add(ElementDescriptor::new("ClockElm", FAMILY, "Add Clock", SOURCES, |x, y| {
        Box::new(RailElm::new(x, y, RailKind::Clock))
    }));
    r.add(
        ElementDescriptor::new("VarRailElm", FAMILY, "Add Variable Voltage", SOURCES, |x, y| {
            Box::new(RailElm::new(x, y, RailKind::Var))
        })
        .dump(172, |a, b, c, d, f, st| Ok(Box::new(RailElm::from_dump_var(a, b, c, d, f, st)?))),
    );
    r.add(
        ElementDescriptor::new("ExtVoltageElm", FAMILY, "Add External Voltage", SOURCES, |x, y| {
            Box::new(RailElm::new(x, y, RailKind::Ext))
        })
        .dump(418, |a, b, c, d, f, st| Ok(Box::new(RailElm::from_dump_ext(a, b, c, d, f, st))))
        .hidden(),
    );
    r.add(
        ElementDescriptor::new("DataInputElm", FAMILY, "Add Data Input", SOURCES, |x, y| {
            Box::new(RailElm::new(x, y, RailKind::Data))
        })
        .dump(424, |a, b, c, d, f, st| Ok(Box::new(RailElm::from_dump_data(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("AudioInputElm", FAMILY, "Add Audio Input", SOURCES, |x, y| {
            Box::new(RailElm::new(x, y, RailKind::Audio))
        })
        .dump(411, |a, b, c, d, f, st| Ok(Box::new(RailElm::from_dump_audio(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("AntennaElm", FAMILY, "Add Antenna", SOURCES, |x, y| {
            Box::new(RailElm::new(x, y, RailKind::Antenna))
        })
        .dump(i32::from(b'A'), |a, b, c, d, f, st| {
            Ok(Box::new(RailElm::from_dump_antenna(a, b, c, d, f, st)))
        })
        .hidden(),
    );
    r.add(
        ElementDescriptor::new(
            "OhmMeterElm",
            FAMILY,
            "Add Ohmmeter",
            "Outputs and Labels",
            |x, y| Box::new(CurrentElm::new_ohmmeter(x, y)),
        )
        .dump(216, |a, b, c, d, f, st| {
            Ok(Box::new(CurrentElm::from_dump_ohmmeter(a, b, c, d, f, st)))
        }),
    );
    r.add(
        ElementDescriptor::new("CurrentElm", FAMILY, "Add Current Source", SOURCES, |x, y| {
            Box::new(CurrentElm::new(x, y))
        })
        .dump(i32::from(b'i'), |a, b, c, d, f, st| {
            Ok(Box::new(CurrentElm::from_dump(a, b, c, d, f, st)))
        })
        .icon("current"),
    );
}

// ---------------------------------------------------------------------------------------------
// Waveform parameters shared by voltage sources and rails
// ---------------------------------------------------------------------------------------------

/// Flag: cosine instead of sine (old files).
pub const FLAG_COS: i32 = 2;
/// Flag: the pulse duty cycle is stored in the file.
pub const FLAG_PULSE_DUTY: i32 = 4;
/// Flag: draw a DC source with the circle symbol.
pub const FLAG_CIRCLE_SYMBOL: i32 = 8;
/// Flag: show the voltage next to the source.
pub const FLAG_SHOW_VOLTAGE: i32 = 16;
/// Flag: square/pulse timing is given as high and low times.
pub const FLAG_TIME_SPEC: i32 = 32;
/// Flag: show the voltage next to a rail.
pub const FLAG_SHOW_VOLTAGE_RAIL: i32 = 64;
/// Rail flag: draw "CLK" instead of a waveform.
pub const FLAG_REPEAT: i32 = 1 << 8;
const FLAG_CLOCK: i32 = 1;

/// Direct current.
pub const WF_DC: i32 = 0;
/// Sine wave.
pub const WF_AC: i32 = 1;
/// Square wave.
pub const WF_SQUARE: i32 = 2;
/// Triangle wave.
pub const WF_TRIANGLE: i32 = 3;
/// Sawtooth wave.
pub const WF_SAWTOOTH: i32 = 4;
/// Pulse.
pub const WF_PULSE: i32 = 5;
/// Noise.
pub const WF_NOISE: i32 = 6;
/// Variable (slider controlled).
pub const WF_VAR: i32 = 7;

const CIRCLE_SIZE: i32 = 17;
const DEFAULT_PULSE_DUTY: f64 = 1.0 / (2.0 * PI);

/// The waveform description shared by two-terminal voltage sources and rails.
#[derive(Clone, Debug)]
pub struct Waveform {
    /// One of the `WF_*` constants.
    pub waveform: i32,
    /// Frequency in hertz.
    pub frequency: f64,
    /// Peak voltage (or the DC voltage).
    pub max_voltage: f64,
    /// Time at which the waveform phase is zero.
    pub freq_time_zero: f64,
    /// DC offset.
    pub bias: f64,
    /// Phase offset in radians.
    pub phase_shift: f64,
    /// Duty cycle of square and pulse waves (0 to 1).
    pub duty_cycle: f64,
    /// Last noise sample.
    pub noise_value: f64,
    /// Rise and fall time of square and pulse waves.
    pub rise_time: f64,
    /// Series resistance of the source.
    pub internal_resistance: f64,
}

impl Waveform {
    fn new(wf: i32) -> Self {
        Waveform {
            waveform: wf,
            frequency: 60.0,
            max_voltage: 5.0,
            freq_time_zero: 0.0,
            bias: 0.0,
            phase_shift: 0.0,
            duty_cycle: 0.5,
            noise_value: 0.0,
            rise_time: 0.0,
            internal_resistance: 0.0,
        }
    }

    /// Reads the waveform fields of a file entry, tolerating missing trailing fields.
    fn from_dump(st: &mut Tokenizer<'_>, flags: &mut i32) -> Self {
        let mut w = Waveform::new(WF_DC);
        w.max_voltage = 5.0;
        w.frequency = 40.0;
        w.duty_cycle = 0.5;
        // each field is optional: stop at the first one that is missing or malformed
        let _ = (|| -> Result<(), TokenError> {
            w.waveform = st.next_i32()?;
            w.frequency = st.next_f64()?;
            w.max_voltage = st.next_f64()?;
            w.bias = st.next_f64()?;
            w.phase_shift = st.next_f64()?;
            w.duty_cycle = st.next_f64()?;
            Ok(())
        })();
        if *flags & FLAG_COS != 0 {
            *flags &= !FLAG_COS;
            w.phase_shift = PI / 2.0;
        }
        // old files have the wrong duty cycle for pulse waveforms (it was not configurable)
        if *flags & FLAG_PULSE_DUTY == 0 && w.waveform == WF_PULSE {
            w.duty_cycle = DEFAULT_PULSE_DUTY;
        }
        w
    }

    fn undump_xml(&mut self, xml: &XmlNode) {
        self.waveform = xml.int_attr("wf", self.waveform);
        self.frequency = xml.double_attr("fr", self.frequency);
        self.max_voltage = xml.double_attr("maxv", self.max_voltage);
        self.bias = xml.double_attr("bias", self.bias);
        self.phase_shift = xml.double_attr("phaseShift", self.phase_shift);
        self.duty_cycle = xml.double_attr("dutyCycle", self.duty_cycle);
        self.rise_time = xml.double_attr("riseTime", self.rise_time);
        self.internal_resistance = xml.double_attr("ir", 0.0);
    }

    fn dump_xml(&self, out: &mut XmlOut) {
        out.int_attr("wf", self.waveform);
        if self.waveform != WF_DC {
            out.double_attr("fr", self.frequency);
        }
        out.double_attr("maxv", self.max_voltage);
        if self.bias != 0.0 {
            out.double_attr("bias", self.bias);
        }
        if self.phase_shift != 0.0 {
            out.double_attr("phaseShift", self.phase_shift);
        }
        if self.duty_cycle != 0.5 {
            out.double_attr("dutyCycle", self.duty_cycle);
        }
        if self.rise_time != 0.0 {
            out.double_attr("riseTime", self.rise_time);
        }
        if self.internal_resistance != 0.0 {
            out.double_attr("ir", self.internal_resistance);
        }
    }

    fn triangle_func(x: f64) -> f64 {
        if x < PI { x * (2.0 / PI) - 1.0 } else { 1.0 - (x - PI) * (2.0 / PI) }
    }

    /// Voltage at time `t`.
    pub fn voltage(&self, t: f64) -> f64 {
        if self.waveform != WF_DC && dc_analysis() {
            return self.bias;
        }
        let w = 2.0 * PI * (t - self.freq_time_zero) * self.frequency + self.phase_shift;
        let (bias, max_v) = (self.bias, self.max_voltage);
        match self.waveform {
            WF_DC => max_v + bias,
            WF_AC => w.sin() * max_v + bias,
            WF_SQUARE => {
                let wm = w % (2.0 * PI);
                let duty_phase = 2.0 * PI * self.duty_cycle;
                if self.rise_time > 0.0 {
                    let rise_phase = self.rise_time * self.frequency * 2.0 * PI;
                    let half_rise = rise_phase / 2.0;
                    if wm < half_rise {
                        let t = (wm + half_rise) / rise_phase;
                        return bias + max_v * (2.0 * t - 1.0);
                    } else if wm < duty_phase - half_rise {
                        return bias + max_v;
                    } else if wm < duty_phase + half_rise {
                        let t = (wm - duty_phase + half_rise) / rise_phase;
                        return bias + max_v * (1.0 - 2.0 * t);
                    } else if wm < 2.0 * PI - half_rise {
                        return bias - max_v;
                    }
                    let t = (wm - (2.0 * PI - half_rise)) / rise_phase;
                    return bias + max_v * (2.0 * t - 1.0);
                }
                bias + if wm > duty_phase { -max_v } else { max_v }
            }
            WF_TRIANGLE => bias + Self::triangle_func(w % (2.0 * PI)) * max_v,
            WF_SAWTOOTH => bias + (w % (2.0 * PI)) * (max_v / PI) - max_v,
            WF_PULSE => {
                let wm = w % (2.0 * PI);
                let duty_phase = 2.0 * PI * self.duty_cycle;
                if self.rise_time > 0.0 {
                    let rise_phase = self.rise_time * self.frequency * 2.0 * PI;
                    let half_rise = rise_phase / 2.0;
                    if wm < half_rise {
                        let t = (wm + half_rise) / rise_phase;
                        return bias + max_v * t;
                    } else if wm < duty_phase - half_rise {
                        return bias + max_v;
                    } else if wm < duty_phase + half_rise {
                        let t = (wm - duty_phase + half_rise) / rise_phase;
                        return bias + max_v * (1.0 - t);
                    } else if wm < 2.0 * PI - half_rise {
                        return bias;
                    }
                    let t = (wm - (2.0 * PI - half_rise)) / rise_phase;
                    return bias + max_v * t;
                }
                if wm < duty_phase { max_v + bias } else { bias }
            }
            WF_NOISE => self.noise_value,
            _ => 0.0,
        }
    }

    fn rms_multiplier(&self) -> f64 {
        match self.waveform {
            WF_AC => 1.0 / 2f64.sqrt(),
            WF_TRIANGLE | WF_SAWTOOTH => 1.0 / 3f64.sqrt(),
            WF_PULSE => self.duty_cycle.sqrt(),
            _ => 1.0,
        }
    }

    fn diff_from_integer(x: f64) -> f64 {
        (x - x.round()).abs()
    }

    fn use_rms_display(&self, peak: f64) -> bool {
        let rms_mult = self.rms_multiplier();
        let rms_val = peak * rms_mult;
        rms_mult != 1.0
            && peak.abs() > 1e-4
            && Self::diff_from_integer(rms_val * 1e4) < Self::diff_from_integer(peak * 1e4)
    }

    /// Short voltage text, using RMS when that is the rounder number.
    pub fn short_voltage_text(&self) -> String {
        if self.bias != 0.0 {
            return short_unit_text(self.bias + self.max_voltage, "V");
        }
        if self.use_rms_display(self.max_voltage) {
            return format!(
                "{}rms",
                short_unit_text(self.max_voltage * self.rms_multiplier(), "V")
            );
        }
        short_unit_text(self.max_voltage, "V")
    }

    fn has_timing_options(&self) -> bool {
        self.waveform == WF_PULSE || self.waveform == WF_SQUARE
    }

    fn time_spec(&self, flags: i32) -> bool {
        flags & FLAG_TIME_SPEC != 0 && self.has_timing_options()
    }

    /// Changes the frequency, keeping the waveform continuous at the current time. A frequency
    /// too high for the time step is limited to what the time step supports.
    fn set_frequency(&mut self, new_freq: f64) {
        let (t, max_time_step) = sim_clock();
        let old_freq = self.frequency;
        self.frequency = new_freq;
        let max_freq = 1.0 / (8.0 * max_time_step);
        if self.frequency > max_freq {
            self.frequency = max_freq;
        }
        self.freq_time_zero = if self.frequency == 0.0 {
            0.0
        } else {
            t - old_freq * (t - self.freq_time_zero) / self.frequency
        };
    }

    fn set_frequency_from_times(&mut self, high: f64, low: f64) {
        let new_freq = 1.0 / (high + low);
        let new_duty = high / (high + low);
        self.set_frequency(new_freq);
        self.duty_cycle = new_duty;
    }

    fn info(&self, rail: bool, current: f64, diff: f64, power: f64, arr: &mut Vec<String>) {
        arr.push(
            match self.waveform {
                WF_DC | WF_VAR => "voltage source",
                WF_AC => "A/C source",
                WF_SQUARE => "square wave gen",
                WF_PULSE => "pulse gen",
                WF_SAWTOOTH => "sawtooth gen",
                WF_TRIANGLE => "triangle gen",
                WF_NOISE => "noise gen",
                _ => "voltage source",
            }
            .into(),
        );
        arr.push(format!("I = {}", current_text(current)));
        arr.push(format!("{} = {}", if rail { "V" } else { "Vd" }, voltage_text(diff)));
        if !matches!(self.waveform, WF_DC | WF_VAR | WF_NOISE) {
            arr.push(format!("f = {}", unit_text(self.frequency, "Hz")));
            arr.push(format!("Vmax = {}", voltage_text(self.max_voltage)));
            if self.bias == 0.0 {
                arr.push(format!(
                    "V(rms) = {}",
                    voltage_text(self.max_voltage * self.rms_multiplier())
                ));
            }
            if self.bias != 0.0 {
                arr.push(format!("Voff = {}", voltage_text(self.bias)));
            } else if self.frequency > 500.0 {
                arr.push(format!("wavelength = {}", unit_text(2.9979e8 / self.frequency, "m")));
            }
        }
        if self.waveform == WF_DC && current != 0.0 && SHOW_RESISTANCE.with(std::cell::Cell::get) {
            arr.push(format!("(R = {})", unit_text(self.max_voltage / current, locale::OHM)));
        }
        arr.push(format!("P = {}", unit_text(power, "W")));
    }

    fn frequency_offset(&self) -> usize {
        5
    }

    fn edit_info(&self, n: usize, flags: i32, rail: bool) -> Option<EditInfo> {
        match n {
            0 => {
                return Some(
                    EditInfo::new(
                        if self.waveform == WF_DC { "Voltage" } else { "Max Voltage" },
                        self.max_voltage,
                        -20.0,
                        20.0,
                    )
                    .set_unit_step(),
                );
            }
            1 => {
                return Some(EditInfo::choice(
                    "Waveform",
                    ["D/C", "A/C", "Square Wave", "Triangle", "Sawtooth", "Pulse", "Noise"]
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect(),
                    self.waveform.max(0) as usize,
                ));
            }
            2 => {
                return Some(
                    EditInfo::new("DC Offset (V)", self.bias, -20.0, 20.0).set_unit_step(),
                );
            }
            3 => {
                return Some(
                    EditInfo::number("Internal Resistance (ohms)", self.internal_resistance)
                        .set_non_negative(),
                );
            }
            _ => {}
        }
        if n == 4 && !(rail && (self.waveform == WF_DC || self.waveform == WF_VAR)) {
            let sv = if rail { FLAG_SHOW_VOLTAGE_RAIL } else { FLAG_SHOW_VOLTAGE };
            return Some(EditInfo::checkbox("Show Voltage", flags & sv != 0));
        }
        if n == 5 && self.waveform == WF_DC && !rail {
            return Some(EditInfo::checkbox("Circle Symbol", flags & FLAG_CIRCLE_SYMBOL != 0));
        }
        if self.waveform == WF_DC || self.waveform == WF_NOISE {
            return None;
        }
        let n2 = n.checked_sub(self.frequency_offset())?;
        if self.has_timing_options() {
            let ts = self.time_spec(flags);
            match n2 {
                0 => {
                    let mut ei = EditInfo::choice(
                        "Specify As",
                        vec!["Frequency/Duty Cycle".into(), "High Time/Low Time".into()],
                        usize::from(ts),
                    );
                    ei.new_column = true;
                    Some(ei)
                }
                1 => Some(if ts {
                    EditInfo::new("High Time (s)", self.duty_cycle / self.frequency, 0.0, 0.0)
                } else {
                    EditInfo::new("Frequency (Hz)", self.frequency, 4.0, 500.0)
                }),
                2 => Some(
                    EditInfo::new(
                        "Phase Offset (degrees)",
                        self.phase_shift * 180.0 / PI,
                        -180.0,
                        180.0,
                    )
                    .set_dimensionless(),
                ),
                3 => Some(if ts {
                    EditInfo::new(
                        "Low Time (s)",
                        (1.0 - self.duty_cycle) / self.frequency,
                        0.0,
                        0.0,
                    )
                } else {
                    EditInfo::new("Duty Cycle", self.duty_cycle * 100.0, 0.0, 100.0)
                        .set_dimensionless()
                }),
                4 => Some(EditInfo::new("Rise/Fall Time (s)", self.rise_time, 0.0, 0.0)),
                _ => None,
            }
        } else {
            match n2 {
                0 => Some(EditInfo::new("Frequency (Hz)", self.frequency, 4.0, 500.0)),
                1 => Some(
                    EditInfo::new(
                        "Phase Offset (degrees)",
                        self.phase_shift * 180.0 / PI,
                        -180.0,
                        180.0,
                    )
                    .set_dimensionless(),
                ),
                _ => None,
            }
        }
    }

    /// Applies an edit; returns true when the points must be recalculated.
    fn set_edit(&mut self, n: usize, ei: &mut EditInfo, flags: &mut i32, rail: bool) -> bool {
        let mut points = false;
        match n {
            0 => self.max_voltage = ei.value,
            2 => self.bias = ei.value,
            3 => self.internal_resistance = ei.value,
            _ => {}
        }
        if n == 4 && ei.checkbox.is_some() {
            let sv = if rail { FLAG_SHOW_VOLTAGE_RAIL } else { FLAG_SHOW_VOLTAGE };
            *flags = ei.change_flag(*flags, sv);
        }
        if n == 5 && self.waveform == WF_DC && ei.checkbox.is_some() && !rail {
            *flags = ei.change_flag(*flags, FLAG_CIRCLE_SYMBOL);
            points = true;
        }
        if n == 1 {
            let ow = self.waveform;
            self.waveform = ei.choice_index as i32;
            if self.waveform == WF_DC && ow != WF_DC {
                ei.new_dialog = true;
                self.bias = 0.0;
            } else if self.waveform != ow {
                ei.new_dialog = true;
            }
            // change the duty cycle when switching to or from pulse
            if self.waveform == WF_PULSE && ow != WF_PULSE {
                self.duty_cycle = DEFAULT_PULSE_DUTY;
            } else if ow == WF_PULSE && self.waveform != WF_PULSE {
                self.duty_cycle = 0.5;
            }
            points = true;
        }
        let Some(n2) = n.checked_sub(self.frequency_offset()) else { return points };
        if self.has_timing_options() {
            let ts = self.time_spec(*flags);
            match n2 {
                0 if ei.choice.is_some() => {
                    let old = *flags;
                    *flags = if ei.choice_index == 1 {
                        *flags | FLAG_TIME_SPEC
                    } else {
                        *flags & !FLAG_TIME_SPEC
                    };
                    if *flags != old {
                        ei.new_dialog = true;
                    }
                }
                1 => {
                    if ts {
                        let high = ei.value;
                        let low = (1.0 - self.duty_cycle) / self.frequency;
                        if high > 0.0 && low > 0.0 {
                            self.set_frequency_from_times(high, low);
                        }
                    } else if ei.value != 0.0 {
                        self.set_frequency(ei.value);
                    }
                }
                2 => {
                    self.phase_shift = ei.value * PI / 180.0;
                    self.phase_shift = ((self.phase_shift % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI);
                }
                3 => {
                    if ts {
                        let high = self.duty_cycle / self.frequency;
                        let low = ei.value;
                        if high > 0.0 && low > 0.0 {
                            self.set_frequency_from_times(high, low);
                        }
                    } else {
                        self.duty_cycle = ei.value * 0.01;
                    }
                }
                4 => self.rise_time = ei.value,
                _ => {}
            }
        } else {
            if n2 == 0 && self.waveform != WF_DC && ei.value != 0.0 {
                self.set_frequency(ei.value);
            }
            if n2 == 1 {
                self.phase_shift = ei.value * PI / 180.0;
                self.phase_shift = ((self.phase_shift % (2.0 * PI)) + 2.0 * PI) % (2.0 * PI);
            }
        }
        points
    }

    /// Draws the waveform symbol in a circle centred on `center`.
    fn draw_waveform(
        &self,
        g: &mut dyn Painter,
        base: &mut Base,
        c: &DrawCtx,
        center: Point,
        rail_text: Option<&str>,
        point1: Point,
        lead1: Point,
    ) {
        let gray_or_select = if base.needs_highlight() { c.select_color } else { Color::GRAY };
        g.set_color(gray_or_select);
        set_power_color_w(g, base, c, base_power(base));
        let (xc, yc) = (center.x, center.y);
        if self.waveform != WF_NOISE {
            draw_thick_circle(g, xc, yc, CIRCLE_SIZE);
        }
        let wl = 8;
        base.adjust_bbox(xc - CIRCLE_SIZE, yc - CIRCLE_SIZE, xc + CIRCLE_SIZE, yc + CIRCLE_SIZE);
        let line = |g: &mut dyn Painter, x1, y1, x2, y2| {
            draw_thick_line(g, Point::new(x1, y1), Point::new(x2, y2));
        };
        match self.waveform {
            WF_SQUARE => {
                let mut xc2 =
                    (f64::from(wl) * 2.0 * self.duty_cycle - f64::from(wl) + f64::from(xc)) as i32;
                xc2 = (xc - wl + 3).max((xc + wl - 3).min(xc2));
                line(g, xc - wl, yc - wl, xc - wl, yc);
                line(g, xc - wl, yc - wl, xc2, yc - wl);
                line(g, xc2, yc - wl, xc2, yc + wl);
                line(g, xc + wl, yc + wl, xc2, yc + wl);
                line(g, xc + wl, yc, xc + wl, yc + wl);
            }
            WF_PULSE => {
                let yc = yc + wl / 2;
                line(g, xc - wl, yc - wl, xc - wl, yc);
                line(g, xc - wl, yc - wl, xc - wl / 2, yc - wl);
                line(g, xc - wl / 2, yc - wl, xc - wl / 2, yc);
                line(g, xc - wl / 2, yc, xc + wl, yc);
            }
            WF_SAWTOOTH => {
                line(g, xc, yc - wl, xc - wl, yc);
                line(g, xc, yc - wl, xc, yc + wl);
                line(g, xc, yc + wl, xc + wl, yc);
            }
            WF_TRIANGLE => {
                let xl = 5;
                line(g, xc - xl * 2, yc, xc - xl, yc - wl);
                line(g, xc - xl, yc - wl, xc, yc);
                line(g, xc, yc, xc + xl, yc + wl);
                line(g, xc + xl, yc + wl, xc + xl * 2, yc);
            }
            WF_NOISE => {
                g.set_color(if base.needs_highlight() { c.select_color } else { c.white });
                set_power_color_w(g, base, c, base_power(base));
                draw_labeled_node(g, base, c, &locale::ls("Noise"), point1, lead1);
            }
            WF_AC => {
                let xl = 10;
                g.begin_path();
                g.set_line_width(3.0);
                for i in -xl..=xl {
                    let yy = yc
                        + (0.95 * (f64::from(i) * PI / f64::from(xl)).sin() * f64::from(wl)) as i32;
                    if i == -xl {
                        g.move_to(f64::from(xc + i), f64::from(yy));
                    } else {
                        g.line_to(f64::from(xc + i), f64::from(yy));
                    }
                }
                g.stroke();
                g.set_line_width(1.0);
            }
            _ => {}
        }
        if let Some(rt) = rail_text {
            // rails show voltage and frequency next to the symbol
            if base.dx == 0 || base.dy == 0 {
                draw_values(g, base, c, Some(rt), f64::from(CIRCLE_SIZE), true, false);
            }
        }
    }
}

fn base_power(base: &Base) -> f64 {
    base.voltage_diff() * base.current
}

thread_local! {
    /// Whether voltage source info shows the equivalent resistance (only when there is a single
    /// voltage source in the circuit).
    static SHOW_RESISTANCE: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Tells voltage sources whether their info text may show the equivalent resistance.
pub fn set_show_resistance_in_voltage_sources(show: bool) {
    SHOW_RESISTANCE.with(|c| c.set(show));
}

// ---------------------------------------------------------------------------------------------
// Two-terminal voltage source
// ---------------------------------------------------------------------------------------------

/// A two-terminal voltage source (DC, A/C, square, triangle, sawtooth, pulse, noise).
#[derive(Clone, Debug)]
pub struct VoltageElm {
    base: Base,
    /// Waveform parameters.
    pub wave: Waveform,
    class: &'static str,
}

impl VoltageElm {
    fn with_waveform(xx: i32, yy: i32, wf: i32, class: &'static str) -> Self {
        let mut base = Base::new(xx, yy);
        base.flags |= FLAG_SHOW_VOLTAGE;
        VoltageElm { base, wave: Waveform::new(wf), class }
    }

    /// A DC source being placed.
    pub fn new_dc(xx: i32, yy: i32) -> Self {
        Self::with_waveform(xx, yy, WF_DC, "DCVoltageElm")
    }

    /// An A/C source being placed.
    pub fn new_ac(xx: i32, yy: i32) -> Self {
        let mut v = Self::with_waveform(xx, yy, WF_AC, "ACVoltageElm");
        v.wave.max_voltage = 120.0 * 2f64.sqrt();
        v
    }

    /// A voltage source read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut base = Base::from_dump(xa, ya, xb, yb, f);
        let wave = Waveform::from_dump(st, &mut base.flags);
        VoltageElm { base, wave, class: "VoltageElm" }
    }
}

impl Element for VoltageElm {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        self.class
    }
    fn dump_type(&self) -> i32 {
        i32::from(b'v')
    }
    fn is_voltage_elm(&self) -> bool {
        true
    }

    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.wave.undump_xml(xml);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        self.wave.dump_xml(out);
    }

    fn reset(&mut self) {
        self.wave.freq_time_zero = 0.0;
        self.base.curcount = 0.0;
    }

    fn internal_node_count(&self) -> usize {
        usize::from(self.wave.internal_resistance > 0.0)
    }
    fn voltage_source_count(&self) -> usize {
        1
    }

    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base.volt_source = Some(v);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let Some(vs) = self.base.volt_source else { return };
        let n = &self.base.nodes;
        let ir = self.wave.internal_resistance > 0.0;
        let vs_node2 = if ir { n[2] } else { n[1] };
        s.set_vs_nodes(vs, n[0], vs_node2);
        if self.wave.waveform == WF_DC {
            s.stamp_voltage_source(n[0], vs_node2, vs, self.wave.voltage(s.t));
        } else {
            s.stamp_voltage_source_var(n[0], vs_node2, vs);
        }
        if ir {
            s.stamp_resistor(n[2], n[1], self.wave.internal_resistance);
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        if self.wave.waveform != WF_DC {
            if let Some(vs) = self.base.volt_source {
                s.update_voltage_source(vs, self.wave.voltage(s.t));
            }
        }
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if self.wave.waveform == WF_NOISE {
            self.wave.noise_value =
                (s.random() * 2.0 - 1.0) * self.wave.max_voltage + self.wave.bias;
        }
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let w = &self.wave;
        if w.waveform == WF_DC && self.base.flags & FLAG_CIRCLE_SYMBOL != 0 {
            self.base.calc_leads(CIRCLE_SIZE * 2);
        } else {
            self.base.calc_leads(if w.waveform == WF_DC || w.waveform == WF_VAR {
                8
            } else {
                CIRCLE_SIZE * 2
            });
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (x, y, x2, y2) = (self.base.x, self.base.y, self.base.x2, self.base.y2);
        self.base.set_bbox(x, y, x2, y2);
        draw_2_leads(g, &self.base, c);
        let wf = self.wave.waveform;
        let flags = self.base.flags;
        let (p1, p2) = (self.base.point1, self.base.point2);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let power = self.get_power();
        if wf == WF_DC && flags & FLAG_CIRCLE_SYMBOL != 0 {
            self.base.set_bbox_pts(p1, p2, f64::from(CIRCLE_SIZE));
            let ps1 = interp_point(l1, l2, 0.5);
            let (xc, yc) = (ps1.x, ps1.y);
            g.set_color(if self.base.needs_highlight() { c.select_color } else { Color::GRAY });
            set_power_color_w(g, &self.base, c, power);
            draw_thick_circle(g, xc, yc, CIRCLE_SIZE);
            self.base.adjust_bbox(
                xc - CIRCLE_SIZE,
                yc - CIRCLE_SIZE,
                xc + CIRCLE_SIZE,
                yc + CIRCLE_SIZE,
            );
            // + and - signs inside the circle
            let sign_size = 4.0;
            let (plus_pos, minus_pos) = (0.74, 0.26);
            let (a, b) = interp_point2(l1, l2, plus_pos, sign_size);
            draw_thick_line(g, a, b);
            let delta = sign_size / (f64::from(CIRCLE_SIZE) * 2.0);
            let pa = interp_point(l1, l2, plus_pos - delta);
            let pb = interp_point(l1, l2, plus_pos + delta);
            draw_thick_line(g, pa, pb);
            let (a, b) = interp_point2(l1, l2, minus_pos, sign_size);
            draw_thick_line(g, a, b);
        } else if wf == WF_DC {
            let b = &self.base;
            set_voltage_color(g, b, c, b.volts[0]);
            set_power_color_w(g, b, c, power);
            let (a1, a2) = interp_point2(l1, l2, 0.0, 10.0);
            draw_thick_line(g, a1, a2);
            set_voltage_color(g, b, c, b.volts[1]);
            set_power_color_w(g, b, c, power);
            let hs = 16.0;
            self.base.set_bbox_pts(p1, p2, hs);
            let (a1, a2) = interp_point2(l1, l2, 1.0, hs);
            draw_thick_line(g, a1, a2);
        } else {
            self.base.set_bbox_pts(p1, p2, f64::from(CIRCLE_SIZE));
            let center = interp_point(l1, l2, 0.5);
            let wave = self.wave.clone();
            wave.draw_waveform(g, &mut self.base, c, center, None, p1, l1);
            let inds =
                if wave.bias > 0.0 || (wave.bias == 0.0 && wf == WF_PULSE) { "+" } else { "*" };
            g.set_color(c.white);
            g.set_font_size(12);
            let dn = self.base.dn;
            let mut plus_point = interp_point_g(
                p1,
                p2,
                (dn / 2.0 + f64::from(CIRCLE_SIZE) + 4.0) / dn,
                f64::from(10 * self.base.dsign),
            );
            plus_point.y += 4;
            let w = g.measure_text(inds) as i32;
            g.draw_string(inds, f64::from(plus_point.x - w / 2), f64::from(plus_point.y));
        }
        if self.base.dx == 0 || self.base.dy == 0 {
            let show_v = flags & FLAG_SHOW_VOLTAGE != 0;
            let show_f = c.show_values && wf != WF_DC && wf != WF_NOISE;
            let s = match (show_v, show_f) {
                (true, true) => Some(format!(
                    "{} {}",
                    self.wave.short_voltage_text(),
                    short_unit_text(self.wave.frequency, "Hz")
                )),
                (true, false) => Some(self.wave.short_voltage_text()),
                (false, true) => Some(short_unit_text(self.wave.frequency, "Hz")),
                (false, false) => None,
            };
            if let Some(s) = s {
                let hs =
                    if wf == WF_DC && flags & FLAG_CIRCLE_SYMBOL == 0 { 16 } else { CIRCLE_SIZE };
                draw_values(g, &self.base, c, Some(&s), f64::from(hs), false, true);
            }
        }
        self.base.update_dot_count(c);
        let b = &self.base;
        if !b.creating {
            if wf == WF_DC && flags & FLAG_CIRCLE_SYMBOL == 0 {
                draw_dots(g, c, b.point1, b.point2, b.curcount);
            } else {
                draw_dots(g, c, b.point1, b.lead1, b.curcount);
                draw_dots(g, c, b.point2, b.lead2, -b.curcount);
            }
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_power(&self) -> f64 {
        -self.get_voltage_diff() * self.base.current
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[1] - self.base.volts[0]
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        self.wave.info(false, self.base.current, self.get_voltage_diff(), self.get_power(), arr);
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        self.wave.edit_info(n, self.base.flags, false)
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        let mut flags = self.base.flags;
        if self.wave.set_edit(n, ei, &mut flags, false) {
            self.base.flags = flags;
            Element::set_points(self);
        }
        self.base.flags = flags;
    }

    fn shortcut(&self) -> i32 {
        if self.class == "DCVoltageElm" { i32::from(b'v') } else { 0 }
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        if self.wave.internal_resistance > 0.0 {
            return true;
        }
        let (n0, n1) = (self.base.nodes[0], self.base.nodes[1]);
        if v.find_path(PathKind::Voltage, idx, n1, n0) {
            self.wave.internal_resistance = 0.001;
            return false;
        }
        true
    }
}

// ---------------------------------------------------------------------------------------------
// Rails (one-terminal voltage sources)
// ---------------------------------------------------------------------------------------------

/// Which kind of rail a [`RailElm`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailKind {
    /// DC rail.
    Dc,
    /// A/C rail.
    Ac,
    /// Square wave rail.
    Square,
    /// Clock (square wave drawn as "CLK").
    Clock,
    /// Variable voltage controlled by a slider.
    Var,
    /// A voltage named for an outside program to set.
    Ext,
    /// An amplified radio antenna.
    Antenna,
    /// Plays back the numbers of a data file, one per sample length.
    Data,
    /// Plays back an audio file.
    Audio,
}

/// What the data input and the audio input play back.
#[derive(Clone, Debug)]
struct Media {
    file_num: i32,
    file_name: Option<String>,
    data: Option<std::sync::Arc<Vec<f64>>>,
    /// Samples per second of an audio file.
    sampling_rate: u32,
    /// Seconds per value of a data file.
    sample_length: f64,
    scale_factor: f64,
    /// Volts of a full-scale audio sample.
    max_voltage: f64,
    start_position: f64,
    /// Playback position in seconds.
    time_offset: f64,
}

impl Default for Media {
    fn default() -> Self {
        Media {
            file_num: 0,
            file_name: None,
            data: None,
            sampling_rate: 0,
            sample_length: 1e-3,
            scale_factor: 1.0,
            max_voltage: 5.0,
            start_position: 0.0,
            time_offset: 0.0,
        }
    }
}

impl Media {
    /// Takes the file stored under the current number, if there is one.
    fn look_up(&mut self) {
        if let Some(f) = super::samples::lookup(self.file_num) {
            self.file_name = Some(f.name);
            self.data = Some(f.data);
            if f.rate > 0 {
                self.sampling_rate = f.rate;
            }
        }
    }

    /// Loads the file at `path` (a data file for `audio` false, else a WAV file).
    fn load(&mut self, path: &str, audio: bool) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        let (data, rate) = if audio {
            super::samples::decode_wav(&bytes)?
        } else {
            (super::samples::parse_data_text(&String::from_utf8_lossy(&bytes))?, 0)
        };
        let file = super::samples::SampleFile {
            name: super::samples::display_name(path),
            data: std::sync::Arc::new(data),
            rate,
        };
        self.file_num = super::samples::store(0, file);
        self.look_up();
        Ok(())
    }

    fn name(&self) -> String {
        self.file_name.clone().unwrap_or_else(|| crate::locale::ls("No file"))
    }
}

/// A one-terminal voltage source: a rail connected to ground through the source.
#[derive(Clone, Debug)]
pub struct RailElm {
    base: Base,
    /// Waveform parameters.
    pub wave: Waveform,
    kind: RailKind,
    slider_text: String,
    slider_value: i32,
    /// Name of an external voltage, and the value it was last given.
    ext_name: String,
    ext_voltage: f64,
    fm_phase: f64,
    media: Media,
}

impl RailElm {
    /// A rail being placed.
    pub fn new(xx: i32, yy: i32, kind: RailKind) -> Self {
        let wf = match kind {
            RailKind::Dc => WF_DC,
            RailKind::Ac | RailKind::Ext | RailKind::Antenna | RailKind::Data | RailKind::Audio => {
                WF_AC
            }
            RailKind::Square | RailKind::Clock => WF_SQUARE,
            RailKind::Var => WF_VAR,
        };
        let mut base = Base::new(xx, yy);
        base.flags |= FLAG_SHOW_VOLTAGE;
        let mut wave = Waveform::new(wf);
        match kind {
            RailKind::Ac => wave.max_voltage = 120.0 * 2f64.sqrt(),
            RailKind::Clock => {
                wave.max_voltage = 2.5;
                wave.bias = 2.5;
                wave.frequency = 100.0;
                base.flags |= FLAG_CLOCK;
            }
            RailKind::Var => wave.frequency = wave.max_voltage,
            _ => {}
        }
        let mut r = RailElm {
            base,
            wave,
            kind,
            slider_text: "Voltage".into(),
            slider_value: 100,
            ext_name: "ext".into(),
            ext_voltage: 0.0,
            fm_phase: 0.0,
            media: Media::default(),
        };
        if kind == RailKind::Var {
            r.slider_value = r.slider_from_frequency();
        }
        r
    }

    /// A rail read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut base = Base::from_dump(xa, ya, xb, yb, f);
        let wave = Waveform::from_dump(st, &mut base.flags);
        RailElm {
            base,
            wave,
            kind: RailKind::Dc,
            slider_text: "Voltage".into(),
            slider_value: 100,
            ext_name: "ext".into(),
            ext_voltage: 0.0,
            fm_phase: 0.0,
            media: Media::default(),
        }
    }

    /// A data input read from a file: sample length, scale factor and file number follow the
    /// waveform.
    pub fn from_dump_data(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut r = Self::from_dump(xa, ya, xb, yb, f, st);
        r.kind = RailKind::Data;
        r.wave.waveform = WF_AC;
        r.media.sample_length = st.next_f64().unwrap_or(1e-3);
        r.media.scale_factor = st.next_f64().unwrap_or(1.0);
        r.media.file_num = st.next_i32().unwrap_or(0);
        r.media.look_up();
        r
    }

    /// An audio input read from a file: maximum voltage, start position and file number follow
    /// the waveform.
    pub fn from_dump_audio(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut r = Self::from_dump(xa, ya, xb, yb, f, st);
        r.kind = RailKind::Audio;
        r.wave.waveform = WF_AC;
        r.media.max_voltage = st.next_f64().unwrap_or(5.0);
        r.media.start_position = st.next_f64().unwrap_or(0.0);
        r.media.file_num = st.next_i32().unwrap_or(0);
        r.media.look_up();
        r
    }

    /// An external voltage read from a file: a rail with a name.
    pub fn from_dump_ext(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut r = Self::from_dump(xa, ya, xb, yb, f, st);
        r.kind = RailKind::Ext;
        r.ext_name =
            st.next_token().map_or_else(|| "ext".into(), crate::format::tokenizer::unescape);
        r.wave.waveform = WF_AC;
        r
    }

    /// An antenna read from a file.
    pub fn from_dump_antenna(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut r = Self::from_dump(xa, ya, xb, yb, f, st);
        r.kind = RailKind::Antenna;
        r.wave.waveform = WF_AC;
        r
    }

    /// Gives an external voltage its value (ignored by other rails).
    pub fn set_external_voltage(&mut self, v: f64) {
        if !v.is_nan() {
            self.ext_voltage = v;
        }
    }

    /// A variable rail read from a file.
    pub fn from_dump_var(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut r = Self::from_dump(xa, ya, xb, yb, f, st);
        r.kind = RailKind::Var;
        let mut text = st.next_token().ok_or(TokenError::Missing)?.to_string();
        while let Some(t) = st.next_token() {
            text.push(' ');
            text.push_str(t);
        }
        r.slider_text = text.replace("%2b", "+").replace("%2B", "+");
        r.wave.waveform = WF_VAR;
        r.slider_value = r.slider_from_frequency();
        Ok(r)
    }

    fn slider_from_frequency(&self) -> i32 {
        ((self.wave.frequency - self.wave.bias) * 100.0 / (self.wave.max_voltage - self.wave.bias))
            as i32
    }

    /// The voltage at the playback position of a data or audio input.
    fn media_voltage(&mut self) -> f64 {
        let m = &mut self.media;
        let Some(data) = m.data.clone() else { return 0.0 };
        if self.kind == RailKind::Data {
            if data.is_empty() {
                return 0.0;
            }
            let mut ptr = (m.time_offset / m.sample_length).max(0.0) as usize;
            if ptr >= data.len() {
                if self.base.flags & FLAG_REPEAT != 0 {
                    ptr = 0;
                    m.time_offset = 0.0;
                } else {
                    ptr = data.len() - 1;
                }
            }
            return data[ptr] * m.scale_factor;
        }
        if m.time_offset < m.start_position {
            m.time_offset = m.start_position;
        }
        let dptr = m.time_offset * f64::from(m.sampling_rate);
        let iptr = dptr as usize;
        let frac = dptr - iptr as f64;
        let Some(&v1) = data.get(iptr) else { return 0.0 };
        let v2 = data.get(iptr + 1).copied().unwrap_or(0.0);
        (v1 * (1.0 - frac) + v2 * frac) * m.max_voltage
    }

    fn voltage(&mut self, t: f64) -> f64 {
        if matches!(self.kind, RailKind::Data | RailKind::Audio) {
            return self.media_voltage();
        }
        if self.kind == RailKind::Ext {
            return self.ext_voltage;
        }
        if self.kind == RailKind::Antenna {
            let tau = std::f64::consts::TAU;
            let fm = 3.0 * self.fm_phase.sin();
            return (tau * t * 3000.0).sin() * (1.3 + (tau * t * 12.0).sin()) * 3.0
                + (tau * t * 2710.0).sin() * (1.3 + (tau * t * 13.0).sin()) * 3.0
                + (tau * t * 2433.0).sin() * (1.3 + (tau * t * 14.0).sin()) * 3.0
                + fm;
        }
        if self.kind == RailKind::Var {
            self.wave.frequency =
                f64::from(self.slider_value) * (self.wave.max_voltage - self.wave.bias) / 100.0
                    + self.wave.bias;
            return self.wave.frequency;
        }
        self.wave.voltage(t)
    }

    fn rail_text(&self) -> Option<String> {
        match self.kind {
            RailKind::Ext => Some(self.ext_name.clone()),
            RailKind::Antenna => Some("Ant".into()),
            RailKind::Data | RailKind::Audio => Some(self.media.name()),
            _ => None,
        }
    }

    fn draw_rail(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2, l1) = (self.base.point1, self.base.point2, self.base.lead1);
        if let Some(name) = self.rail_text() {
            self.draw_rail_text(g, c, &name);
            return;
        }
        if self.wave.waveform == WF_SQUARE && self.base.flags & FLAG_CLOCK != 0 {
            self.draw_rail_text(g, c, "CLK");
        } else if self.wave.waveform == WF_DC || self.wave.waveform == WF_VAR {
            g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
            set_power_color_w(g, &self.base, c, base_power(&self.base));
            let v = self.voltage(c.sim_time);
            let mut s = if v.abs() < 1.0 {
                format!("{} V", format(v, false))
            } else {
                short_unit_text(v, "V")
            };
            if v > 0.0 {
                s = format!("+{s}");
            }
            draw_labeled_node(g, &mut self.base, c, &s, p1, l1);
        } else {
            let show_v = self.base.flags & FLAG_SHOW_VOLTAGE_RAIL != 0;
            let show_f = c.show_values && self.wave.waveform != WF_NOISE;
            let text = match (show_v, show_f) {
                (true, true) => Some(format!(
                    "{} {}",
                    self.wave.short_voltage_text(),
                    short_unit_text(self.wave.frequency, "Hz")
                )),
                (true, false) => Some(self.wave.short_voltage_text()),
                (false, true) => Some(short_unit_text(self.wave.frequency, "Hz")),
                (false, false) => None,
            };
            let wave = self.wave.clone();
            wave.draw_waveform(g, &mut self.base, c, p2, text.as_deref(), p1, l1);
        }
    }

    fn draw_rail_text(&mut self, g: &mut dyn Painter, c: &DrawCtx, s: &str) {
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
        set_power_color_w(g, &self.base, c, base_power(&self.base));
        let (p1, l1) = (self.base.point1, self.base.lead1);
        draw_labeled_node(g, &mut self.base, c, s, p1, l1);
    }
}

impl Element for RailElm {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        match self.kind {
            RailKind::Dc => "RailElm",
            RailKind::Ac => "ACRailElm",
            RailKind::Square => "SquareRailElm",
            RailKind::Clock => "ClockElm",
            RailKind::Var => "VarRailElm",
            RailKind::Ext => "ExtVoltageElm",
            RailKind::Antenna => "AntennaElm",
            RailKind::Data => "DataInputElm",
            RailKind::Audio => "AudioInputElm",
        }
    }
    fn dump_type(&self) -> i32 {
        match self.kind {
            RailKind::Var => 172,
            RailKind::Ext => 418,
            RailKind::Data => 424,
            RailKind::Audio => 411,
            RailKind::Antenna => i32::from(b'A'),
            _ => i32::from(b'R'),
        }
    }
    fn xml_dump_type(&self) -> String {
        match self.kind {
            RailKind::Var => "VarRail".into(),
            RailKind::Ext => "ExtVoltage".into(),
            RailKind::Data => "DataInput".into(),
            RailKind::Audio => "ain".into(),
            _ => "R".into(),
        }
    }
    fn is_rail(&self) -> bool {
        true
    }
    fn is_voltage_elm(&self) -> bool {
        true
    }
    fn post_count(&self) -> usize {
        1
    }
    fn internal_node_count(&self) -> usize {
        usize::from(self.wave.internal_resistance > 0.0)
    }
    fn voltage_source_count(&self) -> usize {
        1
    }

    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.wave.undump_xml(xml);
        if self.kind == RailKind::Var {
            self.slider_text = xml.string_attr("st", &self.slider_text);
            self.slider_value = self.slider_from_frequency();
        }
        if self.kind == RailKind::Ext {
            self.ext_name = xml.string_attr("nm", &self.ext_name);
        }
        let m = &mut self.media;
        match self.kind {
            RailKind::Data => {
                m.sample_length = xml.double_attr("sl", m.sample_length);
                m.scale_factor = xml.double_attr("sf", m.scale_factor);
                m.file_num = xml.int_attr("fn", 0);
                m.look_up();
            }
            RailKind::Audio => {
                m.max_voltage = xml.double_attr("ma", m.max_voltage);
                m.start_position = xml.double_attr("st", m.start_position);
                m.file_num = xml.int_attr("fi", m.file_num);
                m.look_up();
            }
            _ => {}
        }
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        self.wave.dump_xml(out);
        if self.kind == RailKind::Ext {
            out.str_attr("nm", &self.ext_name);
        }
        let m = &self.media;
        match self.kind {
            RailKind::Data => {
                out.double_attr("sl", m.sample_length);
                out.double_attr("sf", m.scale_factor);
            }
            RailKind::Audio => {
                out.double_attr("ma", m.max_voltage);
                out.double_attr("st", m.start_position);
                out.int_attr("fi", m.file_num);
            }
            _ => {}
        }
        if self.kind == RailKind::Var {
            out.str_attr("st", &self.slider_text);
        }
    }

    fn dump_xml_state(&self, out: &mut XmlOut) {
        if self.kind == RailKind::Data && self.media.data.is_some() {
            out.int_attr("fn", self.media.file_num);
        }
    }

    fn reset(&mut self) {
        self.wave.freq_time_zero = 0.0;
        self.base.curcount = 0.0;
        self.media.time_offset =
            if self.kind == RailKind::Audio { self.media.start_position } else { 0.0 };
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let w = &self.wave;
        self.base.calc_leads(if w.waveform == WF_DC || w.waveform == WF_VAR {
            8
        } else {
            CIRCLE_SIZE * 2
        });
        let b = &mut self.base;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - f64::from(CIRCLE_SIZE) / b.dn);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let rt = self.rail_text();
        let dn = self.base.dn;
        let mut w = match &rt {
            None => f64::from(CIRCLE_SIZE),
            Some(t) => g.measure_text(t) / 2.0,
        };
        if w > dn * 0.8 {
            w = dn * 0.8;
        }
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.lead1 = interp_point(p1, p2, 1.0 - w / dn);
        self.base.set_bbox_pts(p1, p2, f64::from(CIRCLE_SIZE));
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        draw_thick_line(g, p1, self.base.lead1);
        self.draw_rail(g, c);
        draw_posts(g, &self.base, self, c);
        self.base.curcount =
            self.base.update_dot_count_for(-self.base.current, self.base.curcount, c);
        let b = &self.base;
        if !b.creating {
            draw_dots(g, c, b.point1, b.lead1, b.curcount);
        }
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_power(&self) -> f64 {
        -self.base.volts[0] * self.base.current
    }

    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base.volt_source = Some(v);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let Some(vs) = self.base.volt_source else { return };
        let n = self.base.nodes.clone();
        let ir = self.wave.internal_resistance > 0.0;
        let vs_node = if ir { n[1] } else { n[0] };
        s.set_vs_nodes(vs, NodeId::GROUND, vs_node);
        if self.wave.waveform == WF_DC {
            let v = self.voltage(s.t);
            s.stamp_voltage_source(NodeId::GROUND, vs_node, vs, v);
        } else {
            s.stamp_voltage_source_var(NodeId::GROUND, vs_node, vs);
        }
        if ir {
            s.stamp_resistor(n[1], n[0], self.wave.internal_resistance);
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        if self.wave.waveform != WF_DC {
            if let Some(vs) = self.base.volt_source {
                let v = self.voltage(s.t);
                s.update_voltage_source(vs, v);
            }
        }
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if matches!(self.kind, RailKind::Data | RailKind::Audio) {
            self.media.time_offset += s.time_step;
        }
        if self.kind == RailKind::Antenna {
            let tau = std::f64::consts::TAU;
            self.fm_phase += tau * (2200.0 + (tau * s.t * 13.0).sin() * 100.0) * s.time_step;
        }
        if self.wave.waveform == WF_NOISE {
            self.wave.noise_value =
                (s.random() * 2.0 - 1.0) * self.wave.max_voltage + self.wave.bias;
        }
    }

    fn has_ground_connection(&self, _n1: usize) -> bool {
        true
    }
    fn shortcut(&self) -> i32 {
        if self.kind == RailKind::Dc { i32::from(b'V') } else { 0 }
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        if self.wave.internal_resistance > 0.0 {
            return true;
        }
        if v.find_path(PathKind::Voltage, idx, self.base.nodes[0], NodeId::GROUND) {
            self.wave.internal_resistance = 0.001;
            return false;
        }
        true
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        self.wave.info(true, self.base.current, self.get_voltage_diff(), self.get_power(), arr);
        if self.kind == RailKind::Ext {
            arr[0] = format!("ext. voltage ({})", self.ext_name);
        }
        if self.kind == RailKind::Antenna {
            arr[0] = "Antenna (amplified)".into();
        }
        if matches!(self.kind, RailKind::Data | RailKind::Audio) {
            let m = &self.media;
            arr.clear();
            arr.push(if self.kind == RailKind::Data { "data input" } else { "audio input" }.into());
            let Some(data) = &m.data else {
                arr.push("no file loaded".into());
                return;
            };
            arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
            arr.push(format!("pos = {}", unit_text(m.time_offset, "s")));
            let dur = if self.kind == RailKind::Data {
                data.len() as f64 * m.sample_length
            } else {
                data.len() as f64 / f64::from(m.sampling_rate.max(1))
            };
            arr.push(format!("dur = {}", unit_text(dur, "s")));
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        if self.kind == RailKind::Antenna {
            return None;
        }
        if self.kind == RailKind::Data {
            let m = &self.media;
            return match n {
                0 => Some(
                    EditInfo::text("Data File (path)", "").open_file(&["*.txt", "*.csv", "*.dat"]),
                ),
                1 => Some(EditInfo::number("Scale Factor", m.scale_factor)),
                2 => Some(EditInfo::number("Sample Length (s)", m.sample_length)),
                3 => Some(EditInfo::checkbox("Repeat", self.base.flags & FLAG_REPEAT != 0)),
                _ => None,
            };
        }
        if self.kind == RailKind::Audio {
            let m = &self.media;
            return match n {
                0 => Some(EditInfo::text("Audio File (path, WAV)", "").open_file(&["*.wav"])),
                1 => Some(EditInfo::number("Max Voltage", m.max_voltage).set_unit_step()),
                2 => Some(EditInfo::number("Start Position (s)", m.start_position)),
                _ => None,
            };
        }
        if self.kind == RailKind::Ext {
            return (n == 0).then(|| EditInfo::text("Name", &self.ext_name));
        }
        if self.kind == RailKind::Var {
            return match n {
                0 => {
                    Some(EditInfo::new("Min Voltage", self.wave.bias, -20.0, 20.0).set_unit_step())
                }
                1 => Some(
                    EditInfo::new("Max Voltage", self.wave.max_voltage, -20.0, 20.0)
                        .set_unit_step(),
                ),
                2 => Some(EditInfo {
                    text: Some(self.slider_text.clone()),
                    ..EditInfo::text("Slider Text", "")
                }),
                _ => None,
            };
        }
        self.wave.edit_info(n, self.base.flags, true)
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if matches!(self.kind, RailKind::Data | RailKind::Audio) {
            let audio = self.kind == RailKind::Audio;
            match n {
                0 => {
                    let path = ei.text.as_deref().map(str::trim).unwrap_or_default();
                    if !path.is_empty() {
                        if let Err(e) = self.media.load(path, audio) {
                            ei.error = Some(e);
                        }
                    }
                }
                1 if audio => self.media.max_voltage = ei.value,
                1 => self.media.scale_factor = ei.value,
                2 if audio => self.media.start_position = ei.value,
                2 => self.media.sample_length = ei.value,
                3 if !audio => self.base.flags = ei.change_flag(self.base.flags, FLAG_REPEAT),
                _ => {}
            }
            return;
        }
        if self.kind == RailKind::Ext {
            if let (0, Some(t)) = (n, &ei.text) {
                self.ext_name.clone_from(t);
            }
            return;
        }
        if self.kind == RailKind::Var {
            match n {
                0 => self.wave.bias = ei.value,
                1 => self.wave.max_voltage = ei.value,
                2 => {
                    if let Some(t) = &ei.text {
                        self.slider_text.clone_from(t);
                    }
                }
                _ => {}
            }
            return;
        }
        let mut flags = self.base.flags;
        let points = self.wave.set_edit(n, ei, &mut flags, true);
        self.base.flags = flags;
        if points {
            Element::set_points(self);
        }
    }

    fn slider(&self) -> Option<ElementSlider> {
        (self.kind == RailKind::Var)
            .then(|| ElementSlider { text: self.slider_text.clone(), value: self.slider_value })
    }
    fn set_slider_value(&mut self, v: i32) {
        self.slider_value = v.clamp(0, 100);
    }
}

// ---------------------------------------------------------------------------------------------
// Current source
// ---------------------------------------------------------------------------------------------

/// An ideal (optionally voltage-limited) current source.
#[derive(Clone, Debug)]
pub struct CurrentElm {
    base: Base,
    /// Source current in amps.
    pub current_value: f64,
    /// Compliance voltage (0 = unlimited).
    pub max_voltage: f64,
    last_volt_diff: f64,
    broken: bool,
    arrow: Polygon,
    ashaft: [Point; 2],
    center: Point,
    /// True for the ohmmeter, a current source that shows voltage over current.
    ohmmeter: bool,
}

impl CurrentElm {
    /// A current source being placed.
    pub fn new(xx: i32, yy: i32) -> Self {
        CurrentElm {
            base: Base::new(xx, yy),
            current_value: 0.01,
            max_voltage: 0.0,
            last_volt_diff: 0.0,
            broken: false,
            arrow: Polygon::new(),
            ashaft: [Point::default(); 2],
            center: Point::default(),
            ohmmeter: false,
        }
    }

    /// An ohmmeter being placed.
    pub fn new_ohmmeter(xx: i32, yy: i32) -> Self {
        let mut c = Self::new(xx, yy);
        c.ohmmeter = true;
        c
    }

    /// An ohmmeter read from a file.
    pub fn from_dump_ohmmeter(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut c = Self::from_dump(xa, ya, xb, yb, f, st);
        c.ohmmeter = true;
        c
    }

    fn resistance(&self) -> f64 {
        (self.base.volts[1] - self.base.volts[0]) / self.base.current
    }

    /// A current source read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut c = CurrentElm::new(xa, ya);
        c.base = Base::from_dump(xa, ya, xb, yb, f);
        if let Ok(v) = st.next_f64() {
            c.current_value = v;
            if let Ok(m) = st.next_f64() {
                c.max_voltage = m;
            }
        }
        if c.current_value == 0.0 {
            c.current_value = 0.01;
        }
        c
    }

    fn is_voltage_limited(&self) -> bool {
        self.max_voltage > 0.0
    }
}

impl Element for CurrentElm {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        if self.ohmmeter { "OhmMeterElm" } else { "CurrentElm" }
    }
    fn dump_type(&self) -> i32 {
        if self.ohmmeter { 216 } else { i32::from(b'i') }
    }
    fn scope_value(&self, x: i32) -> f64 {
        if self.ohmmeter && x == crate::scope::VAL_R {
            return self.resistance();
        }
        match x {
            crate::scope::VAL_CURRENT => self.get_current(),
            crate::scope::VAL_POWER => self.get_power(),
            _ => self.get_voltage_diff(),
        }
    }
    fn scope_units(&self, x: i32) -> i32 {
        if self.ohmmeter && x == crate::scope::VAL_R {
            return crate::scope::UNITS_OHMS;
        }
        match x {
            crate::scope::VAL_CURRENT => crate::scope::UNITS_A,
            crate::scope::VAL_POWER => crate::scope::UNITS_W,
            _ => crate::scope::UNITS_V,
        }
    }
    fn can_show_value_in_scope(&self, v: i32) -> bool {
        self.ohmmeter && v == crate::scope::VAL_R
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.current_value = xml.double_attr("cu", self.current_value);
        self.max_voltage = xml.double_attr("mv", 0.0);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("cu", self.current_value);
        if self.max_voltage > 0.0 {
            out.double_attr("mv", self.max_voltage);
        }
    }
    fn non_linear(&self) -> bool {
        self.is_voltage_limited()
    }
    fn reset(&mut self) {
        let b = &mut self.base;
        b.volts.iter_mut().for_each(|v| *v = 0.0);
        b.curcount = 0.0;
        self.last_volt_diff = 0.0;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(26);
        let b = &self.base;
        self.ashaft = [interp_point(b.lead1, b.lead2, 0.25), interp_point(b.lead1, b.lead2, 0.6)];
        self.center = interp_point(b.lead1, b.lead2, 0.5);
        let p2 = interp_point(b.lead1, b.lead2, 0.75);
        self.arrow = calc_arrow(self.center, p2, 4.0, 4.0);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let cr = 12;
        draw_2_leads(g, &self.base, c);
        let b = &self.base;
        set_voltage_color(g, b, c, (b.volts[0] + b.volts[1]) / 2.0);
        set_power_color_w(g, b, c, self.get_power());
        draw_thick_circle(g, self.center.x, self.center.y, cr);
        if self.ohmmeter {
            let (cx, cy) = (self.center.x, self.center.y);
            draw_centered_text(g, &mut self.base, crate::locale::OHM, cx, cy, true);
        } else {
            draw_thick_line(g, self.ashaft[0], self.ashaft[1]);
            fill_polygon(g, &self.arrow);
        }
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(cr));
        do_dots(g, &mut self.base, c);
        if c.show_values && self.base.current != 0.0 {
            let s = if self.ohmmeter {
                short_unit_text(self.resistance(), crate::locale::OHM)
            } else {
                short_unit_text(self.base.current, "A")
            };
            if self.base.dx == 0 || self.base.dy == 0 {
                draw_values(g, &self.base, c, Some(&s), f64::from(cr), false, false);
            }
        }
        draw_posts(g, &self.base, self, c);
    }

    // stamping is deferred until we know whether the source has a current path
    fn stamp(&mut self, s: &mut Stamper) {
        let n = &self.base.nodes;
        if self.broken {
            // no current path: a current source would cause a matrix error
            s.stamp_resistor(n[0], n[1], 1e8);
            self.base.current = 0.0;
        } else if self.is_voltage_limited() {
            s.stamp_non_linear(n[0]);
            s.stamp_non_linear(n[1]);
        } else {
            s.stamp_current_source(n[0], n[1], self.current_value);
            self.base.current = self.current_value;
        }
    }

    // smooth voltage compliance with a tanh-shaped saturation between 0.95 and 1.0 times the
    // compliance voltage
    fn do_step(&mut self, s: &mut Stamper) {
        if self.broken || !self.is_voltage_limited() {
            return;
        }
        let n = self.base.nodes.clone();
        let mut vd = self.base.volts[1] - self.base.volts[0];
        let v_start = 0.95 * self.max_voltage;
        let v_width = self.max_voltage - v_start;
        let v_mid = (v_start + self.max_voltage) / 2.0;
        let vt = (v_width / 5.0).max(1e-3);
        // limit the step so a Newton iteration cannot jump across the transition region
        if self.last_volt_diff < v_start && vd > v_start {
            vd = v_start;
            s.converged = false;
        } else if self.last_volt_diff > self.max_voltage && vd < self.max_voltage {
            vd = self.max_voltage;
            s.converged = false;
        } else if self.last_volt_diff >= v_start && self.last_volt_diff <= self.max_voltage {
            let max_step = (v_width / 4.0).max(0.01);
            if vd > self.last_volt_diff + max_step {
                vd = self.last_volt_diff + max_step;
                s.converged = false;
            } else if vd < self.last_volt_diff - max_step {
                vd = self.last_volt_diff - max_step;
                s.converged = false;
            }
        }
        self.last_volt_diff = vd;
        let arg = (vd - v_mid) / vt;
        let tanh_arg = arg.tanh();
        let i = self.current_value * 0.5 * (1.0 - tanh_arg);
        let sech2 = 1.0 - tanh_arg * tanh_arg;
        let g = -self.current_value * 0.5 * sech2 / vt * vd;
        // a Gmin floor keeps the Norton resistance finite where sech^2 vanishes
        let abs_g = g.abs() + 1e-6;
        s.stamp_resistor(n[0], n[1], 1.0 / abs_g);
        s.stamp_current_source(n[0], n[1], i - g * vd);
        self.base.current = i;
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        if self.ohmmeter {
            return None;
        }
        match n {
            0 => Some(EditInfo::new("Current (A)", self.current_value, 0.0, 0.1)),
            1 => Some(
                EditInfo::new("Max Voltage (V, 0=unlimited)", self.max_voltage, 0.0, 0.0)
                    .set_unit_step(),
            ),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            self.current_value = ei.value;
        }
        if n == 1 && ei.value >= 0.0 {
            self.max_voltage = ei.value;
        }
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        if self.ohmmeter {
            arr.push("ohmmeter".into());
            arr.push(if self.base.current == 0.0 {
                "R = \u{221e}".to_string()
            } else {
                format!("R = {}", unit_text(self.resistance(), crate::locale::OHM))
            });
            return;
        }
        arr.push("current source".into());
        basic_info(self, arr);
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
        if self.is_voltage_limited() {
            arr.push(format!("Vmax = {}", voltage_text(self.max_voltage)));
        }
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[1] - self.base.volts[0]
    }
    fn get_power(&self) -> f64 {
        -self.get_voltage_diff() * self.base.current
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        let (n0, n1) = (self.base.nodes[0], self.base.nodes[1]);
        let path = v.find_path(PathKind::Inductor, idx, n1, n0);
        self.broken = !path && !self.is_voltage_limited();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_voltage_is_max_plus_bias() {
        let v = VoltageElm::new_dc(0, 0);
        assert_eq!(v.wave.voltage(1.0), 5.0);
    }

    #[test]
    fn sine_wave_follows_frequency_and_phase() {
        let mut v = VoltageElm::new_ac(0, 0);
        v.wave.max_voltage = 2.0;
        v.wave.frequency = 1.0;
        assert!((v.wave.voltage(0.25) - 2.0).abs() < 1e-12);
        assert!(v.wave.voltage(0.5).abs() < 1e-12);
    }

    #[test]
    fn square_wave_respects_duty_cycle() {
        let mut w = Waveform::new(WF_SQUARE);
        w.max_voltage = 1.0;
        w.frequency = 1.0;
        w.duty_cycle = 0.25;
        assert_eq!(w.voltage(0.1), 1.0);
        assert_eq!(w.voltage(0.5), -1.0);
    }

    #[test]
    fn old_files_pulse_duty_and_cosine_flag_are_fixed_up() {
        let mut st = Tokenizer::new("5 40 5 0 0 0.5");
        let v = VoltageElm::from_dump(0, 0, 0, 64, FLAG_COS, &mut st);
        assert_eq!(v.base.flags & FLAG_COS, 0);
        assert!((v.wave.phase_shift - PI / 2.0).abs() < 1e-12);
        assert!((v.wave.duty_cycle - DEFAULT_PULSE_DUTY).abs() < 1e-12);
    }

    #[test]
    fn missing_waveform_fields_keep_defaults() {
        let mut st = Tokenizer::new("1 50");
        let v = VoltageElm::from_dump(0, 0, 0, 64, 0, &mut st);
        assert_eq!((v.wave.waveform, v.wave.frequency, v.wave.max_voltage), (WF_AC, 50.0, 5.0));
    }

    #[test]
    fn rail_has_one_post_and_xml_round_trips() {
        let mut r = RailElm::new(0, 0, RailKind::Dc);
        assert_eq!(r.post_count(), 1);
        r.wave.max_voltage = 9.0;
        let mut out = XmlOut::new("R");
        r.dump_xml(&mut out);
        let node = crate::format::xml::parse(&out.to_pretty_string()).unwrap();
        let mut back = RailElm::new(0, 0, RailKind::Dc);
        back.undump_xml(&node);
        assert_eq!(back.wave.max_voltage, 9.0);
    }

    #[test]
    fn variable_rail_follows_its_slider() {
        let mut r = RailElm::new(0, 0, RailKind::Var);
        r.wave.bias = 0.0;
        r.wave.max_voltage = 10.0;
        r.set_slider_value(50);
        assert!((r.voltage(0.0) - 5.0).abs() < 1e-12);
        assert_eq!(r.slider().unwrap().value, 50);
    }

    #[test]
    fn current_source_with_zero_value_in_file_uses_default() {
        let mut st = Tokenizer::new("0 0");
        let c = CurrentElm::from_dump(0, 0, 0, 64, 0, &mut st);
        assert_eq!(c.current_value, 0.01);
    }
}
