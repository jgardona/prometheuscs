// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (AMElm, FMElm, FuseElm, LDRElm,
// ThermistorNTCElm).
//! Modulated sources, the fuse, the photoresistor and the NTC thermistor.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale;
use crate::render::painter::{Color, Painter};
use crate::solver::path::Validator;
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name used for the feature switch.
pub const FAMILY: &str = "analog";
const SOURCES: &str = "Inputs and Sources";
const PASSIVE: &str = "Passive Components";

/// Registers the elements of this module.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("AMElm", FAMILY, "Add AM Source", SOURCES, |x, y| {
            Box::new(ModSource::new(false, x, y))
        })
        .dump(200, |a, b, c, d, f, st| {
            Ok(Box::new(ModSource::from_dump(false, a, b, c, d, f, st)))
        }),
    );
    r.add(
        ElementDescriptor::new("FMElm", FAMILY, "Add FM Source", SOURCES, |x, y| {
            Box::new(ModSource::new(true, x, y))
        })
        .dump(201, |a, b, c, d, f, st| Ok(Box::new(ModSource::from_dump(true, a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("FuseElm", FAMILY, "Add Fuse", PASSIVE, |x, y| {
            Box::new(Fuse::new(x, y))
        })
        .dump(404, |a, b, c, d, f, st| Ok(Box::new(Fuse::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("LDRElm", FAMILY, "Add Photoresistor", PASSIVE, |x, y| {
            Box::new(Sensor::new_ldr(x, y))
        })
        .dump(374, |a, b, c, d, f, st| Ok(Box::new(Sensor::from_dump_ldr(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new(
            "ThermistorNTCElm",
            FAMILY,
            "Add Thermistor (NTC)",
            PASSIVE,
            |x, y| Box::new(Sensor::new_ntc(x, y)),
        )
        .dump(350, |a, b, c, d, f, st| Ok(Box::new(Sensor::from_dump_ntc(a, b, c, d, f, st)))),
    );
}

// ---------------------------------------------------------------------------------------------
// AM and FM sources
// ---------------------------------------------------------------------------------------------

const CIRCLE: i32 = 17;

/// A one-terminal source of an amplitude- or frequency-modulated sine.
pub struct ModSource {
    base: Base,
    fm: bool,
    carrier: f64,
    signal: f64,
    max_voltage: f64,
    deviation: f64,
    last_time: f64,
    phase: f64,
    source: Option<VsId>,
}

impl ModSource {
    fn blank(base: Base, fm: bool) -> Self {
        ModSource {
            base,
            fm,
            carrier: if fm { 800.0 } else { 1000.0 },
            signal: 40.0,
            max_voltage: 5.0,
            deviation: 200.0,
            last_time: 0.0,
            phase: 0.0,
            source: None,
        }
    }

    /// A source being placed (`fm` selects the frequency-modulated one).
    pub fn new(fm: bool, x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y), fm)
    }

    /// A source read from a file.
    pub fn from_dump(
        fm: bool,
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut s = Self::blank(Base::from_dump(xa, ya, xb, yb, f), fm);
        s.carrier = st.next_f64().unwrap_or(s.carrier);
        s.signal = st.next_f64().unwrap_or(s.signal);
        s.max_voltage = st.next_f64().unwrap_or(s.max_voltage);
        if fm {
            s.deviation = st.next_f64().unwrap_or(s.deviation);
        }
        // the cosine flag of old files means nothing here
        s.base.flags &= !2;
        s
    }

    fn voltage(&mut self, t: f64) -> f64 {
        let w = std::f64::consts::TAU * t;
        if self.fm {
            // the phase is integrated, so it only advances when time does
            let dt = t - self.last_time;
            self.last_time = t;
            let amp = (w * self.signal).sin();
            self.phase += dt * (self.carrier + amp * self.deviation);
            (std::f64::consts::TAU * self.phase).sin() * self.max_voltage
        } else {
            (((w * self.signal).sin() + 1.0) / 2.0) * (w * self.carrier).sin() * self.max_voltage
        }
    }
}

impl Element for ModSource {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        if self.fm { "FMElm" } else { "AMElm" }
    }
    fn dump_type(&self) -> i32 {
        if self.fm { 201 } else { 200 }
    }
    fn dump(&self) -> String {
        let mut s = format!(
            "{} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.carrier),
            js_number(self.signal),
            js_number(self.max_voltage)
        );
        if self.fm {
            s += &format!(" {}", js_number(self.deviation));
        }
        s
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("cf", self.carrier);
        out.double_attr("sf", self.signal);
        out.double_attr("mv", self.max_voltage);
        if self.fm {
            out.double_attr("dv", self.deviation);
        }
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.carrier = xml.double_attr("cf", self.carrier);
        self.signal = xml.double_attr("sf", self.signal);
        self.max_voltage = xml.double_attr("mv", self.max_voltage);
        self.deviation = xml.double_attr("dv", self.deviation);
    }

    fn post_count(&self) -> usize {
        1
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.source = Some(v);
    }
    fn has_ground_connection(&self, _n1: usize) -> bool {
        true
    }
    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        validate_rail_node(v, idx, self.base.nodes[0])
    }
    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
        self.last_time = 0.0;
        self.phase = 0.0;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &mut self.base;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - f64::from(CIRCLE) / b.dn);
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.source {
            s.set_vs_nodes(vs, NodeId::GROUND, self.base.nodes[0]);
            s.stamp_voltage_source_var(NodeId::GROUND, self.base.nodes[0], vs);
        }
    }
    fn do_step(&mut self, s: &mut Stamper) {
        let v = self.voltage(s.t);
        if let Some(vs) = self.source {
            s.update_voltage_source(vs, v);
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2, l1) = (self.base.point1, self.base.point2, self.base.lead1);
        self.base.set_bbox_pts(p1, p2, f64::from(CIRCLE));
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        draw_thick_line(g, p1, l1);
        g.set_font_size(12);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
        set_power_color_w(g, &self.base, c, 0.0);
        let (x2, y2) = (self.base.x2, self.base.y2);
        draw_centered_text(g, &mut self.base, if self.fm { "FM" } else { "AM" }, x2, y2, true);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { Color::GRAY });
        draw_thick_circle(g, p2.x, p2.y, CIRCLE);
        self.base.adjust_bbox(p2.x - CIRCLE, p2.y - CIRCLE, p2.x + CIRCLE, p2.y + CIRCLE);
        draw_posts(g, &self.base, self, c);
        self.base.curcount =
            self.base.update_dot_count_for(-self.base.current, self.base.curcount, c);
        if !self.base.creating {
            draw_dots(g, c, p1, l1, self.base.curcount);
        }
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_power(&self) -> f64 {
        -self.get_voltage_diff() * self.base.current
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(if self.fm { "FM Source" } else { "AM Source" }.into());
        arr.push(format!("I = {}", current_text(self.base.current)));
        arr.push(format!("V = {}", voltage_text(self.get_voltage_diff())));
        arr.push(format!("cf = {}", unit_text(self.carrier, "Hz")));
        arr.push(format!("sf = {}", unit_text(self.signal, "Hz")));
        if self.fm {
            arr.push(format!("dev ={}", unit_text(self.deviation, "Hz")));
        }
        arr.push(format!("Vmax = {}", voltage_text(self.max_voltage)));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Max Voltage", self.max_voltage, -20.0, 20.0).set_unit_step()),
            1 => Some(EditInfo::new("Carrier Frequency (Hz)", self.carrier, 4.0, 500.0)),
            2 => Some(EditInfo::new("Signal Frequency (Hz)", self.signal, 4.0, 500.0)),
            3 if self.fm => Some(EditInfo::new("Deviation (Hz)", self.deviation, 4.0, 500.0)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.max_voltage = ei.value,
            1 => self.carrier = ei.value,
            2 => self.signal = ei.value,
            3 => self.deviation = ei.value,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Fuse
// ---------------------------------------------------------------------------------------------

const FUSE_IEC: i32 = 1;
const BLOWN_RESISTANCE: f64 = 1e9;

/// A fuse that melts when the heat it has absorbed passes its I²t.
pub struct Fuse {
    base: Base,
    resistance: f64,
    i2t: f64,
    heat: f64,
    blown: bool,
}

impl Fuse {
    /// A fuse being placed.
    pub fn new(x: i32, y: i32) -> Self {
        // from the Littelfuse 218 series data sheet
        Fuse { base: Base::new(x, y), resistance: 0.0613, i2t: 6.73, heat: 0.0, blown: false }
    }

    /// A fuse read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut s = Fuse {
            base: Base::from_dump(xa, ya, xb, yb, f),
            resistance: 0.0613,
            i2t: 6.73,
            heat: 0.0,
            blown: false,
        };
        s.resistance = st.next_f64().unwrap_or(s.resistance);
        s.i2t = st.next_f64().unwrap_or(s.i2t);
        s.heat = st.next_f64().unwrap_or(0.0);
        s.blown = st.next_token().is_some_and(|t| t.eq_ignore_ascii_case("true"));
        s
    }

    fn iec(&self) -> bool {
        self.base.flags & FUSE_IEC != 0
    }

    /// The body colour: the voltage colour that warms to white as the fuse melts.
    fn temperature_color(&self, c: &DrawCtx) -> Color {
        let col = voltage_color(&self.base, c, self.base.volts[0]);
        let temp = self.heat / self.i2t;
        if temp < 0.3333 {
            let x = ((temp * 3.0) * 255.0).max(0.0) as i32;
            let mix = |ch: u8| ((255 - x) * i32::from(ch) / 255).clamp(0, 255) as u8;
            Color::new(
                (x + (255 - x) * i32::from(col.r) / 255).clamp(0, 255) as u8,
                mix(col.g),
                mix(col.b),
            )
        } else if temp < 0.6667 {
            Color::new(
                255,
                (((temp - 0.3333) * 3.0 * 255.0).max(0.0) as i32).clamp(0, 255) as u8,
                0,
            )
        } else if temp < 1.0 {
            Color::new(
                255,
                255,
                (((temp - 0.6666) * 3.0 * 255.0).max(0.0) as i32).clamp(0, 255) as u8,
            )
        } else {
            Color::WHITE
        }
    }
}

impl Element for Fuse {
    impl_element_base!("FuseElm");

    fn dump_type(&self) -> i32 {
        404
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(404),
            js_number(self.resistance),
            js_number(self.i2t),
            js_number(self.heat),
            self.blown
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("re", self.resistance);
        out.double_attr("i2", self.i2t);
        out.double_attr("he", self.heat);
        out.bool_attr("bl", self.blown);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.resistance = xml.double_attr("re", self.resistance);
        self.i2t = xml.double_attr("i2", self.i2t);
        self.heat = xml.double_attr("he", self.heat);
        self.blown = xml.bool_attr("bl", self.blown);
    }

    fn non_linear(&self) -> bool {
        true
    }
    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
        self.heat = 0.0;
        self.blown = false;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(if self.iec() { 32 } else { 16 });
    }

    fn stamp(&mut self, s: &mut Stamper) {
        s.stamp_non_linear(self.base.nodes[0]);
        s.stamp_non_linear(self.base.nodes[1]);
    }
    fn start_iteration(&mut self, s: &Stamper) {
        let i = self.base.current;
        // the fuse is assumed to shed its whole I2t in 3 seconds
        self.heat += i * i * s.time_step;
        self.heat -= s.time_step * self.i2t / 3.0;
        if self.heat < 0.0 {
            self.heat = 0.0;
        }
        if self.heat > self.i2t {
            self.blown = true;
        }
    }
    fn do_step(&mut self, s: &mut Stamper) {
        let r = if self.blown { BLOWN_RESISTANCE } else { self.resistance };
        s.stamp_resistor(self.base.nodes[0], self.base.nodes[1], r);
    }
    fn calculate_current(&mut self) {
        let r = if self.blown { BLOWN_RESISTANCE } else { self.resistance };
        self.base.current = (self.base.volts[0] - self.base.volts[1]) / r;
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.heat, if self.blown { 1.0 } else { 0.0 }]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [heat, blown] = state[..] {
            self.heat = heat;
            self.blown = blown != 0.0;
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let segments = 16;
        let hs = 6.0;
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, hs);
        draw_2_leads(g, &self.base, c);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let len = distance(l1, l2);
        g.save();
        g.set_line_width(3.0);
        let (ex, ey) = (f64::from(l2.x - l1.x), f64::from(l2.y - l1.y));
        g.transform(ex / len, ey / len, -ey / len, ex / len, f64::from(l1.x), f64::from(l1.y));
        g.set_color(self.temperature_color(c));
        if !self.blown {
            if self.iec() {
                g.begin_path();
                g.move_to(0.0, 0.0);
                g.line_to(len, 0.0);
                g.stroke();
                g.stroke_rect(0.0, -hs, len, 2.0 * hs);
            } else {
                g.begin_path();
                g.move_to(0.0, 0.0);
                for i in 0..=segments {
                    let a = f64::from(i);
                    g.line_to(
                        a * len / f64::from(segments),
                        hs * (a * std::f64::consts::TAU / f64::from(segments)).sin(),
                    );
                }
                g.stroke();
            }
        }
        g.restore();
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(if self.blown { "fuse (blown)" } else { "fuse" }.into());
        basic_info(self, arr);
        arr.push(format!("R = {}", unit_text(self.resistance, locale::OHM)));
        arr.push(format!("I2t = {}", self.i2t));
        if !self.blown {
            arr.push(format!(
                "{}% {}",
                (self.heat * 100.0 / self.i2t) as i32,
                locale::ls("melted")
            ));
        }
    }
    fn elm_type(&self) -> Option<String> {
        Some("fuse".into())
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("I2t", self.i2t, 0.0, 0.0).set_positive()),
            1 => Some(EditInfo::new("Resistance", self.resistance, 0.0, 0.0).set_positive()),
            2 => Some(EditInfo::checkbox("IEC Symbol", self.iec())),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.i2t = ei.value,
            1 => self.resistance = ei.value,
            2 => {
                self.base.flags = ei.change_flag(self.base.flags, FUSE_IEC);
                self.set_points();
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Photoresistor and NTC thermistor
// ---------------------------------------------------------------------------------------------

const T0: f64 = 273.15;

/// A resistor whose value follows a slider: the photoresistor (light) or the NTC thermistor
/// (temperature).
pub struct Sensor {
    base: Base,
    ntc: bool,
    /// Slider position, from 0 to 1.
    position: f64,
    slider_text: String,
    resistance: f64,
    // photoresistor
    min_lux: f64,
    max_lux: f64,
    // thermistor
    min_t: f64,
    max_t: f64,
    r25: f64,
    r50: f64,
    temperature: f64,
}

impl Sensor {
    fn blank(base: Base, ntc: bool) -> Self {
        let mut s = Sensor {
            base,
            ntc,
            position: 0.34,
            slider_text: if ntc { "Temperature" } else { "Light Brightness" }.into(),
            resistance: 0.0,
            min_lux: 0.1,
            max_lux: 10_000.0,
            min_t: -40.0,
            max_t: 150.0,
            r25: 10_000.0,
            r50: 3605.0,
            temperature: 0.0,
        };
        s.update();
        s
    }

    /// A photoresistor being placed.
    pub fn new_ldr(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y), false)
    }

    /// A thermistor being placed.
    pub fn new_ntc(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y), true)
    }

    /// A photoresistor read from a file.
    pub fn from_dump_ldr(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut s = Self::blank(Base::from_dump(xa, ya, xb, yb, f), false);
        s.position = st.next_f64().unwrap_or(s.position);
        if let Some(t) = st.next_token() {
            s.slider_text = unescape(t);
        }
        s.update();
        s
    }

    /// A thermistor read from a file.
    pub fn from_dump_ntc(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut s = Self::blank(Base::from_dump(xa, ya, xb, yb, f), true);
        s.r25 = st.next_f64().unwrap_or(s.r25);
        s.r50 = st.next_f64().unwrap_or(s.r50);
        s.min_t = st.next_f64().unwrap_or(s.min_t);
        s.max_t = st.next_f64().unwrap_or(s.max_t);
        s.position = st.next_f64().unwrap_or(s.position);
        if let Some(t) = st.next_token() {
            s.slider_text = unescape(t);
        }
        s.update();
        s
    }

    fn b_constant(&self) -> f64 {
        (self.r25.ln() - self.r50.ln()) / (1.0 / (T0 + 25.0) - 1.0 / (T0 + 50.0))
    }

    /// Recomputes the resistance from the slider position.
    fn update(&mut self) {
        if self.ntc {
            self.temperature = (self.position * (self.max_t - self.min_t) + self.min_t).round();
            let b = self.b_constant();
            self.resistance = (self.r25
                * (b * (1.0 / (self.temperature + T0) - 1.0 / (T0 + 25.0))).exp())
            .round();
        } else {
            let lux = self.max_lux * self.position + self.min_lux;
            self.resistance = ((self.max_lux - lux + 1.0) * 10.0).round();
        }
    }

    fn slider_offset(&self) -> f64 {
        if self.ntc { 0.005 } else { 0.0001 }
    }
}

impl Element for Sensor {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        if self.ntc { "ThermistorNTCElm" } else { "LDRElm" }
    }
    fn dump_type(&self) -> i32 {
        if self.ntc { 350 } else { 374 }
    }
    fn dump(&self) -> String {
        let prefix = self.base.dump_prefix(self.dump_type());
        if self.ntc {
            format!(
                "{prefix} {} {} {} {} {} {}",
                js_number(self.r25),
                js_number(self.r50),
                js_number(self.min_t),
                js_number(self.max_t),
                js_number(self.position),
                escape(&self.slider_text)
            )
        } else {
            format!("{prefix} {} {}", js_number(self.position), escape(&self.slider_text))
        }
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        if self.ntc {
            out.double_attr("r25", self.r25);
            out.double_attr("r50", self.r50);
            out.double_attr("mnt", self.min_t);
            out.double_attr("mxt", self.max_t);
        }
        out.double_attr("ps", self.position);
        out.str_attr("st", &self.slider_text);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        if self.ntc {
            self.r25 = xml.double_attr("r25", self.r25);
            self.r50 = xml.double_attr("r50", self.r50);
            self.min_t = xml.double_attr("mnt", self.min_t);
            self.max_t = xml.double_attr("mxt", self.max_t);
        }
        self.position = xml.double_attr("ps", self.position);
        self.slider_text = xml.string_attr("st", &self.slider_text);
        self.update();
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(32);
    }

    fn slider(&self) -> Option<ElementSlider> {
        let v = ((self.position - self.slider_offset()) / 0.0099).round() as i32;
        Some(ElementSlider { text: self.slider_text.clone(), value: v.clamp(0, 100) })
    }
    fn set_slider_value(&mut self, v: i32) {
        self.position = f64::from(v.clamp(0, 100)) * 0.0099 + self.slider_offset();
        self.update();
    }

    fn stamp(&mut self, s: &mut Stamper) {
        self.update();
        s.stamp_resistor(self.base.nodes[0], self.base.nodes[1], self.resistance);
    }
    fn calculate_current(&mut self) {
        self.base.current = (self.base.volts[0] - self.base.volts[1]) / self.resistance;
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let hs = 6.0;
        let (v1, v2) = (self.base.volts[0], self.base.volts[1]);
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, hs);
        draw_2_leads(g, &self.base, c);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let len = distance(l1, l2);
        g.save();
        g.set_line_width(3.0);
        let (ex, ey) = (f64::from(l2.x - l1.x), f64::from(l2.y - l1.y));
        g.transform(ex / len, ey / len, -ey / len, ex / len, f64::from(l1.x), f64::from(l1.y));
        if c.show_volts {
            g.set_stroke_gradient(
                len,
                voltage_color(&self.base, c, v1),
                voltage_color(&self.base, c, v2),
            );
        } else {
            set_power_color_w(g, &self.base, c, self.get_power());
        }
        if c.euro_resistors {
            g.stroke_rect(0.0, -hs, len, 2.0 * hs);
        } else {
            g.begin_path();
            g.move_to(0.0, 0.0);
            for i in 0..4 {
                let i = f64::from(i);
                g.line_to((1.0 + 4.0 * i) * len / 16.0, hs);
                g.line_to((3.0 + 4.0 * i) * len / 16.0, -hs);
            }
            g.line_to(len, 0.0);
            g.stroke();
        }
        g.begin_path();
        if self.ntc {
            // the thermistor mark: a slanted line with a foot
            g.move_to(-hs, hs * 2.0);
            g.line_to(hs, hs * 2.0);
            g.line_to(len, -hs * 2.0);
        } else {
            // two arrows of light
            for (x0, y0) in [(-8.0, 26.0), (12.0, 26.0)] {
                g.move_to(x0, y0);
                g.line_to(x0 + 16.0, y0 - 14.0);
                g.move_to(x0 + 10.0, y0 - 14.0);
                g.line_to(x0 + 16.0, y0 - 14.0);
                g.line_to(x0 + 16.0, y0 - 8.0);
            }
        }
        g.stroke();
        g.restore();
        if c.show_values {
            let r = short_unit_text(self.resistance, "");
            let s = if self.ntc {
                format!("{}\u{b0}C={r}{}", self.temperature, locale::OHM)
            } else {
                format!("{r}{}", locale::OHM)
            };
            draw_values(g, &self.base, c, Some(&s), hs, false, false);
        }
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(if self.ntc { "thermistor" } else { "photoresistor" }.into());
        arr.push(format!("I = {}", current_d_text(self.base.current)));
        arr.push(format!("Vd = {}", voltage_d_text(self.get_voltage_diff())));
        arr.push(format!("R = {}", unit_text(self.resistance, locale::OHM)));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
        if self.ntc {
            arr.push(format!("T = {}", unit_text(self.temperature, "\u{b0}C")));
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        if !self.ntc {
            return (n == 0).then(|| EditInfo::text("Slider Text", &self.slider_text));
        }
        match n {
            0 => Some(EditInfo::new("R at 25\u{b0}C", self.r25, self.r50 + 100.0, 100_000.0)),
            1 => Some(EditInfo::new("R at 50\u{b0}C", self.r50, 100.0, self.r25 - 100.0)),
            2 => Some(EditInfo::new("Slider min temp (\u{b0}C)", self.min_t, -40.0, self.max_t)),
            3 => Some(EditInfo::new("Slider max temp (\u{b0}C)", self.max_t, self.min_t, 150.0)),
            4 => Some(EditInfo::text("Slider Text", &self.slider_text)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match (self.ntc, n) {
            (false, 0) | (true, 4) => {
                if let Some(t) = &ei.text {
                    self.slider_text.clone_from(t);
                }
            }
            (true, 0) => self.r25 = ei.value,
            (true, 1) => self.r50 = ei.value,
            (true, 2) => self.min_t = ei.value,
            (true, 3) => self.max_t = ei.value,
            _ => {}
        }
        self.update();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ntc_is_10k_at_25_degrees_and_falls_when_warmer() {
        let mut s = Sensor::new_ntc(0, 0);
        // slider at 25 degrees: (25 + 40) / 190 of the way
        s.position = 65.0 / 190.0;
        s.update();
        assert_eq!(s.temperature, 25.0);
        assert!((s.resistance - 10_000.0).abs() < 1.0, "{}", s.resistance);
        s.position = 90.0 / 190.0;
        s.update();
        assert!((s.resistance - 3605.0).abs() < 10.0, "R at 50 C {}", s.resistance);
    }

    #[test]
    fn the_photoresistor_gets_darker_resistance_than_bright() {
        let mut s = Sensor::new_ldr(0, 0);
        s.set_slider_value(0);
        let dark = s.resistance;
        s.set_slider_value(100);
        assert!(dark > 90_000.0 && s.resistance < 1000.0, "{dark} {}", s.resistance);
        assert_eq!(s.slider().unwrap().value, 100);
    }

    #[test]
    fn fm_phase_only_advances_with_time() {
        let mut f = ModSource::new(true, 0, 0);
        let a = f.voltage(0.0001);
        let b = f.voltage(0.0001);
        assert_eq!(a, b);
    }
}
