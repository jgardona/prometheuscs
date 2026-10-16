// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (SweepElm).
//! Frequency sweep source: a sine wave whose frequency ramps between two limits.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::{TokenError, Tokenizer};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::Painter;
use crate::solver::stamper::{NodeId, Stamper, VsId};
use std::f64::consts::PI;

const FAMILY: &str = "sources";
const FLAG_LOG: i32 = 1;
const FLAG_BIDIR: i32 = 2;
const CIRCLE_SIZE: i32 = 17;

/// Registers the sweep source.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "SweepElm",
            FAMILY,
            "Add Frequency Sweep",
            "Inputs and Sources",
            |x, y| Box::new(SweepElm::new(x, y)),
        )
        .dump(170, |a, b, c, d, f, st| Ok(Box::new(SweepElm::from_dump(a, b, c, d, f, st)?))),
    );
}

/// A source sweeping its frequency over time.
#[derive(Clone, Debug)]
pub struct SweepElm {
    base: Base,
    /// Lowest frequency.
    pub min_f: f64,
    /// Highest frequency.
    pub max_f: f64,
    /// Amplitude.
    pub max_v: f64,
    /// Seconds for one sweep.
    pub sweep_time: f64,
    frequency: f64,
    fadd: f64,
    fmul: f64,
    freq_time: f64,
    saved_time_step: f64,
    dir: i32,
    v: f64,
}

impl SweepElm {
    fn blank(base: Base) -> Self {
        SweepElm {
            base,
            min_f: 20.0,
            max_f: 4000.0,
            max_v: 5.0,
            sweep_time: 0.1,
            frequency: 20.0,
            fadd: 0.0,
            fmul: 1.0,
            freq_time: 0.0,
            saved_time_step: 0.0,
            dir: 1,
            v: 0.0,
        }
    }

    /// A sweep source being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut base = Base::new(xx, yy);
        base.flags = FLAG_BIDIR;
        SweepElm::blank(base)
    }

    /// A sweep source read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut s = SweepElm::blank(Base::from_dump(xa, ya, xb, yb, f));
        s.min_f = st.next_f64()?;
        s.max_f = st.next_f64()?;
        s.max_v = st.next_f64()?;
        s.sweep_time = st.next_f64()?;
        s.restart();
        Ok(s)
    }

    fn restart(&mut self) {
        self.frequency = self.min_f;
        self.freq_time = 0.0;
        self.dir = 1;
        self.saved_time_step = 0.0; // recompute the ramp at the next step
    }

    fn set_params(&mut self, time_step: f64) {
        if self.frequency < self.min_f || self.frequency > self.max_f {
            self.frequency = self.min_f;
            self.freq_time = 0.0;
            self.dir = 1;
        }
        let dir = f64::from(self.dir);
        if self.base.flags & FLAG_LOG == 0 {
            self.fadd = dir * time_step * (self.max_f - self.min_f) / self.sweep_time;
            self.fmul = 1.0;
        } else {
            self.fadd = 0.0;
            self.fmul = (self.max_f / self.min_f).powf(dir * time_step / self.sweep_time);
        }
        self.saved_time_step = time_step;
    }
}

impl Element for SweepElm {
    impl_element_base!("SweepElm");

    fn dump_type(&self) -> i32 {
        170
    }
    fn xml_dump_type(&self) -> String {
        "sw".into()
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(170),
            js_number(self.min_f),
            js_number(self.max_f),
            js_number(self.max_v),
            js_number(self.sweep_time)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.min_f = xml.double_attr("mi", self.min_f);
        self.max_f = xml.double_attr("ma", self.max_f);
        self.max_v = xml.double_attr("mv", self.max_v);
        self.sweep_time = xml.double_attr("sw", self.sweep_time);
        self.restart();
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("mi", self.min_f);
        out.double_attr("ma", self.max_f);
        out.double_attr("mv", self.max_v);
        out.double_attr("sw", self.sweep_time);
    }

    fn post_count(&self) -> usize {
        1
    }
    fn voltage_source_count(&self) -> usize {
        1
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &mut self.base;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - f64::from(CIRCLE_SIZE) / b.dn);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(CIRCLE_SIZE));
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        draw_thick_line(g, p1, self.base.lead1);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        let (xc, yc) = (p2.x, p2.y);
        draw_thick_circle(g, xc, yc, CIRCLE_SIZE);
        self.base.adjust_bbox(
            xc - CIRCLE_SIZE,
            yc - CIRCLE_SIZE,
            xc + CIRCLE_SIZE,
            yc + CIRCLE_SIZE,
        );
        let wl = 8.0;
        let xl = 10;
        let mut w = {
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() % 2000) as f64;
            1.0 + (if ms > 1000.0 { 2000.0 - ms } else { ms }) * 0.002
        };
        if c.sim_running {
            w = 1.0 + 2.0 * (self.frequency - self.min_f) / (self.max_f - self.min_f);
        }
        g.begin_path();
        g.set_line_width(3.0);
        for i in -xl..=xl {
            let yy =
                f64::from(yc) + (0.95 * (f64::from(i) * PI * w / f64::from(xl)).sin() * wl).trunc();
            if i == -xl {
                g.move_to(f64::from(xc + i), yy);
            } else {
                g.line_to(f64::from(xc + i), yy);
            }
        }
        g.stroke();
        g.set_line_width(1.0);
        if c.show_values && (self.base.dx == 0 || self.base.dy == 0) {
            let s = short_unit_text(self.frequency, "Hz");
            draw_values(g, &self.base, c, Some(&s), f64::from(CIRCLE_SIZE), false, false);
        }
        draw_posts(g, &self.base, self, c);
        self.base.curcount =
            self.base.update_dot_count_for(-self.base.current, self.base.curcount, c);
        if !self.base.creating {
            draw_dots(g, c, p1, self.base.lead1, self.base.curcount);
        }
    }

    fn reset(&mut self) {
        self.base.volts.fill(0.0);
        self.base.curcount = 0.0;
        self.restart();
    }

    fn validate(&mut self, idx: usize, v: &mut crate::solver::path::Validator<'_>) -> bool {
        validate_rail_node(v, idx, self.base.nodes[0])
    }

    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base.volt_source = Some(v);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.base.volt_source {
            s.stamp_voltage_source_var(NodeId::GROUND, self.base.nodes[0], vs);
        }
    }

    fn start_iteration(&mut self, s: &Stamper) {
        if (s.time_step - self.saved_time_step).abs() > 0.0 {
            self.set_params(s.time_step);
        }
        self.v = self.freq_time.sin() * self.max_v;
        self.freq_time += self.frequency * 2.0 * PI * s.time_step;
        self.frequency = self.frequency * self.fmul + self.fadd;
        if self.frequency >= self.max_f && self.dir == 1 {
            if self.base.flags & FLAG_BIDIR != 0 {
                self.fadd = -self.fadd;
                self.fmul = 1.0 / self.fmul;
                self.dir = -1;
            } else {
                self.frequency = self.min_f;
            }
        }
        if self.frequency <= self.min_f && self.dir == -1 {
            self.fadd = -self.fadd;
            self.fmul = 1.0 / self.fmul;
            self.dir = 1;
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.base.volt_source {
            s.update_voltage_source(vs, self.v);
        }
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_power(&self) -> f64 {
        -self.base.volts[0] * self.base.current
    }
    fn has_ground_connection(&self, _n1: usize) -> bool {
        true
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let kind = if self.base.flags & FLAG_LOG == 0 { "(linear)" } else { "(log)" };
        arr.push(format!("sweep {kind}"));
        arr.push(format!("I = {}", current_d_text(self.get_current())));
        arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
        arr.push(format!("f = {}", unit_text(self.frequency, "Hz")));
        arr.push(format!(
            "range = {} .. {}",
            unit_text(self.min_f, "Hz"),
            unit_text(self.max_f, "Hz")
        ));
        arr.push(format!("time = {}", unit_text(self.sweep_time, "s")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Min Frequency (Hz)", self.min_f, 0.0, 0.0)),
            1 => Some(EditInfo::new("Max Frequency (Hz)", self.max_f, 0.0, 0.0)),
            2 => Some(EditInfo::new("Sweep Time (s)", self.sweep_time, 0.0, 0.0)),
            3 => Some(EditInfo::checkbox("Logarithmic", self.base.flags & FLAG_LOG != 0)),
            4 => Some(EditInfo::new("Max Voltage", self.max_v, 0.0, 0.0).set_unit_step()),
            5 => Some(EditInfo::checkbox("Bidirectional", self.base.flags & FLAG_BIDIR != 0)),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.min_f = ei.value,
            1 => self.max_f = ei.value,
            2 => self.sweep_time = ei.value,
            3 => self.base.flags = ei.change_flag(self.base.flags, FLAG_LOG),
            4 => self.max_v = ei.value,
            5 => self.base.flags = ei.change_flag(self.base.flags, FLAG_BIDIR),
            _ => {}
        }
        self.saved_time_step = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_ramps_up_and_back() {
        let mut s = SweepElm::new(0, 0);
        s.min_f = 100.0;
        s.max_f = 200.0;
        s.sweep_time = 0.01;
        s.restart();
        let st = Stamper { time_step: 1e-4, ..Default::default() };
        let mut peak: f64 = 0.0;
        for _ in 0..150 {
            s.start_iteration(&st);
            peak = peak.max(s.frequency);
        }
        assert!(
            peak >= 199.0 && s.frequency < peak,
            "turned around: peak {peak}, now {}",
            s.frequency
        );
    }
}
