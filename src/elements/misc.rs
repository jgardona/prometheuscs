// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (MemristorElm, LampElm,
// AnalogSwitchElm, AnalogSwitch2Elm).
//! Assorted analog components: memristor, incandescent lamp and analog switches.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::{TokenError, Tokenizer};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale;
use crate::render::painter::{Color, Painter, TextAlign};
use crate::solver::stamper::{NodeId, Stamper};

/// Family name used for the feature switch.
pub const FAMILY: &str = "analog";
const ACTIVE: &str = "Active Building Blocks";
const PASSIVE: &str = "Passive Components";

/// Registers the elements of this module.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("MemristorElm", FAMILY, "Add Memristor", PASSIVE, |x, y| {
            Box::new(MemristorElm::new(x, y))
        })
        .dump(i32::from(b'm'), |a, b, c, d, f, st| {
            Ok(Box::new(MemristorElm::from_dump(a, b, c, d, f, st)?))
        }),
    );
    r.add(
        ElementDescriptor::new("LampElm", FAMILY, "Add Lamp", PASSIVE, |x, y| {
            Box::new(LampElm::new(x, y))
        })
        .dump(181, |a, b, c, d, f, st| Ok(Box::new(LampElm::from_dump(a, b, c, d, f, st)?))),
    );
    r.add(
        ElementDescriptor::new(
            "AnalogSwitchElm",
            FAMILY,
            "Add Analog Switch (SPST)",
            ACTIVE,
            |x, y| Box::new(AnalogSwitchElm::new(false, x, y)),
        )
        .dump(159, |a, b, c, d, f, st| {
            Ok(Box::new(AnalogSwitchElm::from_dump(false, a, b, c, d, f, st)))
        }),
    );
    r.add(
        ElementDescriptor::new(
            "AnalogSwitch2Elm",
            FAMILY,
            "Add Analog Switch (SPDT)",
            ACTIVE,
            |x, y| Box::new(AnalogSwitchElm::new(true, x, y)),
        )
        .dump(160, |a, b, c, d, f, st| {
            Ok(Box::new(AnalogSwitchElm::from_dump(true, a, b, c, d, f, st)))
        }),
    );
}

// ---------------------------------------------------------------------------------------------
// Memristor
// ---------------------------------------------------------------------------------------------

/// Memristor (HP model): resistance depends on the charge that flowed through it.
#[derive(Clone, Debug)]
pub struct MemristorElm {
    base: Base,
    r_on: f64,
    r_off: f64,
    dope_width: f64,
    total_width: f64,
    mobility: f64,
    resistance: f64,
}

impl MemristorElm {
    fn blank(base: Base) -> Self {
        MemristorElm {
            base,
            r_on: 100.0,
            r_off: 16000.0,
            dope_width: 0.0,
            total_width: 10e-9,
            mobility: 1e-10,
            resistance: 100.0,
        }
    }

    /// A memristor being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        MemristorElm::blank(Base::new(xx, yy))
    }

    /// A memristor read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut m = MemristorElm::blank(Base::from_dump(xa, ya, xb, yb, f));
        m.r_on = st.next_f64()?;
        m.r_off = st.next_f64()?;
        m.dope_width = st.next_f64()?;
        m.total_width = st.next_f64()?;
        m.mobility = st.next_f64()?;
        if let Ok(c) = st.next_f64() {
            m.base.current = c;
        }
        Ok(m)
    }
}

impl Element for MemristorElm {
    impl_element_base!("MemristorElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'm')
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.r_on),
            js_number(self.r_off),
            js_number(self.dope_width),
            js_number(self.total_width),
            js_number(self.mobility)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.r_on = xml.double_attr("ron", self.r_on);
        self.r_off = xml.double_attr("rof", self.r_off);
        self.dope_width = xml.double_attr("do", self.dope_width);
        self.total_width = xml.double_attr("to", self.total_width);
        self.mobility = xml.double_attr("mo", self.mobility);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("ron", self.r_on);
        out.double_attr("rof", self.r_off);
        out.double_attr("do", self.dope_width);
        out.double_attr("to", self.total_width);
        out.double_attr("mo", self.mobility);
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(32);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let segments = 6;
        let (v1, v2) = (self.base.volts[0], self.base.volts[1]);
        let hs = 2 + (8.0 * (1.0 - self.dope_width / self.total_width)) as i32;
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
        draw_2_leads(g, &self.base, c);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let segf = 1.0 / f64::from(segments);
        let mut ox = 0;
        for i in 0..=segments {
            let mut nx = if i & 1 == 0 { 1 } else { -1 };
            if i == segments {
                nx = 0;
            }
            let v = v1 + (v2 - v1) * f64::from(i) / f64::from(segments);
            set_voltage_color(g, &self.base, c, v);
            let a = interp_point_g(l1, l2, f64::from(i) * segf, f64::from(hs * ox));
            let b = interp_point_g(l1, l2, f64::from(i) * segf, f64::from(hs * nx));
            draw_thick_line(g, a, b);
            if i == segments {
                break;
            }
            let b2 = interp_point_g(l1, l2, f64::from(i + 1) * segf, f64::from(hs * nx));
            draw_thick_line(g, b, b2);
            ox = nx;
        }
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn non_linear(&self) -> bool {
        true
    }
    fn calculate_current(&mut self) {
        self.base.current = (self.base.volts[0] - self.base.volts[1]) / self.resistance;
    }
    fn reset(&mut self) {
        self.dope_width = 0.0;
    }
    fn start_iteration(&mut self, s: &Stamper) {
        let wd = self.dope_width / self.total_width;
        self.dope_width +=
            s.time_step * self.mobility * self.r_on * self.base.current / self.total_width;
        self.dope_width = self.dope_width.clamp(0.0, self.total_width);
        self.resistance = self.r_on * wd + self.r_off * (1.0 - wd);
    }
    fn do_step(&mut self, s: &mut Stamper) {
        s.stamp_resistor(self.base.nodes[0], self.base.nodes[1], self.resistance);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("memristor".into());
        basic_info(self, arr);
        arr.push(format!("R = {}", unit_text(self.resistance, locale::OHM)));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }
    fn scope_value(&self, x: i32) -> f64 {
        if x == crate::scope::VAL_R {
            self.resistance
        } else {
            match x {
                crate::scope::VAL_CURRENT => self.get_current(),
                crate::scope::VAL_POWER => self.get_power(),
                _ => self.get_voltage_diff(),
            }
        }
    }
    fn scope_units(&self, x: i32) -> i32 {
        match x {
            crate::scope::VAL_R => crate::scope::UNITS_OHMS,
            crate::scope::VAL_CURRENT => crate::scope::UNITS_A,
            crate::scope::VAL_POWER => crate::scope::UNITS_W,
            _ => crate::scope::UNITS_V,
        }
    }
    fn can_show_value_in_scope(&self, v: i32) -> bool {
        v == crate::scope::VAL_R
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Min Resistance (ohms)", self.r_on, 0.0, 0.0)),
            1 => Some(EditInfo::new("Max Resistance (ohms)", self.r_off, 0.0, 0.0)),
            2 => Some(EditInfo::new("Width of Doped Region (nm)", self.dope_width * 1e9, 0.0, 0.0)),
            3 => Some(EditInfo::new("Total Width (nm)", self.total_width * 1e9, 0.0, 0.0)),
            4 => Some(EditInfo::new("Mobility (um^2/(s*V))", self.mobility * 1e12, 0.0, 0.0)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.r_on = ei.value,
            1 => self.r_off = ei.value,
            2 => self.dope_width = ei.value * 1e-9,
            3 => self.total_width = ei.value * 1e-9,
            4 => self.mobility = ei.value * 1e-12,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Lamp
// ---------------------------------------------------------------------------------------------

const ROOM_TEMP: f64 = 300.0;
const FILAMENT_LEN: i32 = 24;

/// Incandescent lamp whose resistance follows the filament temperature.
#[derive(Clone, Debug)]
pub struct LampElm {
    base: Base,
    resistance: f64,
    temp: f64,
    nom_pow: f64,
    nom_v: f64,
    warm_time: f64,
    cool_time: f64,
    filament: [Point; 2],
    bulb: Point,
}

const BULB_R: i32 = 20;

impl LampElm {
    fn blank(base: Base) -> Self {
        LampElm {
            base,
            resistance: 0.0,
            temp: ROOM_TEMP,
            nom_pow: 100.0,
            nom_v: 120.0,
            warm_time: 0.4,
            cool_time: 0.4,
            filament: [Point::default(); 2],
            bulb: Point::default(),
        }
    }

    /// A lamp being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut l = LampElm::blank(Base::new(xx, yy));
        l.update_resistance();
        l
    }

    /// A lamp read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut l = LampElm::blank(Base::from_dump(xa, ya, xb, yb, f));
        l.temp = st.next_f64()?;
        if l.temp.is_nan() {
            l.temp = ROOM_TEMP;
        }
        l.nom_pow = st.next_f64()?;
        l.nom_v = st.next_f64()?;
        l.warm_time = st.next_f64()?;
        l.cool_time = st.next_f64()?;
        l.update_resistance();
        Ok(l)
    }

    fn update_resistance(&mut self) {
        let nom_r = self.nom_v * self.nom_v / self.nom_pow;
        let tp = self.temp.min(5390.0);
        self.resistance = nom_r
            * (1.26104 - 4.90662 * (17.1839 / tp - 0.00318794).sqrt() - 7.8569 / (tp - 187.56));
    }

    fn temp_color(&self) -> Color {
        let t = self.temp;
        if t < 1200.0 {
            Color::new((255.0 * (t - 800.0) / 400.0).max(0.0) as u8, 0, 0)
        } else if t < 1700.0 {
            Color::new(255, (255.0 * (t - 1200.0) / 500.0).max(0.0) as u8, 0)
        } else if t < 2400.0 {
            Color::new(255, 255, (255.0 * (t - 1700.0) / 700.0).max(0.0) as u8)
        } else {
            Color::WHITE
        }
    }
}

impl Element for LampElm {
    impl_element_base!("LampElm");

    fn dump_type(&self) -> i32 {
        181
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.base.dump_prefix(181),
            js_number(self.temp),
            js_number(self.nom_pow),
            js_number(self.nom_v),
            js_number(self.warm_time),
            js_number(self.cool_time)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.temp = xml.double_attr("te", self.temp);
        self.nom_pow = xml.double_attr("np", self.nom_pow);
        self.nom_v = xml.double_attr("nv", self.nom_v);
        self.warm_time = xml.double_attr("wa", self.warm_time);
        self.cool_time = xml.double_attr("co", self.cool_time);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("te", self.temp);
        out.double_attr("np", self.nom_pow);
        out.double_attr("nv", self.nom_v);
        out.double_attr("wa", self.warm_time);
        out.double_attr("co", self.cool_time);
    }

    fn reset(&mut self) {
        self.base.volts.fill(0.0);
        self.base.curcount = 0.0;
        self.temp = ROOM_TEMP;
        self.update_resistance();
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let llen = 16;
        self.base.calc_leads(llen);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        self.filament[0] = interp_point_g(l1, l2, 0.0, f64::from(FILAMENT_LEN));
        self.filament[1] = interp_point_g(l1, l2, 1.0, f64::from(FILAMENT_LEN));
        self.bulb = interp_point(self.filament[0], self.filament[1], 0.5);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (v1, v2) = (self.base.volts[0], self.base.volts[1]);
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, 4.0);
        let b = self.bulb;
        self.base.adjust_bbox(b.x - BULB_R, b.y - BULB_R, b.x + BULB_R, b.y + BULB_R);
        draw_2_leads(g, &self.base, c);
        g.set_color(self.temp_color());
        g.fill_oval(b.x - BULB_R, b.y - BULB_R, BULB_R * 2, BULB_R * 2);
        g.set_color(c.white);
        draw_thick_circle(g, b.x, b.y, BULB_R);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        set_voltage_color(g, &self.base, c, v1);
        draw_thick_line(g, l1, self.filament[0]);
        set_voltage_color(g, &self.base, c, v2);
        draw_thick_line(g, l2, self.filament[1]);
        set_voltage_color(g, &self.base, c, (v1 + v2) * 0.5);
        draw_thick_line(g, self.filament[0], self.filament[1]);
        self.base.update_dot_count(c);
        if !self.base.creating {
            let cc0 = self.base.curcount;
            draw_dots(g, c, p1, l1, cc0);
            let mut cc = Base::add_cur_count(cc0, (self.base.dn - 16.0) / 2.0);
            draw_dots(g, c, l1, self.filament[0], cc);
            cc = Base::add_cur_count(cc, f64::from(FILAMENT_LEN));
            draw_dots(g, c, self.filament[0], self.filament[1], cc);
            cc = Base::add_cur_count(cc, 16.0);
            draw_dots(g, c, self.filament[1], l2, cc);
            draw_dots(g, c, l2, p2, cc0);
        }
        draw_posts(g, &self.base, self, c);
    }

    fn calculate_current(&mut self) {
        self.base.current = if self.resistance == 0.0 {
            0.0
        } else {
            (self.base.volts[0] - self.base.volts[1]) / self.resistance
        };
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn start_iteration(&mut self, s: &Stamper) {
        self.update_resistance();
        let cap = 1.57e-4 * self.nom_pow;
        let capw = cap * self.warm_time / 0.4;
        let capc = cap * self.cool_time / 0.4;
        self.temp += self.get_power() * s.time_step / capw;
        let cr = 2600.0 / self.nom_pow;
        self.temp -= s.time_step * (self.temp - ROOM_TEMP) / (capc * cr);
    }
    fn do_step(&mut self, s: &mut Stamper) {
        s.stamp_resistor(self.base.nodes[0], self.base.nodes[1], self.resistance);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("lamp".into());
        basic_info(self, arr);
        arr.push(format!("R = {}", unit_text(self.resistance, locale::OHM)));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
        arr.push(format!("T = {} K", self.temp as i32));
    }
    fn scope_value(&self, x: i32) -> f64 {
        match x {
            crate::scope::VAL_R => self.resistance,
            crate::scope::VAL_CURRENT => self.get_current(),
            crate::scope::VAL_POWER => self.get_power(),
            _ => self.get_voltage_diff(),
        }
    }
    fn scope_units(&self, x: i32) -> i32 {
        match x {
            crate::scope::VAL_R => crate::scope::UNITS_OHMS,
            crate::scope::VAL_CURRENT => crate::scope::UNITS_A,
            crate::scope::VAL_POWER => crate::scope::UNITS_W,
            _ => crate::scope::UNITS_V,
        }
    }
    fn can_show_value_in_scope(&self, v: i32) -> bool {
        v == crate::scope::VAL_R
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Nominal Power", self.nom_pow, 0.0, 0.0).set_positive()),
            1 => Some(
                EditInfo::new("Nominal Voltage", self.nom_v, 0.0, 0.0)
                    .set_positive()
                    .set_unit_step(),
            ),
            2 => Some(EditInfo::new("Warmup Time (s)", self.warm_time, 0.0, 0.0).set_positive()),
            3 => Some(EditInfo::new("Cooldown Time (s)", self.cool_time, 0.0, 0.0).set_positive()),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if ei.value <= 0.0 {
            return;
        }
        match n {
            0 => self.nom_pow = ei.value,
            1 => self.nom_v = ei.value,
            2 => self.warm_time = ei.value,
            3 => self.cool_time = ei.value,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Analog switches
// ---------------------------------------------------------------------------------------------

const FLAG_INVERT: i32 = 1;
const FLAG_PULLDOWN: i32 = 2;
const FLAG_FLIPPED_X: i32 = 4;
const FLAG_FLIPPED_Y: i32 = 8;
const FLAG_FLIPPED: i32 = 16;

/// Analog switch controlled by a voltage: single throw, or double throw when `spdt`.
#[derive(Clone, Debug)]
pub struct AnalogSwitchElm {
    base: Base,
    spdt: bool,
    resistance: f64,
    r_on: f64,
    r_off: f64,
    threshold: f64,
    open: bool,
    openhs: i32,
    point3: Point,
    lead3: Point,
    swposts: [Point; 2],
    swpoles: [Point; 2],
    ctl_point: Point,
    label_pts: [Point; 2],
}

impl AnalogSwitchElm {
    fn blank(spdt: bool, base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        AnalogSwitchElm {
            base,
            spdt,
            resistance: 0.0,
            r_on: 20.0,
            r_off: 1e10,
            threshold: 2.5,
            open: false,
            openhs: 16,
            point3: Point::default(),
            lead3: Point::default(),
            swposts: [Point::default(); 2],
            swpoles: [Point::default(); 2],
            ctl_point: Point::default(),
            label_pts: [Point::default(); 2],
        }
    }

    /// A switch being placed at (`xx`, `yy`).
    pub fn new(spdt: bool, xx: i32, yy: i32) -> Self {
        let mut s = AnalogSwitchElm::blank(spdt, Base::new(xx, yy));
        s.base.flags |= FLAG_PULLDOWN;
        s
    }

    /// A switch read from a file.
    pub fn from_dump(
        spdt: bool,
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut s = AnalogSwitchElm::blank(spdt, Base::from_dump(xa, ya, xb, yb, f));
        let mut next = || st.next_f64();
        if let (Ok(a), Ok(b), Ok(c)) = (next(), next(), next()) {
            s.r_on = a;
            s.r_off = b;
            s.threshold = c;
        }
        s
    }

    fn has(&self, f: i32) -> bool {
        self.base.flags & f != 0
    }
    fn pulldown(&self) -> bool {
        self.has(FLAG_PULLDOWN)
    }
    fn control(&self) -> usize {
        if self.spdt { 3 } else { 2 }
    }
}

impl Element for AnalogSwitchElm {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        if self.spdt { "AnalogSwitch2Elm" } else { "AnalogSwitchElm" }
    }
    fn dump_type(&self) -> i32 {
        if self.spdt { 160 } else { 159 }
    }
    fn xml_dump_type(&self) -> String {
        if self.spdt { "as2".into() } else { "as".into() }
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.r_on),
            js_number(self.r_off),
            js_number(self.threshold)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.r_on = xml.double_attr("ron", self.r_on);
        self.r_off = xml.double_attr("roff", self.r_off);
        self.threshold = xml.double_attr("th", self.threshold);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("ron", self.r_on);
        out.double_attr("roff", self.r_off);
        out.double_attr("th", self.threshold);
    }

    fn post_count(&self) -> usize {
        if self.spdt { 4 } else { 3 }
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        if self.spdt {
            match n {
                0 => Some(self.base.point1),
                1 | 2 => Some(self.swposts[n - 1]),
                3 => Some(self.ctl_point),
                _ => None,
            }
        } else {
            match n {
                0 => Some(self.base.point1),
                1 => Some(self.base.point2),
                2 => Some(self.point3),
                _ => None,
            }
        }
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(32);
        let fx = self.has(FLAG_FLIPPED_X);
        let fy = self.has(FLAG_FLIPPED_Y);
        self.base.adjust_leads_to_grid(fx, fy);
        self.openhs = if (fx != fy) != self.has(FLAG_FLIPPED) { -16 } else { 16 };
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        self.point3 = interp_point_g(l1, l2, 0.5, f64::from(-self.openhs));
        self.lead3 = interp_point_g(l1, l2, 0.5, f64::from(-self.openhs / 2));
        if self.spdt {
            let (p1, p2) = (self.base.point1, self.base.point2);
            let hs = f64::from(self.openhs);
            let (a, b) = interp_point2(l1, l2, 1.0, hs);
            self.swpoles = [a, b];
            let (a, b) = interp_point2(p1, p2, 1.0, hs);
            self.swposts = [a, b];
            let extra = if self.openhs > 0 { 10.0 } else { -10.0 };
            let (a, b) = interp_point2(p1, p2, 1.0 - 10.0 / self.base.dn, hs + extra);
            self.label_pts = [a, b];
            self.ctl_point = interp_point_g(l1, l2, 0.5, hs);
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(self.openhs));
        if self.spdt {
            set_voltage_color(g, &self.base, c, self.base.volts[0]);
            draw_thick_line(g, p1, self.base.lead1);
            set_voltage_color(g, &self.base, c, self.base.volts[1]);
            draw_thick_line(g, self.swpoles[0], self.swposts[0]);
            set_voltage_color(g, &self.base, c, self.base.volts[2]);
            draw_thick_line(g, self.swpoles[1], self.swposts[1]);
            g.set_color(c.light_gray);
            let pos = usize::from(self.open);
            draw_thick_line(g, self.base.lead1, self.swpoles[pos]);
            self.base.update_dot_count(c);
            draw_dots(g, c, p1, self.base.lead1, self.base.curcount);
            draw_dots(g, c, self.swpoles[pos], self.swposts[pos], self.base.curcount);
            draw_posts(g, &self.base, self, c);
            if self.base.needs_highlight() {
                let inverted = self.has(FLAG_INVERT);
                g.set_color(c.select_color);
                let (a, b) = (self.label_pts[0], self.label_pts[1]);
                let (na, nb) = if inverted { ("NC", "NO") } else { ("NO", "NC") };
                draw_centered_text(g, &mut self.base, na, a.x, a.y, true);
                draw_centered_text(g, &mut self.base, nb, b.x, b.y, true);
            }
            return;
        }
        let hs1 = if self.open { 0 } else { 2 };
        let hs2 = if self.open { self.openhs } else { 2 };
        draw_2_leads(g, &self.base, c);
        g.set_color(c.light_gray);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let a = interp_point_g(l1, l2, 0.0, f64::from(hs1));
        let b = interp_point_g(l1, l2, 1.0, f64::from(hs2));
        draw_thick_line(g, a, b);
        set_voltage_color(g, &self.base, c, self.base.volts[2]);
        draw_thick_line(g, self.point3, self.lead3);
        if !self.open {
            do_dots(g, &mut self.base, c);
        }
        draw_posts(g, &self.base, self, c);
        let _ = (TextAlign::Center, p2);
    }

    fn calculate_current(&mut self) {
        let v = &self.base.volts;
        if self.spdt {
            self.base.current =
                if self.open { (v[0] - v[2]) / self.r_on } else { (v[0] - v[1]) / self.r_on };
            return;
        }
        if self.resistance == 0.0 {
            return;
        }
        self.base.current =
            if self.pulldown() && self.open { 0.0 } else { (v[0] - v[1]) / self.resistance };
    }
    fn non_linear(&self) -> bool {
        true
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if self.pulldown() {
            let n = &self.base.nodes;
            if self.spdt {
                s.stamp_resistor(n[1], NodeId::GROUND, self.r_off);
                s.stamp_resistor(n[2], NodeId::GROUND, self.r_off);
            } else {
                s.stamp_resistor(n[0], NodeId::GROUND, self.r_off);
                s.stamp_resistor(n[1], NodeId::GROUND, self.r_off);
            }
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        self.open = self.base.volts[self.control()] < self.threshold;
        if self.has(FLAG_INVERT) {
            self.open = !self.open;
        }
        if self.spdt {
            let (on, off) = if self.open { (n[2], n[1]) } else { (n[1], n[2]) };
            s.stamp_resistor(n[0], on, self.r_on);
            if !self.pulldown() {
                s.stamp_resistor(n[0], off, self.r_off);
            }
            return;
        }
        if !(self.pulldown() && self.open) {
            self.resistance = if self.open { self.r_off } else { self.r_on };
            s.stamp_resistor(n[0], n[1], self.resistance);
        }
    }

    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        let c = self.control();
        n1 != c && n2 != c
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        if self.spdt { self.pulldown() && (n == 1 || n == 2) } else { self.pulldown() && n < 2 }
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        let cur = self.base.current;
        if self.spdt {
            if n == 0 {
                return -cur;
            }
            return if n == usize::from(self.open) + 1 { cur } else { 0.0 };
        }
        match n {
            2 => 0.0,
            0 => -cur,
            _ => cur,
        }
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        if self.spdt {
            arr.push("analog switch (SPDT)".into());
            arr.push(format!("I = {}", current_d_text(self.get_current())));
            return;
        }
        arr.push("analog switch".into());
        arr.push(if self.open { "open" } else { "closed" }.into());
        arr.push(format!("Vd = {}", voltage_d_text(self.get_voltage_diff())));
        arr.push(format!("I = {}", current_d_text(self.get_current())));
        arr.push(format!("Vc = {}", voltage_text(self.base.volts[2])));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::checkbox(
                if self.spdt { "Swap NO/NC" } else { "Normally closed" },
                self.has(FLAG_INVERT),
            )),
            1 => Some(EditInfo::new("On Resistance (ohms)", self.r_on, 0.0, 0.0).set_positive()),
            2 => Some(EditInfo::new("Off Resistance (ohms)", self.r_off, 0.0, 0.0).set_positive()),
            3 => Some(EditInfo::checkbox("Pulldown Resistor", self.pulldown())),
            4 => Some(EditInfo::new("Threshold", self.threshold, 10.0, -10.0)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.base.flags = ei.change_flag(self.base.flags, FLAG_INVERT),
            1 if ei.value > 0.0 => self.r_on = ei.value,
            2 if ei.value > 0.0 => self.r_off = ei.value,
            3 => self.base.flags = ei.change_flag(self.base.flags, FLAG_PULLDOWN),
            4 => self.threshold = ei.value,
            _ => {}
        }
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIPPED_X;
        self.base.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIPPED_Y;
        self.base.flip_y(center2);
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIPPED;
        self.base.flip_xy(xmy);
        self.set_points();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memristor_dump_round_trips() {
        let m = MemristorElm::new(0, 0);
        let text = m.dump();
        let mut st = Tokenizer::new(&text);
        assert_eq!(st.next_token(), Some("m"));
    }

    #[test]
    fn lamp_resistance_rises_with_temperature() {
        let mut cold = LampElm::new(0, 0);
        let mut hot = LampElm::new(0, 0);
        hot.temp = 2800.0;
        cold.update_resistance();
        hot.update_resistance();
        assert!(hot.resistance > 10.0 * cold.resistance);
    }

    #[test]
    fn switch_posts_follow_throw_count() {
        assert_eq!(AnalogSwitchElm::new(false, 0, 0).post_count(), 3);
        assert_eq!(AnalogSwitchElm::new(true, 0, 0).post_count(), 4);
    }
}
