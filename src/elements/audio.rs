// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (AudioOutputElm).
//! The audio output: records the voltage of its post as audio samples.
//!
//! The element only records. The samples are kept in a ring buffer holding the last
//! `duration` seconds; [`AudioOutput::samples`] returns them in time order and [`wav_bytes`]
//! makes a WAV file from them, which is what a player needs.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Color, Painter};
use crate::solver::stamper::Stamper;
use std::cell::Cell;

/// Family name used for the feature switch.
pub const FAMILY: &str = "outputs";

/// Sampling rates the dialog offers.
pub const SAMPLING_RATES: [i32; 6] = [8000, 11025, 16000, 22050, 44100, 48000];

thread_local! {
    static LAST_RATE: Cell<i32> = const { Cell::new(8000) };
    static NEXT_LABEL: Cell<i32> = const { Cell::new(1) };
}

/// Makes a 16-bit mono WAV file from samples in volts (clipped to the range of -1 to 1,
/// as the original does before scaling).
pub fn wav_bytes(samples: &[f64], rate: i32) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut v = Vec::with_capacity(44 + data_len);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&1u16.to_le_bytes()); // mono
    v.extend_from_slice(&(rate as u32).to_le_bytes());
    v.extend_from_slice(&((rate as u32) * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        let q = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        v.extend_from_slice(&q.to_le_bytes());
    }
    v
}

/// An element with one post that records its voltage as audio.
pub struct AudioOutput {
    base: Base,
    duration: f64,
    sampling_rate: i32,
    label_num: i32,
    data: Vec<f64>,
    ptr: usize,
    full: bool,
    sample_step: f64,
    next_sample: Option<f64>,
    sum: f64,
    count: u32,
    start: f64,
    last_time: f64,
}

impl AudioOutput {
    fn blank(base: Base, duration: f64, rate: i32, label: i32) -> Self {
        let mut a = AudioOutput {
            base,
            duration,
            sampling_rate: rate,
            label_num: label,
            data: Vec::new(),
            ptr: 0,
            full: false,
            sample_step: 0.0,
            next_sample: None,
            sum: 0.0,
            count: 0,
            start: 0.0,
            last_time: 0.0,
        };
        a.set_data_count();
        a
    }

    /// An audio output being placed.
    pub fn new(x: i32, y: i32) -> Self {
        // the registry's probe at the origin must not use up a label number
        let label = if x == 0 && y == 0 {
            1
        } else {
            NEXT_LABEL.with(|n| {
                let v = n.get();
                n.set(v + 1);
                v
            })
        };
        Self::blank(Base::new(x, y), 1.0, LAST_RATE.with(Cell::get), label)
    }

    /// An audio output read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let duration = st.next_f64().unwrap_or(1.0);
        let rate = st.next_i32().unwrap_or(8000);
        let label = st.next_i32().unwrap_or(1);
        NEXT_LABEL.with(|n| n.set(n.get().max(label + 1)));
        Self::blank(Base::from_dump(xa, ya, xb, yb, f), duration, rate, label)
    }

    fn set_data_count(&mut self) {
        let rate = self.sampling_rate.max(1);
        let count = ((f64::from(rate) * self.duration) as usize).clamp(1, 48000 * 5);
        self.data = vec![0.0; count];
        self.ptr = 0;
        self.full = false;
        self.sample_step = 1.0 / f64::from(rate);
        self.next_sample = None;
        self.sum = 0.0;
        self.count = 0;
    }

    /// The recorded samples, oldest first.
    pub fn samples(&self) -> Vec<f64> {
        if self.full {
            let mut v = self.data[self.ptr..].to_vec();
            v.extend_from_slice(&self.data[..self.ptr]);
            v
        } else {
            self.data[..self.ptr].to_vec()
        }
    }

    /// Samples per second.
    pub fn sampling_rate(&self) -> i32 {
        self.sampling_rate
    }

    /// The name the element and its play button carry.
    pub fn label(&self) -> String {
        if self.label_num > 1 { format!("Audio {}", self.label_num) } else { "Audio Out".into() }
    }
}

impl Element for AudioOutput {
    impl_element_base!("AudioOutputElm");

    fn dump_type(&self) -> i32 {
        211
    }
    fn xml_dump_type(&self) -> String {
        "aout".into()
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {}",
            self.base.dump_prefix(211),
            js_number(self.duration),
            self.sampling_rate,
            self.label_num
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("du", self.duration);
        out.int_attr("sa", self.sampling_rate);
        out.int_attr("la", self.label_num);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.duration = xml.double_attr("du", self.duration);
        self.sampling_rate = xml.int_attr("sa", self.sampling_rate);
        self.label_num = xml.int_attr("la", self.label_num);
        self.set_data_count();
    }

    fn post_count(&self) -> usize {
        1
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.ptr = 0;
        self.full = false;
        self.next_sample = None;
        self.sum = 0.0;
        self.count = 0;
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        let t = s.t;
        if self.next_sample.is_none() {
            self.start = t;
            self.next_sample = Some(t + self.sample_step);
        }
        self.last_time = t;
        self.sum += self.base.volts[0];
        self.count += 1;
        if let Some(next) = self.next_sample {
            if t >= next {
                self.next_sample = Some(next + self.sample_step);
                self.data[self.ptr] = self.sum / f64::from(self.count);
                self.ptr += 1;
                self.sum = 0.0;
                self.count = 0;
                if self.ptr >= self.data.len() {
                    self.ptr = 0;
                    self.full = true;
                }
            }
        }
    }

    // what the drawing needs: how full the buffer is
    fn visual_state(&self) -> Vec<f64> {
        vec![self.ptr as f64, if self.full { 1.0 } else { 0.0 }, self.data.len() as f64]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [ptr, full, len] = state[..] {
            if len as usize == self.data.len() {
                self.ptr = ptr as usize;
                self.full = full != 0.0;
            }
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        let selected = self.base.needs_highlight();
        g.set_font_size(14);
        let s = self.label();
        let tw = g.measure_text(&s) as i32;
        g.set_color(Color::DARK_GRAY);
        let pct = if self.full { tw } else { tw * self.ptr as i32 / self.data.len().max(1) as i32 };
        let (x2, y2) = (self.base.x2, self.base.y2);
        g.fill_rect(f64::from(x2 - tw / 2), f64::from(y2 - 10), f64::from(pct), 20.0);
        g.set_color(if selected { c.select_color } else { c.white });
        let dn = self.base.dn;
        self.base.lead1 = interp_point(
            self.base.point1,
            self.base.point2,
            1.0 - (f64::from(tw) / 2.0 + 8.0) / dn,
        );
        let (p1, l1) = (self.base.point1, self.base.lead1);
        self.base.set_bbox_pts(p1, l1, 0.0);
        draw_centered_text(g, &mut self.base, &s, x2, y2, true);
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        if selected {
            g.set_color(c.select_color);
        }
        draw_thick_line(g, self.base.point1, self.base.lead1);
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("audio output".into());
        arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
        let ct = if self.full { self.data.len() } else { self.ptr };
        let start = if self.full { self.last_time - self.duration } else { self.start };
        arr.push(format!("start = {}", unit_text(start, "s")));
        arr.push(format!("dur = {}", unit_text(self.sample_step * ct as f64, "s")));
        let total = if self.full { String::new() } else { format!("/{}", self.data.len()) };
        arr.push(format!("samples = {ct}{total}"));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Duration (s)", self.duration, 0.0, 5.0).set_positive()),
            1 => {
                let names = SAMPLING_RATES.iter().map(ToString::to_string).collect();
                let sel = SAMPLING_RATES.iter().position(|r| *r == self.sampling_rate);
                Some(EditInfo::choice("Sampling Rate", names, sel.unwrap_or(0)))
            }
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 if ei.value > 0.0 => {
                self.duration = ei.value;
                self.set_data_count();
            }
            1 => {
                if let Some(&rate) = SAMPLING_RATES.get(ei.choice_index) {
                    if rate != self.sampling_rate {
                        self.sampling_rate = rate;
                        LAST_RATE.with(|r| r.set(rate));
                        self.set_data_count();
                    }
                }
            }
            _ => {}
        }
    }
}

/// Registers the audio output.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "AudioOutputElm",
            FAMILY,
            "Add Audio Output",
            "Outputs and Labels",
            |x, y| Box::new(AudioOutput::new(x, y)),
        )
        .dump(211, |a, b, c, d, f, st| Ok(Box::new(AudioOutput::from_dump(a, b, c, d, f, st)))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_and_samples_are_well_formed() {
        let w = wav_bytes(&[0.0, 0.5, -2.0], 8000);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()), 36 + 6);
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 8000);
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(i16::from_le_bytes(w[48..50].try_into().unwrap()), -32767);
    }

    #[test]
    fn the_buffer_returns_samples_oldest_first_once_full() {
        let mut a = AudioOutput::new(0, 0);
        a.data = vec![3.0, 4.0, 1.0, 2.0];
        a.ptr = 2;
        a.full = true;
        assert_eq!(a.samples(), vec![1.0, 2.0, 3.0, 4.0]);
        a.full = false;
        assert_eq!(a.samples(), vec![3.0, 4.0]);
    }
}
