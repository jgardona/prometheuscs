// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Diode, DiodeElm, ZenerElm, LEDElm,
// VaractorElm, TunnelDiodeElm, SparkGapElm).
//! Diodes: plain, Zener, LED, varactor, tunnel diode and spark gap.

use super::element::*;
use super::geometry::*;
use super::junction::Diode;
use super::model_edit::ModelEditor;
use super::models::DiodeModel;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::{TokenError, Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale::{self, ls};
use crate::render::painter::{Color, Painter};
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "diodes";
const ACTIVE: &str = "Active Components";

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("DiodeElm", FAMILY, "Add Diode", ACTIVE, |x, y| {
            Box::new(DiodeElm::new(DiodeKind::Diode, x, y))
        })
        .dump(i32::from(b'd'), |a, b, c, d, f, st| {
            Ok(Box::new(DiodeElm::from_dump(DiodeKind::Diode, a, b, c, d, f, st)?))
        })
        .shortcut('d'),
    );
    r.add(
        ElementDescriptor::new("ZenerElm", FAMILY, "Add Zener Diode", ACTIVE, |x, y| {
            Box::new(DiodeElm::new(DiodeKind::Zener, x, y))
        })
        .dump(i32::from(b'z'), |a, b, c, d, f, st| {
            Ok(Box::new(DiodeElm::from_dump(DiodeKind::Zener, a, b, c, d, f, st)?))
        })
        .shortcut('z'),
    );
    r.add(
        ElementDescriptor::new("LEDElm", FAMILY, "Add LED", "Outputs and Labels", |x, y| {
            Box::new(DiodeElm::new(DiodeKind::Led, x, y))
        })
        .dump(162, |a, b, c, d, f, st| {
            Ok(Box::new(DiodeElm::from_dump(DiodeKind::Led, a, b, c, d, f, st)?))
        })
        .shortcut('l'),
    );
    r.add(
        ElementDescriptor::new("VaractorElm", FAMILY, "Add Varactor/Varicap", ACTIVE, |x, y| {
            Box::new(DiodeElm::new(DiodeKind::Varactor, x, y))
        })
        .dump(176, |a, b, c, d, f, st| {
            Ok(Box::new(DiodeElm::from_dump(DiodeKind::Varactor, a, b, c, d, f, st)?))
        }),
    );
    r.add(
        ElementDescriptor::new("TunnelDiodeElm", FAMILY, "Add Tunnel Diode", ACTIVE, |x, y| {
            Box::new(TunnelDiodeElm::new(x, y))
        })
        .dump(175, |a, b, c, d, f, _| Ok(Box::new(TunnelDiodeElm::from_dump(a, b, c, d, f)))),
    );
    r.add(
        ElementDescriptor::new(
            "SparkGapElm",
            FAMILY,
            "Add Spark Gap",
            "Passive Components",
            |x, y| Box::new(SparkGapElm::new(x, y)),
        )
        .dump(187, |a, b, c, d, f, st| Ok(Box::new(SparkGapElm::from_dump(a, b, c, d, f, st)?))),
    );
}

// ---------------------------------------------------------------------------------------------
// DiodeElm and its variants
// ---------------------------------------------------------------------------------------------

const FLAG_FWDROP: i32 = 1;
const FLAG_MODEL: i32 = 2;
const HS: i32 = 8;

/// Which diode a [`DiodeElm`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiodeKind {
    /// A plain diode.
    Diode,
    /// A Zener diode.
    Zener,
    /// A light-emitting diode.
    Led,
    /// A voltage-variable capacitor.
    Varactor,
}

/// A diode, Zener diode, LED or varactor.
#[derive(Clone, Debug)]
pub struct DiodeElm {
    base: Base,
    kind: DiodeKind,
    model_name: String,
    model: DiodeModel,
    diode: Diode,
    has_resistance: bool,
    diode_end_node: usize,
    poly: Polygon,
    cathode: [Point; 2],
    wing: [Point; 2],
    // LED
    color: [f64; 3],
    max_brightness_current: f64,
    led_lead1: Point,
    led_lead2: Point,
    led_center: Point,
    // varactor
    base_capacitance: f64,
    capacitance: f64,
    cap_current: f64,
    comp_resistance: f64,
    cap_voltdiff: f64,
    volt_source_value: f64,
    plate1: [Point; 2],
    plate2: [Point; 2],
}

impl DiodeElm {
    /// Sets the LED colour from `#rrggbb`; text that is not a colour, or the colour already
    /// shown, leaves the stored value as it is.
    fn set_color(&mut self, hex: Option<&str>) {
        let Some(chosen) = hex.and_then(Color::parse_hex) else { return };
        if chosen != Color::from_unit(self.color) {
            self.color = chosen.to_unit();
        }
    }

    fn blank(kind: DiodeKind, base: Base) -> Self {
        let model = DiodeModel::default_model();
        DiodeElm {
            base,
            kind,
            model_name: model.name.clone(),
            model,
            diode: Diode::default(),
            has_resistance: false,
            diode_end_node: 1,
            poly: Polygon::new(),
            cathode: [Point::default(); 2],
            wing: [Point::default(); 2],
            color: [1.0, 0.0, 0.0],
            max_brightness_current: 0.01,
            led_lead1: Point::default(),
            led_lead2: Point::default(),
            led_center: Point::default(),
            base_capacitance: 4e-12,
            capacitance: 0.0,
            cap_current: 0.0,
            comp_resistance: 0.0,
            cap_voltdiff: 0.0,
            volt_source_value: 0.0,
            plate1: [Point::default(); 2],
            plate2: [Point::default(); 2],
        }
    }

    /// A diode being placed at (`xx`, `yy`).
    pub fn new(kind: DiodeKind, xx: i32, yy: i32) -> Self {
        let mut d = Self::blank(kind, Base::new(xx, yy));
        d.model_name = match kind {
            DiodeKind::Zener => "default-zener",
            DiodeKind::Led => "default-led",
            _ => "default",
        }
        .into();
        d.setup();
        d
    }

    /// A diode read from a file.
    pub fn from_dump(
        kind: DiodeKind,
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut d = Self::blank(kind, Base::from_dump(xa, ya, xb, yb, f));
        const DEFAULT_DROP: f64 = 0.805_904_783;
        if f & FLAG_MODEL != 0 {
            d.model_name = unescape(st.next_token().ok_or(TokenError::Missing)?);
        } else {
            let mut fwdrop = DEFAULT_DROP;
            if f & FLAG_FWDROP > 0 {
                if let Ok(v) = st.next_f64() {
                    fwdrop = v;
                }
            }
            d.model = DiodeModel::with_parameters(fwdrop, 0.0);
            d.model_name = d.model.name.clone();
        }
        d.setup();
        match kind {
            DiodeKind::Zener if f & FLAG_MODEL == 0 => {
                let zvoltage = st.next_f64()?;
                d.model = DiodeModel::with_parameters(d.model.fwdrop, zvoltage);
                d.model_name = d.model.name.clone();
                d.setup();
            }
            DiodeKind::Led => {
                if f & (FLAG_MODEL | FLAG_FWDROP) == 0 {
                    d.model = DiodeModel::with_parameters(2.102_425_9, 0.0);
                    d.model_name = d.model.name.clone();
                    d.setup();
                }
                d.color = [st.next_f64()?, st.next_f64()?, st.next_f64()?];
                d.max_brightness_current = st.next_f64().unwrap_or(0.01);
            }
            DiodeKind::Varactor => {
                d.cap_voltdiff = st.next_f64()?;
                d.base_capacitance = st.next_f64()?;
            }
            _ => {}
        }
        Ok(d)
    }

    fn setup(&mut self) {
        self.model = DiodeModel::named_or_copy(&self.model_name, Some(&self.model));
        self.model_name = self.model.name.clone();
        self.diode.setup(&self.model);
        self.has_resistance = self.model.series_resistance > 0.0;
        self.diode_end_node = if self.has_resistance { 2 } else { 1 };
        self.alloc_nodes();
    }

    fn voltdiff(&self) -> f64 {
        self.base.volts[0] - self.base.volts[self.diode_end_node]
    }

    fn draw_diode(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(HS));
        let (v1, v2) = (self.base.volts[0], self.base.volts[1]);
        draw_2_leads(g, &self.base, c);
        let power = self.get_power();
        set_voltage_color(g, &self.base, c, v1);
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.poly);
        set_voltage_color(g, &self.base, c, v2);
        set_power_color_w(g, &self.base, c, power);
        draw_thick_line(g, self.cathode[0], self.cathode[1]);
    }

    fn model_description(&self, label: &str) -> String {
        if self.model.old_style {
            label.to_string()
        } else {
            format!("{} ({})", ls(label), self.model_name)
        }
    }
}

impl Element for DiodeElm {
    impl_element_base!();

    fn model_buttons(&self) -> Vec<String> {
        self.model_buttons_list()
    }
    fn start_model_edit(&self, button: usize) -> Option<ModelEditor> {
        self.start_model_editor(button)
    }
    fn use_model(&mut self, name: &str) {
        self.model_name = name.to_string();
        self.setup();
    }
    fn update_models(&mut self) {
        self.setup();
    }

    fn class_name(&self) -> &'static str {
        match self.kind {
            DiodeKind::Diode => "DiodeElm",
            DiodeKind::Zener => "ZenerElm",
            DiodeKind::Led => "LEDElm",
            DiodeKind::Varactor => "VaractorElm",
        }
    }
    fn dump_type(&self) -> i32 {
        match self.kind {
            DiodeKind::Diode => i32::from(b'd'),
            DiodeKind::Zener => i32::from(b'z'),
            DiodeKind::Led => 162,
            DiodeKind::Varactor => 176,
        }
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        let mut b = self.base.clone();
        b.flags |= FLAG_MODEL;
        let mut s = format!("{} {}", b.dump_prefix(self.dump_type()), escape(&self.model_name));
        match self.kind {
            DiodeKind::Led => {
                s += &format!(
                    " {} {} {} {}",
                    js_number(self.color[0]),
                    js_number(self.color[1]),
                    js_number(self.color[2]),
                    js_number(self.max_brightness_current)
                );
            }
            DiodeKind::Varactor => {
                s += &format!(
                    " {} {}",
                    js_number(self.cap_voltdiff),
                    js_number(self.base_capacitance)
                );
            }
            _ => {}
        }
        s
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.model_name = xml.string_attr("mo", &self.model_name);
        if self.kind == DiodeKind::Led {
            self.color[0] = xml.double_attr("cr", self.color[0]);
            self.color[1] = xml.double_attr("cg", self.color[1]);
            self.color[2] = xml.double_attr("cb", self.color[2]);
            self.max_brightness_current = xml.double_attr("mbc", self.max_brightness_current);
        }
        if self.kind == DiodeKind::Varactor {
            self.cap_voltdiff = xml.double_attr("ca", self.cap_voltdiff);
            self.base_capacitance = xml.double_attr("ba", self.base_capacitance);
        }
        self.setup();
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.str_attr("mo", &self.model_name);
        match self.kind {
            DiodeKind::Led => {
                out.double_attr("cr", self.color[0]);
                out.double_attr("cg", self.color[1]);
                out.double_attr("cb", self.color[2]);
                out.double_attr("mbc", self.max_brightness_current);
            }
            DiodeKind::Varactor => {
                out.double_attr("ca", self.cap_voltdiff);
                out.double_attr("ba", self.base_capacitance);
            }
            _ => {}
        }
    }
    fn dump_model_xml(&self) -> Option<XmlOut> {
        (!self.model.built_in).then(|| self.model.dump_xml())
    }

    fn non_linear(&self) -> bool {
        true
    }
    fn internal_node_count(&self) -> usize {
        match self.kind {
            DiodeKind::Varactor => 1,
            _ => usize::from(self.has_resistance),
        }
    }
    fn voltage_source_count(&self) -> usize {
        usize::from(self.kind == DiodeKind::Varactor)
    }
    fn voltage_source_nodes(&self) -> Option<(NodeId, NodeId)> {
        (self.kind == DiodeKind::Varactor).then(|| (self.base.nodes[0], self.base.nodes[2]))
    }
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base.volt_source = Some(v);
    }
    fn set_current(&mut self, _vs: VsId, c: f64) {
        if self.kind == DiodeKind::Varactor {
            self.cap_current = c;
        }
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(16);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let hs = f64::from(HS);
        let pa = interp_point2(l1, l2, 0.0, hs);
        self.cathode = {
            let (a, b) = interp_point2(l1, l2, 1.0, hs);
            [a, b]
        };
        self.poly = Polygon::from_points(&[pa.0, pa.1, l2]);
        match self.kind {
            DiodeKind::Zener => {
                self.wing[0] = interp_point_g(self.cathode[0], self.cathode[1], -0.2, -hs);
                self.wing[1] = interp_point_g(self.cathode[1], self.cathode[0], -0.2, -hs);
            }
            DiodeKind::Led => {
                let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
                self.led_lead1 = interp_point(p1, p2, 0.5 - 12.0 / dn);
                self.led_lead2 = interp_point(p1, p2, 0.5 + 12.0 / dn);
                self.led_center = interp_point(p1, p2, 0.5);
            }
            DiodeKind::Varactor => {
                let platef = 0.6;
                let (a, b) = interp_point2(l1, l2, platef, hs);
                self.cathode = [a, b];
                let arrow = interp_point(l1, l2, platef);
                self.poly = Polygon::from_points(&[pa.0, pa.1, arrow]);
                let (a, b) = interp_point2(l1, l2, platef, hs);
                self.plate1 = [a, b];
                let (a, b) = interp_point2(l1, l2, 1.0, hs);
                self.plate2 = [a, b];
            }
            DiodeKind::Diode => {}
        }
        self.alloc_nodes();
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        match self.kind {
            DiodeKind::Diode => {
                self.draw_diode(g, c);
                do_dots(g, &mut self.base, c);
                draw_posts(g, &self.base, self, c);
            }
            DiodeKind::Zener => {
                let (p1, p2) = (self.base.point1, self.base.point2);
                self.base.set_bbox_pts(p1, p2, f64::from(HS));
                let (v1, v2) = (self.base.volts[0], self.base.volts[1]);
                draw_2_leads(g, &self.base, c);
                set_power_color_w(g, &self.base, c, self.get_power());
                set_voltage_color(g, &self.base, c, v1);
                fill_polygon(g, &self.poly);
                set_voltage_color(g, &self.base, c, v2);
                draw_thick_line(g, self.cathode[0], self.cathode[1]);
                draw_thick_line(g, self.wing[0], self.cathode[0]);
                draw_thick_line(g, self.wing[1], self.cathode[1]);
                do_dots(g, &mut self.base, c);
                draw_posts(g, &self.base, self, c);
            }
            DiodeKind::Led => {
                if self.base.needs_highlight() || self.base.creating {
                    self.draw_diode(g, c);
                    do_dots(g, &mut self.base, c);
                    draw_posts(g, &self.base, self, c);
                    return;
                }
                set_voltage_color(g, &self.base, c, self.base.volts[0]);
                draw_thick_line(g, self.base.point1, self.led_lead1);
                set_voltage_color(g, &self.base, c, self.base.volts[1]);
                draw_thick_line(g, self.led_lead2, self.base.point2);
                g.set_color(Color::GRAY);
                let mut cr = 12;
                draw_thick_circle(g, self.led_center.x, self.led_center.y, cr);
                cr -= 4;
                let mut w = self.base.current / self.max_brightness_current;
                if w > 0.0 {
                    w = 255.0 * (1.0 + 0.2 * w.ln());
                }
                w = w.clamp(0.0, 255.0);
                g.set_color(Color::new(
                    (self.color[0] * w) as u8,
                    (self.color[1] * w) as u8,
                    (self.color[2] * w) as u8,
                ));
                g.fill_oval(self.led_center.x - cr, self.led_center.y - cr, cr * 2, cr * 2);
                let (p1, p2) = (self.base.point1, self.base.point2);
                self.base.set_bbox_pts(p1, p2, f64::from(cr));
                self.base.update_dot_count(c);
                let cc = self.base.curcount;
                draw_dots(g, c, p1, self.led_lead1, cc);
                draw_dots(g, c, p2, self.led_lead2, -cc);
                draw_posts(g, &self.base, self, c);
            }
            DiodeKind::Varactor => {
                self.draw_diode(g, c);
                set_voltage_color(g, &self.base, c, self.base.volts[0]);
                set_power_color_w(g, &self.base, c, 0.0);
                draw_thick_line(g, self.plate1[0], self.plate1[1]);
                if c.show_power {
                    g.set_color(Color::GRAY);
                }
                set_voltage_color(g, &self.base, c, self.base.volts[1]);
                set_power_color_w(g, &self.base, c, 0.0);
                draw_thick_line(g, self.plate2[0], self.plate2[1]);
                do_dots(g, &mut self.base, c);
                draw_posts(g, &self.base, self, c);
            }
        }
    }

    fn reset(&mut self) {
        self.diode.reset();
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
        self.cap_voltdiff = 0.0;
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        if self.has_resistance {
            self.diode.stamp(s, n[0], n[2]);
            s.stamp_resistor(n[1], n[2], self.model.series_resistance);
        } else {
            self.diode.stamp(s, n[0], n[1]);
        }
        if self.kind == DiodeKind::Varactor {
            if let Some(vs) = self.base.volt_source {
                s.stamp_voltage_source_var(n[0], n[2], vs);
            }
            s.stamp_non_linear(n[2]);
        }
    }

    fn start_iteration(&mut self, s: &Stamper) {
        if self.kind != DiodeKind::Varactor {
            return;
        }
        let c0 = self.base_capacitance;
        self.capacitance = if self.cap_voltdiff > 0.0 {
            c0
        } else {
            c0 / (1.0 - self.cap_voltdiff / self.model.fwdrop).powf(0.5)
        };
        self.comp_resistance = s.time_step / (2.0 * self.capacitance);
        self.volt_source_value = -self.cap_voltdiff - self.cap_current * self.comp_resistance;
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let v = self.voltdiff();
        self.diode.do_step(s, v);
        if self.kind == DiodeKind::Varactor {
            s.stamp_resistor(self.base.nodes[2], self.base.nodes[1], self.comp_resistance);
            if let Some(vs) = self.base.volt_source {
                s.update_voltage_source(vs, self.volt_source_value);
            }
        }
    }

    fn calculate_current(&mut self) {
        let v = self.voltdiff();
        self.base.current = self.diode.calculate_current(v);
        if self.kind == DiodeKind::Varactor {
            self.base.current += self.cap_current;
        }
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if self.kind == DiodeKind::Varactor {
            self.cap_voltdiff = self.base.volts[0] - self.base.volts[1];
        } else if self.base.current.abs() > 1e12 {
            s.stop("max current exceeded", None);
        }
    }

    fn elm_type(&self) -> Option<String> {
        Some(
            match self.kind {
                DiodeKind::Diode => "diode",
                DiodeKind::Zener => "Zener diode",
                DiodeKind::Led => "LED",
                DiodeKind::Varactor => "varactor",
            }
            .into(),
        )
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let title = match self.kind {
            DiodeKind::Diode => self.model_description("diode"),
            DiodeKind::Zener => "Zener diode".to_string(),
            DiodeKind::Led => self.model_description("LED"),
            DiodeKind::Varactor => "varactor".to_string(),
        };
        arr.push(title);
        arr.push(format!("I = {}", current_text(self.get_current())));
        arr.push(format!("Vd = {}", voltage_text(self.get_voltage_diff())));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
        if self.model.old_style {
            arr.push(format!("Vf = {}", voltage_text(self.model.fwdrop)));
        }
        match self.kind {
            DiodeKind::Zener => {
                arr.push(format!("Vz = {}", voltage_text(self.model.breakdown_voltage)));
            }
            DiodeKind::Varactor => arr.push(format!("C = {}", unit_text(self.capacitance, "F"))),
            _ => {}
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        let n = match self.kind {
            DiodeKind::Led => {
                return match n {
                    0 => Some(EditInfo::color("Color", &Color::from_unit(self.color))),
                    // the former green and blue fields; kept so the later indices (also used by
                    // saved sliders) do not move
                    1 | 2 => Some(EditInfo::hidden()),
                    3 => Some(EditInfo::new(
                        "Max Brightness Current (A)",
                        self.max_brightness_current,
                        0.0,
                        0.1,
                    )),
                    _ => self.model_edit_info(n - 4),
                };
            }
            DiodeKind::Varactor if n == 1 => {
                return Some(EditInfo::new(
                    "Capacitance @ 0V (F)",
                    self.base_capacitance,
                    10.0,
                    1000.0,
                ));
            }
            _ => n,
        };
        self.model_edit_info(n)
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match self.kind {
            DiodeKind::Led => match n {
                0 => self.set_color(ei.text.as_deref()),
                1 | 2 => {}
                3 => self.max_brightness_current = ei.value,
                _ => self.set_model_edit(n - 4, ei),
            },
            DiodeKind::Varactor if n == 1 => self.base_capacitance = ei.value,
            _ => self.set_model_edit(n, ei),
        }
    }

    fn shortcut(&self) -> i32 {
        match self.kind {
            DiodeKind::Diode => i32::from(b'd'),
            DiodeKind::Zener => i32::from(b'z'),
            DiodeKind::Led => i32::from(b'l'),
            DiodeKind::Varactor => 0,
        }
    }
}

impl DiodeElm {
    fn model_buttons_list(&self) -> Vec<String> {
        let mut v = vec!["Create New Simple Model".to_string(), "Create New Advanced Model".into()];
        if !self.model.read_only {
            v.push("Edit Model".into());
        }
        v
    }

    fn start_model_editor(&self, button: usize) -> Option<ModelEditor> {
        match button {
            0 | 1 => Some(ModelEditor::new_diode(&self.model, button == 0)),
            2 if !self.model.read_only => Some(ModelEditor::edit_diode(&self.model)),
            _ => None,
        }
    }

    fn model_edit_info(&self, n: usize) -> Option<EditInfo> {
        if n != 0 {
            return None;
        }
        let models = DiodeModel::list(self.kind == DiodeKind::Zener);
        let selected = models.iter().position(|m| m.name == self.model_name).unwrap_or(0);
        Some(EditInfo::choice(
            "Model",
            models.iter().map(DiodeModel::description_text).collect(),
            selected,
        ))
    }

    fn set_model_edit(&mut self, n: usize, ei: &mut EditInfo) {
        if n != 0 {
            return;
        }
        let models = DiodeModel::list(self.kind == DiodeKind::Zener);
        if let Some(m) = models.get(ei.choice_index) {
            self.model_name = m.name.clone();
            self.model = m.clone();
            self.setup();
            ei.new_dialog = true;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Tunnel diode
// ---------------------------------------------------------------------------------------------

const PVP: f64 = 0.1;
const PIP: f64 = 4.7e-3;
const PVV: f64 = 0.37;
const PVT: f64 = 0.026;
const PVPP: f64 = 0.525;
const PIV: f64 = 370e-6;

/// A tunnel diode.
#[derive(Clone, Debug)]
pub struct TunnelDiodeElm {
    base: Base,
    lastvoltdiff: f64,
    poly: Polygon,
    cathode: [Point; 4],
}

impl TunnelDiodeElm {
    /// A tunnel diode being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        TunnelDiodeElm {
            base: Base::new(xx, yy),
            lastvoltdiff: 0.0,
            poly: Polygon::new(),
            cathode: [Point::default(); 4],
        }
    }

    /// A tunnel diode read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32) -> Self {
        let mut d = Self::new(xa, ya);
        d.base = Base::from_dump(xa, ya, xb, yb, f);
        d
    }

    fn current_at(voltdiff: f64) -> f64 {
        let i0 = PIV * (-PVV).exp();
        PIP * (-PVPP / PVT).exp() * ((voltdiff / PVT).exp() - 1.0)
            + PIP * (voltdiff / PVP) * (1.0 - voltdiff / PVP).exp()
            + PIV * (voltdiff - PVV).exp()
            - i0
    }
}

impl Element for TunnelDiodeElm {
    impl_element_base!("TunnelDiodeElm");

    fn dump_type(&self) -> i32 {
        175
    }
    fn non_linear(&self) -> bool {
        true
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(16);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let hs = f64::from(HS);
        let pa = interp_point2(l1, l2, 0.0, hs);
        let (c0, c1) = interp_point2(l1, l2, 1.0, hs);
        let (c2, c3) = interp_point2(l1, l2, 0.8, hs);
        self.cathode = [c0, c1, c2, c3];
        self.poly = Polygon::from_points(&[pa.0, pa.1, l2]);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(HS));
        let (v1, v2) = (self.base.volts[0], self.base.volts[1]);
        draw_2_leads(g, &self.base, c);
        set_power_color_w(g, &self.base, c, self.get_power());
        set_voltage_color(g, &self.base, c, v1);
        fill_polygon(g, &self.poly);
        set_voltage_color(g, &self.base, c, v2);
        draw_thick_line(g, self.cathode[0], self.cathode[1]);
        draw_thick_line(g, self.cathode[2], self.cathode[0]);
        draw_thick_line(g, self.cathode[3], self.cathode[1]);
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn reset(&mut self) {
        self.lastvoltdiff = 0.0;
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
    }

    fn stamp(&mut self, s: &mut Stamper) {
        s.stamp_non_linear(self.base.nodes[0]);
        s.stamp_non_linear(self.base.nodes[1]);
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let mut voltdiff = self.base.volts[0] - self.base.volts[1];
        if (voltdiff - self.lastvoltdiff).abs() > 0.01 {
            s.converged = false;
        }
        // limit the step to one volt
        voltdiff = voltdiff.min(self.lastvoltdiff + 1.0).max(self.lastvoltdiff - 1.0);
        self.lastvoltdiff = voltdiff;
        let i = Self::current_at(voltdiff);
        let geq = PIP * (-PVPP / PVT).exp() * (voltdiff / PVT).exp() / PVT
            + PIP * (1.0 - voltdiff / PVP).exp() / PVP
            - (1.0 - voltdiff / PVP).exp() * PIP * voltdiff / (PVP * PVP)
            + (voltdiff - PVV).exp() * PIV;
        let nc = i - geq * voltdiff;
        s.stamp_conductance(self.base.nodes[0], self.base.nodes[1], geq);
        s.stamp_current_source(self.base.nodes[0], self.base.nodes[1], nc);
    }

    fn calculate_current(&mut self) {
        self.base.current = Self::current_at(self.base.volts[0] - self.base.volts[1]);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("tunnel diode".into());
        arr.push(format!("I = {}", current_text(self.get_current())));
        arr.push(format!("Vd = {}", voltage_text(self.get_voltage_diff())));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }
}

// ---------------------------------------------------------------------------------------------
// Spark gap
// ---------------------------------------------------------------------------------------------

/// A spark gap: open until the voltage across it passes the breakdown voltage, then a low
/// resistance until the current falls below the holding current.
#[derive(Clone, Debug)]
pub struct SparkGapElm {
    base: Base,
    resistance: f64,
    onresistance: f64,
    offresistance: f64,
    breakdown: f64,
    holdcurrent: f64,
    state: bool,
    arrow1: Polygon,
    arrow2: Polygon,
}

impl SparkGapElm {
    /// A spark gap being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        SparkGapElm {
            base: Base::new(xx, yy),
            resistance: 0.0,
            onresistance: 1e3,
            offresistance: 1e9,
            breakdown: 1e3,
            holdcurrent: 0.001,
            state: false,
            arrow1: Polygon::new(),
            arrow2: Polygon::new(),
        }
    }

    /// A spark gap read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut g = Self::new(xa, ya);
        g.base = Base::from_dump(xa, ya, xb, yb, f);
        g.onresistance = st.next_f64()?;
        g.offresistance = st.next_f64()?;
        g.breakdown = st.next_f64()?;
        g.holdcurrent = st.next_f64()?;
        Ok(g)
    }
}

impl Element for SparkGapElm {
    impl_element_base!("SparkGapElm");

    fn dump_type(&self) -> i32 {
        187
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.onresistance),
            js_number(self.offresistance),
            js_number(self.breakdown),
            js_number(self.holdcurrent)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.onresistance = xml.double_attr("on", self.onresistance);
        self.offresistance = xml.double_attr("of", self.offresistance);
        self.breakdown = xml.double_attr("br", self.breakdown);
        self.holdcurrent = xml.double_attr("ho", self.holdcurrent);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("on", self.onresistance);
        out.double_attr("of", self.offresistance);
        out.double_attr("br", self.breakdown);
        out.double_attr("ho", self.holdcurrent);
    }
    fn non_linear(&self) -> bool {
        true
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let (dist, alen) = (16, 8);
        self.base.calc_leads(dist + alen);
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let a = f64::from(alen);
        let m = interp_point(p1, p2, (dn - a) / (2.0 * dn));
        self.arrow1 = calc_arrow(p1, m, a, a);
        let m = interp_point(p1, p2, (dn + a) / (2.0 * dn));
        self.arrow2 = calc_arrow(p2, m, a, a);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, 8.0);
        draw_2_leads(g, &self.base, c);
        let power = self.get_power();
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.arrow1);
        set_voltage_color(g, &self.base, c, self.base.volts[1]);
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.arrow2);
        if self.state {
            do_dots(g, &mut self.base, c);
        }
        draw_posts(g, &self.base, self, c);
    }

    fn calculate_current(&mut self) {
        let vd = self.base.volts[0] - self.base.volts[1];
        self.base.current = vd / self.resistance;
    }
    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.current = 0.0;
        self.base.curcount = 0.0;
        self.state = false;
    }
    fn start_iteration(&mut self, _s: &Stamper) {
        if self.base.current.abs() < self.holdcurrent {
            self.state = false;
        }
        let vd = self.base.volts[0] - self.base.volts[1];
        if vd.abs() > self.breakdown {
            self.state = true;
        }
    }
    fn do_step(&mut self, s: &mut Stamper) {
        self.resistance = if self.state { self.onresistance } else { self.offresistance };
        s.stamp_resistor(self.base.nodes[0], self.base.nodes[1], self.resistance);
    }
    fn stamp(&mut self, s: &mut Stamper) {
        s.stamp_non_linear(self.base.nodes[0]);
        s.stamp_non_linear(self.base.nodes[1]);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("spark gap".into());
        basic_info(self, arr);
        arr.push(if self.state { "on" } else { "off" }.into());
        arr.push(format!("Ron = {}", unit_text(self.onresistance, locale::OHM)));
        arr.push(format!("Roff = {}", unit_text(self.offresistance, locale::OHM)));
        arr.push(format!("Vbreakdown = {}", unit_text(self.breakdown, "V")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("On resistance (ohms)", self.onresistance, 0.0, 0.0)),
            1 => Some(EditInfo::new("Off resistance (ohms)", self.offresistance, 0.0, 0.0)),
            2 => Some(EditInfo::new("Breakdown voltage", self.breakdown, 0.0, 0.0)),
            3 => Some(EditInfo::new("Holding current (A)", self.holdcurrent, 0.0, 0.0)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if ei.value > 0.0 {
            match n {
                0 => self.onresistance = ei.value,
                1 => self.offresistance = ei.value,
                2 => self.breakdown = ei.value,
                3 => self.holdcurrent = ei.value,
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::models::THERMAL_VOLTAGE;
    use super::*;
    use crate::format::parser::parse;
    use crate::solver::sim::Simulation;

    fn run(text: &str, steps: usize) -> Simulation {
        let r = parse(text, &Registry::standard());
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        sim.analyze().unwrap();
        sim.run_steps(steps).unwrap();
        sim
    }

    const HDR: &str = "$ 1 0.000005 10 50 5 50 5e-11";

    /// A 5 V source, a 1 k resistor and a diode of the given dump line in series.
    fn diode_loop(diode: &str) -> Simulation {
        run(
            &format!(
                "{HDR}\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\n{diode}\nw 64 64 0 64 0\n"
            ),
            200,
        )
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    #[test]
    fn forward_diode_conducts_with_the_models_drop() {
        // 'd' with the model flag: anode at (64, 0), cathode at (64, 64); source + at (0, 0)
        let sim = diode_loop("d 64 0 64 64 2 default");
        let d = &sim.elements[2];
        let drop = d.get_voltage_diff();
        // Shockley: V = n Vt ln(I / Is) for the current the circuit settles at
        let m = DiodeModel::default_model();
        let i = sim.elements[2].get_current();
        let shockley = m.emission_coefficient * THERMAL_VOLTAGE * (i / m.saturation_current).ln();
        assert!((drop - shockley).abs() < 5e-3, "{drop} vs {shockley}");
        let i = d.get_current();
        assert!((i - (5.0 - drop) / 1000.0).abs() < 1e-6, "{i}");
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    #[test]
    fn reverse_diode_blocks_and_zener_clamps() {
        let sim = diode_loop("d 64 64 64 0 2 default");
        assert!(sim.elements[2].get_current().abs() < 1e-6);
        // a 5.6 V Zener, driven backwards from 12 V, holds its breakdown voltage
        let sim = run(
            &format!(
                "{HDR}\nv 0 64 0 0 0 0 40 12 0 0 0.5\nr 0 0 64 0 0 1000\nz 64 64 64 0 2 default-zener\nw 64 64 0 64 0\n"
            ),
            400,
        );
        let vz = sim.elements[2].get_voltage_diff().abs();
        assert!(vz > 5.2 && vz < 6.2, "{vz}");
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    #[test]
    fn old_files_without_a_model_use_the_forward_drop() {
        // flag 1: a forward drop follows
        let sim = diode_loop("d 64 0 64 64 1 0.7");
        // the 0.7 V forward drop is the voltage at 1 A, so at a few mA the drop is much lower
        let d = sim.elements[2].get_voltage_diff();
        assert!(d > 0.35 && d < 0.5, "{d}");
        // the name of the made-up model is written escaped (`=` becomes `\q`)
        assert!(sim.elements[2].dump().contains("fwdrop\\q0.7"), "{}", sim.elements[2].dump());
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    #[test]
    fn model_definitions_travel_through_xml() {
        let text = format!(
            "{HDR}\n34 mydiode 0 1e-12 0 2.5 0 0\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nd 64 0 64 64 2 mydiode\nw 64 64 0 64 0\n"
        );
        let r = parse(&text, &Registry::standard());
        assert!(r.errors.is_empty());
        let xml = crate::format::writer::write_xml(&r.circuit);
        assert!(xml.contains("<dm nm=\"mydiode\""), "{xml}");
        assert_eq!(xml.matches("<dm ").count(), 1);
        // a built-in model is not written
        let r = parse(&format!("{HDR}\nd 64 0 64 64 2 1N4148\n"), &Registry::standard());
        assert!(!crate::format::writer::write_xml(&r.circuit).contains("<dm "));
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    #[test]
    fn led_dump_reads_colours_and_brightness() {
        let sim = diode_loop("162 64 0 64 64 2 default-led 0 1 0 0.02");
        let led = &sim.elements[2];
        assert!(led.dump().ends_with("default-led 0 1 0 0.02"), "{}", led.dump());
        assert!(led.get_current() > 1e-3);
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    #[test]
    fn varactor_and_tunnel_diode_and_spark_gap_run() {
        let sim = diode_loop("176 64 0 64 64 2 default 0 4e-12");
        assert!(sim.elements[2].get_current().is_finite());
        let sim = diode_loop("175 64 0 64 64 0");
        assert!(sim.elements[2].get_current().is_finite());
        // a spark gap far below its breakdown voltage stays off
        let sim = diode_loop("187 64 0 64 64 0 1000 1000000000 1000 0.001");
        assert!(sim.elements[2].get_current().abs() < 1e-5);
    }
}
