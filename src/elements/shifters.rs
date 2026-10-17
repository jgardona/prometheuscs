// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (PisoShiftElm, SipoShiftElm,
// SeqGenElm, BusTransceiverElm); the first three were contributed by Edward Calver.
//! Shift registers, the sequence generator and the bus transceiver.

use super::chip::*;
use super::element::*;
use super::registry::{ElementDescriptor, Registry};
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::solver::stamper::{NodeId, Stamper};

/// Family name used for the feature switch.
pub const FAMILY: &str = "digital";
const CAT: &str = "Digital Chips";

/// Registers the chips of this module.
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
    chip!("PisoShiftElm", "Add PISO Shift Register", 186, Piso::default());
    chip!("SipoShiftElm", "Add SIPO Shift Register", 189, Sipo::default());
    chip!("SeqGenElm", "Add Sequence Generator", 188, SeqGen::default());
    // the original gives this chip no file code: it is saved by its XML tag only
    r.add(ElementDescriptor::new(
        "BusTransceiverElm",
        FAMILY,
        "Add Bus Transceiver",
        CAT,
        |x, y| Box::new(ChipElm::new(BusTransceiver::default(), x, y)),
    ));
}

fn has(c: &ChipCore, flag: i32) -> bool {
    c.base.flags & flag != 0
}

/// Packs bits 32 to an integer, written as decimal numbers separated by spaces.
fn bits_to_string(data: &[bool]) -> String {
    data.chunks(32)
        .map(|chunk| {
            let v = chunk.iter().enumerate().fold(0u32, |a, (i, b)| a | (u32::from(*b) << i));
            (v as i32).to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The opposite of [`bits_to_string`], filling `out` as far as the text goes.
fn bits_from_string(s: &str, out: &mut [bool]) {
    let mut words = s.split_whitespace().filter_map(|w| w.parse::<i32>().ok());
    for chunk in out.chunks_mut(32) {
        let Some(w) = words.next() else { break };
        for (i, b) in chunk.iter_mut().enumerate() {
            *b = (w as u32) & (1 << i) != 0;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// PISO shift register
// ---------------------------------------------------------------------------------------------

/// Parallel-in, serial-out shift register.
#[derive(Clone, Debug, Default)]
pub struct Piso {
    data: Vec<bool>,
    data_index: i32,
    clock_state: bool,
    load_state: bool,
    data_pin: usize,
    pending: Option<String>,
}

impl Piso {
    const FLAG_NEW_BEHAVIOR: i32 = 2;
    fn new_behavior(c: &ChipCore) -> bool {
        has(c, Self::FLAG_NEW_BEHAVIOR)
    }
}

impl ChipLogic for Piso {
    fn class_name(&self) -> &'static str {
        "PisoShiftElm"
    }
    fn dump_type(&self) -> i32 {
        186
    }
    fn chip_name(&self) -> &'static str {
        "PISO shift register"
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn default_bit_count(&self) -> i32 {
        8
    }
    fn before_new(&mut self, c: &mut ChipCore) {
        c.base.flags |= Self::FLAG_NEW_BEHAVIOR;
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        if self.data.len() != bits {
            self.data = vec![false; bits];
        }
        let new = Self::new_behavior(c);
        c.size_x = bits as i32 + 2;
        c.size_y = 3;
        let mut pins = Vec::new();
        pins.push(Pin::new(1, SIDE_W, "LD"));
        let mut clk = Pin::new(2, SIDE_W, "");
        clk.clock = true;
        pins.push(clk);
        let q = if new { bits - 1 } else { bits };
        pins.push(Pin::output(1, SIDE_E, &format!("Q{q}")));
        if new {
            pins.push(Pin::new(0, SIDE_W, "SER"));
            pins[2].value = self.data[0];
        }
        self.data_pin = pins.len();
        for i in 0..bits {
            pins.push(Pin::new((bits - i) as i32, SIDE_N, &format!("D{}", bits - (i + 1))));
        }
        c.pins = pins;
    }
    fn on_reset(&mut self, c: &mut ChipCore) {
        self.data = vec![false; c.bits.max(1) as usize];
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.pending = xml.attr("dt").map(str::to_string);
    }
    fn write_xml_state(&self, _c: &ChipCore, out: &mut XmlOut) {
        // the register as it will be after loading: rotated so the next output is first
        let n = self.data.len();
        let start = self.data_index.max(0) as usize;
        let rotated: Vec<bool> = (0..n).map(|i| self.data[(i + start) % n]).collect();
        out.str_attr("dt", &bits_to_string(&rotated));
    }
    fn after_xml(&mut self, c: &mut ChipCore) {
        self.data = vec![false; c.bits.max(1) as usize];
        self.data_index = 0;
        if let Some(dt) = self.pending.take() {
            bits_from_string(&dt, &mut self.data);
        }
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let new = Self::new_behavior(c);
        let dp = self.data_pin;
        if c.pins[0].value != self.load_state {
            self.load_state = c.pins[0].value;
            if self.load_state && !self.data.is_empty() {
                if new {
                    // the output takes the first bit at once
                    c.pins[2].value = c.pins[dp].value;
                    self.data_index = 0;
                } else {
                    self.data_index = -1;
                }
                for i in 0..self.data.len() {
                    self.data[i] = c.pins[dp + i].value;
                }
            }
        }
        if c.pins[1].value != self.clock_state {
            self.clock_state = c.pins[1].value;
            if self.clock_state && !self.data.is_empty() {
                if self.data_index >= 0 {
                    self.data[self.data_index as usize] = new && c.pins[3].value;
                }
                self.data_index += 1;
                if self.data_index as usize >= self.data.len() {
                    self.data_index = 0;
                }
                c.pins[2].value = self.data[self.data_index as usize];
            }
        }
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        (n == 0)
            .then(|| EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0).set_dimensionless())
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        if n == 0 && ei.value >= 1.0 && ei.value as i32 != c.bits {
            c.bits = ei.value as i32;
            self.data = vec![false; c.bits as usize];
        }
    }
}

// ---------------------------------------------------------------------------------------------
// SIPO shift register
// ---------------------------------------------------------------------------------------------

/// Serial-in, parallel-out shift register.
#[derive(Clone, Debug, Default)]
pub struct Sipo {
    clock_state: bool,
    pending: Option<String>,
}

const SIPO_DATA: usize = 2;

impl ChipLogic for Sipo {
    fn class_name(&self) -> &'static str {
        "SipoShiftElm"
    }
    fn dump_type(&self) -> i32 {
        189
    }
    fn chip_name(&self) -> &'static str {
        "SIPO shift register"
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn default_bit_count(&self) -> i32 {
        8
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        c.size_x = bits as i32 + 1;
        c.size_y = 3;
        let old: Vec<bool> = c.pins.iter().skip(SIPO_DATA).map(|p| p.value).collect();
        let mut pins = vec![Pin::new(1, SIDE_W, "D"), Pin::new(2, SIDE_W, "")];
        pins[1].clock = true;
        for i in 0..bits {
            let mut p = Pin::output(i as i32 + 1, SIDE_N, &format!("Q{i}"));
            p.value = old.get(i).copied().unwrap_or(false);
            pins.push(p);
        }
        c.pins = pins;
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.pending = xml.attr("dt").map(str::to_string);
    }
    fn write_xml_state(&self, c: &ChipCore, out: &mut XmlOut) {
        let data: Vec<bool> = c.pins.iter().skip(SIPO_DATA).map(|p| p.value).collect();
        out.str_attr("dt", &bits_to_string(&data));
    }
    fn after_xml(&mut self, c: &mut ChipCore) {
        if let Some(dt) = self.pending.take() {
            let mut data = vec![false; c.pins.len().saturating_sub(SIPO_DATA)];
            bits_from_string(&dt, &mut data);
            for (i, v) in data.into_iter().enumerate() {
                c.pins[SIPO_DATA + i].value = v;
            }
        }
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        if c.pins[1].value != self.clock_state {
            self.clock_state = c.pins[1].value;
            let bits = c.pins.len() - SIPO_DATA;
            if self.clock_state && bits > 0 {
                for i in (0..bits.saturating_sub(1)).rev() {
                    c.pins[SIPO_DATA + i + 1].value = c.pins[SIPO_DATA + i].value;
                }
                c.pins[SIPO_DATA].value = c.pins[0].value;
            }
        }
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        (n == 0)
            .then(|| EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0).set_dimensionless())
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        if n == 0 && ei.value >= 1.0 {
            c.bits = ei.value as i32;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Sequence generator
// ---------------------------------------------------------------------------------------------

/// Plays a stored bit sequence on its output, one bit per clock pulse.
#[derive(Clone, Debug)]
pub struct SeqGen {
    bit_position: usize,
    bit_count: usize,
    data: Vec<i32>,
    clock_state: bool,
}

impl Default for SeqGen {
    fn default() -> Self {
        SeqGen { bit_position: 0, bit_count: 8, data: vec![0], clock_state: false }
    }
}

impl SeqGen {
    const FLAG_NEW_VERSION: i32 = 2;
    const FLAG_PLAY_ONCE: i32 = 4;
    const FLAG_HAS_RESET: i32 = 8;

    fn bit(&self, i: usize) -> bool {
        self.data.get(i / 32).is_some_and(|w| (*w as u32) & (1 << (i % 32)) != 0)
    }

    fn sequence(&self) -> String {
        (0..self.bit_count).map(|i| if self.bit(i) { '1' } else { '0' }).collect()
    }

    fn clamp_count(&mut self) {
        self.bit_count = self.bit_count.min(self.data.len() * 32);
    }

    fn next_bit(&mut self, c: &mut ChipCore) {
        if !self.data.is_empty() && self.bit_count > 0 {
            if self.bit_position >= self.bit_count {
                if has(c, Self::FLAG_PLAY_ONCE) {
                    c.pins[1].value = false;
                    return;
                }
                self.bit_position = 0;
            }
            c.pins[1].value = self.bit(self.bit_position);
            self.bit_position += 1;
        } else {
            c.pins[1].value = false;
        }
    }
}

impl ChipLogic for SeqGen {
    fn class_name(&self) -> &'static str {
        "SeqGenElm"
    }
    fn dump_type(&self) -> i32 {
        188
    }
    fn chip_name(&self) -> &'static str {
        "sequence generator"
    }
    fn before_new(&mut self, c: &mut ChipCore) {
        c.base.flags |= Self::FLAG_NEW_VERSION | Self::FLAG_HAS_RESET;
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 2;
        let mut pins = vec![Pin::new(0, SIDE_W, ""), Pin::output(1, SIDE_E, "Q")];
        pins[0].clock = true;
        if has(c, Self::FLAG_HAS_RESET) {
            pins.push(Pin::new(1, SIDE_W, "R"));
        }
        c.pins = pins;
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if !has(c, Self::FLAG_NEW_VERSION) {
            // an old generator: one byte of data, upgraded to the flexible version
            c.base.flags |= Self::FLAG_NEW_VERSION;
            if let Ok(old) = st.next_i32() {
                self.bit_count = 8;
                self.data = vec![i32::from(old as i8)];
            }
        } else if let Ok(count) = st.next_i32() {
            self.bit_count = count.max(0) as usize;
            let words = self.bit_count.div_ceil(32);
            self.data = (0..words).map_while(|_| st.next_i32().ok()).collect();
        }
        self.clamp_count();
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        let mut s = format!(" {}", self.bit_count);
        for w in &self.data {
            s += &format!(" {w}");
        }
        s
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.bit_count = xml.int_attr("bc", self.bit_count as i32).max(0) as usize;
        if let Some(dt) = xml.attr("dt") {
            self.data = dt.split_whitespace().filter_map(|w| w.parse().ok()).collect();
        }
        self.clamp_count();
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("bc", self.bit_count as i32);
        let s: Vec<String> = self.data.iter().map(ToString::to_string).collect();
        out.str_attr("dt", &s.join(" "));
    }
    fn on_reset(&mut self, _c: &mut ChipCore) {
        self.bit_position = 0;
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        if has(c, Self::FLAG_HAS_RESET) && c.pins[2].value {
            // suspended while reset is high
            self.bit_position = 0;
            self.clock_state = c.pins[0].value;
            self.next_bit(c);
        } else if c.pins[0].value != self.clock_state {
            self.clock_state = c.pins[0].value;
            if self.clock_state {
                self.next_bit(c);
            }
        }
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::checkbox("Play Once", has(c, Self::FLAG_PLAY_ONCE))),
            1 => Some(EditInfo::text_area("Sequence", &self.sequence())),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => c.base.flags = ei.change_flag(c.base.flags, Self::FLAG_PLAY_ONCE),
            1 => {
                let text = ei.text.clone().unwrap_or_default();
                let bits: Vec<bool> = text
                    .chars()
                    .filter_map(|ch| match ch {
                        '0' => Some(false),
                        '1' => Some(true),
                        _ => None,
                    })
                    .collect();
                self.bit_count = bits.len();
                self.data = vec![0; bits.len().div_ceil(32).max(1)];
                for (i, b) in bits.iter().enumerate() {
                    if *b {
                        self.data[i / 32] |= 1 << (i % 32);
                    }
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Bus transceiver
// ---------------------------------------------------------------------------------------------

/// A bidirectional buffer between two groups of pins: OE (active low) enables it and DIR
/// chooses the way. Handles `bits` separate lines; the bus pin mode is not implemented.
#[derive(Clone, Debug)]
pub struct BusTransceiver {
    data_bits: usize,
}

impl Default for BusTransceiver {
    fn default() -> Self {
        BusTransceiver { data_bits: 4 }
    }
}

impl BusTransceiver {
    fn a(&self) -> usize {
        2
    }
    fn b(&self) -> usize {
        2 + self.data_bits
    }
    fn internal(&self) -> usize {
        2 + 2 * self.data_bits
    }
}

impl ChipLogic for BusTransceiver {
    fn class_name(&self) -> &'static str {
        "BusTransceiverElm"
    }
    fn dump_type(&self) -> i32 {
        0
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("BusTransceiver")
    }
    fn chip_name(&self) -> &'static str {
        "Bus Transceiver"
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        self.data_bits
    }
    fn internal_nodes(&self) -> usize {
        self.data_bits
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        if self.data_bits == 0 {
            self.data_bits = 4;
        }
        let n = self.data_bits;
        c.size_x = 2;
        c.size_y = n as i32 + 2;
        c.bits = n as i32;
        let mut pins = vec![Pin::new(0, SIDE_W, "OE"), Pin::new(0, SIDE_E, "DIR")];
        pins[0].line_over = true;
        for i in 0..n {
            // the first line is at the bottom, as in the original (reversed order)
            pins.push(Pin::new((n - 1 - i) as i32 + 2, SIDE_W, &format!("A{i}")));
        }
        for i in 0..n {
            pins.push(Pin::new((n - 1 - i) as i32 + 2, SIDE_E, &format!("B{i}")));
        }
        c.pins = pins;
    }
    fn output_pin_for_source(&self, _c: &ChipCore, j: usize) -> Option<usize> {
        Some(self.a() + j)
    }
    fn voltage_source_node(&self, _c: &ChipCore, j: usize) -> Option<usize> {
        Some(self.internal() + j)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.data_bits = xml.int_attr("db", self.data_bits as i32).clamp(1, 16) as usize;
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("db", self.data_bits as i32);
    }
    fn matrix_connection(&self, n1: usize, n2: usize) -> bool {
        // each internal node joins its A and its B pin through resistors
        (0..self.data_bits).any(|i| {
            let int = self.internal() + i;
            [self.a() + i, self.b() + i]
                .iter()
                .any(|&p| (n1 == int && n2 == p) || (n2 == int && n1 == p))
        })
    }
    fn ground_connection(&self, _c: &ChipCore, _n: usize) -> bool {
        false
    }
    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        for i in 0..self.data_bits {
            if let Some(vs) = c.pins[self.a() + i].volt_source {
                s.stamp_voltage_source_var(NodeId::GROUND, c.base.nodes[self.internal() + i], vs);
            }
            s.stamp_non_linear(c.base.nodes[self.internal() + i]);
            s.stamp_non_linear(c.base.nodes[self.a() + i]);
            s.stamp_non_linear(c.base.nodes[self.b() + i]);
        }
        true
    }
    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let th = c.threshold();
        let hv = c.high_voltage;
        let enabled = c.base.volts[0] < th;
        let a_to_b = c.base.volts[1] > th;
        for i in 0..self.data_bits {
            let (a, b, int) = (self.a() + i, self.b() + i, self.internal() + i);
            // read from the source side, drive the destination side
            let src = if a_to_b { c.base.volts[a] > th } else { c.base.volts[b] > th };
            if let Some(vs) = c.pins[a].volt_source {
                s.update_voltage_source(vs, if src { hv } else { 0.0 });
            }
            let r_dst = if enabled { 1.0 } else { 1e10 };
            let n = &c.base.nodes;
            if a_to_b {
                s.stamp_resistor(n[int], n[a], 1e8);
                s.stamp_resistor(n[int], n[b], r_dst);
            } else {
                s.stamp_resistor(n[int], n[a], r_dst);
                s.stamp_resistor(n[int], n[b], 1e8);
            }
        }
        true
    }
    fn info_extra(&self, c: &ChipCore, arr: &mut Vec<String>) {
        arr.clear();
        arr.push("bus transceiver".into());
        let th = c.threshold();
        let enabled = c.base.volts[0] < th;
        let a_to_b = c.base.volts[1] > th;
        arr.push(
            if enabled { if a_to_b { "A\u{2192}B" } else { "B\u{2192}A" } } else { "hi-Z" }.into(),
        );
    }
    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| {
            EditInfo::new("# of Bits", self.data_bits as f64, 1.0, 1.0).set_dimensionless()
        })
    }
    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            if (1.0..=16.0).contains(&ei.value) {
                self.data_bits = ei.value as usize;
            } else {
                ei.error = Some("must be between 1 and 16".into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_pack_into_integers_and_back() {
        let mut data = vec![false; 40];
        for i in [0, 3, 31, 33] {
            data[i] = true;
        }
        let s = bits_to_string(&data);
        let mut back = vec![false; 40];
        bits_from_string(&s, &mut back);
        assert_eq!(back, data);
        assert_eq!(s.split(' ').count(), 2);
    }
}
