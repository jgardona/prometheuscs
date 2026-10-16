// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (flip-flops, counters, adders,
// multiplexers and latches).
//! Digital chips built on the [`ChipElm`] framework: flip-flops, latches, counters, adders and
//! (de)multiplexers.

use super::chip::*;
use super::element::*;
use super::registry::{ElementDescriptor, Registry};
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::solver::stamper::Stamper;

/// Family name used for the feature switch.
pub const FAMILY: &str = "digital";
const CAT: &str = "Digital Chips";

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

fn value(c: &ChipCore, n: usize) -> bool {
    c.pins.get(n).is_some_and(|p| p.value)
}

fn bits_field(c: &ChipCore) -> EditInfo {
    EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0).set_dimensionless()
}

fn check(name: &str, on: bool) -> Option<EditInfo> {
    Some(EditInfo::checkbox(name, on))
}

/// Registers the digital chips.
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
    chip!("DFlipFlopElm", "Add D Flip-Flop", 155, DFlipFlop::default());
    chip!("JKFlipFlopElm", "Add JK Flip-Flop", 156, JkFlipFlop::default());
    chip!("TFlipFlopElm", "Add T Flip-Flop", 193, TFlipFlop);
    chip!("Counter2Elm", "Add Counter (with Load)", 421, Counter2::default());
    chip!("MonostableElm", "Add Monostable", 194, Monostable::default());
    chip!("BusSplitterElm", "Add Bus Splitter", 433, BusSplitter::default());
    chip!("HalfAdderElm", "Add Half Adder", 195, HalfAdder);
    chip!("CounterElm", "Add Counter", 164, Counter::default());
    chip!("RingCounterElm", "Add Ring Counter", 163, RingCounter::default());
    chip!("MultiplexerElm", "Add Multiplexer", 184, Multiplexer::default());
    chip!("DeMultiplexerElm", "Add Demultiplexer", 185, DeMultiplexer::default());
    r.add(
        ElementDescriptor::new("FullAdderElm", FAMILY, "Add Full Adder", CAT, |x, y| {
            Box::new(ChipElm::new(FullAdder { has_bits: true }, x, y))
        })
        .dump(196, |a, b, c, d, f, st| {
            let logic = FullAdder { has_bits: f & FullAdder::FLAG_BITS != 0 };
            Ok(Box::new(ChipElm::from_dump(logic, a, b, c, d, f, st)))
        }),
    );
    r.add(
        ElementDescriptor::new("LatchElm", FAMILY, "Add Latch", CAT, |x, y| {
            Box::new(ChipElm::new(Latch::default(), x, y))
        })
        .dump(168, |a, b, c, d, f, st| {
            let mut chip = ChipElm::from_dump(Latch::default(), a, b, c, d, f, st);
            if f & Latch::FLAG_STATE == 0 {
                chip.core.base.flags |= Latch::FLAG_STATE;
                chip.rebuild_pins();
            }
            Ok(Box::new(chip))
        }),
    );
}

// ---------------------------------------------------------------------------------------------
// D flip-flop
// ---------------------------------------------------------------------------------------------

/// D flip-flop with optional set and reset.
#[derive(Clone, Debug, Default)]
pub struct DFlipFlop {
    just_loaded: bool,
}

impl DFlipFlop {
    const FLAG_RESET: i32 = 2;
    const FLAG_SET: i32 = 4;
    const FLAG_INVERT: i32 = 8;
    fn has_set(c: &ChipCore) -> bool {
        has(c, Self::FLAG_SET)
    }
    fn has_reset(c: &ChipCore) -> bool {
        has(c, Self::FLAG_RESET) || Self::has_set(c)
    }
    fn invert(c: &ChipCore) -> bool {
        has(c, Self::FLAG_INVERT)
    }
}

impl ChipLogic for DFlipFlop {
    fn class_name(&self) -> &'static str {
        "DFlipFlopElm"
    }
    fn dump_type(&self) -> i32 {
        155
    }
    fn chip_name(&self) -> &'static str {
        "D flip-flop"
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 3;
        let set = Self::has_set(c);
        let n = 4 + usize::from(Self::has_reset(c)) + usize::from(set);
        fill(c, n);
        c.pins[0] = Pin::new(0, SIDE_W, "D");
        c.pins[1] = Pin::output_state(0, SIDE_E, "Q");
        c.pins[2] = Pin::output(if set { 1 } else { 2 }, SIDE_E, "Q");
        c.pins[2].line_over = true;
        c.pins[3] = Pin::new(1, SIDE_W, "");
        c.pins[3].clock = true;
        let inv = Self::invert(c);
        if !set {
            if Self::has_reset(c) {
                c.pins[4] = Pin::new(2, SIDE_W, "R");
                c.pins[4].bubble = inv;
            }
        } else {
            c.pins[5] = Pin::new(2, SIDE_W, "S");
            c.pins[4] = Pin::new(2, SIDE_E, "R");
            c.pins[4].bubble = inv;
            c.pins[5].bubble = inv;
        }
    }
    fn after_new(&mut self, c: &mut ChipCore) {
        c.pins[2].value = !c.pins[1].value;
    }
    fn after_load(&mut self, c: &mut ChipCore) {
        c.pins[2].value = !c.pins[1].value;
        self.just_loaded = true;
    }
    fn on_reset(&mut self, c: &mut ChipCore) {
        c.base.volts[2] = c.high_voltage;
        c.pins[2].value = true;
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        if self.just_loaded {
            self.just_loaded = false;
            return;
        }
        let inv = Self::invert(c);
        let is_set = Self::has_set(c) && value(c, 5) != inv;
        let is_reset = Self::has_reset(c) && value(c, 4) != inv;
        if is_set || is_reset {
            c.write_output(1, is_set);
            c.write_output(2, is_reset);
        } else {
            if value(c, 3) && !c.last_clock {
                let d = value(c, 0);
                c.write_output(1, d);
            }
            let q = value(c, 1);
            c.write_output(2, !q);
        }
        c.last_clock = value(c, 3);
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        2
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => check("Reset Pin", Self::has_reset(c)),
            1 => check("Set Pin", Self::has_set(c)),
            2 => check("Invert Set/Reset", Self::invert(c)),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => {
                if on {
                    c.base.flags |= Self::FLAG_RESET;
                } else {
                    // the original's precedence quirk clears nothing but the reset bit
                    c.base.flags &= !(Self::FLAG_RESET | Self::FLAG_SET);
                }
            }
            1 => set_flag(c, Self::FLAG_SET, on),
            2 => set_flag(c, Self::FLAG_INVERT, on),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// JK flip-flop
// ---------------------------------------------------------------------------------------------

/// JK flip-flop with optional reset.
#[derive(Clone, Debug, Default)]
pub struct JkFlipFlop {
    just_loaded: bool,
}

impl JkFlipFlop {
    const FLAG_RESET: i32 = 2;
    const FLAG_POSITIVE: i32 = 4;
    const FLAG_INVERT_RESET: i32 = 8;
}

impl ChipLogic for JkFlipFlop {
    fn class_name(&self) -> &'static str {
        "JKFlipFlopElm"
    }
    fn dump_type(&self) -> i32 {
        156
    }
    fn chip_name(&self) -> &'static str {
        "JK flip-flop"
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 3;
        let reset = has(c, Self::FLAG_RESET);
        fill(c, 5 + usize::from(reset));
        c.pins[0] = Pin::new(0, SIDE_W, "J");
        c.pins[1] = Pin::new(1, SIDE_W, "");
        c.pins[1].clock = true;
        c.pins[1].bubble = !has(c, Self::FLAG_POSITIVE);
        c.pins[2] = Pin::new(2, SIDE_W, "K");
        c.pins[3] = Pin::output_state(0, SIDE_E, "Q");
        c.pins[4] = Pin::output(2, SIDE_E, "Q");
        c.pins[4].line_over = true;
        if reset {
            c.pins[5] = Pin::new(1, SIDE_E, "R");
            c.pins[5].bubble = has(c, Self::FLAG_INVERT_RESET);
        }
    }
    fn after_new(&mut self, c: &mut ChipCore) {
        c.pins[4].value = !c.pins[3].value;
    }
    fn after_load(&mut self, c: &mut ChipCore) {
        c.pins[4].value = !c.pins[3].value;
        self.just_loaded = true;
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        if self.just_loaded {
            self.just_loaded = false;
            return;
        }
        let clk = value(c, 1);
        let transition =
            if has(c, Self::FLAG_POSITIVE) { clk && !c.last_clock } else { !clk && c.last_clock };
        if transition {
            let mut q = value(c, 3);
            match (value(c, 0), value(c, 2)) {
                (true, true) => q = !q,
                (true, false) => q = true,
                (false, true) => q = false,
                _ => {}
            }
            c.write_output(3, q);
        }
        c.last_clock = clk;
        if has(c, Self::FLAG_RESET) && value(c, 5) != has(c, Self::FLAG_INVERT_RESET) {
            c.write_output(3, false);
        }
        let q = value(c, 3);
        c.write_output(4, !q);
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        2
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => check("Reset Pin", has(c, Self::FLAG_RESET)),
            1 => check("Positive Edge Triggered", has(c, Self::FLAG_POSITIVE)),
            2 => check("Invert Reset", has(c, Self::FLAG_INVERT_RESET)),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => set_flag(c, Self::FLAG_RESET, on),
            1 => set_flag(c, Self::FLAG_POSITIVE, on),
            2 => set_flag(c, Self::FLAG_INVERT_RESET, on),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// T flip-flop
// ---------------------------------------------------------------------------------------------

/// T (toggle) flip-flop with optional set and reset.
#[derive(Clone, Debug, Default)]
pub struct TFlipFlop;

impl TFlipFlop {
    const FLAG_RESET: i32 = 2;
    const FLAG_SET: i32 = 4;
    fn has_set(c: &ChipCore) -> bool {
        has(c, Self::FLAG_SET)
    }
    fn has_reset(c: &ChipCore) -> bool {
        has(c, Self::FLAG_RESET) || Self::has_set(c)
    }
}

impl ChipLogic for TFlipFlop {
    fn class_name(&self) -> &'static str {
        "TFlipFlopElm"
    }
    fn dump_type(&self) -> i32 {
        193
    }
    fn chip_name(&self) -> &'static str {
        "T flip-flop"
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 3;
        let set = Self::has_set(c);
        fill(c, 4 + usize::from(Self::has_reset(c)) + usize::from(set));
        c.pins[0] = Pin::new(0, SIDE_W, "T");
        c.pins[1] = Pin::output_state(0, SIDE_E, "Q");
        c.pins[2] = Pin::output(if set { 1 } else { 2 }, SIDE_E, "Q");
        c.pins[2].line_over = true;
        c.pins[3] = Pin::new(1, SIDE_W, "");
        c.pins[3].clock = true;
        if !set {
            if Self::has_reset(c) {
                c.pins[4] = Pin::new(2, SIDE_W, "R");
            }
        } else {
            c.pins[5] = Pin::new(2, SIDE_W, "S");
            c.pins[4] = Pin::new(2, SIDE_E, "R");
        }
    }
    fn after_new(&mut self, c: &mut ChipCore) {
        c.pins[2].value = !c.pins[1].value;
    }
    fn after_load(&mut self, c: &mut ChipCore) {
        c.pins[2].value = !c.pins[1].value;
    }
    fn on_reset(&mut self, c: &mut ChipCore) {
        c.base.volts[2] = c.high_voltage;
        c.pins[2].value = true;
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        if value(c, 3) && !c.last_clock && value(c, 0) {
            let q = value(c, 1);
            c.write_output(1, !q);
        }
        if Self::has_set(c) && value(c, 5) {
            c.write_output(1, true);
        }
        if Self::has_reset(c) && value(c, 4) {
            c.write_output(1, false);
        }
        let q = value(c, 1);
        c.write_output(2, !q);
        c.last_clock = value(c, 3);
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        2
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => check("Reset Pin", Self::has_reset(c)),
            1 => check("Set Pin", Self::has_set(c)),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => {
                if on {
                    c.base.flags |= Self::FLAG_RESET;
                } else {
                    c.base.flags &= !(Self::FLAG_RESET | Self::FLAG_SET);
                }
            }
            1 => set_flag(c, Self::FLAG_SET, on),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Adders
// ---------------------------------------------------------------------------------------------

/// Half adder: sum and carry of two bits.
#[derive(Clone, Debug, Default)]
pub struct HalfAdder;

impl ChipLogic for HalfAdder {
    fn class_name(&self) -> &'static str {
        "HalfAdderElm"
    }
    fn dump_type(&self) -> i32 {
        195
    }
    fn chip_name(&self) -> &'static str {
        "Half Adder"
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 2;
        fill(c, 4);
        c.pins[0] = Pin::output(0, SIDE_E, "S");
        c.pins[1] = Pin::output(1, SIDE_E, "C");
        c.pins[2] = Pin::new(0, SIDE_W, "A");
        c.pins[3] = Pin::new(1, SIDE_W, "B");
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let (a, b) = (value(c, 2), value(c, 3));
        c.pins[0].value = a ^ b;
        c.pins[1].value = a && b;
    }
}

/// Full adder with a configurable number of bits.
#[derive(Clone, Debug)]
pub struct FullAdder {
    has_bits: bool,
}

impl FullAdder {
    const FLAG_BITS: i32 = 2;
}

impl ChipLogic for FullAdder {
    fn class_name(&self) -> &'static str {
        "FullAdderElm"
    }
    fn dump_type(&self) -> i32 {
        196
    }
    fn chip_name(&self) -> &'static str {
        "Adder"
    }
    fn needs_bits(&self) -> bool {
        self.has_bits
    }
    fn allow_bus(&self) -> bool {
        self.has_bits
    }
    fn before_new(&mut self, c: &mut ChipCore) {
        c.base.flags |= Self::FLAG_BITS;
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        if !self.has_bits {
            c.bits = 1;
        }
        let bits = c.bits.max(1) as usize;
        c.size_x = 2;
        let bits_y = if c.use_bus() { 1 } else { bits as i32 };
        c.size_y = bits_y * 2 + 1;
        fill(c, bits * 3 + 2);
        c.make_bit_pins(bits, 0, SIDE_W, 0, "A", false, false, false);
        c.make_bit_pins(bits, bits_y, SIDE_W, bits, "B", false, false, false);
        c.make_bit_pins(bits, 2, SIDE_E, bits * 2, "S", true, false, false);
        c.pins[bits * 3 + 1] = Pin::output(0, SIDE_E, "C");
        c.pins[bits * 3] = Pin::new(bits_y * 2, SIDE_W, "Cin");
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let bits = c.bits.max(1) as usize;
        let mut carry = usize::from(value(c, bits * 3));
        for i in 0..bits {
            let v = usize::from(value(c, i)) + usize::from(value(c, i + bits)) + carry;
            carry = usize::from(v > 1);
            c.write_output(i + bits * 2, v & 1 == 1);
        }
        c.write_output(bits * 3 + 1, carry == 1);
    }
    fn voltage_source_count(&self, c: &ChipCore) -> usize {
        c.bits.max(1) as usize + 1
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| {
            EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0)
                .set_dimensionless()
                .set_positive()
        })
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        if n == 0 && ei.value >= 1.0 {
            c.bits = ei.value as i32;
            self.has_bits = true;
            c.base.flags |= Self::FLAG_BITS;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Counters
// ---------------------------------------------------------------------------------------------

/// Binary counter with optional modulus and up/down input.
#[derive(Clone, Debug, Default)]
pub struct Counter {
    invert_reset: bool,
    modulus: i32,
}

impl Counter {
    const FLAG_UP_DOWN: i32 = 4;
    const FLAG_NEGATIVE_EDGE: i32 = 8;
}

impl ChipLogic for Counter {
    fn class_name(&self) -> &'static str {
        "CounterElm"
    }
    fn dump_type(&self) -> i32 {
        164
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("ctr")
    }
    fn chip_name(&self) -> &'static str {
        "Counter"
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn allow_bus(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        let updown = has(c, Self::FLAG_UP_DOWN);
        c.size_x = 2;
        c.size_y = if c.use_bus() { 3 } else { bits as i32 };
        fill(c, bits + 2 + usize::from(updown));
        c.pins[0] = Pin::new(0, SIDE_W, "");
        c.pins[0].clock = true;
        c.pins[0].bubble = has(c, Self::FLAG_NEGATIVE_EDGE);
        c.pins[1] = Pin::new(c.size_y - 1, SIDE_W, "R");
        c.pins[1].bubble = self.invert_reset;
        c.make_bit_pins(bits, 0, SIDE_E, 2, "Q", true, true, true);
        if updown {
            c.pins[bits + 2] = Pin::new(c.size_y - 2, SIDE_W, "U/D");
        }
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        self.invert_reset = true;
        if let Some(t) = st.next_token() {
            self.invert_reset = t.eq_ignore_ascii_case("true");
            if let Ok(m) = st.next_i32() {
                self.modulus = m;
            }
        }
        c.pins[1].bubble = self.invert_reset;
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {} {}", self.invert_reset, self.modulus)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.invert_reset = xml.bool_attr("in", self.invert_reset);
        self.modulus = xml.int_attr("mo", self.modulus);
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.bool_attr("in", self.invert_reset);
        out.int_attr("mo", self.modulus);
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let bits = c.bits.max(1) as usize;
        let neg = has(c, Self::FLAG_NEGATIVE_EDGE);
        if value(c, 0) != neg && c.last_clock == neg {
            let dir = if has(c, Self::FLAG_UP_DOWN) && value(c, bits + 2) { -1 } else { 1 };
            let last = 2 + bits - 1;
            let mut v: i64 = 0;
            for i in 0..bits {
                if value(c, last - i) {
                    v |= 1 << i;
                }
            }
            v += dir;
            if self.modulus != 0 {
                let m = i64::from(self.modulus);
                v = (v + m).rem_euclid(m);
            }
            for i in 0..bits {
                c.write_output(last - i, v & (1 << i) != 0);
            }
        }
        if !value(c, 1) == self.invert_reset {
            for i in 0..bits {
                c.write_output(i + 2, false);
            }
        }
        c.last_clock = value(c, 0);
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => check("Invert reset pin", self.invert_reset),
            1 => Some(EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0).set_dimensionless()),
            2 => Some(
                EditInfo::new("Modulus", f64::from(self.modulus), 1.0, 1.0).set_dimensionless(),
            ),
            3 => check("Up/Down Pin", has(c, Self::FLAG_UP_DOWN)),
            4 => check("Negative Edge Triggered", has(c, Self::FLAG_NEGATIVE_EDGE)),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => self.invert_reset = on,
            1 => {
                if ei.value >= 3.0 {
                    c.bits = ei.value as i32;
                } else {
                    ei.error = Some("must be >= 3".into());
                }
            }
            2 => self.modulus = ei.value as i32,
            3 => set_flag(c, Self::FLAG_UP_DOWN, on),
            4 => set_flag(c, Self::FLAG_NEGATIVE_EDGE, on),
            _ => {}
        }
    }
}

/// Ring counter: one output high at a time, advancing on each clock edge.
#[derive(Clone, Debug, Default)]
pub struct RingCounter {
    just_loaded: bool,
}

impl RingCounter {
    const FLAG_CLOCK_INHIBIT: i32 = 2;
    const FLAG_RESET_HIGH: i32 = 4;
    fn has_inhibit(c: &ChipCore) -> bool {
        has(c, Self::FLAG_CLOCK_INHIBIT) && c.bits >= 3
    }
    fn invert_reset(c: &ChipCore) -> bool {
        !has(c, Self::FLAG_RESET_HIGH)
    }
}

impl ChipLogic for RingCounter {
    fn class_name(&self) -> &'static str {
        "RingCounterElm"
    }
    fn dump_type(&self) -> i32 {
        163
    }
    fn chip_name(&self) -> &'static str {
        "ring counter"
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn default_bit_count(&self) -> i32 {
        10
    }
    fn before_new(&mut self, c: &mut ChipCore) {
        c.base.flags |= Self::FLAG_CLOCK_INHIBIT;
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        let inhibit = Self::has_inhibit(c);
        c.size_x = if bits > 2 { bits as i32 } else { 2 };
        c.size_y = 2;
        fill(c, bits + 2 + usize::from(inhibit));
        c.pins[0] = Pin::new(1, SIDE_W, "");
        c.pins[0].clock = true;
        c.pins[1] = Pin::new(c.size_x - 1, SIDE_S, "R");
        c.pins[1].line_over = Self::invert_reset(c);
        for i in 0..bits {
            c.pins[i + 2] = Pin::output_state(i as i32, SIDE_N, &format!("Q{i}"));
        }
        if inhibit {
            let last = c.pins.len() - 1;
            c.pins[last] = Pin::new(1, SIDE_S, "CE");
            c.pins[last].line_over = true;
        }
    }
    fn after_load(&mut self, _c: &mut ChipCore) {
        self.just_loaded = true;
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        if self.just_loaded {
            self.just_loaded = false;
            return;
        }
        let bits = c.bits.max(1) as usize;
        let running = !(Self::has_inhibit(c) && value(c, c.pins.len() - 1));
        let mut i = (0..bits).find(|&k| value(c, k + 2)).unwrap_or(bits);
        if value(c, 0) && !c.last_clock && running {
            if i < bits {
                c.pins[i + 2].value = false;
                i += 1;
            }
            i %= bits;
            c.pins[i + 2].value = true;
        }
        if value(c, 1) != Self::invert_reset(c) || i == bits {
            for k in 1..bits {
                c.pins[k + 2].value = false;
            }
            c.pins[2].value = true;
        }
        c.last_clock = value(c, 0);
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => check("Invert reset pin", Self::invert_reset(c)),
            1 => Some(EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0).set_dimensionless()),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => set_flag(c, Self::FLAG_RESET_HIGH, !ei.checkbox.unwrap_or(false)),
            1 => {
                if ei.value >= 2.0 {
                    c.bits = ei.value as i32;
                } else {
                    ei.error = Some("must be >= 2".into());
                }
            }
            _ => {}
        }
    }
}

/// Counter with synchronous load, enables and ripple carry out (74163 style).
#[derive(Clone, Debug, Default)]
pub struct Counter2 {
    modulus: i32,
    carry: bool,
}

impl Counter2 {
    fn pin(c: &ChipCore, k: usize) -> usize {
        c.bits.max(1) as usize * 2 + k
    }
}

impl ChipLogic for Counter2 {
    fn class_name(&self) -> &'static str {
        "Counter2Elm"
    }
    fn dump_type(&self) -> i32 {
        421
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("ctr2")
    }
    fn chip_name(&self) -> &'static str {
        "Counter"
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn allow_bus(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        let bits_y = if c.use_bus() { 1 } else { bits as i32 };
        c.size_x = 2;
        c.size_y = bits_y + 3;
        fill(c, bits * 2 + 6);
        c.make_bit_pins(bits, 1, SIDE_E, 0, "Q", true, true, true);
        c.make_bit_pins(bits, 1, SIDE_W, bits, "I", false, false, true);
        let (clk, clr, enp, rco, load, ent) = (
            Self::pin(c, 0),
            Self::pin(c, 1),
            Self::pin(c, 2),
            Self::pin(c, 3),
            Self::pin(c, 4),
            Self::pin(c, 5),
        );
        c.pins[clk] = Pin::new(0, SIDE_W, "");
        c.pins[clk].clock = true;
        c.pins[clr] = Pin::new(bits_y + 1, SIDE_W, "CLR");
        c.pins[clr].bubble = true;
        c.pins[enp] = Pin::new(bits_y + 2, SIDE_W, "EnP");
        c.pins[rco] = Pin::output(0, SIDE_E, "RCO");
        c.pins[load] = Pin::new(bits_y + 1, SIDE_E, "LOAD");
        c.pins[load].bubble = true;
        c.pins[ent] = Pin::new(bits_y + 2, SIDE_E, "EnT");
    }
    fn read_extra(&mut self, _c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Ok(m) = st.next_i32() {
            self.modulus = m;
        }
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {}", self.modulus)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.modulus = xml.int_attr("mo", self.modulus);
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("mo", self.modulus);
    }
    fn after_xml(&mut self, c: &mut ChipCore) {
        let clr = Self::pin(c, 1);
        c.pins[clr].value = true;
        c.base.volts[clr] = c.high_voltage;
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let bits = c.bits.max(1) as usize;
        let (clk, clr, enp, rco, load, ent) = (
            Self::pin(c, 0),
            Self::pin(c, 1),
            Self::pin(c, 2),
            Self::pin(c, 3),
            Self::pin(c, 4),
            Self::pin(c, 5),
        );
        let last = bits - 1;
        let realmod: i64 = if self.modulus == 0 { 1 << bits } else { i64::from(self.modulus) };
        let read = |c: &ChipCore| -> i64 {
            (0..bits).filter(|&i| value(c, last - i)).fold(0, |a, i| a | (1 << i))
        };
        if value(c, clk) && !c.last_clock {
            if value(c, enp) && value(c, ent) {
                let v = (read(c) + 1) % realmod;
                for i in 0..bits {
                    c.write_output(last - i, v & (1 << i) != 0);
                }
                self.carry = v == realmod - 1;
            }
            if !value(c, load) {
                for i in 0..bits {
                    let v = value(c, i + bits);
                    c.write_output(i, v);
                }
                self.carry = read(c) == realmod - 1;
            }
        }
        if !value(c, clr) {
            for i in 0..bits {
                c.write_output(i, false);
            }
            self.carry = false;
        }
        c.last_clock = value(c, clk);
        let r = self.carry && value(c, ent);
        c.write_output(rco, r);
    }
    fn voltage_source_count(&self, c: &ChipCore) -> usize {
        c.bits.max(1) as usize + 1
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0).set_dimensionless()),
            1 => Some(
                EditInfo::new("Modulus", f64::from(self.modulus), 1.0, 1.0).set_dimensionless(),
            ),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                if ei.value >= 2.0 {
                    c.bits = ei.value as i32;
                } else {
                    ei.error = Some("must be >= 2".into());
                }
            }
            1 => self.modulus = ei.value as i32,
            _ => {}
        }
    }
}

/// Monostable multivibrator: a pulse of fixed length after a rising edge.
#[derive(Clone, Debug)]
pub struct Monostable {
    prev_input: bool,
    retriggerable: bool,
    triggered: bool,
    last_rising_edge: f64,
    delay: f64,
}

impl Default for Monostable {
    fn default() -> Self {
        Monostable {
            prev_input: false,
            retriggerable: false,
            triggered: false,
            last_rising_edge: 0.0,
            delay: 0.01,
        }
    }
}

impl ChipLogic for Monostable {
    fn class_name(&self) -> &'static str {
        "MonostableElm"
    }
    fn dump_type(&self) -> i32 {
        194
    }
    fn chip_name(&self) -> &'static str {
        "Monostable"
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = 2;
        c.size_y = 2;
        fill(c, 3);
        c.pins[0] = Pin::new(0, SIDE_W, "");
        c.pins[0].clock = true;
        c.pins[1] = Pin::output(0, SIDE_E, "Q");
        c.pins[2] = Pin::output(1, SIDE_E, "Q");
        c.pins[2].line_over = true;
    }
    fn after_new(&mut self, c: &mut ChipCore) {
        self.on_reset(c);
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Some(t) = st.next_token() {
            self.retriggerable = t.eq_ignore_ascii_case("true");
        }
        if let Ok(d) = st.next_f64() {
            self.delay = d;
        }
        self.on_reset(c);
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {} {}", self.retriggerable, crate::format::number::js_number(self.delay))
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.retriggerable = xml.bool_attr("rt", self.retriggerable);
        self.delay = xml.double_attr("dl", self.delay);
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.bool_attr("rt", self.retriggerable);
        out.double_attr("dl", self.delay);
    }
    fn after_xml(&mut self, c: &mut ChipCore) {
        self.on_reset(c);
    }
    fn on_reset(&mut self, c: &mut ChipCore) {
        c.pins[2].value = true;
        self.triggered = false;
        self.prev_input = false;
    }
    fn execute(&mut self, c: &mut ChipCore, s: &Stamper) {
        let input = value(c, 0);
        if input && !self.prev_input && (self.retriggerable || !self.triggered) {
            self.last_rising_edge = s.t;
            c.pins[1].value = true;
            c.pins[2].value = false;
            self.triggered = true;
        }
        if self.triggered && s.t > self.last_rising_edge + self.delay {
            c.pins[1].value = false;
            c.pins[2].value = true;
            self.triggered = false;
        }
        self.prev_input = input;
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        2
    }
    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => check("Retriggerable", self.retriggerable),
            1 => Some(EditInfo::new("Period (s)", self.delay, 0.001, 0.1)),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.retriggerable = ei.checkbox.unwrap_or(false),
            1 => self.delay = ei.value,
            _ => {}
        }
    }
}

/// Splits a bus into its individual bits (pure wiring).
#[derive(Clone, Debug, Default)]
pub struct BusSplitter {
    currents: Vec<f64>,
}

impl ChipLogic for BusSplitter {
    fn class_name(&self) -> &'static str {
        "BusSplitterElm"
    }
    fn dump_type(&self) -> i32 {
        433
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("bs")
    }
    fn chip_name(&self) -> &'static str {
        "Bus Splitter"
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        c.size_x = 2;
        c.size_y = bits as i32;
        self.currents = vec![0.0; bits];
        fill(c, bits * 2);
        for i in 0..bits {
            c.pins[i] = Pin::new(0, SIDE_W, "Bus");
            c.pins[i].bus_width = bits as i32;
            c.pins[i].bus_z = i as i32;
            c.pins[i + bits] = Pin::new((bits - 1 - i) as i32, SIDE_E, &i.to_string());
        }
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        0
    }
    fn connection(&self, n1: usize, n2: usize) -> bool {
        // the pin count is twice the bit count, so the distance between a bit's two pins
        // equals the number of bits
        n1.abs_diff(n2) == self.currents.len()
    }
    fn wire_equivalent(&self) -> bool {
        true
    }
    fn connected_pin(&self, n: usize) -> Option<usize> {
        let bits = self.currents.len();
        (n < bits).then_some(n + bits)
    }
    fn bus_width(&self, c: &ChipCore) -> usize {
        c.bits.max(1) as usize
    }
    fn current_into_node(&self, c: &ChipCore, n: usize) -> Option<f64> {
        let bits = c.bits.max(1) as usize;
        let cur = |i: usize| self.currents.get(i).copied().unwrap_or(0.0);
        Some(if n < bits { -cur(n) } else { cur(n - bits) })
    }
    fn set_wire_current(&mut self, c: &mut ChipCore, bit: usize, current: f64) {
        let bits = self.currents.len();
        if bit >= bits {
            return;
        }
        self.currents[bit] = current;
        c.pins[bit + bits].current = current;
        c.pins[0].current = -self.currents.iter().sum::<f64>();
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| bits_field(c))
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            if ei.value >= 2.0 {
                c.bits = ei.value as i32;
            } else {
                ei.error = Some("must be >= 2".into());
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Latch / register
// ---------------------------------------------------------------------------------------------

/// Multi-bit latch or edge-triggered register. The optional enable pins are drawn and saved,
/// but the tri-state output they control is not simulated.
#[derive(Clone, Debug, Default)]
pub struct Latch {
    load_pin: usize,
    reset_pin: usize,
    set_pin: usize,
    ie: [usize; 2],
    last_load: bool,
    stored: Vec<bool>,
}

impl Latch {
    const FLAG_STATE: i32 = 2;
    const FLAG_NO_EDGE: i32 = 4;
    const FLAG_RESET: i32 = 8;
    const FLAG_SET: i32 = 16;
    const FLAG_ENABLE_MASK: i32 = 32 | 64;
    const FLAG_ENABLE_SHIFT: i32 = 5;
    const FLAG_RESET_INVERT: i32 = 128;

    fn enable_mode(c: &ChipCore) -> usize {
        ((c.base.flags & Self::FLAG_ENABLE_MASK) >> Self::FLAG_ENABLE_SHIFT) as usize
    }
    fn edge(c: &ChipCore) -> bool {
        !has(c, Self::FLAG_NO_EDGE)
    }
    fn restore(&mut self, c: &ChipCore) {
        let bits = c.bits.max(1) as usize;
        self.stored = (0..bits).map(|i| value(c, i + bits)).collect();
    }
    fn do_load(&mut self, c: &ChipCore) {
        let bits = c.bits.max(1) as usize;
        if self.stored.len() != bits {
            self.stored = vec![false; bits];
        }
        let load = value(c, self.load_pin);
        if has(c, Self::FLAG_SET) && value(c, self.set_pin) {
            self.stored.iter_mut().for_each(|b| *b = true);
            self.last_load = load;
            return;
        }
        if has(c, Self::FLAG_RESET) && value(c, self.reset_pin) != has(c, Self::FLAG_RESET_INVERT) {
            self.stored.iter_mut().for_each(|b| *b = false);
            self.last_load = load;
            return;
        }
        let mode = Self::enable_mode(c);
        let disabled = (0..mode.min(2)).any(|k| value(c, self.ie[k]));
        if !disabled && load && (!Self::edge(c) || !self.last_load) {
            for i in 0..bits {
                self.stored[i] = value(c, i);
            }
        }
        self.last_load = load;
    }
}

impl ChipLogic for Latch {
    fn class_name(&self) -> &'static str {
        "LatchElm"
    }
    fn dump_type(&self) -> i32 {
        168
    }
    fn chip_name(&self) -> &'static str {
        "Latch"
    }
    fn needs_bits(&self) -> bool {
        true
    }
    fn allow_bus(&self) -> bool {
        true
    }
    fn before_new(&mut self, c: &mut ChipCore) {
        c.base.flags |= Self::FLAG_STATE;
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let bits = c.bits.max(1) as usize;
        let mode = Self::enable_mode(c).min(2);
        let reset = has(c, Self::FLAG_RESET);
        let set = has(c, Self::FLAG_SET);
        let extra_left = usize::from(reset) + usize::from(set) + mode;
        let bits_y = if c.use_bus() { 1 } else { bits as i32 };
        c.size_x = 2;
        c.size_y = (bits_y + 1 + extra_left as i32).max(bits_y + mode as i32);
        fill(c, bits * 2 + 1 + extra_left + mode);
        let state = has(c, Self::FLAG_STATE);
        c.make_bit_pins(bits, 0, SIDE_W, 0, "I", false, false, false);
        c.make_bit_pins(bits, 0, SIDE_E, bits, "O", true, state, false);
        let mut idx = bits * 2;
        let mut left = bits_y;
        self.load_pin = idx;
        c.pins[idx] = Pin::new(left, SIDE_W, if Self::edge(c) { "" } else { "Ld" });
        c.pins[idx].clock = Self::edge(c);
        idx += 1;
        left += 1;
        if reset {
            self.reset_pin = idx;
            c.pins[idx] = Pin::new(left, SIDE_W, "R");
            c.pins[idx].line_over = has(c, Self::FLAG_RESET_INVERT);
            idx += 1;
            left += 1;
        }
        if set {
            self.set_pin = idx;
            c.pins[idx] = Pin::new(left, SIDE_W, "S");
            idx += 1;
            left += 1;
        }
        let mut right = left;
        for k in 0..mode {
            self.ie[k] = idx;
            c.pins[idx] = Pin::new(left, SIDE_W, "IE");
            c.pins[idx].line_over = true;
            idx += 1;
            left += 1;
        }
        for _ in 0..mode {
            c.pins[idx] = Pin::new(right, SIDE_E, "OE");
            c.pins[idx].line_over = true;
            idx += 1;
            right += 1;
        }
    }
    fn after_load(&mut self, c: &mut ChipCore) {
        self.restore(c);
    }
    fn on_reset(&mut self, _c: &mut ChipCore) {
        self.stored.clear();
        self.last_load = false;
    }
    fn read_xml(&mut self, _c: &mut ChipCore, _xml: &XmlNode) {}
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        self.do_load(c);
        let bits = c.bits.max(1) as usize;
        for i in 0..bits {
            let v = self.stored[i];
            c.write_output(i + bits, v);
        }
    }
    fn voltage_source_count(&self, c: &ChipCore) -> usize {
        c.bits.max(1) as usize
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("# of Bits", f64::from(c.bits), 1.0, 1.0).set_dimensionless()),
            1 => check("Edge Triggered", Self::edge(c)),
            2 => check("Reset Pin", has(c, Self::FLAG_RESET)),
            3 => check("Set Pin", has(c, Self::FLAG_SET)),
            4 => Some(EditInfo::choice(
                "Enable Pins",
                vec![
                    "None".into(),
                    "1 Input/Output Enable".into(),
                    "2 Input/Output Enables".into(),
                ],
                Self::enable_mode(c).min(2),
            )),
            5 if has(c, Self::FLAG_RESET) => check("Invert Reset", has(c, Self::FLAG_RESET_INVERT)),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => {
                if ei.value >= 2.0 {
                    c.bits = ei.value as i32;
                } else {
                    ei.error = Some("must be >= 2".into());
                }
            }
            1 => set_flag(c, Self::FLAG_NO_EDGE, !on),
            2 => {
                set_flag(c, Self::FLAG_RESET, on);
                ei.new_dialog = true;
            }
            3 => set_flag(c, Self::FLAG_SET, on),
            4 => {
                let mode = ei.choice_index.min(2) as i32;
                c.base.flags =
                    (c.base.flags & !Self::FLAG_ENABLE_MASK) | (mode << Self::FLAG_ENABLE_SHIFT);
            }
            5 => set_flag(c, Self::FLAG_RESET_INVERT, on),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Multiplexer / demultiplexer
// ---------------------------------------------------------------------------------------------

fn select_pin(bus: bool, sel: usize, i: usize, pos0: i32) -> Pin {
    if bus {
        bus_pin(Pin::new(0, SIDE_S, "S"), sel, i)
    } else {
        Pin::new(i as i32 + pos0, SIDE_S, &format!("S{i}"))
    }
}

fn bus_pin(mut p: Pin, width: usize, z: usize) -> Pin {
    p.bus_width = width as i32;
    p.bus_z = z as i32;
    p
}

fn read_select(c: &ChipCore, first: usize, count: usize) -> usize {
    (0..count).filter(|&i| value(c, first + i)).fold(0, |acc, i| acc | (1 << i))
}

/// Multiplexer with individual or bussed inputs.
#[derive(Clone, Debug)]
pub struct Multiplexer {
    select_bits: usize,
    input_mode: i32,
    data_width: usize,
    output_pin: usize,
    select_first: usize,
    strobe: Option<usize>,
}

impl Default for Multiplexer {
    fn default() -> Self {
        Multiplexer {
            select_bits: 2,
            input_mode: 0,
            data_width: 4,
            output_pin: 0,
            select_first: 0,
            strobe: None,
        }
    }
}

impl Multiplexer {
    const FLAG_INVERTED: i32 = 1 << 1;
    const FLAG_STROBE: i32 = 1 << 2;
    const FLAG_BUS_SELECT: i32 = 1 << 3;
}

impl ChipLogic for Multiplexer {
    fn class_name(&self) -> &'static str {
        "MultiplexerElm"
    }
    fn dump_type(&self) -> i32 {
        184
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("mux")
    }
    fn chip_name(&self) -> &'static str {
        "Multiplexer"
    }
    fn is_digital(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let sel = self.select_bits.clamp(1, 8);
        let outputs = 1usize << sel;
        let bus_sel = has(c, Self::FLAG_BUS_SELECT);
        let inverted = has(c, Self::FLAG_INVERTED);
        let strobe = has(c, Self::FLAG_STROBE);
        let mut pins: Vec<Pin> = Vec::new();
        let bus_bus = self.input_mode == 2;
        let bus_bit = self.input_mode == 1;
        c.size_x = sel as i32 + 1;
        c.size_y = if bus_bit { 3 } else { outputs as i32 + 1 };
        if bus_bus {
            for g in 0..outputs {
                for i in 0..self.data_width {
                    pins.push(bus_pin(
                        Pin::new(g as i32, SIDE_W, &format!("I{g}")),
                        self.data_width,
                        i,
                    ));
                }
            }
        } else if bus_bit {
            for i in 0..outputs {
                pins.push(bus_pin(Pin::new(0, SIDE_W, "I"), outputs, i));
            }
        } else {
            for i in 0..outputs {
                pins.push(Pin::new(i as i32, SIDE_W, &format!("I{i}")));
            }
        }
        self.select_first = pins.len();
        for i in 0..sel {
            pins.push(select_pin(bus_sel, sel, i, 1));
        }
        self.output_pin = pins.len();
        if bus_bus {
            for i in 0..self.data_width {
                pins.push(bus_pin(Pin::output(0, SIDE_E, "Q"), self.data_width, i));
            }
            if inverted {
                for i in 0..self.data_width {
                    let mut p = bus_pin(Pin::output(1, SIDE_E, "Q"), self.data_width, i);
                    p.line_over = true;
                    p.bubble = i == 0;
                    pins.push(p);
                }
            }
        } else {
            pins.push(Pin::output(0, SIDE_E, "Q"));
            if inverted {
                let mut p = Pin::output(1, SIDE_E, "Q");
                p.line_over = true;
                p.bubble = true;
                pins.push(p);
            }
        }
        self.strobe = strobe.then_some(pins.len());
        if strobe {
            pins.push(Pin::new(0, SIDE_S, "STR"));
        }
        c.pins = pins;
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Ok(n) = st.next_i32() {
            self.select_bits = n.max(1) as usize;
        }
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len());
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {}", self.select_bits)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.select_bits = xml.int_attr("se", self.select_bits as i32).max(1) as usize;
        self.input_mode = xml.int_attr("im", 0);
        self.data_width = xml.int_attr("dw", 4).max(2) as usize;
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("se", self.select_bits as i32);
        if self.input_mode != 0 {
            out.int_attr("im", self.input_mode);
        }
        if self.data_width != 4 {
            out.int_attr("dw", self.data_width as i32);
        }
    }
    fn voltage_source_count(&self, c: &ChipCore) -> usize {
        c.pins.iter().filter(|p| p.output).count()
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let sel = read_select(c, self.select_first, self.select_bits.clamp(1, 8));
        let inverted = has(c, Self::FLAG_INVERTED);
        let strobed = self.strobe.is_some_and(|s| value(c, s));
        if self.input_mode == 2 {
            for i in 0..self.data_width {
                let v = !strobed && value(c, sel * self.data_width + i);
                c.pins[self.output_pin + i].value = v;
                if inverted {
                    c.pins[self.output_pin + self.data_width + i].value = !v;
                }
            }
        } else {
            let v = !strobed && value(c, sel);
            c.pins[self.output_pin].value = v;
            if inverted {
                c.pins[self.output_pin + 1].value = !v;
            }
        }
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("# of Select Bits", self.select_bits as f64, 1.0, 8.0)
                    .set_dimensionless(),
            ),
            1 => check("Inverted Output", has(c, Self::FLAG_INVERTED)),
            2 => check("Strobe Pin", has(c, Self::FLAG_STROBE)),
            3 => Some(EditInfo::choice(
                "Input Mode",
                vec![
                    "Individual Inputs".into(),
                    "Bus Input (Bit Select)".into(),
                    "Bus Input/Output".into(),
                ],
                self.input_mode.clamp(0, 2) as usize,
            )),
            4 => check("Bus Select", has(c, Self::FLAG_BUS_SELECT)),
            5 if self.input_mode == 2 => Some(
                EditInfo::new("Data Bus Width", self.data_width as f64, 2.0, 32.0)
                    .set_dimensionless(),
            ),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => {
                if (1.0..=6.0).contains(&ei.value) {
                    self.select_bits = ei.value as usize;
                } else {
                    ei.error = Some("must be between 1 and 6".into());
                }
            }
            1 => set_flag(c, Self::FLAG_INVERTED, on),
            2 => set_flag(c, Self::FLAG_STROBE, on),
            3 => self.input_mode = ei.choice_index as i32,
            4 => set_flag(c, Self::FLAG_BUS_SELECT, on),
            5 => {
                if ei.value >= 2.0 {
                    self.data_width = ei.value as usize;
                } else {
                    ei.error = Some("must be >= 2".into());
                }
            }
            _ => {}
        }
    }
}

/// Demultiplexer with individual or bussed outputs.
#[derive(Clone, Debug)]
pub struct DeMultiplexer {
    select_bits: usize,
    output_mode: i32,
    data_width: usize,
    input_pin: usize,
    select_first: usize,
    output_first: usize,
}

impl Default for DeMultiplexer {
    fn default() -> Self {
        DeMultiplexer {
            select_bits: 0,
            output_mode: 0,
            data_width: 4,
            input_pin: 0,
            select_first: 0,
            output_first: 0,
        }
    }
}

impl DeMultiplexer {
    const FLAG_BUS_SELECT: i32 = 1 << 3;
    const FLAG_INVERT_OUTPUTS: i32 = 1 << 4;
}

impl ChipLogic for DeMultiplexer {
    fn class_name(&self) -> &'static str {
        "DeMultiplexerElm"
    }
    fn dump_type(&self) -> i32 {
        185
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("dmux")
    }
    fn chip_name(&self) -> &'static str {
        "Demultiplexer"
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        if self.select_bits == 0 {
            self.select_bits = 2;
        }
        let sel = self.select_bits.min(8);
        let outputs = 1usize << sel;
        let bus_sel = has(c, Self::FLAG_BUS_SELECT);
        let mut pins: Vec<Pin> = Vec::new();
        match self.output_mode {
            2 => {
                c.size_x = sel as i32 + 1;
                c.size_y = outputs as i32 + 1;
                self.input_pin = 0;
                for i in 0..self.data_width {
                    pins.push(bus_pin(Pin::new(0, SIDE_W, "Q"), self.data_width, i));
                }
                self.select_first = pins.len();
                for i in 0..sel {
                    pins.push(select_pin(bus_sel, sel, i, 1));
                }
                self.output_first = pins.len();
                for g in 0..outputs {
                    for i in 0..self.data_width {
                        pins.push(bus_pin(
                            Pin::output(g as i32, SIDE_E, &format!("Q{g}")),
                            self.data_width,
                            i,
                        ));
                    }
                }
            }
            1 => {
                c.size_x = sel as i32 + 1;
                c.size_y = 3;
                self.input_pin = 0;
                pins.push(Pin::new(0, SIDE_W, "Q"));
                self.select_first = 1;
                for i in 0..sel {
                    pins.push(select_pin(bus_sel, sel, i, 1));
                }
                self.output_first = pins.len();
                for i in 0..outputs {
                    pins.push(bus_pin(Pin::output(1, SIDE_E, "Q"), outputs, i));
                }
            }
            _ => {
                c.size_x = 1 + sel as i32;
                c.size_y = 1 + outputs as i32;
                self.output_first = 0;
                for i in 0..outputs {
                    pins.push(Pin::output(i as i32, SIDE_E, &format!("Q{i}")));
                }
                self.select_first = outputs;
                for i in 0..sel {
                    pins.push(select_pin(bus_sel, sel, i, 0));
                }
                self.input_pin = pins.len();
                pins.push(Pin::new(0, SIDE_W, "Q"));
            }
        }
        c.pins = pins;
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Ok(n) = st.next_i32() {
            self.select_bits = n.max(1) as usize;
            self.setup_pins(c);
            c.base.alloc_nodes(c.pins.len());
        }
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {}", self.select_bits)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.select_bits = xml.int_attr("se", self.select_bits as i32).max(0) as usize;
        self.output_mode = xml.int_attr("om", 0);
        self.data_width = xml.int_attr("dw", 4).max(2) as usize;
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("se", self.select_bits as i32);
        if self.output_mode != 0 {
            out.int_attr("om", self.output_mode);
        }
        if self.data_width != 4 {
            out.int_attr("dw", self.data_width as i32);
        }
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let sel = read_select(c, self.select_first, self.select_bits.min(8));
        let outputs = 1usize << self.select_bits.min(8);
        let width = if self.output_mode == 2 { self.data_width } else { 1 };
        let idle = has(c, Self::FLAG_INVERT_OUTPUTS);
        for i in 0..outputs * width {
            c.pins[self.output_first + i].value = idle;
        }
        for i in 0..width {
            let v = value(c, self.input_pin + i);
            c.pins[self.output_first + sel * width + i].value = v;
        }
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("# of Select Bits", self.select_bits as f64, 0.0, 0.0)
                    .set_dimensionless(),
            ),
            1 => Some(EditInfo::choice(
                "Output Mode",
                vec![
                    "Individual Outputs".into(),
                    "Bus Output (Bit Distribute)".into(),
                    "Bus Input/Output".into(),
                ],
                self.output_mode.clamp(0, 2) as usize,
            )),
            2 => check("Bus Select", has(c, Self::FLAG_BUS_SELECT)),
            3 => check("Keep Inactive Outputs High (74139)", has(c, Self::FLAG_INVERT_OUTPUTS)),
            4 if self.output_mode == 2 => Some(
                EditInfo::new("Data Bus Width", self.data_width as f64, 2.0, 32.0)
                    .set_dimensionless(),
            ),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => {
                if (1.0..=6.0).contains(&ei.value) {
                    self.select_bits = ei.value as usize;
                } else {
                    ei.error = Some("must be between 1 and 6".into());
                }
            }
            1 => self.output_mode = ei.choice_index as i32,
            2 => set_flag(c, Self::FLAG_BUS_SELECT, on),
            3 => set_flag(c, Self::FLAG_INVERT_OUTPUTS, on),
            4 => {
                if ei.value >= 2.0 {
                    self.data_width = ei.value as usize;
                } else {
                    ei.error = Some("must be >= 2".into());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_in<L: ChipLogic>(chip: &mut ChipElm<L>, pin: usize, v: bool) {
        chip.core.pins[pin].value = v;
    }

    fn exec<L: ChipLogic>(chip: &mut ChipElm<L>) {
        let s = Stamper::default();
        chip.logic.execute(&mut chip.core, &s);
    }

    #[test]
    fn d_flip_flop_latches_on_rising_edge() {
        let mut ff = ChipElm::new(DFlipFlop::default(), 0, 0);
        set_in(&mut ff, 0, true);
        exec(&mut ff); // clock low: nothing
        assert!(!ff.core.pins[1].value);
        set_in(&mut ff, 3, true);
        exec(&mut ff);
        assert!(ff.core.pins[1].value);
        assert!(!ff.core.pins[2].value);
        set_in(&mut ff, 0, false);
        exec(&mut ff); // clock stays high: no new edge
        assert!(ff.core.pins[1].value);
        set_in(&mut ff, 3, false);
        exec(&mut ff);
        set_in(&mut ff, 3, true);
        exec(&mut ff);
        assert!(!ff.core.pins[1].value);
        assert!(ff.core.pins[2].value);
    }

    #[test]
    fn jk_flip_flop_toggles() {
        let mut ff = ChipElm::new(JkFlipFlop::default(), 0, 0);
        ff.core.base.flags |= JkFlipFlop::FLAG_POSITIVE;
        set_in(&mut ff, 0, true);
        set_in(&mut ff, 2, true);
        for expect in [true, false, true] {
            set_in(&mut ff, 1, false);
            exec(&mut ff);
            set_in(&mut ff, 1, true);
            exec(&mut ff);
            assert_eq!(ff.core.pins[3].value, expect);
        }
    }

    #[test]
    fn counter_counts_with_modulus() {
        let mut ctr = ChipElm::new(Counter::default(), 0, 0);
        ctr.logic.modulus = 3;
        ctr.core.bits = 4;
        ctr.rebuild_pins();
        set_in(&mut ctr, 1, false); // active-high reset, not asserted
        let value_of = |c: &ChipElm<Counter>| {
            (0..4).fold(0, |a, i| a | (usize::from(c.core.pins[5 - i].value) << i))
        };
        let mut seen = vec![];
        for _ in 0..4 {
            set_in(&mut ctr, 0, true);
            exec(&mut ctr);
            seen.push(value_of(&ctr));
            set_in(&mut ctr, 0, false);
            exec(&mut ctr);
        }
        assert_eq!(seen, vec![1, 2, 0, 1]);
    }

    #[test]
    fn full_adder_adds() {
        let mut fa = ChipElm::new(FullAdder { has_bits: true }, 0, 0);
        fa.core.bits = 4;
        fa.rebuild_pins();
        // 11 + 7 + carry 1 = 19 = 0b1_0011
        for (i, bit) in [1, 1, 0, 1].iter().enumerate() {
            set_in(&mut fa, i, *bit == 1);
        }
        for (i, bit) in [1, 1, 1, 0].iter().enumerate() {
            set_in(&mut fa, 4 + i, *bit == 1);
        }
        set_in(&mut fa, 12, true);
        exec(&mut fa);
        let sum: usize = (0..4).map(|i| usize::from(fa.core.pins[8 + i].value) << i).sum();
        assert_eq!(sum, 3);
        assert!(fa.core.pins[13].value);
    }

    #[test]
    fn multiplexer_selects_input() {
        let mut m = ChipElm::new(Multiplexer::default(), 0, 0);
        set_in(&mut m, 2, true); // I2
        set_in(&mut m, 4, false); // S0
        set_in(&mut m, 5, true); // S1 -> 2
        exec(&mut m);
        assert!(m.core.pins[6].value);
        set_in(&mut m, 5, false);
        exec(&mut m);
        assert!(!m.core.pins[6].value);
    }

    #[test]
    fn demultiplexer_routes_input() {
        let mut d = ChipElm::new(DeMultiplexer::default(), 0, 0);
        let input = d.logic.input_pin;
        set_in(&mut d, input, true);
        set_in(&mut d, 4, true); // S0 -> output 1
        exec(&mut d);
        let outs: Vec<bool> = (0..4).map(|i| d.core.pins[i].value).collect();
        assert_eq!(outs, vec![false, true, false, false]);
    }

    #[test]
    fn latch_holds_value() {
        let mut l = ChipElm::new(Latch::default(), 0, 0);
        l.core.bits = 2;
        l.rebuild_pins();
        let load = l.logic.load_pin;
        set_in(&mut l, 0, true);
        set_in(&mut l, load, true);
        exec(&mut l);
        assert!(l.core.pins[2].value && !l.core.pins[3].value);
        set_in(&mut l, 0, false);
        set_in(&mut l, load, false);
        exec(&mut l);
        assert!(l.core.pins[2].value);
    }

    #[test]
    fn dump_roundtrip_preserves_text() {
        let ff = ChipElm::new(DFlipFlop::default(), 16, 32);
        let text = ff.dump();
        let mut tok = Tokenizer::new(&text);
        assert_eq!(tok.next_i32().unwrap(), 155);
    }
}
