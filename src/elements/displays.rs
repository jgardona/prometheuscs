// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (SevenSegElm, SevenSegDecoderElm,
// DecimalDisplayElm, LEDArrayElm).
//! Displays built on the chip framework: segment displays and their decoder, the numeric
//! display and the LED matrix.

use super::chip::*;
use super::element::*;
use super::geometry::*;
use super::junction::Diode;
use super::models::DiodeModel;
use super::registry::{ElementDescriptor, Registry};
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::render::painter::{Color, Painter, TextBaseline};
use crate::solver::stamper::Stamper;

/// Family name used for the feature switch.
pub const FAMILY: &str = "digital";
const CAT: &str = "Digital Chips";

/// Registers the displays.
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
    chip!("SevenSegElm", "Add Seven Segment Display", 157, SevenSeg::new());
    chip!("SevenSegDecoderElm", "Add Seven Segment Decoder", 197, SevenSegDecoder::default());
    chip!("DecimalDisplayElm", "Add Decimal Display", 419, DecimalDisplay::new());
    chip!("LEDArrayElm", "Add LED Array", 405, LedArray::new());
}

fn fill(c: &mut ChipCore, n: usize) {
    c.pins = vec![Pin::new(0, SIDE_W, ""); n];
}

fn value(c: &ChipCore, n: usize) -> bool {
    c.base.volts.get(n).is_some_and(|v| *v > c.threshold())
}

/// Brightness of an LED segment from its current: logarithmic, clamped.
fn led_level(current: f64, min: f64) -> f64 {
    let mut w = current / 0.01;
    if w > 0.0 {
        w = 255.0 * (1.0 + 0.2 * w.ln());
    }
    w.clamp(min, 255.0)
}

fn choice(name: &str, items: &[&str], selected: usize) -> EditInfo {
    EditInfo::choice(name, items.iter().map(|s| (*s).to_string()).collect(), selected)
}

// ---------------------------------------------------------------------------------------------
// Seven (or fourteen, sixteen) segment display
// ---------------------------------------------------------------------------------------------

const ES_NONE: i32 = 0;
const ES_DP: i32 = 1;
const ES_COLON: i32 = 2;

const DISPLAY7: [i32; 28] =
    [0, 0, 2, 0, 2, 0, 2, 1, 2, 1, 2, 2, 0, 2, 2, 2, 0, 1, 0, 2, 0, 0, 0, 1, 0, 1, 2, 1];
const DISPLAY16: [i32; 64] = [
    0, 0, 1, 0, 1, 0, 2, 0, 2, 0, 2, 1, 2, 1, 2, 2, 2, 2, 1, 2, 1, 2, 0, 2, 0, 2, 0, 1, 0, 1, 0, 0,
    0, 0, 1, 1, 1, 0, 1, 1, 2, 0, 1, 1, 1, 1, 2, 1, 1, 1, 2, 2, 1, 1, 1, 2, 1, 1, 0, 2, 0, 1, 1, 1,
];
const DISPLAY14: [i32; 56] = [
    0, 0, 2, 0, 2, 0, 2, 1, 2, 1, 2, 2, 2, 2, 0, 2, 0, 2, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0, 1, 1,
    2, 0, 1, 1, 1, 1, 2, 1, 1, 1, 2, 2, 1, 1, 1, 2, 1, 1, 0, 2, 0, 1, 1, 1,
];

/// Segment display, either driven by logic inputs or by LEDs with a common pin.
#[derive(Clone, Debug)]
pub struct SevenSeg {
    base_segments: i32,
    segments: i32,
    extra: i32,
    pin_count: usize,
    common_pin: Option<usize>,
    diode_direction: i32,
    diodes: Vec<Diode>,
    currents: Vec<f64>,
}

impl SevenSeg {
    fn new() -> Self {
        let mut s = SevenSeg {
            base_segments: 7,
            segments: 7,
            extra: ES_NONE,
            pin_count: 7,
            common_pin: None,
            diode_direction: 0,
            diodes: Vec::new(),
            currents: Vec::new(),
        };
        s.set_pin_count();
        s
    }

    fn set_pin_count(&mut self) {
        self.segments = self.base_segments + i32::from(self.extra > 0);
        if self.diode_direction == 0 {
            self.pin_count = self.segments as usize;
            self.common_pin = None;
        } else {
            self.pin_count = self.segments as usize + 1;
            self.common_pin = Some(self.pin_count - 1);
        }
    }

    fn color(&self, c: &ChipCore, ctx: &DrawCtx, p: usize) -> Color {
        let white = ctx.printable;
        if self.diode_direction == 0 {
            return if value(c, p) {
                Color::new(255, 0, 0)
            } else if white {
                Color::new(245, 245, 245)
            } else {
                Color::new(30, 0, 0)
            };
        }
        let cur = self.currents.get(p).copied().unwrap_or(0.0);
        let w =
            led_level(-f64::from(self.diode_direction) * cur, if white { 5.0 } else { 30.0 }) as u8;
        if white { Color::new(255, 255 - w, 255 - w) } else { Color::new(w, 0, 0) }
    }
}

fn draw_segment(g: &mut dyn Painter, p1: Point, p2: Point, thick: i32) {
    let dn = f64::from(((p1.x - p2.x).pow(2) + (p1.y - p2.y).pow(2)) as f32).sqrt().max(1.0);
    let t = f64::from(thick);
    let (p3, p4) = interp_point2(p1, p2, t / dn, t);
    let (p5, p6) = interp_point2(p1, p2, 1.0 - t / dn, t);
    g.begin_path();
    g.move_to(f64::from(p1.x), f64::from(p1.y));
    for p in [p3, p5, p2, p6, p4, p1] {
        g.line_to(f64::from(p.x), f64::from(p.y));
    }
    g.fill();
}

fn draw_dot(g: &mut dyn Painter, x: i32, y: i32, sp: i32) {
    g.begin_path();
    g.move_to(f64::from(x), f64::from(y - sp));
    for (px, py) in [(x - sp, y), (x, y + sp), (x + sp, y), (x, y - sp)] {
        g.line_to(f64::from(px), f64::from(py));
    }
    g.fill();
}

impl ChipLogic for SevenSeg {
    fn class_name(&self) -> &'static str {
        "SevenSegElm"
    }
    fn dump_type(&self) -> i32 {
        157
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("ssd")
    }
    fn chip_name(&self) -> &'static str {
        "segment display"
    }
    fn allow_bus(&self) -> bool {
        self.diode_direction == 0
    }
    fn non_linear(&self) -> bool {
        self.diode_direction != 0
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        0
    }

    fn setup_pins(&mut self, c: &mut ChipCore) {
        if self.pin_count == 0 {
            return;
        }
        let seg = self.segments as usize;
        c.bits = self.segments;
        fill(c, self.pin_count);
        if c.use_bus() {
            c.size_x = if self.base_segments == 7 { 4 } else { 5 };
            c.size_y = if self.base_segments == 7 { 4 } else { 6 };
            c.make_bit_pins(seg, 0, SIDE_W, 0, "I", false, false, false);
            if let Some(cp) = self.common_pin {
                c.pins[cp] =
                    Pin::new(1, SIDE_W, if self.diode_direction == 1 { "gnd" } else { "Vcc" });
            }
            return;
        }
        let left = ((self.base_segments + 1) / 2) as usize;
        c.size_y = left as i32;
        c.size_x = if self.base_segments == 7 && self.pin_count <= 7 { 4 } else { 5 };
        if self.pin_count > left * 2 {
            c.size_y += 1;
        }
        for i in 0..left {
            c.pins[i] = Pin::new(i as i32, SIDE_W, &char::from(b'a' + i as u8).to_string());
        }
        let compat = seg == 7 && self.diode_direction == 0 && self.extra == ES_NONE;
        let mut s = i32::from(compat);
        for i in left..seg {
            c.pins[i] = Pin::new(
                s,
                if compat { SIDE_S } else { SIDE_E },
                &char::from(b'a' + i as u8).to_string(),
            );
            s += 1;
        }
        if self.extra == ES_DP {
            c.pins[seg - 1].text = "dp".into();
        }
        if let Some(cp) = self.common_pin {
            let mut side = SIDE_E;
            if seg != 7 {
                side = SIDE_W;
                s = left as i32;
            }
            c.pins[cp] = Pin::new(s, side, if self.diode_direction == 1 { "gnd" } else { "Vcc" });
        }
    }

    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let (Ok(b), Ok(e), Ok(d)) = (st.next_i32(), st.next_i32(), st.next_i32()) {
            self.base_segments = b;
            self.extra = e;
            self.diode_direction = d;
        }
        self.set_pin_count();
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len());
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {} {} {}", self.base_segments, self.extra, self.diode_direction)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.base_segments = xml.int_attr("ba", self.base_segments);
        self.extra = xml.int_attr("ex", self.extra);
        self.diode_direction = xml.int_attr("di", self.diode_direction);
        self.set_pin_count();
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("ba", self.base_segments);
        out.int_attr("ex", self.extra);
        out.int_attr("di", self.diode_direction);
    }

    fn connection(&self, _n1: usize, _n2: usize) -> bool {
        self.diode_direction != 0
    }
    fn matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        self.diode_direction != 0
    }
    fn ground_connection(&self, _c: &ChipCore, _n: usize) -> bool {
        false
    }

    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        self.diodes.clear();
        let Some(cp) = self.common_pin else { return true };
        let model = DiodeModel::lookup("default-led").unwrap_or_else(DiodeModel::default_model);
        for i in 0..self.segments as usize {
            let mut d = Diode::default();
            d.setup(&model);
            let n = &c.base.nodes;
            if self.diode_direction == 1 {
                d.stamp(s, n[i], n[cp]);
            } else {
                d.stamp(s, n[cp], n[i]);
            }
            self.diodes.push(d);
        }
        true
    }

    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        if let Some(cp) = self.common_pin {
            let dir = f64::from(self.diode_direction);
            for (i, d) in self.diodes.iter_mut().enumerate() {
                d.do_step(s, dir * (c.base.volts[i] - c.base.volts[cp]));
            }
        }
        true
    }

    fn calculate_current(&mut self, c: &mut ChipCore) {
        let Some(cp) = self.common_pin.filter(|_| !self.diodes.is_empty()) else {
            for p in &mut c.pins {
                p.current = 0.0;
            }
            return;
        };
        let dir = f64::from(self.diode_direction);
        c.pins[cp].current = 0.0;
        for (i, d) in self.diodes.iter().enumerate() {
            let cur = -dir * d.calculate_current(dir * (c.base.volts[i] - c.base.volts[cp]));
            c.pins[i].current = cur;
            c.pins[cp].current -= cur;
        }
    }

    fn step_finished(&mut self, c: &mut ChipCore, s: &mut Stamper) {
        if self.common_pin.is_some_and(|cp| c.pins[cp].current.abs() > 1e12) {
            s.stop("max current exceeded", None);
        }
    }

    fn visual_state(&self, c: &ChipCore) -> Vec<f64> {
        c.pins.iter().map(|p| p.current).collect()
    }
    fn set_visual_state(&mut self, c: &mut ChipCore, state: &[f64]) {
        self.currents = state.to_vec();
        for (p, v) in c.pins.iter_mut().zip(state) {
            p.current = *v;
        }
    }

    fn draw_label(&self, g: &mut dyn Painter, c: &mut ChipCore, ctx: &DrawCtx, _x: i32, _y: i32) {
        let cspc = c.cspc;
        let flipped_xy = c.is_flipped_xy();
        let mut spx = cspc * 2;
        if self.extra != ES_NONE {
            spx = (f64::from(spx) * 0.9) as i32;
        }
        if c.size_y <= 4 || flipped_xy {
            spx /= 2;
        }
        let spy = spx * 2;
        let xl = c.base.x + cspc + c.flipped_size_x * cspc - spx;
        let mut yl = c.base.y - cspc + c.flipped_size_y * cspc - spy;
        if c.size_y <= 4 && c.base.flags & (FLAG_FLIP_Y | FLAG_FLIP_XY) != 0 {
            yl += 10;
        }
        let disp: &[i32] = match self.base_segments {
            7 => &DISPLAY7,
            14 => &DISPLAY14,
            _ => &DISPLAY16,
        };
        let thick = if c.size_y <= 4 { 5 } else { spx / 6 };
        let dpsize = if c.size_y <= 4 || !flipped_xy { 7 } else { 3 };
        for step in 0..2 {
            for i in 0..self.segments as usize {
                if i * 4 + 3 >= disp.len() {
                    break;
                }
                let d = &disp[i * 4..i * 4 + 4];
                let diag = d[0] != d[2] && d[1] != d[3];
                if diag != (step == 0) {
                    continue;
                }
                g.set_color(self.color(c, ctx, i));
                let a = Point::new(xl + d[0] * spx, yl + d[1] * spy);
                let b = Point::new(xl + d[2] * spx, yl + d[3] * spy);
                draw_segment(g, a, b, thick);
            }
        }
        let bs = self.base_segments as usize;
        if self.extra == ES_DP {
            g.set_color(self.color(c, ctx, bs));
            let dist = (f64::from(spx) * 1.5).max(f64::from(spx + 12)) as i32;
            draw_dot(g, xl + spx + dist, yl + spy * 2, dpsize);
        }
        if self.extra == ES_COLON {
            g.set_color(self.color(c, ctx, bs));
            let dist = (f64::from(spx) * 1.5).max(f64::from(spx + 14)) as i32;
            draw_dot(g, xl + spx + dist, yl + (f64::from(spy) * 0.5) as i32, dpsize);
            draw_dot(g, xl + spx + dist, yl + (f64::from(spy) * 1.5) as i32, dpsize);
        }
    }

    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => {
                let sel = match self.base_segments {
                    7 => 0,
                    14 => 1,
                    _ => 2,
                };
                Some(choice("Segments", &["7 Segment", "14 Segment", "16 Segment"], sel))
            }
            1 => Some(choice(
                "Extra Segment",
                &["None", "Decimal Point", "Colon"],
                self.extra.clamp(0, 2) as usize,
            )),
            2 => {
                let sel = match self.diode_direction {
                    1 => 0,
                    -1 => 1,
                    _ => 2,
                };
                Some(choice(
                    "Diodes",
                    &["Common Cathode", "Common Anode", "None (logic inputs)"],
                    sel,
                ))
            }
            _ => None,
        }
    }

    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.base_segments = [7, 14, 16][ei.choice_index.min(2)],
            1 => self.extra = ei.choice_index as i32,
            2 => self.diode_direction = [1, -1, 0][ei.choice_index.min(2)],
            _ => return,
        }
        self.set_pin_count();
    }
}

// ---------------------------------------------------------------------------------------------
// Segment decoder
// ---------------------------------------------------------------------------------------------

const SYMBOLS7: [[bool; 7]; 16] = {
    const T: bool = true;
    const F: bool = false;
    [
        [T, T, T, T, T, T, F],
        [F, T, T, F, F, F, F],
        [T, T, F, T, T, F, T],
        [T, T, T, T, F, F, T],
        [F, T, T, F, F, T, T],
        [T, F, T, T, F, T, T],
        [T, F, T, T, T, T, T],
        [T, T, T, F, F, F, F],
        [T, T, T, T, T, T, T],
        [T, T, T, F, F, T, T],
        [T, T, T, F, T, T, T],
        [F, F, T, T, T, T, T],
        [T, F, F, T, T, T, F],
        [F, T, T, T, T, F, T],
        [T, F, F, T, T, T, T],
        [T, F, F, F, T, T, T],
    ]
};

/// Segment patterns of the 14 segment decoder, one string of 0/1 per digit (from the original).
const SYMBOLS14: [&str; 16] = [
    "11111100100010",
    "01100000100000",
    "11011000010001",
    "11110000010001",
    "01100100010001",
    "10110100010001",
    "10111100010001",
    "10000000100100",
    "11111100010001",
    "11110100010001",
    "11101100010001",
    "11110001010100",
    "10011100000000",
    "11110001000100",
    "10011100010001",
    "10001100010001",
];

/// Segment patterns of the 16 segment decoder.
const SYMBOLS16: [&str; 16] = [
    "1111111100100010",
    "0011000000100000",
    "1110111000010001",
    "1111110000010001",
    "0011000100010001",
    "1101110100010001",
    "1101111100010001",
    "1100000000100100",
    "1111111100010001",
    "1111110100010001",
    "1111001100010001",
    "1111110001010100",
    "1100111100000000",
    "1111110001000100",
    "1100111100010001",
    "1100001100010001",
];

fn pattern<const N: usize>(table: &[&str; 16], digit: usize) -> [bool; N] {
    let mut out = [false; N];
    for (o, c) in out.iter_mut().zip(table[digit].chars()) {
        *o = c == '1';
    }
    out
}

/// Turns a four-bit code into the segments of a display.
#[derive(Clone, Debug, Default)]
pub struct SevenSegDecoder {
    segment_type: i32,
}

impl SevenSegDecoder {
    const FLAG_ENABLE: i32 = 1 << 1;
    const FLAG_BLANK_F: i32 = 1 << 2;

    fn segments(&self) -> usize {
        match self.segment_type {
            1 => 14,
            2 => 16,
            _ => 7,
        }
    }
    fn has_blank(c: &ChipCore) -> bool {
        c.base.flags & Self::FLAG_ENABLE != 0
    }
}

impl ChipLogic for SevenSegDecoder {
    fn class_name(&self) -> &'static str {
        "SevenSegDecoderElm"
    }
    fn dump_type(&self) -> i32 {
        197
    }
    fn chip_name(&self) -> &'static str {
        match self.segments() {
            7 => "7-Segment Decoder",
            14 => "14-Segment Decoder",
            _ => "16-Segment Decoder",
        }
    }
    fn allow_bus(&self) -> bool {
        true
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let seg = self.segments();
        c.bits = 4;
        c.size_x = 3;
        let input_y = if c.use_bus() { 1 } else { 4 };
        let output_y = if c.use_bus() { 1 } else { seg as i32 };
        let blank = Self::has_blank(c);
        c.size_y = output_y.max(input_y + i32::from(blank));
        fill(c, seg + 4 + usize::from(blank));
        c.make_bit_pins(4, 0, SIDE_W, seg, "I", false, false, true);
        if c.use_bus() {
            c.make_bit_pins(seg, 0, SIDE_E, 0, "seg", true, false, false);
        } else {
            for i in 0..seg {
                c.pins[i] = Pin::output(i as i32, SIDE_E, &char::from(b'a' + i as u8).to_string());
            }
        }
        if blank {
            c.pins[seg + 4] = Pin::new(input_y, SIDE_W, "BI");
            c.pins[seg + 4].bubble = true;
        }
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Ok(t) = st.next_i32() {
            self.segment_type = t;
            self.setup_pins(c);
            c.base.alloc_nodes(c.pins.len());
        }
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {}", self.segment_type)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.segment_type = xml.int_attr("sgt", self.segment_type);
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("sgt", self.segment_type);
    }
    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let seg = self.segments();
        let bit = |c: &ChipCore, k: usize| usize::from(c.pins[seg + k].value);
        let input = bit(c, 0) * 8 + bit(c, 1) * 4 + bit(c, 2) * 2 + bit(c, 3);
        let enabled = !Self::has_blank(c) || c.pins[seg + 4].value;
        let blank_f = c.base.flags & Self::FLAG_BLANK_F != 0;
        if !enabled || (input == 15 && blank_f) {
            for i in 0..seg {
                c.write_output(i, false);
            }
            return;
        }
        for i in 0..seg {
            let on = match seg {
                14 => pattern::<14>(&SYMBOLS14, input)[i],
                16 => pattern::<16>(&SYMBOLS16, input)[i],
                _ => SYMBOLS7[input][i],
            };
            c.write_output(i, on);
        }
    }
    fn chip_edit_info(&self, c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(choice(
                "Segments",
                &["7 Segment", "14 Segment", "16 Segment"],
                self.segment_type.clamp(0, 2) as usize,
            )),
            1 => Some(EditInfo::checkbox("Blank Pin", Self::has_blank(c))),
            2 => Some(EditInfo::checkbox("Blank on 1111", c.base.flags & Self::FLAG_BLANK_F != 0)),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let on = ei.checkbox.unwrap_or(false);
        let set = |c: &mut ChipCore, f: i32| {
            if on {
                c.base.flags |= f;
            } else {
                c.base.flags &= !f;
            }
        };
        match n {
            0 => self.segment_type = ei.choice_index as i32,
            1 => set(c, Self::FLAG_ENABLE),
            2 => set(c, Self::FLAG_BLANK_F),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Decimal display
// ---------------------------------------------------------------------------------------------

/// Shows the value on its input pins as a number.
#[derive(Clone, Debug)]
pub struct DecimalDisplay {
    bit_count: i32,
    display_mode: i32,
}

impl DecimalDisplay {
    fn new() -> Self {
        DecimalDisplay { bit_count: 4, display_mode: 0 }
    }
}

impl ChipLogic for DecimalDisplay {
    fn class_name(&self) -> &'static str {
        "DecimalDisplayElm"
    }
    fn dump_type(&self) -> i32 {
        419
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("dd")
    }
    fn chip_name(&self) -> &'static str {
        match self.display_mode {
            1 => "hex display",
            2 => "octal display",
            _ => "decimal display",
        }
    }
    fn allow_bus(&self) -> bool {
        true
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        0
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        let n = self.bit_count.clamp(1, 16) as usize;
        c.size_x = 3;
        c.size_y = if c.use_bus() { 2 } else { n as i32 };
        fill(c, n);
        c.make_bit_pins(n, 0, SIDE_W, 0, "I", false, false, false);
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Ok(b) = st.next_i32() {
            self.bit_count = b;
            if let Ok(m) = st.next_i32() {
                self.display_mode = m;
            }
        }
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len());
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {} {}", self.bit_count, self.display_mode)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.bit_count = xml.int_attr("bc", self.bit_count);
        self.display_mode = xml.int_attr("dm", self.display_mode);
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("bc", self.bit_count);
        out.int_attr("dm", self.display_mode);
    }

    fn draw_label(&self, g: &mut dyn Painter, c: &mut ChipCore, ctx: &DrawCtx, _x: i32, _y: i32) {
        let cspc = c.cspc;
        let xl = c.base.x + cspc + c.flipped_size_x * cspc;
        let mut yl = c.base.y - cspc + c.flipped_size_y * cspc;
        if c.is_flipped_xy() {
            yl += if c.base.flags & FLAG_FLIP_Y != 0 { -cspc / 2 } else { cspc / 2 };
        }
        g.save();
        g.set_font_size(15 * c.csize);
        g.set_color(ctx.white);
        g.set_text_baseline(TextBaseline::Middle);
        let n = self.bit_count.clamp(1, 16) as usize;
        let value: u32 = (0..n).filter(|&i| value(c, i)).fold(0, |v, i| v | (1 << i));
        let text = match self.display_mode {
            1 => format!("{value:X}"),
            2 => format!("{value:o}"),
            _ => value.to_string(),
        };
        let w = g.measure_text(&text) as i32;
        g.draw_string(&text, f64::from(xl + 5 * c.csize - w / 2), f64::from(yl));
        g.restore();
    }

    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("# of Bits", f64::from(self.bit_count), 1.0, 8.0).set_dimensionless(),
            ),
            1 => Some(choice(
                "Display Mode",
                &["Decimal", "Hexadecimal", "Octal"],
                self.display_mode.clamp(0, 2) as usize,
            )),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                if (1.0..=16.0).contains(&ei.value) {
                    self.bit_count = ei.value as i32;
                } else {
                    ei.error = Some("must be between 1 and 16".into());
                }
            }
            1 => self.display_mode = ei.choice_index as i32,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// LED array
// ---------------------------------------------------------------------------------------------

const BRIGHTNESS_TAU: f64 = 0.03;

/// A matrix of LEDs: columns on the bottom pins, rows on the left pins.
#[derive(Clone, Debug)]
pub struct LedArray {
    cols: usize,
    rows: usize,
    diodes: Vec<Diode>,
    currents: Vec<f64>,
    brightness: Vec<f64>,
    last_draw_time: f64,
}

impl LedArray {
    fn new() -> Self {
        LedArray {
            cols: 8,
            rows: 8,
            diodes: Vec::new(),
            currents: Vec::new(),
            brightness: Vec::new(),
            last_draw_time: 0.0,
        }
    }
}

impl ChipLogic for LedArray {
    fn class_name(&self) -> &'static str {
        "LEDArrayElm"
    }
    fn dump_type(&self) -> i32 {
        405
    }
    fn chip_name(&self) -> &'static str {
        "LED array"
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
        c.size_x = self.cols as i32;
        c.size_y = self.rows as i32;
        fill(c, self.cols + self.rows);
        for i in 0..self.cols {
            c.pins[i] = Pin::new(i as i32, SIDE_S, "");
        }
        for i in 0..self.rows {
            c.pins[i + self.cols] = Pin::new(i as i32, SIDE_W, "");
        }
        self.brightness = vec![0.0; self.cols * self.rows];
        self.currents = vec![0.0; self.cols * self.rows];
    }
    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let (Ok(x), Ok(y)) = (st.next_i32(), st.next_i32()) {
            self.cols = x.clamp(2, 16) as usize;
            self.rows = y.clamp(2, 16) as usize;
        }
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len());
    }
    fn write_extra(&self, _c: &ChipCore) -> String {
        format!(" {} {}", self.cols, self.rows)
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        self.cols = xml.int_attr("sx", self.cols as i32).clamp(2, 16) as usize;
        self.rows = xml.int_attr("sy", self.rows as i32).clamp(2, 16) as usize;
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.int_attr("sx", self.cols as i32);
        out.int_attr("sy", self.rows as i32);
    }
    fn on_reset(&mut self, _c: &mut ChipCore) {
        self.brightness.fill(0.0);
    }
    fn matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn ground_connection(&self, _c: &ChipCore, _n: usize) -> bool {
        false
    }

    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let model = DiodeModel::lookup("default-led").unwrap_or_else(DiodeModel::default_model);
        self.diodes = (0..self.cols * self.rows)
            .map(|i| {
                let mut d = Diode::default();
                d.setup(&model);
                let n = &c.base.nodes;
                d.stamp(s, n[self.cols + i / self.cols], n[i % self.cols]);
                d
            })
            .collect();
        self.currents = vec![0.0; self.diodes.len()];
        true
    }

    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let v = &c.base.volts;
        let (cols, rows) = (self.cols, self.rows);
        for iy in 0..rows {
            for ix in 0..cols {
                self.diodes[iy * cols + ix].do_step(s, v[cols + iy] - v[ix]);
            }
        }
        true
    }

    fn calculate_current(&mut self, c: &mut ChipCore) {
        let (cols, rows) = (self.cols, self.rows);
        for p in &mut c.pins[..cols] {
            p.current = 0.0;
        }
        if self.diodes.len() != cols * rows {
            return;
        }
        for iy in 0..rows {
            let mut cur = 0.0;
            for ix in 0..cols {
                let i = iy * cols + ix;
                let this =
                    self.diodes[i].calculate_current(c.base.volts[cols + iy] - c.base.volts[ix]);
                self.currents[i] = this;
                cur += this;
                c.pins[ix].current += this;
            }
            c.pins[iy + cols].current = -cur;
        }
    }

    fn step_finished(&mut self, _c: &mut ChipCore, s: &mut Stamper) {
        if self.currents.iter().any(|i| i.abs() > 1e12) {
            s.stop("max current exceeded", None);
        }
    }

    fn visual_state(&self, _c: &ChipCore) -> Vec<f64> {
        self.currents.clone()
    }
    fn set_visual_state(&mut self, _c: &mut ChipCore, state: &[f64]) {
        if state.len() == self.cols * self.rows {
            self.currents = state.to_vec();
        }
    }

    fn draw_label(
        &self,
        _g: &mut dyn Painter,
        _c: &mut ChipCore,
        _ctx: &DrawCtx,
        _x: i32,
        _y: i32,
    ) {
    }

    fn custom_draw(&mut self, g: &mut dyn Painter, c: &mut ChipCore, ctx: &DrawCtx) -> bool {
        let elapsed = ctx.sim_time - self.last_draw_time;
        self.last_draw_time = ctx.sim_time;
        let decay = if elapsed > 0.0 { (-elapsed / BRIGHTNESS_TAU).exp() } else { 1.0 };
        let (cols, rows) = (self.cols, self.rows);
        let r = c.cspc;
        let flipped = c.is_flipped_xy();
        for ix in 0..cols {
            for iy in 0..rows {
                let i = ix + iy * cols;
                let level = led_level(self.currents.get(i).copied().unwrap_or(0.0), 20.0);
                let level = level.max(self.brightness[i]);
                self.brightness[i] = level * decay;
                g.set_color(Color::new(level as u8, 0, 0));
                let (cx, cy) = if flipped {
                    (c.pins[iy + cols].post().x, c.pins[ix].post().y)
                } else {
                    (c.pins[ix].post().x, c.pins[iy + cols].post().y)
                };
                g.fill_oval(cx - r / 2, cy - r / 2, r, r);
            }
        }
        false
    }

    fn info_extra(&self, _c: &ChipCore, _arr: &mut Vec<String>) {}

    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::number("Grid Width", self.cols as f64).set_dimensionless()),
            1 => Some(EditInfo::number("Grid Height", self.rows as f64).set_dimensionless()),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        if n > 1 {
            return;
        }
        if (2.0..=16.0).contains(&ei.value) {
            if n == 0 {
                self.cols = ei.value as usize;
            } else {
                self.rows = ei.value as usize;
            }
        } else {
            ei.error = Some("must be between 2 and 16".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_shows_the_digits() {
        let mut d = ChipElm::new(SevenSegDecoder::default(), 0, 0);
        // inputs are pins 7..11, most significant first: 0101 = 5
        for (k, bit) in [false, true, false, true].iter().enumerate() {
            d.core.pins[7 + k].value = *bit;
        }
        d.logic.execute(&mut d.core, &Stamper::default());
        let segs: Vec<bool> = (0..7).map(|i| d.core.pins[i].value).collect();
        assert_eq!(segs, SYMBOLS7[5].to_vec());
    }

    #[test]
    fn segment_patterns_have_the_right_lengths() {
        assert!(SYMBOLS14.iter().all(|p| p.len() == 14));
        assert!(SYMBOLS16.iter().all(|p| p.len() == 16));
        assert_eq!(pattern::<14>(&SYMBOLS14, 1).iter().filter(|b| **b).count(), 3);
    }

    #[test]
    fn display_pin_counts_follow_the_options() {
        let mut s = SevenSeg::new();
        s.extra = ES_DP;
        s.diode_direction = 1;
        s.set_pin_count();
        assert_eq!((s.segments, s.pin_count, s.common_pin), (8, 9, Some(8)));
        let chip = ChipElm::new(SevenSeg::new(), 0, 0);
        assert_eq!(chip.core.pins.len(), 7);
        assert!(chip.dump().ends_with(" 7 0 0"), "{}", chip.dump());
    }

    #[test]
    fn decimal_display_reads_its_bits() {
        let text = "8 1";
        let mut chip =
            ChipElm::from_dump(DecimalDisplay::new(), 0, 0, 32, 0, 0, &mut Tokenizer::new(text));
        assert_eq!(chip.core.pins.len(), 8);
        assert_eq!(chip.logic.display_mode, 1);
        chip.core.base.volts[0] = 5.0;
        chip.core.base.volts[3] = 5.0;
        assert!(value(&chip.core, 3));
        assert_eq!(chip.dump().rsplit(' ').take(2).collect::<Vec<_>>(), ["1", "8"]);
    }

    #[test]
    fn led_array_has_a_pin_per_row_and_column() {
        let chip = ChipElm::new(LedArray::new(), 0, 0);
        assert_eq!(chip.core.pins.len(), 16);
        assert!(chip.dump().ends_with(" 8 8"));
    }
}
