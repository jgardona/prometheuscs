// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (ThreePhaseMotorElm,
// MotorProtectionSwitchElm).
//! The three-phase induction motor and the motor protection switch.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Color, LineCap, Painter, TextAlign, TextBaseline};
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name used for the feature switch.
pub const FAMILY: &str = "outputs";
const CAT: &str = "Outputs and Labels";

/// Registers the motors.
pub fn register(r: &mut Registry) {
    #[cfg(feature = "reactive")]
    r.add(
        ElementDescriptor::new("DCMotorElm", FAMILY, "Add DC Motor", CAT, |x, y| {
            Box::new(DcMotor::new(x, y))
        })
        .dump(415, |a, b, c, d, f, st| Ok(Box::new(DcMotor::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("ThreePhaseMotorElm", FAMILY, "Add 3-Phase Motor", CAT, |x, y| {
            Box::new(ThreePhaseMotor::new(x, y))
        })
        .dump(427, |a, b, c, d, f, st| Ok(Box::new(ThreePhaseMotor::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new(
            "MotorProtectionSwitchElm",
            FAMILY,
            "Add Motor Protection Switch",
            CAT,
            |x, y| Box::new(MotorProtectionSwitch::new(x, y)),
        )
        .dump(428, |a, b, c, d, f, st| {
            Ok(Box::new(MotorProtectionSwitch::from_dump(a, b, c, d, f, st)))
        }),
    );
}

// ---------------------------------------------------------------------------------------------
// Three-phase motor
// ---------------------------------------------------------------------------------------------

const ZP: f64 = 2.0;
const COILS: usize = 5;
const COIL_NODES: [usize; 10] = [6, 1, 8, 3, 10, 5, 7, 9, 11, 12];
const N002: usize = 7;
const N006: usize = 11;
const CR: i32 = 37;

/// Inverts a small square matrix in place (Gauss-Jordan with partial pivoting). Returns false
/// when it is singular.
fn invert(m: &mut [[f64; COILS]; COILS]) -> bool {
    let n = COILS;
    let mut a = [[0.0; 2 * COILS]; COILS];
    for i in 0..n {
        a[i][..n].copy_from_slice(&m[i]);
        a[i][n + i] = 1.0;
    }
    for col in 0..n {
        let Some(piv) = (col..n).max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))
        else {
            return false;
        };
        if a[piv][col].abs() < 1e-300 {
            return false;
        }
        a.swap(col, piv);
        let d = a[col][col];
        for v in &mut a[col] {
            *v /= d;
        }
        for r in 0..n {
            if r != col {
                let f = a[r][col];
                if f != 0.0 {
                    for k in 0..2 * n {
                        let t = a[col][k];
                        a[r][k] -= f * t;
                    }
                }
            }
        }
    }
    for i in 0..n {
        m[i].copy_from_slice(&a[i][n..]);
    }
    true
}

/// A three-phase induction motor (stator and rotor coils, rotor speed and angle).
pub struct ThreePhaseMotor {
    base: Base,
    rs: f64,
    rr: f64,
    ls: f64,
    lr: f64,
    lm: f64,
    friction: f64,
    inertia: f64,
    angle: f64,
    speed: f64,
    filtered_speed: f64,
    posts: [Point; 6],
    leads: [Point; 6],
    center: Point,
    curcounts: [f64; 3],
    coil_currents: [f64; COILS],
    coil_sources: [f64; COILS],
    xform: [[f64; COILS]; COILS],
    sources: [Option<VsId>; 2],
    vs_values: [f64; 2],
}

impl ThreePhaseMotor {
    fn blank(base: Base) -> Self {
        ThreePhaseMotor {
            base,
            rs: 0.435,
            rr: 0.816,
            ls: 0.0294,
            lr: 0.0297,
            lm: 0.0287,
            friction: 0.05,
            inertia: 1.0,
            angle: std::f64::consts::FRAC_PI_2,
            speed: 0.0,
            filtered_speed: 0.0,
            posts: [Point::default(); 6],
            leads: [Point::default(); 6],
            center: Point::default(),
            curcounts: [0.0; 3],
            coil_currents: [0.0; COILS],
            coil_sources: [0.0; COILS],
            xform: [[0.0; COILS]; COILS],
            sources: [None; 2],
            vs_values: [0.0; 2],
        }
    }

    /// A motor being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y))
    }

    /// A motor read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut m = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        m.rs = st.next_f64().unwrap_or(m.rs);
        m.rr = st.next_f64().unwrap_or(m.rr);
        m.ls = st.next_f64().unwrap_or(m.ls);
        m.lr = st.next_f64().unwrap_or(m.lr);
        m.lm = st.next_f64().unwrap_or(m.lm);
        m.friction = st.next_f64().unwrap_or(m.friction);
        m.inertia = st.next_f64().unwrap_or(1.0);
        m
    }
}

fn interp_fix(a: Point, b: Point, f: f64, g: f64) -> Point {
    let gx = f64::from(b.y - a.y);
    let gy = f64::from(a.x - b.x);
    Point::new(
        (f64::from(a.x) * (1.0 - f) + f64::from(b.x) * f + g * gx).round() as i32,
        (f64::from(a.y) * (1.0 - f) + f64::from(b.y) * f + g * gy).round() as i32,
    )
}

fn rpm_text(v: f64) -> String {
    let va = v.abs();
    if va < 1e-14 {
        return "0 RPM".into();
    }
    unit_text(v, "RPM")
}

impl Element for ThreePhaseMotor {
    impl_element_base!("ThreePhaseMotorElm");

    fn dump_type(&self) -> i32 {
        427
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {} {} {} {}",
            self.base.dump_prefix(427),
            js_number(self.rs),
            js_number(self.rr),
            js_number(self.ls),
            js_number(self.lr),
            js_number(self.lm),
            js_number(self.friction),
            js_number(self.inertia)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("rs", self.rs);
        out.double_attr("rr", self.rr);
        out.double_attr("ls", self.ls);
        out.double_attr("lr", self.lr);
        out.double_attr("lm", self.lm);
        out.double_attr("b", self.friction);
        out.double_attr("j", self.inertia);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.rs = xml.double_attr("rs", self.rs);
        self.rr = xml.double_attr("rr", self.rr);
        self.ls = xml.double_attr("ls", self.ls);
        self.lr = xml.double_attr("lr", self.lr);
        self.lm = xml.double_attr("lm", self.lm);
        self.friction = xml.double_attr("b", self.friction);
        self.inertia = xml.double_attr("j", self.inertia);
    }

    fn post_count(&self) -> usize {
        6
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.posts.get(n).copied()
    }
    fn internal_node_count(&self) -> usize {
        7
    }
    fn voltage_source_count(&self) -> usize {
        2
    }
    fn can_flip_x(&self) -> bool {
        false
    }
    fn can_flip_y(&self) -> bool {
        false
    }
    fn non_linear(&self) -> bool {
        false
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let (p1, p2) = (self.base.point1, self.base.point2);
        let q = if self.base.dy.abs() > self.base.dx.abs() { -1 } else { 1 };
        for i in 0..3 {
            let k = f64::from(q * 32 * (i as i32 - 1));
            self.posts[i * 2] = interp_point_g(p1, p2, 0.0, -k);
            self.leads[i * 2] = interp_point_g(p1, p2, 0.45, -k);
            self.posts[i * 2 + 1] = interp_point_g(p1, p2, 1.0, k);
            self.leads[i * 2 + 1] = interp_point_g(p1, p2, 0.55, k);
        }
        self.center = interp_point(p1, p2, 0.5);
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.speed = 0.0;
        self.filtered_speed = 0.0;
        self.coil_sources = [0.0; COILS];
        self.coil_currents = [0.0; COILS];
    }

    fn set_voltage_source(&mut self, n: usize, v: VsId) {
        if let Some(s) = self.sources.get_mut(n) {
            *s = Some(v);
        }
    }
    fn voltage_source_nodes_at(&self, j: usize) -> Option<(NodeId, NodeId)> {
        let n = &self.base.nodes;
        Some((n[if j == 0 { N002 } else { N006 }], NodeId::GROUND))
    }

    // based on https://forum.kicad.info/t/ac-motors-simulation-1-phase-3-phase/14188/3
    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        s.stamp_resistor(n[0], n[6], self.rs);
        s.stamp_resistor(n[2], n[8], self.rs);
        s.stamp_resistor(n[4], n[10], self.rs);
        s.stamp_resistor(n[9], NodeId::GROUND, 1.5 * self.rr);
        s.stamp_resistor(n[12], NodeId::GROUND, 1.5 * self.rr);
        let lr2 = self.lr * 1.5;
        let ind = [self.ls, self.ls, self.ls, lr2, lr2];
        let mut coupling = [[0.0; COILS]; COILS];
        let mut m = [[0.0; COILS]; COILS];
        for i in 0..COILS {
            m[i][i] = ind[i];
        }
        let k0 = self.lm / (self.ls * lr2).sqrt();
        let r3 = 3f64.sqrt();
        coupling[0][3] = k0;
        coupling[3][0] = k0;
        coupling[1][3] = -k0 / 2.0;
        coupling[3][1] = -k0 / 2.0;
        coupling[1][4] = k0 * r3 / 2.0;
        coupling[4][1] = k0 * r3 / 2.0;
        coupling[2][3] = -k0 / 2.0;
        coupling[3][2] = -k0 / 2.0;
        coupling[2][4] = -k0 * r3 / 2.0;
        coupling[4][2] = -k0 * r3 / 2.0;
        for i in 0..COILS {
            for j in 0..i {
                let v = coupling[i][j] * (ind[i] * ind[j]).sqrt();
                m[i][j] = v;
                m[j][i] = v;
            }
        }
        if !invert(&mut m) {
            s.stop("Singular motor inductance matrix", None);
            return;
        }
        let ts = s.time_step;
        for i in 0..COILS {
            for j in 0..COILS {
                m[i][j] *= ts;
                let (ni1, nj1) = (COIL_NODES[i * 2], COIL_NODES[j * 2]);
                let (ni2, nj2) = (COIL_NODES[i * 2 + 1], COIL_NODES[j * 2 + 1]);
                if i == j {
                    s.stamp_conductance(n[ni1], n[ni2], m[i][i]);
                } else {
                    s.stamp_vccs(n[ni1], n[ni2], n[nj1], n[nj2], m[i][j]);
                }
            }
        }
        self.xform = m;
        if let Some(vs) = self.sources[0] {
            s.stamp_voltage_source_var(n[N002], NodeId::GROUND, vs);
        }
        if let Some(vs) = self.sources[1] {
            s.stamp_voltage_source_var(n[N006], NodeId::GROUND, vs);
        }
    }

    fn start_iteration(&mut self, s: &Stamper) {
        self.coil_sources = self.coil_currents;
        let c = &self.coil_currents;
        let r3 = 3f64.sqrt();
        let torque = ZP * r3 / 2.0 * self.lm * ((c[1] - c[2]) * c[3] - r3 * c[0] * c[4]);
        self.speed += s.time_step * (torque - self.friction * self.speed) / self.inertia;
        self.angle += self.speed * s.time_step;
        self.vs_values[0] =
            -ZP * self.speed * (self.lm * r3 / 2.0 * (c[1] - c[2]) + 1.5 * self.lr * c[4]);
        self.vs_values[1] = ZP * self.speed * (1.5 * self.lm * c[0] + 1.5 * self.lr * c[3]);
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        for i in 0..COILS {
            s.stamp_current_source(
                n[COIL_NODES[i * 2]],
                n[COIL_NODES[i * 2 + 1]],
                self.coil_sources[i],
            );
        }
        for j in 0..2 {
            if let Some(vs) = self.sources[j] {
                s.update_voltage_source(vs, -self.vs_values[j]);
            }
        }
    }

    fn calculate_current(&mut self) {
        let v = self.base.volts.clone();
        for i in 0..COILS {
            let mut val = self.coil_sources[i];
            for j in 0..COILS {
                let diff = v[COIL_NODES[j * 2]] - v[COIL_NODES[j * 2 + 1]];
                val += diff * self.xform[i][j];
            }
            self.coil_currents[i] = val;
        }
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        match n {
            0..=5 if n % 2 == 1 => self.coil_currents[n / 2],
            0..=5 => -self.coil_currents[n / 2],
            _ => 0.0,
        }
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn has_ground_connection(&self, _n1: usize) -> bool {
        false
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.angle, self.filtered_speed, self.speed]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [angle, filtered, speed] = state[..] {
            self.angle = angle;
            self.filtered_speed = filtered;
            self.speed = speed;
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(CR));
        let v = self.base.volts.clone();
        for i in 0..6 {
            set_voltage_color(g, &self.base, c, v[i]);
            draw_thick_line(g, self.posts[i], self.leads[i]);
        }
        for i in 0..3 {
            self.curcounts[i] =
                self.base.update_dot_count_for(self.coil_currents[i], self.curcounts[i], c);
            draw_dots(g, c, self.posts[i * 2], self.leads[i * 2], self.curcounts[i]);
            draw_dots(g, c, self.leads[i * 2 + 1], self.posts[i * 2 + 1], self.curcounts[i]);
        }
        let power = self.get_power();
        set_power_color_w(g, &self.base, c, power);
        g.set_color(Color::new(165, 165, 165));
        g.fill_oval(self.center.x - CR, self.center.y - CR, CR * 2, CR * 2);
        g.set_color(Color::new(10, 10, 10));
        let r = (f64::from(CR) / 2.2) as i32;
        g.fill_oval(self.center.x - r, self.center.y - r, 2 * r, 2 * r);
        let angle = (self.angle * 300.0).round() / 300.0;
        let q = 0.28 * 1.7 * 36.0 / self.base.dn * 37.0 / 27.0;
        for k in 0..3 {
            let a = angle + f64::from(k) * std::f64::consts::PI / 3.0;
            let a1 = interp_fix(p1, p2, 0.5 + q * a.cos(), q * a.sin());
            let a2 = interp_fix(p1, p2, 0.5 - q * a.cos(), -q * a.sin());
            draw_thick_line_w(g, a1, a2, 6.0);
        }
        draw_posts(g, &self.base, self, c);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
        g.save();
        let names = ["U", "V", "W"];
        if self.base.dy.abs() > self.base.dx.abs() {
            for i in 0..3 {
                g.draw_string(
                    &format!("{}1", names[i]),
                    f64::from(self.posts[i * 2].x + 5),
                    f64::from(self.posts[i * 2].y + 8),
                );
                g.draw_string(
                    &format!("{}2", names[i]),
                    f64::from(self.posts[i * 2 + 1].x + 5),
                    f64::from(self.posts[i * 2 + 1].y - 2),
                );
            }
        } else {
            g.set_text_align(TextAlign::Center);
            for i in 0..3 {
                g.draw_string(
                    &format!("{}1", names[i]),
                    f64::from(self.posts[i * 2].x + 11),
                    f64::from(self.posts[i * 2].y - 7),
                );
                g.draw_string(
                    &format!("{}2", names[i]),
                    f64::from(self.posts[i * 2 + 1].x - 11),
                    f64::from(self.posts[i * 2 + 1].y - 7),
                );
            }
        }
        g.restore();
        self.filtered_speed = self.filtered_speed * 0.98 + self.speed * 0.02;
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("3-Phase Motor".into());
        basic_info(self, arr);
        arr.push(format!(
            "speed = {}",
            rpm_text(60.0 * self.filtered_speed.abs() / std::f64::consts::TAU)
        ));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Stator Inductance (H)", self.ls, 0.0, 0.0)),
            1 => Some(EditInfo::new("Rotor Inductance (H)", self.lr, 0.0, 0.0)),
            2 => Some(
                EditInfo::new(
                    "Coupling Coefficient",
                    self.lm / (self.ls * self.lr).sqrt(),
                    0.0,
                    0.0,
                )
                .set_dimensionless(),
            ),
            3 => Some(EditInfo::new("Stator Resistance (ohms)", self.rs, 0.0, 0.0)),
            4 => Some(EditInfo::new("Rotor Resistance (ohms)", self.rr, 0.0, 0.0)),
            5 => Some(EditInfo::new("Friction coefficient (Nms/rad)", self.friction, 0.0, 0.0)),
            6 => Some(EditInfo::new("Moment of inertia (Kg.m^2)", self.inertia, 0.0, 0.0)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        let v = ei.value;
        match n {
            0 if v > 0.0 => self.ls = v,
            1 if v > 0.0 => self.lr = v,
            2 if v > 0.0 && v < 1.0 => self.lm = v * (self.ls * self.lr).sqrt(),
            3 if v > 0.0 => self.rs = v,
            4 if v > 0.0 => self.rr = v,
            5 => self.friction = v,
            6 if v > 0.0 => self.inertia = v,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Motor protection switch
// ---------------------------------------------------------------------------------------------

const BLOWN_RESISTANCE: f64 = 1e9;

/// A three-pole thermal protection switch: it opens the contacts labelled like it when the
/// I²t of any pole is used up.
pub struct MotorProtectionSwitch {
    base: Base,
    resistance: f64,
    i2t: f64,
    heats: [f64; 3],
    currents: [f64; 3],
    curcounts: [f64; 3],
    blown: bool,
    label: String,
    posts: [Point; 6],
    leads: [Point; 6],
}

impl MotorProtectionSwitch {
    fn blank(base: Base) -> Self {
        MotorProtectionSwitch {
            base,
            resistance: 0.0613,
            i2t: 6.73,
            heats: [0.0; 3],
            currents: [0.0; 3],
            curcounts: [0.0; 3],
            blown: false,
            label: String::new(),
            posts: [Point::default(); 6],
            leads: [Point::default(); 6],
        }
    }

    /// A switch being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y))
    }

    /// A switch read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut m = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        m.resistance = st.next_f64().unwrap_or(m.resistance);
        m.i2t = st.next_f64().unwrap_or(m.i2t);
        m.blown = st.next_token().is_some_and(|t| t.eq_ignore_ascii_case("true"));
        m.label = st.next_token().map(unescape).unwrap_or_default();
        m
    }

    fn push_positions(&self) {
        #[cfg(all(feature = "switches", feature = "reactive"))]
        super::relays::set_contact_link(&self.label, if self.blown { 0 } else { 1 }, 0);
    }

    fn temperature_color(&self, g_color: Color, pole: usize) -> Color {
        let c = g_color;
        let temp = self.heats[pole] / self.i2t;
        if temp < 0.3333 {
            let x = ((temp * 3.0) * 255.0).max(0.0) as i32;
            let mix = |ch: u8| ((255 - x) * i32::from(ch) / 255).clamp(0, 255) as u8;
            Color::new(
                (x + (255 - x) * i32::from(c.r) / 255).clamp(0, 255) as u8,
                mix(c.g),
                mix(c.b),
            )
        } else if temp < 0.6667 {
            let x = (((temp - 0.3333) * 3.0 * 255.0).max(0.0)) as i32;
            Color::new(255, x.clamp(0, 255) as u8, 0)
        } else if temp < 1.0 {
            let x = (((temp - 0.6666) * 3.0 * 255.0).max(0.0)) as i32;
            Color::new(255, 255, x.clamp(0, 255) as u8)
        } else {
            Color::WHITE
        }
    }
}

impl Element for MotorProtectionSwitch {
    impl_element_base!("MotorProtectionSwitchElm");

    fn dump_type(&self) -> i32 {
        428
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(428),
            js_number(self.resistance),
            js_number(self.i2t),
            self.blown,
            escape(&self.label)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("re", self.resistance);
        out.double_attr("i2", self.i2t);
        out.bool_attr("bl", self.blown);
        out.str_attr("la", &self.label);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.resistance = xml.double_attr("re", self.resistance);
        self.i2t = xml.double_attr("i2", self.i2t);
        self.blown = xml.bool_attr("bl", self.blown);
        self.label = xml.string_attr("la", &self.label);
    }

    fn post_count(&self) -> usize {
        6
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.posts.get(n).copied()
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn can_flip_x(&self) -> bool {
        false
    }
    fn can_flip_y(&self) -> bool {
        false
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.heats = [0.0; 3];
        self.currents = [0.0; 3];
        self.blown = false;
        self.push_positions();
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let (x, y) = (self.base.x, self.base.y);
        for i in 0..3 {
            let dx = 48 * i as i32;
            self.posts[i * 2] = Point::new(x + dx, y);
            self.posts[i * 2 + 1] = Point::new(x + dx, y + 192);
            self.leads[i * 2] = Point::new(x + dx, y + 80);
            self.leads[i * 2 + 1] = Point::new(x + dx, y + 176);
        }
        self.alloc_nodes();
    }

    fn stamp(&mut self, s: &mut Stamper) {
        for &n in &self.base.nodes.clone() {
            s.stamp_non_linear(n);
        }
    }

    fn start_iteration(&mut self, s: &Stamper) {
        let was = self.blown;
        for j in 0..3 {
            let i = self.currents[j];
            let mut heat = self.heats[j] + i * i * s.time_step;
            // the fuse is assumed to shed its whole I2t in 3 seconds
            heat -= s.time_step * self.i2t / 3.0;
            if heat < 0.0 {
                heat = 0.0;
            }
            if heat > self.i2t {
                self.blown = true;
            }
            self.heats[j] = heat;
        }
        if self.blown != was {
            self.push_positions();
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let r = if self.blown { BLOWN_RESISTANCE } else { self.resistance };
        let n = self.base.nodes.clone();
        for i in 0..3 {
            s.stamp_resistor(n[i * 2], n[i * 2 + 1], r);
        }
    }

    fn calculate_current(&mut self) {
        let r = if self.blown { BLOWN_RESISTANCE } else { self.resistance };
        for i in 0..3 {
            self.currents[i] = (self.base.volts[i * 2] - self.base.volts[i * 2 + 1]) / r;
        }
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n % 2 == 1 { self.currents[n / 2] } else { -self.currents[n / 2] }
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 / 2 == n2 / 2
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.heats[0], self.heats[1], self.heats[2], if self.blown { 1.0 } else { 0.0 }]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [a, b, c, blown] = state[..] {
            self.heats = [a, b, c];
            self.blown = blown != 0.0;
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let hs = 6.0;
        let (p0, p5) = (self.posts[0], self.posts[5]);
        self.base.set_bbox_pts(p0, p5, hs);
        g.save();
        g.translate(f64::from(self.base.x), f64::from(self.base.y));
        let spx = 48;
        let (square_x, square_y) = (-spx - 12, 48 - 12);
        g.set_line_dash(4, 4);
        g.set_color(Color::LIGHT_GRAY);
        let blown_off = if self.blown { 8 } else { 0 };
        g.draw_line(square_x + 24 + 4, square_y + 12, 48 * 2 - blown_off, square_y + 12);
        g.draw_line(square_x + 12, square_y + 24 + 4, square_x + 12, 80 + 48 + 48 / 2);
        g.draw_line(square_x + 12, 80 + 48 + 48 / 2, -spx / 2, 80 + 48 + 48 / 2);
        g.draw_line(square_x + 12, 80 + 48 / 2, -spx / 2, 80 + 48 / 2);
        g.set_line_dash(0, 0);
        for i in 0..3 {
            let x = i as i32 * spx;
            set_voltage_color(g, &self.base, c, self.base.volts[i * 2]);
            draw_thick_line(g, Point::new(x, 0), Point::new(x, 32));
            if self.blown {
                draw_thick_line(g, Point::new(x - 4, 32), Point::new(x + 4, 32));
            }
            let sw = if self.blown { 16 } else { 0 };
            draw_thick_line(g, Point::new(x - sw, 32), Point::new(x, 64));
            g.set_line_cap(LineCap::Butt);
            draw_thick_line(g, Point::new(x, 64), Point::new(x, 80));
            g.draw_line(x - 4, 16 - 4, x + 4, 16 + 4);
            g.draw_line(x + 4, 16 - 4, x - 4, 16 + 4);
            set_voltage_color(g, &self.base, c, self.base.volts[i * 2 + 1]);
            draw_thick_line(g, Point::new(x, 176), Point::new(x, 192));
            g.set_line_cap(LineCap::Round);
            let volt = voltage_color(&self.base, c, self.base.volts[i * 2]);
            g.set_color(self.temperature_color(volt, i));
            g.draw_line(x, 80, x, 96);
            let q = 12;
            g.draw_line(x - q, 96, x, 96);
            g.draw_line(x - q, 96, x - q, 112);
            g.draw_line(x - q, 112, x, 112);
            g.draw_line(x, 112, x, 128);
            g.set_color(Color::LIGHT_GRAY);
            g.set_font_size(30);
            g.set_text_baseline(TextBaseline::Middle);
            g.set_text_align(TextAlign::Center);
            g.draw_string("I >", f64::from(x), f64::from(80 + 48 + 48 / 2));
        }
        g.set_color(Color::LIGHT_GRAY);
        for i in 0..3 {
            g.draw_line(-spx / 2, 80 + 48 * i, 2 * spx + spx / 2, 80 + 48 * i);
        }
        for i in 0..4 {
            g.draw_line(i * spx - spx / 2, 80, i * spx - spx / 2, 80 + 48 * 2);
        }
        for i in 0..3 {
            g.draw_line(square_x + 12 * i, square_y, square_x + 12 * i, square_y + 24);
        }
        for i in 0..3 {
            g.draw_line(square_x, square_y + 12 * i, square_x + 24, square_y + 12 * i);
        }
        g.draw_line(square_x - spx / 2, square_y + 12, square_x, square_y + 12);
        g.draw_line(square_x - spx / 2, square_y, square_x - spx / 2, square_y + 24);
        g.set_font_size(12);
        g.set_color(Color::WHITE);
        g.draw_string(&self.label, 120.0, f64::from(square_y + 12));
        g.restore();
        if !self.blown {
            for i in 0..3 {
                self.curcounts[i] =
                    self.base.update_dot_count_for(self.currents[i], self.curcounts[i], c);
                draw_dots(g, c, self.posts[i * 2], self.leads[i * 2], self.curcounts[i]);
                draw_dots(g, c, self.posts[i * 2 + 1], self.leads[i * 2 + 1], -self.curcounts[i]);
            }
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("motor protection switch".into());
        basic_info(self, arr);
        arr.push(format!("R = {}", unit_text(self.resistance, crate::locale::OHM)));
        arr.push(format!("I2t = {}", self.i2t));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("I2t", self.i2t, 0.0, 0.0).set_positive()),
            1 => Some(EditInfo::new("On Resistance", self.resistance, 0.0, 0.0).set_positive()),
            2 => Some(EditInfo::text("Label (for linking)", &self.label)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.i2t = ei.value,
            1 => self.resistance = ei.value,
            2 => {
                if let Some(t) = &ei.text {
                    self.label.clone_from(t);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// DC motor
// ---------------------------------------------------------------------------------------------

/// A brushed DC motor modelled by its electrical and mechanical equivalent circuits: the
/// armature (inductor, resistor and back-EMF source) and, as a circuit too, the rotor inertia
/// (an inductor whose current is the speed), friction and torque source.
#[cfg(feature = "reactive")]
pub struct DcMotor {
    base: Base,
    ind: super::reactive::Inductor,
    ind_inertia: super::reactive::Inductor,
    inductance: f64,
    resistance: f64,
    k: f64,
    kb: f64,
    j: f64,
    b: f64,
    gear_ratio: f64,
    tau: f64,
    angle: f64,
    speed: f64,
    coil_current: f64,
    inertia_current: f64,
    sources: [Option<VsId>; 2],
    center: Point,
}

#[cfg(feature = "reactive")]
impl DcMotor {
    fn blank(base: Base) -> Self {
        DcMotor {
            base,
            ind: Default::default(),
            ind_inertia: Default::default(),
            inductance: 0.5,
            resistance: 1.0,
            k: 0.15,
            kb: 0.15,
            j: 0.02,
            b: 0.05,
            gear_ratio: 1.0,
            tau: 0.0,
            angle: std::f64::consts::FRAC_PI_2,
            speed: 0.0,
            coil_current: 0.0,
            inertia_current: 0.0,
            sources: [None; 2],
            center: Point::default(),
        }
    }

    fn setup_inductors(&mut self) {
        self.ind.setup(self.inductance, 0.0, super::reactive::FLAG_BACK_EULER);
        self.ind_inertia.setup(self.j, 0.0, super::reactive::FLAG_BACK_EULER);
    }

    /// A motor being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut m = Self::blank(Base::new(x, y));
        m.setup_inductors();
        m
    }

    /// A motor read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut m = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        m.inductance = st.next_f64().unwrap_or(m.inductance);
        m.resistance = st.next_f64().unwrap_or(m.resistance);
        m.k = st.next_f64().unwrap_or(m.k);
        m.kb = st.next_f64().unwrap_or(m.kb);
        m.j = st.next_f64().unwrap_or(m.j);
        m.b = st.next_f64().unwrap_or(m.b);
        m.gear_ratio = st.next_f64().unwrap_or(m.gear_ratio);
        m.tau = st.next_f64().unwrap_or(m.tau);
        m.setup_inductors();
        m
    }
}

#[cfg(feature = "reactive")]
impl Element for DcMotor {
    impl_element_base!("DCMotorElm");

    fn dump_type(&self) -> i32 {
        415
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {} {} {} {} {}",
            self.base.dump_prefix(415),
            js_number(self.inductance),
            js_number(self.resistance),
            js_number(self.k),
            js_number(self.kb),
            js_number(self.j),
            js_number(self.b),
            js_number(self.gear_ratio),
            js_number(self.tau)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("in", self.inductance);
        out.double_attr("rs", self.resistance);
        out.double_attr("k", self.k);
        out.double_attr("kb", self.kb);
        out.double_attr("j", self.j);
        out.double_attr("b", self.b);
        out.double_attr("gr", self.gear_ratio);
        out.double_attr("ta", self.tau);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.inductance = xml.double_attr("in", self.inductance);
        self.resistance = xml.double_attr("rs", self.resistance);
        self.k = xml.double_attr("k", self.k);
        self.kb = xml.double_attr("kb", self.kb);
        self.j = xml.double_attr("j", self.j);
        self.b = xml.double_attr("b", self.b);
        self.gear_ratio = xml.double_attr("gr", self.gear_ratio);
        self.tau = xml.double_attr("ta", self.tau);
        self.setup_inductors();
    }

    fn post_count(&self) -> usize {
        2
    }
    fn internal_node_count(&self) -> usize {
        4
    }
    fn voltage_source_count(&self) -> usize {
        2
    }
    fn non_linear(&self) -> bool {
        false
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(36);
        self.center = interp_point(self.base.point1, self.base.point2, 0.5);
        self.alloc_nodes();
    }

    fn set_voltage_source(&mut self, n: usize, v: VsId) {
        if let Some(s) = self.sources.get_mut(n) {
            *s = Some(v);
        }
    }
    fn voltage_source_nodes_at(&self, j: usize) -> Option<(NodeId, NodeId)> {
        let n = &self.base.nodes;
        Some(if j == 0 { (n[3], n[1]) } else { (n[4], NodeId::GROUND) })
    }
    fn set_current(&mut self, vs: VsId, c: f64) {
        if self.sources[0] == Some(vs) {
            self.base.current = c;
        }
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
        self.ind.reset();
        self.ind_inertia.reset();
        self.coil_current = 0.0;
        self.inertia_current = 0.0;
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        // electrical part: inductor, resistor and the back-EMF source in series
        self.ind.stamp(s, n[0], n[2]);
        s.stamp_resistor(n[2], n[3], self.resistance);
        if let Some(vs) = self.sources[0] {
            s.stamp_voltage_source_var(n[3], n[1], vs);
        }
        // mechanical part: the inertia inductor, friction and the torque source
        self.ind_inertia.stamp(s, n[4], n[5]);
        s.stamp_resistor(n[5], NodeId::GROUND, self.b);
        if let Some(vs) = self.sources[1] {
            s.stamp_voltage_source_var(n[4], NodeId::GROUND, vs);
        }
    }

    fn start_iteration(&mut self, s: &Stamper) {
        let v = self.base.volts.clone();
        self.ind.start_iteration(s, v[0] - v[2]);
        self.ind_inertia.start_iteration(s, v[4] - v[5]);
        self.angle += self.speed * s.time_step;
    }

    fn do_step(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.sources[1] {
            s.update_voltage_source(vs, self.coil_current * self.k);
        }
        if let Some(vs) = self.sources[0] {
            s.update_voltage_source(vs, self.inertia_current * self.kb);
        }
        self.ind.do_step(s);
        self.ind_inertia.do_step(s);
    }

    fn calculate_current(&mut self) {
        let v = self.base.volts.clone();
        self.coil_current = self.ind.calculate_current(v[0] - v[2]);
        self.inertia_current = self.ind_inertia.calculate_current(v[4] - v[5]);
        self.speed = self.inertia_current;
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.angle, self.speed]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [angle, speed] = state[..] {
            self.angle = angle;
            self.speed = speed;
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let cr = 18;
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(cr));
        draw_2_leads(g, &self.base, c);
        do_dots(g, &mut self.base, c);
        let power = self.get_power();
        set_power_color_w(g, &self.base, c, power);
        g.set_color(Color::new(165, 165, 165));
        g.fill_oval(self.center.x - cr, self.center.y - cr, cr * 2, cr * 2);
        g.set_color(Color::new(10, 10, 10));
        let r = (f64::from(cr) / 2.2) as i32;
        g.fill_oval(self.center.x - r, self.center.y - r, 2 * r, 2 * r);
        let angle = (self.angle * 300.0).round() / 300.0 * self.gear_ratio;
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        for k in 0..3 {
            let a = angle + f64::from(k) * std::f64::consts::PI / 3.0;
            let a1 = interp_fix(l1, l2, 0.5 + 0.28 * a.cos(), 0.28 * a.sin());
            let a2 = interp_fix(l1, l2, 0.5 - 0.28 * a.cos(), -0.28 * a.sin());
            draw_thick_line_w(g, a1, a2, 6.0);
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("DC Motor".into());
        basic_info(self, arr);
        arr.push(format!("speed = {}", rpm_text(60.0 * self.speed.abs() / std::f64::consts::TAU)));
        arr.push(format!("L = {}", unit_text(self.inductance, "H")));
        arr.push(format!("R = {}", unit_text(self.resistance, crate::locale::OHM)));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("Armature inductance (H)", self.inductance, 0.0, 0.0).set_positive(),
            ),
            1 => Some(
                EditInfo::new("Armature Resistance (ohms)", self.resistance, 0.0, 0.0)
                    .set_positive(),
            ),
            2 => Some(EditInfo::new("Torque constant (Nm/A)", self.k, 0.0, 0.0).set_positive()),
            3 => Some(EditInfo::new("Moment of inertia (Kg.m^2)", self.j, 0.0, 0.0).set_positive()),
            4 => Some(
                EditInfo::new("Friction coefficient (Nms/rad)", self.b, 0.0, 0.0).set_positive(),
            ),
            5 => Some(EditInfo::new("Gear Ratio", self.gear_ratio, 0.0, 0.0).set_positive()),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if ei.value <= 0.0 {
            return;
        }
        match n {
            0 => {
                self.inductance = ei.value;
                self.ind.setup(
                    self.inductance,
                    self.base.current,
                    super::reactive::FLAG_BACK_EULER,
                );
            }
            1 => self.resistance = ei.value,
            2 => {
                self.k = ei.value;
                self.kb = ei.value;
            }
            3 => {
                self.j = ei.value;
                self.ind_inertia.setup(
                    self.j,
                    self.inertia_current,
                    super::reactive::FLAG_BACK_EULER,
                );
            }
            4 => self.b = ei.value,
            5 => self.gear_ratio = ei.value,
            _ => {}
        }
    }
}
