// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (555 timer, converters, phase
// comparator and VCO chips).
//! Mixed analog/digital chips built on the [`ChipElm`] framework.

use super::chip::*;
use super::element::*;
use super::registry::{ElementDescriptor, Registry};
use crate::format::number::js_number;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::solver::stamper::{NodeId, Stamper};

/// Family name used for the feature switch.
pub const FAMILY: &str = "analog";
const CAT: &str = "Analog and Hybrid Chips";

fn fill(c: &mut ChipCore, n: usize) {
    c.pins = vec![Pin::new(0, SIDE_W, ""); n];
}

fn has(c: &ChipCore, flag: i32) -> bool {
    c.base.flags & flag != 0
}

fn set_flag(c: &mut ChipCore, flag: i32, on: bool) {
    if on {
        c.base.flags |= flag;
    } else {
        c.base.flags &= !flag;
    }
}

fn bits_field(c: &ChipCore) -> EditInfo {
    EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0).set_dimensionless()
}

fn set_bits(c: &mut ChipCore, ei: &mut EditInfo) {
    if ei.value >= 2.0 {
        c.bits = ei.value as i32;
    } else {
        ei.error = Some("must be >= 2".into());
    }
}

/// Registers the hybrid chips.
pub fn register(r: &mut Registry) {
    macro_rules! chip {
        ($id:expr, $menu:expr, $code:expr, $ctor:expr) => {
            r.add(
                ElementDescriptor::new($id, FAMILY, $menu, CAT, |x, y| {
                    Box::new(ChipElm::new($ctor, x, y))
                })
                .dump($code, |a, b, c, d, f, st| {
                    Ok(Box::new(ChipElm::from_dump($ctor, a, b, c, d, f, st)))
                }),
            );
        };
    }
    chip!("TimerElm", "Add 555 Timer", 165, Timer::default());
    chip!("PhaseCompElm", "Add Phase Comparator", 161, PhaseComp::default());
    chip!("VCOElm", "Add VCO", 158, Vco::default());
    chip!("CC2Elm", "Add Current Conveyor (+)", 179, Cc2 { gain: 1.0 });
    r.add(
        ElementDescriptor::new("CC2NegElm", FAMILY, "Add Current Conveyor (-)", CAT, |x, y| {
            Box::new(ChipElm::new(Cc2 { gain: -1.0 }, x, y))
        })
        .hidden(),
    );
    chip!("TimeDelayRelayElm", "Add Time Delay Relay", 414, TimeDelayRelay::new());
    chip!("AnalogMuxElm", "Add Analog Multiplexer", 432, AnalogMux::new());
    chip!("ADCElm", "Add ADC", 167, Adc);
    chip!("DACElm", "Add DAC", 166, Dac);
}

// ---------------------------------------------------------------------------------------------
// 555 timer
// ---------------------------------------------------------------------------------------------

const N_DIS: usize = 0;
const N_TRIG: usize = 1;
const N_THRES: usize = 2;
const N_VCC: usize = 3;
const N_CTL: usize = 4;
const N_OUT: usize = 5;
const N_RST: usize = 6;
const N_GND: usize = 7;

/// The 555 timer.
#[derive(Clone, Debug, Default)]
pub struct Timer {
    out: bool,
    trigger_suppressed: bool,
}

impl Timer {
    const FLAG_RESET: i32 = 2;
    const FLAG_GROUND: i32 = 4;
    const FLAG_NUMBERS: i32 = 8;

    fn has_ground_pin(c: &ChipCore) -> bool {
        has(c, Self::FLAG_GROUND)
    }
    fn has_reset(c: &ChipCore) -> bool {
        has(c, Self::FLAG_RESET) || Self::has_ground_pin(c)
    }
    fn ground(c: &ChipCore) -> NodeId {
        if Self::has_ground_pin(c) { c.base.nodes[N_GND] } else { NodeId::GROUND }
    }
}

impl ChipLogic for Timer {
    fn class_name(&self) -> &'static str {
        "TimerElm"
    }
    fn dump_type(&self) -> i32 {
        165
    }
    fn chip_name(&self) -> &'static str {
        "555 Timer"
    }
    fn is_digital(&self) -> bool {
        false
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn before_new(&mut self, c: &mut ChipCore) {
        c.base.flags |= Self::FLAG_RESET | Self::FLAG_GROUND;
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 3;
        c.size_y = 5;
        let names = !has(c, Self::FLAG_NUMBERS);
        let n = |name: &'static str, num: &'static str| if names { name } else { num };
        fill(c, 8);
        c.pins[N_DIS] = Pin::new(1, SIDE_W, n("dis", "7"));
        c.pins[N_TRIG] = Pin::new(3, SIDE_W, n("tr", "2"));
        c.pins[N_TRIG].line_over = names;
        c.pins[N_THRES] = Pin::new(4, SIDE_W, n("th", "6"));
        c.pins[N_VCC] = Pin::new(1, SIDE_N, n("Vcc", "8"));
        c.pins[N_CTL] = Pin::new(1, SIDE_S, n("ctl", "5"));
        c.pins[N_OUT] = Pin::new(2, SIDE_E, n("out", "3"));
        c.pins[N_OUT].state = true;
        c.pins[N_RST] = Pin::new(1, SIDE_E, n("rst", "4"));
        c.pins[N_RST].line_over = names;
        c.pins[N_GND] = Pin::new(2, SIDE_S, n("gnd", "1"));
        let posts = if Self::has_ground_pin(c) {
            8
        } else if Self::has_reset(c) {
            7
        } else {
            6
        };
        c.pins.truncate(posts);
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        0
    }
    fn matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn ground_connection(&self, _c: &ChipCore, _n: usize) -> bool {
        false
    }
    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let n = &c.base.nodes;
        let ground = Self::ground(c);
        s.stamp_resistor(n[N_VCC], n[N_CTL], 5000.0);
        s.stamp_resistor(n[N_CTL], ground, 10000.0);
        true
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let v = &c.base.volts;
        let ground_volts = if Self::has_ground_pin(c) { v[N_GND] } else { 0.0 };
        self.out = v[N_OUT] > (v[N_VCC] + ground_volts) / 2.0;
        if v[N_THRES] > v[N_CTL] {
            self.out = false;
        }
        let triggered = (v[N_CTL] + ground_volts) / 2.0 > v[N_TRIG];
        if triggered || self.trigger_suppressed {
            self.out = true;
        }
        if Self::has_reset(c) && v[N_RST] < 0.7 + ground_volts {
            self.out = false;
            self.trigger_suppressed = triggered;
        } else {
            self.trigger_suppressed = false;
        }
    }
    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let n = &c.base.nodes;
        let ground = Self::ground(c);
        if !self.out {
            s.stamp_resistor(n[N_DIS], ground, 10.0);
        }
        s.stamp_resistor(if self.out { n[N_VCC] } else { ground }, n[N_OUT], 1.0);
        true
    }
    fn calculate_current(&mut self, c: &mut ChipCore) {
        let v = c.base.volts.clone();
        let has_ground = Self::has_ground_pin(c);
        let gv = if has_ground { v[N_GND] } else { 0.0 };
        let p = &mut c.pins;
        p[N_VCC].current = (v[N_CTL] - v[N_VCC]) / 5000.0;
        p[N_CTL].current = -(v[N_CTL] - gv) / 10000.0 - p[N_VCC].current;
        p[N_DIS].current = if self.out { 0.0 } else { -(v[N_DIS] - gv) / 10.0 };
        p[N_OUT].current = -(v[N_OUT] - if self.out { v[N_VCC] } else { gv });
        if self.out {
            p[N_VCC].current -= p[N_OUT].current;
        }
        if has_ground {
            let mut g = (v[N_CTL] - gv) / 10000.0;
            if !self.out {
                g += (v[N_DIS] - gv) / 10.0 + (v[N_OUT] - gv);
            }
            p[N_GND].current = g;
        }
    }
    fn on_reset(&mut self, _c: &mut ChipCore) {
        self.out = false;
        self.trigger_suppressed = false;
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::checkbox("Ground Pin", Self::has_ground_pin(c))),
            1 => Some(EditInfo::checkbox("Show Pin Numbers", has(c, Self::FLAG_NUMBERS))),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => set_flag(c, Self::FLAG_GROUND, on),
            1 => set_flag(c, Self::FLAG_NUMBERS, on),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Phase comparator
// ---------------------------------------------------------------------------------------------

/// Phase comparator (edge-triggered, three-state output).
#[derive(Clone, Debug, Default)]
pub struct PhaseComp {
    ff1: bool,
    ff2: bool,
}

impl ChipLogic for PhaseComp {
    fn class_name(&self) -> &'static str {
        "PhaseCompElm"
    }
    fn dump_type(&self) -> i32 {
        161
    }
    fn chip_name(&self) -> &'static str {
        "phase comparator"
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 2;
        fill(c, 3);
        c.pins[0] = Pin::new(0, SIDE_W, "I1");
        c.pins[1] = Pin::new(1, SIDE_W, "I2");
        c.pins[2] = Pin::output(0, SIDE_E, "O");
    }
    fn matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn stamp(&mut self, _c: &mut ChipCore, _s: &mut Stamper) -> bool {
        true
    }
    fn reads_own_inputs(&self) -> bool {
        true
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let th = c.threshold();
        let v1 = c.base.volts[0] > th;
        let v2 = c.base.volts[1] > th;
        if v1 && !c.pins[0].value {
            self.ff1 = true;
        }
        if v2 && !c.pins[1].value {
            self.ff2 = true;
        }
        if self.ff1 && self.ff2 {
            self.ff1 = false;
            self.ff2 = false;
        }
        c.pins[0].value = v1;
        c.pins[1].value = v2;
    }
    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let Some(vs) = c.pins[2].volt_source else { return true };
        let out = if self.ff1 {
            Some(c.high_voltage)
        } else if self.ff2 {
            Some(0.0)
        } else {
            None
        };
        match out {
            Some(v) => s.stamp_voltage_source(NodeId::GROUND, c.base.nodes[2], vs, v),
            None => s.stamp_matrix(vs, vs, 1.0),
        }
        true
    }
    fn on_reset(&mut self, _c: &mut ChipCore) {
        self.ff1 = false;
        self.ff2 = false;
    }
}

// ---------------------------------------------------------------------------------------------
// VCO
// ---------------------------------------------------------------------------------------------

const VCO_C_RESISTANCE: f64 = 1e6;

/// Voltage-controlled oscillator.
#[derive(Clone, Debug)]
pub struct Vco {
    c_dir: f64,
}

impl Default for Vco {
    fn default() -> Self {
        Vco { c_dir: 1.0 }
    }
}

impl ChipLogic for Vco {
    fn class_name(&self) -> &'static str {
        "VCOElm"
    }
    fn dump_type(&self) -> i32 {
        158
    }
    fn chip_name(&self) -> &'static str {
        "VCO"
    }
    fn is_digital(&self) -> bool {
        false
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 4;
        fill(c, 6);
        c.pins[0] = Pin::new(0, SIDE_W, "Vi");
        c.pins[1] = Pin::output(3, SIDE_W, "Vo");
        c.pins[2] = Pin::new(0, SIDE_E, "C");
        c.pins[3] = Pin::new(1, SIDE_E, "C");
        c.pins[4] = Pin::output(2, SIDE_E, "R1");
        c.pins[5] = Pin::output(3, SIDE_E, "R2");
    }
    fn matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let n = c.base.nodes.clone();
        let vs = |i: usize| c.pins[i].volt_source;
        if let (Some(v1), Some(v4), Some(v5)) = (vs(1), vs(4), vs(5)) {
            s.stamp_voltage_source_var(NodeId::GROUND, n[1], v1);
            s.stamp_voltage_source(n[0], n[4], v4, 0.0);
            s.stamp_voltage_source(NodeId::GROUND, n[5], v5, 5.0);
        }
        s.stamp_resistor(n[2], n[3], VCO_C_RESISTANCE);
        true
    }
    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let vc = c.base.volts[3] - c.base.volts[2];
        let mut vo = c.base.volts[1];
        let mut dir = if vo < 2.5 { 1.0 } else { -1.0 };
        if vo < 2.5 && vc > 4.5 {
            vo = 5.0;
            dir = -1.0;
        }
        if vo > 2.5 && vc < 0.5 {
            vo = 0.0;
            dir = 1.0;
        }
        let n = c.base.nodes.clone();
        if let (Some(v1), Some(v4), Some(v5)) =
            (c.pins[1].volt_source, c.pins[4].volt_source, c.pins[5].volt_source)
        {
            s.update_voltage_source(v1, vo);
            s.stamp_matrix(n[2], v4, dir);
            s.stamp_matrix(n[2], v5, dir);
            s.stamp_matrix(n[3], v4, -dir);
            s.stamp_matrix(n[3], v5, -dir);
        }
        self.c_dir = dir;
        true
    }
    fn calculate_current(&mut self, c: &mut ChipCore) {
        let cur = self.c_dir * (c.pins[4].current + c.pins[5].current)
            + (c.base.volts[3] - c.base.volts[2]) / VCO_C_RESISTANCE;
        c.pins[2].current = -cur;
        c.pins[3].current = cur;
        c.pins[0].current = -c.pins[4].current;
    }
}

// ---------------------------------------------------------------------------------------------
// ADC / DAC
// ---------------------------------------------------------------------------------------------

/// Analog to digital converter.
#[derive(Clone, Debug, Default)]
pub struct Adc;

impl ChipLogic for Adc {
    fn class_name(&self) -> &'static str {
        "ADCElm"
    }
    fn dump_type(&self) -> i32 {
        167
    }
    fn chip_name(&self) -> &'static str {
        "ADC"
    }
    fn is_digital(&self) -> bool {
        false
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn allow_bus(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        c.size_x = 2;
        let bits_y = if c.use_bus() { 1 } else { c.bits.max(2) };
        c.size_y = bits_y.max(2);
        fill(c, bits + 2);
        c.make_bit_pins(bits, 0, SIDE_E, 0, "D", true, false, false);
        c.pins[bits] = Pin::new(0, SIDE_W, "In");
        c.pins[bits + 1] = Pin::new(c.size_y - 1, SIDE_W, "V+");
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let bits = c.bits.max(1) as usize;
        let imax = (1i64 << bits) - 1;
        let val = imax as f64 * c.base.volts[bits] / c.base.volts[bits + 1];
        let ival = (val as i64).clamp(0, imax);
        for i in 0..bits {
            c.pins[i].value = ival & (1 << i) != 0;
        }
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| bits_field(c))
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            set_bits(c, ei);
        }
    }
}

/// Digital to analog converter.
#[derive(Clone, Debug, Default)]
pub struct Dac;

impl ChipLogic for Dac {
    fn class_name(&self) -> &'static str {
        "DACElm"
    }
    fn dump_type(&self) -> i32 {
        166
    }
    fn chip_name(&self) -> &'static str {
        "DAC"
    }
    fn is_digital(&self) -> bool {
        false
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn allow_bus(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        c.size_x = 2;
        let bits_y = if c.use_bus() { 1 } else { c.bits.max(2) };
        c.size_y = bits_y.max(2);
        fill(c, bits + 2);
        c.make_bit_pins(bits, 0, SIDE_W, 0, "D", false, false, false);
        c.pins[bits] = Pin::output(0, SIDE_E, "O");
        c.pins[bits + 1] = Pin::new(c.size_y - 1, SIDE_E, "V+");
    }
    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let bits = c.bits.max(1) as usize;
        let th = c.threshold();
        let ival: i64 = (0..bits).filter(|&i| c.base.volts[i] > th).fold(0, |a, i| a | (1 << i));
        let max = (1i64 << bits) - 1;
        let v = ival as f64 * c.base.volts[bits + 1] / max as f64;
        if let Some(vs) = c.pins[bits].volt_source {
            s.update_voltage_source(vs, v);
        }
        true
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| bits_field(c))
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            set_bits(c, ei);
        }
    }
}

/// Second-generation current conveyor.
#[derive(Clone, Debug)]
pub struct Cc2 {
    gain: f64,
}

impl ChipLogic for Cc2 {
    fn class_name(&self) -> &'static str {
        "CC2Elm"
    }
    fn dump_type(&self) -> i32 {
        179
    }
    fn chip_name(&self) -> &'static str {
        "CC2"
    }
    fn is_digital(&self) -> bool {
        false
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 3;
        fill(c, 3);
        c.pins[0] = Pin::output(0, SIDE_W, "X");
        c.pins[1] = Pin::new(2, SIDE_W, "Y");
        c.pins[2] = Pin::new(1, SIDE_E, "Z");
    }
    fn read_extra(&mut self, _c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Ok(g) = st.next_f64() {
            self.gain = g;
        }
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {}", js_number(self.gain))
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.gain = xml.double_attr("ga", self.gain);
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.double_attr("ga", self.gain);
    }
    fn matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        if let Some(vs) = c.pins[0].volt_source {
            let n = &c.base.nodes;
            s.stamp_voltage_source_var(NodeId::GROUND, n[0], vs);
            s.stamp_vcvs(NodeId::GROUND, n[1], 1.0, vs);
            s.stamp_cccs(NodeId::GROUND, n[2], vs, self.gain);
        }
        true
    }
    fn do_step(&mut self, _c: &mut ChipCore, _s: &mut Stamper) -> bool {
        true
    }
    fn calculate_current(&mut self, c: &mut ChipCore) {
        c.pins[2].current = c.pins[0].current * self.gain;
    }
    fn info_extra(&self, c: &ChipCore, arr: &mut Vec<String>) {
        arr.push(if self.gain == 1.0 { "CCII+" } else { "CCII-" }.into());
        arr.push(format!("X,Y = {}", crate::elements::units::voltage_text(c.base.volts[0])));
        arr.push(format!("Z = {}", crate::elements::units::voltage_text(c.base.volts[2])));
        arr.push(format!("I = {}", crate::elements::units::current_text(c.pins[0].current)));
    }
}

// ---------------------------------------------------------------------------------------------
// Time delay relay
// ---------------------------------------------------------------------------------------------

const VIN_RESISTANCE: f64 = 10e3;

/// A relay whose contact closes some time after its supply has been applied.
#[derive(Clone, Debug)]
pub struct TimeDelayRelay {
    last_transition: f64,
    powered: bool,
    on_state: bool,
    on_delay: f64,
    off_delay: f64,
    on_resistance: f64,
    off_resistance: f64,
}

impl TimeDelayRelay {
    fn new() -> Self {
        TimeDelayRelay {
            last_transition: 0.0,
            powered: false,
            on_state: false,
            on_delay: 1.0,
            off_delay: 0.0,
            on_resistance: 1.0,
            off_resistance: 10e6,
        }
    }

    fn resistance(&self) -> f64 {
        if self.on_state { self.on_resistance } else { self.off_resistance }
    }
}

impl ChipLogic for TimeDelayRelay {
    fn class_name(&self) -> &'static str {
        "TimeDelayRelayElm"
    }
    fn dump_type(&self) -> i32 {
        414
    }
    fn chip_name(&self) -> &'static str {
        "time delay relay"
    }
    fn is_digital(&self) -> bool {
        false
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        0
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 2;
        c.pins = vec![
            Pin::new(1, SIDE_W, "Vin"),
            Pin::new(1, SIDE_E, "gnd"),
            Pin::new(0, SIDE_W, "in"),
            Pin::new(0, SIDE_E, "out"),
        ];
    }
    fn read_extra(&mut self, _c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        self.on_delay = st.next_f64().unwrap_or(self.on_delay);
        self.off_delay = st.next_f64().unwrap_or(self.off_delay);
        self.on_resistance = st.next_f64().unwrap_or(self.on_resistance);
        self.off_resistance = st.next_f64().unwrap_or(self.off_resistance);
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(
            " {} {} {} {}",
            js_number(self.on_delay),
            js_number(self.off_delay),
            js_number(self.on_resistance),
            js_number(self.off_resistance)
        )
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.on_delay = xml.double_attr("ond", self.on_delay);
        self.off_delay = xml.double_attr("ofd", self.off_delay);
        self.on_resistance = xml.double_attr("onr", self.on_resistance);
        self.off_resistance = xml.double_attr("ofr", self.off_resistance);
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.double_attr("ond", self.on_delay);
        out.double_attr("ofd", self.off_delay);
        out.double_attr("onr", self.on_resistance);
        out.double_attr("ofr", self.off_resistance);
    }
    fn on_reset(&mut self, _c: &mut ChipCore) {
        self.last_transition = 0.0;
        self.powered = false;
        self.on_state = false;
    }

    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let n = &c.base.nodes;
        s.stamp_resistor(n[0], n[1], VIN_RESISTANCE);
        s.stamp_non_linear(n[2]);
        s.stamp_non_linear(n[3]);
        true
    }
    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let n = &c.base.nodes;
        s.stamp_resistor(n[2], n[3], self.resistance());
        true
    }
    fn step_finished(&mut self, c: &mut ChipCore, s: &mut Stamper) {
        // power applied, then the delay, then in and out are connected
        let old = self.powered;
        self.powered = c.base.volts[0] - c.base.volts[1] > 2.5;
        if old != self.powered {
            self.last_transition = s.t;
        }
        if s.t > self.last_transition + if self.powered { self.on_delay } else { self.off_delay } {
            self.on_state = self.powered;
        }
    }
    fn calculate_current(&mut self, c: &mut ChipCore) {
        let v = &c.base.volts;
        let (a, b) = (-(v[0] - v[1]) / VIN_RESISTANCE, -(v[2] - v[3]) / self.resistance());
        c.pins[0].current = a;
        c.pins[1].current = -a;
        c.pins[2].current = b;
        c.pins[3].current = -b;
    }
    fn connection(&self, n1: usize, n2: usize) -> bool {
        matches!((n1.min(n2), n1.max(n2)), (0, 1) | (2, 3))
    }
    fn ground_connection(&self, _c: &ChipCore, _n: usize) -> bool {
        false
    }
    fn visual_state(&self, _c: &ChipCore) -> Vec<f64> {
        vec![if self.on_state { 1.0 } else { 0.0 }]
    }
    fn set_visual_state(&mut self, _c: &mut ChipCore, state: &[f64]) {
        if let Some(v) = state.first() {
            self.on_state = *v != 0.0;
        }
    }
    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("On Delay (s)", self.on_delay, 0.0, 0.0)),
            1 => Some(EditInfo::new("Off Delay (s)", self.off_delay, 0.0, 0.0)),
            2 => Some(
                EditInfo::new("On Resistance (ohms)", self.on_resistance, 0.0, 0.0).set_positive(),
            ),
            3 => Some(
                EditInfo::new("Off Resistance (ohms)", self.off_resistance, 0.0, 0.0)
                    .set_positive(),
            ),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.on_delay = ei.value,
            1 => self.off_delay = ei.value,
            2 => self.on_resistance = ei.value,
            3 => self.off_resistance = ei.value,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Analog multiplexer
// ---------------------------------------------------------------------------------------------

/// An analog multiplexer: the selected input is joined to the output through `r_on`.
#[derive(Clone, Debug)]
pub struct AnalogMux {
    select_bits: usize,
    r_on: f64,
    r_off: f64,
    threshold: f64,
}

impl AnalogMux {
    const FLAG_PULLDOWN: i32 = 2;

    fn new() -> Self {
        AnalogMux { select_bits: 2, r_on: 20.0, r_off: 1e10, threshold: 2.5 }
    }

    fn inputs(&self) -> usize {
        1 << self.select_bits
    }
    fn output(&self) -> usize {
        self.inputs() + self.select_bits
    }
    fn selected(&self, c: &ChipCore) -> usize {
        let ni = self.inputs();
        (0..self.select_bits)
            .filter(|&i| c.base.volts[ni + i] > self.threshold)
            .fold(0, |a, i| a | (1 << i))
    }
    fn pulldown(c: &ChipCore) -> bool {
        has(c, Self::FLAG_PULLDOWN)
    }
}

impl ChipLogic for AnalogMux {
    fn class_name(&self) -> &'static str {
        "AnalogMuxElm"
    }
    fn dump_type(&self) -> i32 {
        432
    }
    fn chip_name(&self) -> &'static str {
        "Analog Mux"
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        0
    }
    fn after_new(&mut self, c: &mut ChipCore) {
        c.base.flags |= Self::FLAG_PULLDOWN;
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let ni = self.inputs();
        c.size_x = self.select_bits as i32 + 1;
        c.size_y = ni as i32 + 1;
        let mut pins = Vec::with_capacity(ni + self.select_bits + 1);
        for i in 0..ni {
            pins.push(Pin::new(i as i32, SIDE_W, &format!("I{i}")));
        }
        for i in 0..self.select_bits {
            pins.push(Pin::new(i as i32 + 1, SIDE_S, &format!("S{i}")));
        }
        pins.push(Pin::new(0, SIDE_E, "Z"));
        c.pins = pins;
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Ok(b) = st.next_i32() {
            self.select_bits = b.clamp(1, 8) as usize;
            self.r_on = st.next_f64().unwrap_or(self.r_on);
            self.r_off = st.next_f64().unwrap_or(self.r_off);
            self.threshold = st.next_f64().unwrap_or(self.threshold);
        }
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len());
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(
            " {} {} {} {}",
            self.select_bits,
            js_number(self.r_on),
            js_number(self.r_off),
            js_number(self.threshold)
        )
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.select_bits = xml.int_attr("sb", self.select_bits as i32).clamp(1, 8) as usize;
        self.r_on = xml.double_attr("ron", self.r_on);
        self.r_off = xml.double_attr("rof", self.r_off);
        self.threshold = xml.double_attr("thr", self.threshold);
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("sb", self.select_bits as i32);
        out.double_attr("ron", self.r_on);
        out.double_attr("rof", self.r_off);
        out.double_attr("thr", self.threshold);
    }

    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        // the selected input changes while the circuit runs
        for i in 0..self.inputs() {
            s.stamp_non_linear(c.base.nodes[i]);
        }
        s.stamp_non_linear(c.base.nodes[self.output()]);
        true
    }
    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let sel = self.selected(c);
        let out = c.base.nodes[self.output()];
        for i in 0..self.inputs() {
            let n = c.base.nodes[i];
            if i == sel {
                s.stamp_resistor(n, out, self.r_on);
            } else if Self::pulldown(c) {
                s.stamp_resistor(n, NodeId::GROUND, self.r_off);
            } else {
                s.stamp_resistor(n, out, self.r_off);
            }
        }
        true
    }
    fn calculate_current(&mut self, c: &mut ChipCore) {
        let sel = self.selected(c);
        let o = self.output();
        let mut total = 0.0;
        for i in 0..self.inputs() {
            if i == sel {
                let cur = (c.base.volts[i] - c.base.volts[o]) / self.r_on;
                c.pins[i].current = -cur;
                total += cur;
            } else if Self::pulldown(c) {
                c.pins[i].current = -c.base.volts[i] / self.r_off;
            } else {
                let cur = (c.base.volts[i] - c.base.volts[o]) / self.r_off;
                c.pins[i].current = -cur;
                total += cur;
            }
        }
        c.pins[o].current = total;
        for i in 0..self.select_bits {
            c.pins[self.inputs() + i].current = 0.0;
        }
    }
    fn connection(&self, n1: usize, n2: usize) -> bool {
        let (ni, o) = (self.inputs(), self.output());
        // select pins are not connected to anything
        !((ni..o).contains(&n1) || (ni..o).contains(&n2))
    }
    fn ground_connection(&self, c: &ChipCore, n: usize) -> bool {
        Self::pulldown(c) && n < self.inputs()
    }
    fn info_extra(&self, c: &ChipCore, arr: &mut Vec<String>) {
        arr.clear();
        arr.push("analog multiplexer".into());
        arr.push(format!("selected: I{}", self.selected(c)));
        arr.push(format!(
            "Vout = {}",
            crate::elements::units::voltage_text(c.base.volts[self.output()])
        ));
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("# of Select Bits", self.select_bits as f64, 1.0, 8.0)
                    .set_dimensionless(),
            ),
            1 => Some(EditInfo::new("On Resistance (ohms)", self.r_on, 0.0, 0.0).set_positive()),
            2 => Some(EditInfo::new("Off Resistance (ohms)", self.r_off, 0.0, 0.0).set_positive()),
            3 => Some(EditInfo::new("Threshold Voltage", self.threshold, 0.0, 0.0)),
            4 => Some(EditInfo::checkbox("Pulldown Resistor", Self::pulldown(c))),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 if (1.0..=8.0).contains(&ei.value) => self.select_bits = ei.value as usize,
            1 => self.r_on = ei.value,
            2 => self.r_off = ei.value,
            3 => self.threshold = ei.value,
            4 => c.base.flags = ei.change_flag(c.base.flags, Self::FLAG_PULLDOWN),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adc_converts_proportionally() {
        let mut adc = ChipElm::new(Adc, 0, 0);
        adc.core.bits = 3;
        adc.rebuild_pins();
        adc.core.base.volts[3] = 2.5;
        adc.core.base.volts[4] = 5.0;
        adc.logic.execute(&mut adc.core, &Stamper::default());
        let code: usize = (0..3).map(|i| usize::from(adc.core.pins[i].value) << i).sum();
        assert_eq!(code, 3); // 7 * 0.5 = 3.5 -> 3
    }

    #[test]
    fn timer_posts_follow_flags() {
        let t = ChipElm::new(Timer::default(), 0, 0);
        assert_eq!(t.core.pins.len(), 8);
        let mut st = crate::format::tokenizer::Tokenizer::new("0");
        let old = ChipElm::from_dump(Timer::default(), 0, 0, 32, 0, 0, &mut st);
        assert_eq!(old.core.pins.len(), 6);
    }
}
