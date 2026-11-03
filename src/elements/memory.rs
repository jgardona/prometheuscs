// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (SRAMElm, ROMElm).
//! Static RAM and ROM chips.

use super::chip::*;
use super::element::*;
use super::registry::{ElementDescriptor, Registry};
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::solver::stamper::{NodeId, Stamper};
use std::collections::BTreeMap;

/// Family name used for the feature switch.
pub const FAMILY: &str = "digital";
const CAT: &str = "Digital Chips";

/// Registers the memory chips.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("SRAMElm", FAMILY, "Add Static RAM", CAT, |x, y| {
            Box::new(ChipElm::new(Memory::new(false), x, y))
        })
        .dump(413, |a, b, c, d, f, st| {
            Ok(Box::new(ChipElm::from_dump(Memory::new(false), a, b, c, d, f, st)))
        }),
    );
    r.add(
        ElementDescriptor::new("ROMElm", FAMILY, "Add ROM", CAT, |x, y| {
            Box::new(ChipElm::new(Memory::new(true), x, y))
        })
        .dump(436, |a, b, c, d, f, st| {
            Ok(Box::new(ChipElm::from_dump(Memory::new(true), a, b, c, d, f, st)))
        }),
    );
}

const FLAG_RELOAD_ON_RESET: i32 = 2;
const FLAG_HEX_DISPLAY: i32 = 4;

/// A RAM (`rom` false) or a ROM: contents are a sparse address-to-word map.
#[derive(Clone, Debug)]
pub struct Memory {
    rom: bool,
    address_bits: i32,
    data_bits: i32,
    map: BTreeMap<i32, i32>,
    initial: Option<BTreeMap<i32, i32>>,
    address: i32,
}

/// Files this size or larger are refused by Load Contents From File (as in the original).
const MAX_LOAD_BYTES: usize = 128_000;

impl Memory {
    /// Replaces the contents with the raw bytes of a file, from address 0: one value per byte,
    /// or per big-endian pair of bytes when the data is wider than 8 bits (as the original's
    /// Load Contents From File). A file too large is refused and the contents are kept.
    fn load_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() >= MAX_LOAD_BYTES {
            return Err("Cannot load: That file is too large!".into());
        }
        let per_value = if self.data_bits > 8 { 2 } else { 1 };
        self.map.clear();
        for (addr, chunk) in bytes.chunks_exact(per_value).enumerate() {
            let value = chunk.iter().fold(0i32, |v, b| (v << 8) | i32::from(*b));
            if value != 0 {
                self.map.insert(addr as i32, value);
            }
        }
        Ok(())
    }

    fn new(rom: bool) -> Self {
        Memory {
            rom,
            address_bits: 4,
            data_bits: 4,
            map: BTreeMap::new(),
            initial: None,
            address: 0,
        }
    }

    /// Index of the first address pin.
    fn address_nodes(&self) -> usize {
        if self.rom { 1 } else { 2 }
    }
    fn data_nodes(&self) -> usize {
        self.address_nodes() + self.address_bits as usize
    }
    fn internal_base(&self) -> usize {
        self.data_nodes() + self.data_bits as usize
    }

    fn contents_to_string(&self, hex: bool) -> String {
        let to_hex = |v: i32| {
            let mask = (1i64 << self.data_bits) - 1;
            format!("{:02X}", i64::from(v) & mask)
        };
        let mut s = String::new();
        let max = 1i32 << self.address_bits;
        let mut i = 0;
        while i < max {
            let val = self.map.get(&i).copied().unwrap_or(0);
            if val == 0 {
                i += 1;
                continue;
            }
            s += &if hex { format!("{i:X}: {}", to_hex(val)) } else { format!("{i}: {val}") };
            let mut count = 1;
            loop {
                i += 1;
                let v = self.map.get(&i).copied().unwrap_or(0);
                if v == 0 {
                    break;
                }
                s += &format!(" {}", if hex { to_hex(v) } else { v.to_string() });
                count += 1;
                if count == 8 {
                    i += 1;
                    break;
                }
            }
            s.push('\n');
        }
        s
    }

    fn parse_number(s: &str, hex: bool) -> Option<i32> {
        let s = s.trim();
        if let Some(h) = s.strip_prefix("0x") {
            return i32::from_str_radix(h, 16).ok();
        }
        if hex {
            return i32::from_str_radix(s, 16).ok();
        }
        if let Some(b) = s.strip_prefix("0b") {
            return i32::from_str_radix(b, 2).ok();
        }
        s.parse().ok()
    }

    fn parse_contents(&mut self, text: &str, hex: bool) {
        self.map.clear();
        for line in text.lines() {
            let Some((addr, vals)) = line.split_once(':') else { continue };
            let Some(mut a) = Self::parse_number(addr, hex) else { continue };
            for v in vals.split_whitespace() {
                let Some(v) = Self::parse_number(v, hex) else { break };
                self.map.insert(a, v);
                a += 1;
            }
        }
    }

    fn read_address(&self, c: &ChipCore) -> i32 {
        let th = c.threshold();
        (0..self.address_bits as usize)
            .filter(|&i| c.base.volts[self.address_nodes() + i] > th)
            .fold(0, |a, i| a | (1 << (self.address_bits as usize - 1 - i)))
    }
}

impl ChipLogic for Memory {
    fn class_name(&self) -> &'static str {
        if self.rom { "ROMElm" } else { "SRAMElm" }
    }
    fn dump_type(&self) -> i32 {
        if self.rom { 436 } else { 413 }
    }
    fn chip_name(&self) -> &'static str {
        if self.rom { "ROM" } else { "Static RAM" }
    }
    fn allow_bus(&self) -> bool {
        true
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn internal_nodes(&self) -> usize {
        self.data_bits as usize
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        self.data_bits as usize
    }
    fn voltage_source_node(&self, _c: &ChipCore, j: usize) -> Option<usize> {
        Some(self.internal_base() + j)
    }

    fn setup_pins(&mut self, c: &mut ChipCore) {
        let (ab, db) = (self.address_bits.max(1) as usize, self.data_bits.max(1) as usize);
        c.size_x = 2;
        let addr_y = if c.use_bus() { 1 } else { ab as i32 };
        let data_y = if c.use_bus() { 1 } else { db as i32 };
        c.size_y = addr_y.max(data_y) + 1;
        c.bits = ab as i32;
        let n = self.address_nodes() + ab + db;
        c.pins = vec![Pin::new(0, SIDE_W, ""); n];
        c.pins[0] = Pin::new(0, SIDE_W, if self.rom { "OE" } else { "WE" });
        c.pins[0].line_over = true;
        if !self.rom {
            c.pins[1] = Pin::new(0, SIDE_E, "OE");
            c.pins[1].line_over = true;
        }
        let (an, dn) = (self.address_nodes(), self.data_nodes());
        c.make_bit_pins(ab, c.size_y - addr_y, SIDE_W, an, "A", false, false, true);
        c.make_bit_pins(db, c.size_y - data_y, SIDE_E, dn, "D", true, false, true);
    }

    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        self.address_bits = st.next_i32().unwrap_or(4);
        self.data_bits = st.next_i32().unwrap_or(4);
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len() + self.data_bits as usize);
        self.map.clear();
        'runs: while let Ok(mut a) = st.next_i32() {
            if a < 0 {
                break;
            }
            let Ok(v) = st.next_i32() else { break };
            self.map.insert(a, v);
            loop {
                let Ok(v) = st.next_i32() else { break 'runs };
                if v < 0 {
                    break;
                }
                a += 1;
                self.map.insert(a, v);
            }
        }
        if c.base.flags & FLAG_RELOAD_ON_RESET != 0 {
            self.initial = Some(self.map.clone());
        }
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        let mut s = format!(" {} {}", self.address_bits, self.data_bits);
        let mut it = self.map.iter().filter(|(_, v)| **v >= 0).peekable();
        while let Some((&a, &v)) = it.next() {
            s += &format!(" {a} {v}");
            let mut last = a;
            while let Some(&(&na, &nv)) = it.peek() {
                if na != last + 1 {
                    break;
                }
                s += &format!(" {nv}");
                last = na;
                it.next();
            }
            s += " -1";
        }
        s
    }
    fn read_xml(&mut self, c: &mut ChipCore, xml: &XmlNode) {
        self.address_bits = xml.int_attr("ab", self.address_bits);
        self.data_bits = xml.int_attr("db", self.data_bits);
        let hex = c.base.flags & FLAG_HEX_DISPLAY != 0;
        self.parse_contents(&xml.text, hex);
        if c.base.flags & FLAG_RELOAD_ON_RESET != 0 {
            self.initial = Some(self.map.clone());
        }
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("ab", self.address_bits);
        out.int_attr("db", self.data_bits);
    }
    fn write_xml_state(&self, c: &ChipCore, out: &mut XmlOut) {
        if !self.map.is_empty() {
            out.text = Some(self.contents_to_string(c.base.flags & FLAG_HEX_DISPLAY != 0));
        }
    }
    fn on_reset(&mut self, c: &mut ChipCore) {
        if c.base.flags & FLAG_RELOAD_ON_RESET != 0 {
            if let Some(m) = &self.initial {
                self.map = m.clone();
            }
        }
    }

    fn matrix_connection(&self, n1: usize, n2: usize) -> bool {
        let (ib, dn) = (self.internal_base(), self.data_nodes());
        (0..self.data_bits as usize)
            .any(|i| (n1 == ib + i && n2 == dn + i) || (n2 == ib + i && n1 == dn + i))
    }
    fn validate(
        &mut self,
        _c: &mut ChipCore,
        _idx: usize,
        _v: &mut crate::solver::path::Validator<'_>,
    ) -> Option<bool> {
        Some(true)
    }

    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let (dn, ib) = (self.data_nodes(), self.internal_base());
        for i in 0..self.data_bits as usize {
            if let Some(vs) = c.pins[dn + i].volt_source {
                s.stamp_voltage_source_var(NodeId::GROUND, c.base.nodes[ib + i], vs);
            }
        }
        true
    }

    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let th = c.threshold();
        let v = &c.base.volts;
        let output_enabled = if self.rom {
            v[0] < th
        } else {
            let write = v[0] < th;
            v[1] < th && !write
        };
        self.address = self.read_address(c);
        let data = self.map.get(&self.address).copied().unwrap_or(0);
        let (dn, ib, db) = (self.data_nodes(), self.internal_base(), self.data_bits as usize);
        for i in 0..db {
            let high = data & (1 << (db - 1 - i)) != 0;
            if let Some(vs) = c.pins[dn + i].volt_source {
                s.update_voltage_source(vs, if high { c.high_voltage } else { 0.0 });
            }
            let n = &c.base.nodes;
            if output_enabled {
                s.stamp_resistor(n[ib + i], n[dn + i], 1.0);
            } else {
                s.stamp_resistor(n[dn + i], NodeId::GROUND, 1e8);
            }
        }
        true
    }

    fn step_finished(&mut self, c: &mut ChipCore, _s: &mut Stamper) {
        if self.rom || c.base.volts[0] >= c.threshold() {
            return;
        }
        let th = c.threshold();
        let (dn, db) = (self.data_nodes(), self.data_bits as usize);
        let data =
            (0..db).filter(|&i| c.base.volts[dn + i] > th).fold(0, |a, i| a | (1 << (db - 1 - i)));
        self.map.insert(self.address, data);
    }

    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        // as the original: the file chooser at 4 (RAM only), then the reload option
        let reload = if self.rom { 4 } else { 5 };
        match n {
            0 => Some(
                EditInfo::new("# of Address Bits", f64::from(self.address_bits), 1.0, 1.0)
                    .set_dimensionless(),
            ),
            1 => Some(
                EditInfo::new("# of Data Bits", f64::from(self.data_bits), 1.0, 1.0)
                    .set_dimensionless(),
            ),
            2 => Some(EditInfo::text_area(
                "Contents",
                &self.contents_to_string(c.base.flags & FLAG_HEX_DISPLAY != 0),
            )),
            3 => Some(EditInfo::checkbox("Hex Display", c.base.flags & FLAG_HEX_DISPLAY != 0)),
            4 if !self.rom => Some(
                EditInfo::text("Load Contents From File", "").open_file(&[]).disallow_sliders(),
            ),
            n if n == reload => Some(EditInfo::checkbox(
                "Restore Contents on Reset",
                c.base.flags & FLAG_RELOAD_ON_RESET != 0,
            )),
            _ => None,
        }
    }

    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let hex = c.base.flags & FLAG_HEX_DISPLAY != 0;
        match n {
            0 | 1 => {
                if (2.0..=16.0).contains(&ei.value) {
                    if n == 0 {
                        self.address_bits = ei.value as i32;
                    } else {
                        self.data_bits = ei.value as i32;
                    }
                } else {
                    ei.error = Some("must be between 2 and 16".into());
                }
            }
            2 => {
                if let Some(t) = &ei.text {
                    self.parse_contents(t, hex);
                }
                if c.base.flags & FLAG_RELOAD_ON_RESET != 0 {
                    self.initial = Some(self.map.clone());
                }
            }
            3 => {
                if ei.checkbox.unwrap_or(false) {
                    c.base.flags |= FLAG_HEX_DISPLAY;
                } else {
                    c.base.flags &= !FLAG_HEX_DISPLAY;
                }
            }
            4 if !self.rom => {
                let path = ei.text.as_deref().map(str::trim).unwrap_or_default();
                if path.is_empty() {
                    return;
                }
                match std::fs::read(path) {
                    Ok(bytes) => match self.load_bytes(&bytes) {
                        Ok(()) => {
                            if c.base.flags & FLAG_RELOAD_ON_RESET != 0 {
                                self.initial = Some(self.map.clone());
                            }
                            // the Contents field shows the new contents
                            ei.new_dialog = true;
                        }
                        Err(e) => ei.error = Some(e),
                    },
                    Err(e) => ei.error = Some(format!("{path}: {e}")),
                }
            }
            n if n == 4 || n == 5 => {
                if ei.checkbox.unwrap_or(false) {
                    c.base.flags |= FLAG_RELOAD_ON_RESET;
                    self.initial = Some(self.map.clone());
                } else {
                    c.base.flags &= !FLAG_RELOAD_ON_RESET;
                    self.initial = None;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_loaded_as_raw_bytes() {
        let mut m = Memory::new(false);
        m.load_bytes(&[1, 2, 0, 0xff]).unwrap();
        assert_eq!(m.contents_to_string(false), "0: 1 2\n3: 255\n");
        // wider than 8 bits: big-endian pairs, an odd last byte is dropped
        m.data_bits = 12;
        m.load_bytes(&[0x01, 0x02, 0x0a, 0xbc, 0x07]).unwrap();
        assert_eq!(m.map.get(&0), Some(&0x0102));
        assert_eq!(m.map.get(&1), Some(&0x0abc));
        assert_eq!(m.map.len(), 2);
        // too large: refused, contents kept
        let err = m.load_bytes(&vec![1; MAX_LOAD_BYTES]).unwrap_err();
        assert_eq!(err, "Cannot load: That file is too large!");
        assert_eq!(m.map.len(), 2);
    }

    #[test]
    fn contents_round_trip_through_text() {
        let mut m = Memory::new(true);
        m.parse_contents("0: 1 2 3\n8: 15\n", false);
        assert_eq!(m.map.get(&2), Some(&3));
        assert_eq!(m.contents_to_string(false), "0: 1 2 3\n8: 15\n");
        let mut hexm = Memory::new(true);
        hexm.parse_contents("A: 0F 1\n", true);
        assert_eq!(hexm.map.get(&11), Some(&1));
    }

    #[test]
    fn dump_runs_read_back() {
        let mut m = Memory::new(true);
        m.map.extend([(0, 5), (1, 6), (9, 7)]);
        let mut chip = ChipElm::new(m, 0, 0);
        let text = chip.dump();
        assert!(text.ends_with(" 4 4 0 5 6 -1 9 7 -1"), "{text}");
        let mut st = Tokenizer::new(&text);
        for _ in 0..6 {
            st.next_token();
        }
        let back = ChipElm::from_dump(
            Memory::new(true),
            0,
            0,
            0,
            0,
            0,
            &mut Tokenizer::new("4 4 0 5 6 -1 9 7 -1"),
        );
        assert_eq!(back.logic.map.len(), 3);
        let _ = &mut chip;
    }

    #[test]
    fn rom_drives_the_addressed_word() {
        let mut m = Memory::new(true);
        m.map.insert(3, 0b1010);
        let mut chip = ChipElm::new(m, 0, 0);
        // address pins start at 1; most significant address bit first
        let an = chip.logic.address_nodes();
        chip.core.base.volts[an + 2] = 5.0;
        chip.core.base.volts[an + 3] = 5.0;
        assert_eq!(chip.logic.read_address(&chip.core), 3);
    }
}
