// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (MosfetElm, JfetElm, MosfetModel).
//! MOSFETs and JFETs (square-law models with optional body diode and gate capacitances).

use super::element::*;
use super::geometry::*;
use super::junction::Diode;
use super::model_edit::ModelEditor;
use super::models::{DiodeModel, MosfetModel};
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Color, Painter};
use crate::solver::stamper::Stamper;

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "mosfets";
const ACTIVE: &str = "Active Components";

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("NMosfetElm", FAMILY, "Add MOSFET (N-Channel)", ACTIVE, |x, y| {
            Box::new(MosfetElm::new(false, false, x, y))
        })
        .dump(i32::from(b'f'), |a, b, c, d, f, st| {
            Ok(Box::new(MosfetElm::from_dump(false, a, b, c, d, f, st)))
        })
        .shortcut('N'),
    );
    r.add(ElementDescriptor::new(
        "PMosfetElm",
        FAMILY,
        "Add MOSFET (P-Channel)",
        ACTIVE,
        |x, y| Box::new(MosfetElm::new(false, true, x, y)),
    ));
    r.add(
        ElementDescriptor::new("NJfetElm", FAMILY, "Add JFET (N-Channel)", ACTIVE, |x, y| {
            Box::new(MosfetElm::new(true, false, x, y))
        })
        .dump(i32::from(b'j'), |a, b, c, d, f, st| {
            Ok(Box::new(MosfetElm::from_dump(true, a, b, c, d, f, st)))
        }),
    );
    r.add(ElementDescriptor::new("PJfetElm", FAMILY, "Add JFET (P-Channel)", ACTIVE, |x, y| {
        Box::new(MosfetElm::new(true, true, x, y))
    }));
}

const FLAG_PNP: i32 = 1;
const FLAG_SHOWVT: i32 = 2;
const FLAG_FLIP: i32 = 8;
const FLAG_DIGITAL_LEGACY: i32 = 4;
const FLAG_HIDE_BULK_LEGACY: i32 = 16;
const FLAG_BODY_DIODE_LEGACY: i32 = 32;
const FLAG_BODY_TERMINAL_LEGACY: i32 = 64;
const FLAG_SHOW_BODY_DIODE_LEGACY: i32 = 128;
const HS: i32 = 16;

/// A MOSFET or, when `jfet` is set, a JFET.
#[derive(Clone, Debug)]
pub struct MosfetElm {
    base: Base,
    jfet: bool,
    pnp: i32,
    body_terminal: usize,
    vt: f64,
    beta: f64,
    cap_volt_gs: f64,
    cap_volt_gd: f64,
    cap_cur_gs: f64,
    cap_cur_gd: f64,
    geq_gs: f64,
    geq_gd: f64,
    ceq_gs: f64,
    ceq_gd: f64,
    diode_b1: Diode,
    diode_b2: Diode,
    diode_current1: f64,
    diode_current2: f64,
    curcount_body1: f64,
    curcount_body2: f64,
    curcount_gate: f64,
    curcount_source: f64,
    curcount_drain: f64,
    model_name: String,
    model: MosfetModel,
    bulk_shown: bool,
    digital_symbol_shown: bool,
    body_diode_simulated: bool,
    body_terminal_shown: bool,
    body_diode_symbol_shown: bool,
    lastv0: f64,
    lastv1: f64,
    lastv2: f64,
    ids: f64,
    mode: i32,
    gm: f64,
    // JFET
    jfet_diode: Diode,
    gate_current: f64,
    curcountg: f64,
    curcounts: f64,
    curcountd: f64,
    gate_poly: Polygon,
    gate_pt: Point,
    // geometry
    pcircler: i32,
    src: [Point; 3],
    drn: [Point; 3],
    gate: [Point; 3],
    body: [Point; 2],
    pcircle: Point,
    arrow_poly: Polygon,
    body_diode_poly: Polygon,
    body_diode_poly2: Polygon,
    body_diode_cathode: [Point; 2],
    body_diode_cathode2: [Point; 2],
    body_diode_leads: [Point; 4],
}

impl MosfetElm {
    fn blank(jfet: bool, base: Base, pnp: i32) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        let mut dflt = Diode::default();
        dflt.setup(&DiodeModel::default_model());
        MosfetElm {
            base,
            jfet,
            pnp,
            body_terminal: 0,
            vt: 0.0,
            beta: 0.0,
            cap_volt_gs: 0.0,
            cap_volt_gd: 0.0,
            cap_cur_gs: 0.0,
            cap_cur_gd: 0.0,
            geq_gs: 0.0,
            geq_gd: 0.0,
            ceq_gs: 0.0,
            ceq_gd: 0.0,
            diode_b1: dflt.clone(),
            diode_b2: dflt.clone(),
            diode_current1: 0.0,
            diode_current2: 0.0,
            curcount_body1: 0.0,
            curcount_body2: 0.0,
            curcount_gate: 0.0,
            curcount_source: 0.0,
            curcount_drain: 0.0,
            model_name: String::new(),
            model: MosfetModel::default_model(jfet),
            bulk_shown: false,
            digital_symbol_shown: false,
            body_diode_simulated: false,
            body_terminal_shown: false,
            body_diode_symbol_shown: false,
            lastv0: 0.0,
            lastv1: 0.0,
            lastv2: 0.0,
            ids: 0.0,
            mode: 0,
            gm: 0.0,
            jfet_diode: dflt,
            gate_current: 0.0,
            curcountg: 0.0,
            curcounts: 0.0,
            curcountd: 0.0,
            gate_poly: Polygon::new(),
            gate_pt: Point::default(),
            pcircler: 0,
            src: [Point::default(); 3],
            drn: [Point::default(); 3],
            gate: [Point::default(); 3],
            body: [Point::default(); 2],
            pcircle: Point::default(),
            arrow_poly: Polygon::new(),
            body_diode_poly: Polygon::new(),
            body_diode_poly2: Polygon::new(),
            body_diode_cathode: [Point::default(); 2],
            body_diode_cathode2: [Point::default(); 2],
            body_diode_leads: [Point::default(); 4],
        }
    }

    /// A transistor being placed at (`xx`, `yy`).
    pub fn new(jfet: bool, pnp: bool, xx: i32, yy: i32) -> Self {
        let mut m = Self::blank(jfet, Base::new(xx, yy), if pnp { -1 } else { 1 });
        m.base.flags = if pnp { FLAG_PNP } else { 0 };
        m.model_name = if jfet { "default-jfet" } else { "default" }.into();
        m.setup();
        m
    }

    /// A transistor read from a file.
    pub fn from_dump(
        jfet: bool,
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let pnp = if f & FLAG_PNP != 0 { -1 } else { 1 };
        let mut m = Self::blank(jfet, Base::from_dump(xa, ya, xb, yb, f), pnp);
        let mut vt0 = if jfet { -4.0 } else { 1.5 };
        let mut beta0 = if jfet { 0.00125 } else { 0.02 };
        // like the original, keep what was read before the first missing field
        if let Ok(v) = st.next_f64() {
            vt0 = v;
            if let Ok(b) = st.next_f64() {
                beta0 = b;
            }
        }
        m.model = m.legacy_model(vt0, beta0);
        m.model_name = m.model.name.clone();
        m.setup();
        m
    }

    fn legacy_model(&mut self, vt0: f64, beta0: f64) -> MosfetModel {
        let f = self.base.flags;
        let show_bulk = !self.jfet && f & (FLAG_DIGITAL_LEGACY | FLAG_HIDE_BULK_LEGACY) == 0;
        let digital = f & FLAG_DIGITAL_LEGACY != 0;
        let body_diode = f & FLAG_BODY_DIODE_LEGACY != 0;
        let body_terminal = f & FLAG_BODY_TERMINAL_LEGACY != 0;
        let show_body_diode = f & FLAG_SHOW_BODY_DIODE_LEGACY != 0;
        self.base.flags &= !(FLAG_DIGITAL_LEGACY
            | FLAG_HIDE_BULK_LEGACY
            | FLAG_BODY_DIODE_LEGACY
            | FLAG_BODY_TERMINAL_LEGACY
            | FLAG_SHOW_BODY_DIODE_LEGACY);
        MosfetModel::with_parameters(
            vt0,
            beta0,
            self.jfet,
            show_bulk,
            body_diode,
            body_terminal,
            digital,
            show_body_diode,
        )
    }

    fn setup(&mut self) {
        self.model = MosfetModel::named_or_copy(&self.model_name, Some(&self.model), self.jfet);
        self.model_name = self.model.name.clone();
        self.vt = self.model.threshold;
        self.beta = self.model.beta;
        self.bulk_shown = self.model.show_bulk;
        self.digital_symbol_shown = !self.bulk_shown && self.model.digital_symbol;
        self.body_diode_simulated = self.bulk_shown && self.model.body_diode;
        self.body_terminal_shown = self.body_diode_simulated && self.model.body_terminal;
        self.body_diode_symbol_shown =
            self.body_diode_simulated && self.model.show_body_diode_symbol;
        self.alloc_nodes();
    }

    fn has_gate_caps(&self) -> bool {
        self.model.cap_gs > 0.0 || self.model.cap_gd > 0.0
    }
    fn show_bulk(&self) -> bool {
        !self.jfet && self.bulk_shown
    }
    fn draw_digital(&self) -> bool {
        self.digital_symbol_shown
    }
    fn has_body_terminal(&self) -> bool {
        self.body_terminal_shown
    }
    fn do_body_diode(&self) -> bool {
        self.body_diode_simulated
    }
    fn show_body_diode(&self) -> bool {
        self.body_diode_symbol_shown
    }

    fn non_convergence(last: f64, now: f64, s: &Stamper, beta: f64) -> bool {
        let mut diff = (last - now).abs();
        if beta > 1.0 {
            diff *= 100.0;
        }
        if diff < 0.01 {
            return false;
        }
        let sub = s.sub_iterations as f64;
        if sub > 10.0 && diff < now.abs() * 0.001 {
            return false;
        }
        if sub > 100.0 && diff < 0.01 + (sub - 100.0) * 0.0001 {
            return false;
        }
        true
    }

    /// Solves the channel for the present voltages; with `finished` set only the currents are
    /// updated, otherwise the linearised channel is stamped.
    fn calculate(&mut self, s: &mut Stamper, finished: bool) {
        let mut vs = [self.base.volts[0], self.base.volts[1], self.base.volts[2]];
        if !finished {
            vs[1] = vs[1].min(self.lastv1 + 0.5).max(self.lastv1 - 0.5);
            vs[2] = vs[2].min(self.lastv2 + 0.5).max(self.lastv2 - 0.5);
        }
        let pnp = f64::from(self.pnp);
        let (mut source, mut drain) = (1_usize, 2_usize);
        if pnp * vs[1] > pnp * vs[2] {
            source = 2;
            drain = 1;
        }
        let gate = 0_usize;
        let mut vgs = vs[gate] - vs[source];
        let mut vds = vs[drain] - vs[source];
        if !finished
            && (Self::non_convergence(self.lastv1, vs[1], s, self.beta)
                || Self::non_convergence(self.lastv2, vs[2], s, self.beta)
                || Self::non_convergence(self.lastv0, vs[0], s, self.beta))
        {
            s.converged = false;
        }
        self.lastv0 = vs[0];
        self.lastv1 = vs[1];
        self.lastv2 = vs[2];
        let (realvgs, realvds) = (vgs, vds);
        vgs *= pnp;
        vds *= pnp;
        let vt = self.vt;
        let beta = self.beta;
        let lambda = self.model.lambda;
        let mut gds;
        self.gm = 0.0;
        if vgs < vt {
            gds = 1e-8;
            self.ids = vds * gds;
            self.mode = 0;
        } else if vds < vgs - vt {
            self.ids = beta * ((vgs - vt) * vds - vds * vds * 0.5) * (1.0 + lambda * vds);
            self.gm = beta * vds * (1.0 + lambda * vds);
            gds = beta
                * ((vgs - vds - vt) * (1.0 + lambda * vds)
                    + lambda * ((vgs - vt) * vds - vds * vds * 0.5));
            self.mode = 1;
        } else {
            let vgs_vt = vgs - vt;
            self.gm = beta * vgs_vt * (1.0 + lambda * vds);
            gds = 0.5 * beta * vgs_vt * vgs_vt * lambda;
            if gds < 1e-8 {
                gds = 1e-8;
            }
            self.ids = 0.5 * beta * vgs_vt * vgs_vt * (1.0 + lambda * vds);
            self.mode = 2;
        }
        let bt = self.body_terminal;
        if self.do_body_diode() {
            let v = &self.base.volts;
            let d1 = pnp * (v[bt] - v[1]);
            let d2 = pnp * (v[bt] - v[2]);
            self.diode_b1.do_step(s, d1);
            self.diode_current1 = self.diode_b1.calculate_current(d1) * pnp;
            self.diode_b2.do_step(s, d2);
            self.diode_current2 = self.diode_b2.calculate_current(d2) * pnp;
        } else {
            self.diode_current1 = 0.0;
            self.diode_current2 = 0.0;
        }
        let ids0 = self.ids;
        if (source == 2 && self.pnp == 1) || (source == 1 && self.pnp == -1) {
            self.ids = -self.ids;
        }
        if finished {
            return;
        }
        let gm = self.gm;
        let rs = -pnp * ids0 + gds * realvds + gm * realvgs;
        let n = self.base.nodes.clone();
        s.stamp_matrix(n[drain], n[drain], gds);
        s.stamp_matrix(n[drain], n[source], -gds - gm);
        s.stamp_matrix(n[drain], n[gate], gm);
        s.stamp_matrix(n[source], n[drain], -gds);
        s.stamp_matrix(n[source], n[source], gds + gm);
        s.stamp_matrix(n[source], n[gate], -gm);
        s.stamp_right_side(n[drain], rs);
        s.stamp_right_side(n[source], -rs);
        if self.model.cap_gs > 0.0 && self.geq_gs > 0.0 {
            s.stamp_matrix(n[0], n[0], self.geq_gs);
            s.stamp_matrix(n[1], n[1], self.geq_gs);
            s.stamp_matrix(n[0], n[1], -self.geq_gs);
            s.stamp_matrix(n[1], n[0], -self.geq_gs);
            s.stamp_right_side(n[0], -self.ceq_gs);
            s.stamp_right_side(n[1], self.ceq_gs);
        }
        if self.model.cap_gd > 0.0 && self.geq_gd > 0.0 {
            s.stamp_matrix(n[0], n[0], self.geq_gd);
            s.stamp_matrix(n[2], n[2], self.geq_gd);
            s.stamp_matrix(n[0], n[2], -self.geq_gd);
            s.stamp_matrix(n[2], n[0], -self.geq_gd);
            s.stamp_right_side(n[0], -self.ceq_gd);
            s.stamp_right_side(n[2], self.ceq_gd);
        }
    }

    fn draw_mosfet(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(HS));
        let v = self.base.volts.clone();
        set_voltage_color(g, &self.base, c, v[1]);
        draw_thick_line(g, self.src[0], self.src[1]);
        set_voltage_color(g, &self.base, c, v[2]);
        draw_thick_line(g, self.drn[0], self.drn[1]);
        let segments = 6;
        let power = self.get_power();
        set_power_color_w(g, &self.base, c, power);
        let show_power = c.show_power;
        let segf = 1.0 / f64::from(segments);
        let enhancement = self.vt > 0.0 && self.show_bulk();
        for i in 0..segments {
            if (i == 1 || i == 4) && enhancement {
                continue;
            }
            let vi = v[1] + (v[2] - v[1]) * f64::from(i) / f64::from(segments);
            if !show_power {
                set_voltage_color(g, &self.base, c, vi);
            }
            let ps1 = interp_point(self.src[1], self.drn[1], f64::from(i) * segf);
            let ps2 = interp_point(self.src[1], self.drn[1], f64::from(i + 1) * segf);
            draw_thick_line(g, ps1, ps2);
        }
        if !show_power {
            set_voltage_color(g, &self.base, c, v[1]);
        }
        draw_thick_line(g, self.src[1], self.src[2]);
        if !show_power {
            set_voltage_color(g, &self.base, c, v[2]);
        }
        draw_thick_line(g, self.drn[1], self.drn[2]);
        let bt = self.body_terminal;
        if self.show_bulk() {
            set_voltage_color(g, &self.base, c, v[bt]);
            if !self.has_body_terminal() {
                let from = if self.pnp == -1 { self.drn[0] } else { self.src[0] };
                draw_thick_line(g, from, self.body[0]);
            }
            draw_thick_line(g, self.body[0], self.body[1]);
        }
        if self.show_body_diode() {
            if !self.has_body_terminal() {
                set_voltage_color(g, &self.base, c, v[1]);
                fill_polygon(g, &self.body_diode_poly);
                draw_thick_line(g, self.src[0], self.body_diode_leads[0]);
                draw_thick_line(g, self.body_diode_leads[0], self.body_diode_leads[1]);
                set_voltage_color(g, &self.base, c, v[2]);
                draw_thick_line(g, self.body_diode_cathode[0], self.body_diode_cathode[1]);
                draw_thick_line(g, self.body_diode_leads[2], self.body_diode_leads[3]);
                draw_thick_line(g, self.drn[0], self.body_diode_leads[3]);
                let (a, b) = (self.body_diode_leads[0], self.body_diode_leads[3]);
                self.base.adjust_bbox_pts(a, b);
            } else {
                let (anode1, cathode1) = if self.pnp == 1 { (bt, 1) } else { (1, bt) };
                set_voltage_color(g, &self.base, c, v[anode1]);
                fill_polygon(g, &self.body_diode_poly);
                set_voltage_color(g, &self.base, c, v[cathode1]);
                let (a, b) = (self.body_diode_cathode[0], self.body_diode_cathode[1]);
                g.draw_line(a.x, a.y, b.x, b.y);
                let (anode2, cathode2) = if self.pnp == 1 { (bt, 2) } else { (2, bt) };
                set_voltage_color(g, &self.base, c, v[anode2]);
                fill_polygon(g, &self.body_diode_poly2);
                set_voltage_color(g, &self.base, c, v[cathode2]);
                let (a, b) = (self.body_diode_cathode2[0], self.body_diode_cathode2[1]);
                g.draw_line(a.x, a.y, b.x, b.y);
            }
        }
        if !self.draw_digital() {
            set_voltage_color(g, &self.base, c, v[bt]);
            fill_polygon(g, &self.arrow_poly);
        }
        if show_power {
            g.set_color(Color::GRAY);
        }
        set_voltage_color(g, &self.base, c, v[0]);
        draw_thick_line(g, p1, self.gate[1]);
        draw_thick_line(g, self.gate[0], self.gate[2]);
        if self.draw_digital() && self.pnp == -1 {
            draw_thick_circle(g, self.pcircle.x, self.pcircle.y, self.pcircler);
        }
        if self.base.flags & FLAG_SHOWVT != 0 {
            let s = format!("{}", self.vt * f64::from(self.pnp));
            g.set_color(c.white);
            let (x2, y2) = (self.base.x2, self.base.y2);
            g.set_font_size(c.value_font_size);
            draw_centered_text(g, &mut self.base, &s, x2 + 2, y2, false);
        }
        self.curcount_source =
            self.base.update_dot_count_for(-(self.ids + self.cap_cur_gs), self.curcount_source, c);
        self.curcount_drain =
            self.base.update_dot_count_for(-self.ids + self.cap_cur_gd, self.curcount_drain, c);
        draw_dots(g, c, self.src[0], self.src[1], self.curcount_source);
        draw_dots(g, c, self.src[1], self.drn[1], self.curcount_source);
        draw_dots(g, c, self.drn[1], self.drn[0], self.curcount_drain);
        if self.model.cap_gs > 0.0 || self.model.cap_gd > 0.0 {
            self.curcount_gate = self.base.update_dot_count_for(
                self.cap_cur_gs + self.cap_cur_gd,
                self.curcount_gate,
                c,
            );
            draw_dots(g, c, p1, self.gate[1], self.curcount_gate);
        }
        if self.show_bulk() {
            self.curcount_body1 =
                self.base.update_dot_count_for(self.diode_current1, self.curcount_body1, c);
            self.curcount_body2 =
                self.base.update_dot_count_for(self.diode_current2, self.curcount_body2, c);
            if self.show_body_diode() && !self.has_body_terminal() {
                let cur = -self.curcount_body1 + self.curcount_body2;
                let l = self.body_diode_leads;
                draw_dots(g, c, self.src[0], l[0], cur);
                draw_dots(g, c, l[0], l[3], cur);
                draw_dots(g, c, l[3], self.drn[0], cur);
            } else {
                draw_dots(g, c, self.src[0], self.body[0], -self.curcount_body1);
                draw_dots(g, c, self.body[0], self.drn[0], self.curcount_body2);
            }
        }
        if self.base.needs_highlight() || self.base.creating {
            g.set_color(c.white);
            g.set_font_size(c.value_font_size);
            let b = &self.base;
            let (dsx, dsy) = (sign(b.dx), sign(b.dy));
            let dsyn = i32::from(b.dy != 0);
            let gt = self.gate[1];
            g.draw_string(
                "G",
                f64::from(gt.x - if b.dx < 0 { -2 } else { 12 }),
                f64::from(gt.y + if b.dy > 0 { -5 } else { 12 }),
            );
            let _ = dsy;
            let extra = if self.show_body_diode() && !self.has_body_terminal() && b.dy == 0 {
                16 * b.dsign
            } else {
                0
            };
            let off = -3 + 9 * (dsx - dsyn * self.pnp) + extra;
            g.draw_string(
                if self.pnp == -1 { "D" } else { "S" },
                f64::from(self.src[0].x + off),
                f64::from(self.src[0].y + 4),
            );
            g.draw_string(
                if self.pnp == -1 { "S" } else { "D" },
                f64::from(self.drn[0].x + off),
                f64::from(self.drn[0].y + 4),
            );
            if self.has_body_terminal() {
                g.draw_string(
                    "B",
                    f64::from(self.body[0].x - 3 + 9 * (dsx - dsyn * self.pnp)),
                    f64::from(self.body[0].y + 4),
                );
            }
        }
        draw_posts(g, &self.base, self, c);
    }

    fn draw_jfet(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(HS));
        let v = self.base.volts.clone();
        set_voltage_color(g, &self.base, c, v[1]);
        draw_thick_line(g, self.src[0], self.src[1]);
        draw_thick_line(g, self.src[1], self.src[2]);
        set_voltage_color(g, &self.base, c, v[2]);
        draw_thick_line(g, self.drn[0], self.drn[1]);
        draw_thick_line(g, self.drn[1], self.drn[2]);
        set_voltage_color(g, &self.base, c, v[0]);
        draw_thick_line(g, p1, self.gate_pt);
        fill_polygon(g, &self.arrow_poly);
        set_power_color_w(g, &self.base, c, self.get_power());
        fill_polygon(g, &self.gate_poly);
        self.curcountd = self.base.update_dot_count_for(-self.ids, self.curcountd, c);
        self.curcountg = self.base.update_dot_count_for(self.gate_current, self.curcountg, c);
        self.curcounts =
            self.base.update_dot_count_for(-self.gate_current - self.ids, self.curcounts, c);
        if self.curcountd != 0.0 || self.curcounts != 0.0 {
            draw_dots(g, c, self.src[0], self.src[1], self.curcounts);
            draw_dots(g, c, self.src[1], self.src[2], Base::add_cur_count(self.curcounts, 8.0));
            draw_dots(g, c, self.drn[0], self.drn[1], -self.curcountd);
            draw_dots(g, c, self.drn[1], self.drn[2], -Base::add_cur_count(self.curcountd, 8.0));
            draw_dots(g, c, p1, self.gate_pt, self.curcountg);
        }
        if self.base.needs_highlight() {
            g.set_color(c.white);
            g.set_font_size(c.value_font_size);
            let b = &self.base;
            let (dsx, dsy) = (sign(b.dx), sign(b.dy));
            let _ = dsy;
            let dsyn = i32::from(b.dy != 0);
            let gt = self.gate[1];
            let (gx, gy) = (
                f64::from(gt.x - if b.dx < 0 { -2 } else { 12 }),
                f64::from(gt.y + if b.dy > 0 { -5 } else { 12 }),
            );
            let (sn, dn_) =
                (if self.pnp == -1 { "D" } else { "S" }, if self.pnp == -1 { "S" } else { "D" });
            if b.dy == 0 {
                g.draw_string("G", gx, gy);
                let off = -3 + 9 * (dsx - dsyn * self.pnp);
                g.draw_string(sn, f64::from(self.src[0].x + off), f64::from(self.src[0].y + 4));
                g.draw_string(dn_, f64::from(self.drn[0].x + off), f64::from(self.drn[0].y + 4));
            } else if b.dx == 0 {
                g.draw_string("G", gx, gy);
                let dy_off = |y: i32| f64::from(y - if b.dy < 0 { 7 } else { -14 });
                g.draw_string(sn, f64::from(self.src[0].x - 4), dy_off(self.src[0].y));
                g.draw_string(dn_, f64::from(self.drn[0].x - 4), dy_off(self.drn[0].y));
            }
        }
        draw_posts(g, &self.base, self, c);
    }

    fn set_points_mosfet(&mut self) {
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let hs = f64::from(HS);
        let mut hs2 = HS * self.base.dsign;
        if self.base.flags & FLAG_FLIP != 0 {
            hs2 = -hs2;
        }
        let hs2f = f64::from(hs2);
        let (s0, d0) = interp_point2(p1, p2, 1.0, -hs2f);
        let (s1, d1) = interp_point2(p1, p2, 1.0 - 22.0 / dn, -hs2f);
        let (s2, d2) = interp_point2(p1, p2, 1.0 - 22.0 / dn, -hs2f * 4.0 / 3.0);
        self.src = [s0, s1, s2];
        self.drn = [d0, d1, d2];
        let (g0, g2) = interp_point2(p1, p2, 1.0 - 28.0 / dn, hs2f / 2.0);
        let g1 = interp_point(g0, g2, 0.5);
        self.gate = [g0, g1, g2];
        if self.show_bulk() {
            self.body = [
                interp_point(self.src[0], self.drn[0], 0.5),
                interp_point(self.src[1], self.drn[1], 0.5),
            ];
        }
        if !self.draw_digital() {
            self.arrow_poly = if self.pnp == 1 {
                if !self.show_bulk() {
                    calc_arrow(self.src[1], self.src[0], 10.0, 4.0)
                } else {
                    calc_arrow(self.body[0], self.body[1], 12.0, 5.0)
                }
            } else if !self.show_bulk() {
                calc_arrow(self.drn[0], self.drn[1], 12.0, 5.0)
            } else {
                calc_arrow(self.body[1], self.body[0], 12.0, 5.0)
            };
        } else if self.pnp == -1 {
            self.gate[1] = interp_point(p1, p2, 1.0 - 36.0 / dn);
            let dist = if self.base.dsign < 0 { 32.0 } else { 31.0 };
            self.pcircle = interp_point(p1, p2, 1.0 - dist / dn);
            self.pcircler = 3;
        }
        if self.show_body_diode() {
            if !self.has_body_terminal() {
                let diode_hs = 6.0;
                let dp1 =
                    interp_point_g(self.src[0], self.drn[0], 0.5 - (diode_hs / 2.0) / hs, -hs2f);
                let dp2 =
                    interp_point_g(self.src[0], self.drn[0], 0.5 + (diode_hs / 2.0) / hs, -hs2f);
                let (pa0, pa1) = interp_point2(dp1, dp2, 0.0, diode_hs);
                let (c0, c1) = interp_point2(dp1, dp2, 1.0, diode_hs);
                self.body_diode_cathode = [c0, c1];
                self.body_diode_poly = Polygon::from_points(&[pa0, pa1, dp2]);
                self.body_diode_leads = [
                    interp_point_g(self.src[0], self.drn[0], 0.0, -hs2f),
                    dp1,
                    dp2,
                    interp_point_g(self.src[0], self.drn[0], 1.0, -hs2f),
                ];
            } else {
                let diode_hs = 3.0;
                let (a1, b1) = if self.pnp == 1 {
                    (self.body[0], self.src[0])
                } else {
                    (self.src[0], self.body[0])
                };
                let dp1 = interp_point(a1, b1, 0.3);
                let dp2 = interp_point(a1, b1, 0.7);
                let (pa0, pa1) = interp_point2(dp1, dp2, 0.0, diode_hs);
                let (c0, c1) = interp_point2(dp1, dp2, 1.0, diode_hs);
                self.body_diode_cathode = [c0, c1];
                self.body_diode_poly = Polygon::from_points(&[pa0, pa1, dp2]);
                let (a2, b2) = if self.pnp == 1 {
                    (self.body[0], self.drn[0])
                } else {
                    (self.drn[0], self.body[0])
                };
                let dp1 = interp_point(a2, b2, 0.3);
                let dp2 = interp_point(a2, b2, 0.7);
                let (pa0, pa1) = interp_point2(dp1, dp2, 0.0, diode_hs);
                let (c0, c1) = interp_point2(dp1, dp2, 1.0, diode_hs);
                self.body_diode_cathode2 = [c0, c1];
                self.body_diode_poly2 = Polygon::from_points(&[pa0, pa1, dp2]);
            }
        }
    }

    fn set_points_jfet(&mut self) {
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let mut hs2 = HS * self.base.dsign;
        if self.base.flags & FLAG_FLIP != 0 {
            hs2 = -hs2;
        }
        let hs2f = f64::from(hs2);
        let (s0, d0) = interp_point2(p1, p2, 1.0, -hs2f);
        let (s1, d1) = interp_point2(p1, p2, 1.0, -hs2f / 2.0);
        let (s2, d2) = interp_point2(p1, p2, 1.0 - 10.0 / dn, -hs2f / 2.0);
        self.src = [s0, s1, s2];
        self.drn = [d0, d1, d2];
        // the gate pins of the MOSFET layout are still used for the labels
        let (g0, g2) = interp_point2(p1, p2, 1.0 - 28.0 / dn, hs2f / 2.0);
        self.gate = [g0, interp_point(g0, g2, 0.5), g2];
        self.gate_pt = interp_point(p1, p2, 1.0 - 14.0 / dn);
        let (r0, r1) = interp_point2(p1, p2, 1.0 - 13.0 / dn, f64::from(HS));
        let (r2, r3) = interp_point2(p1, p2, 1.0 - 10.0 / dn, f64::from(HS));
        self.gate_poly = Polygon::from_points(&[r0, r1, r3, r2]);
        self.arrow_poly = if self.pnp == -1 {
            let x = interp_point(self.gate_pt, p1, 18.0 / dn);
            calc_arrow(self.gate_pt, x, 12.0, 5.0)
        } else {
            calc_arrow(p1, self.gate_pt, 12.0, 5.0)
        };
    }
}

impl Element for MosfetElm {
    impl_element_base!();

    fn visual_state(&self) -> Vec<f64> {
        vec![self.ids, self.cap_cur_gs, self.cap_cur_gd, self.diode_current1, self.diode_current2]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [ids, gs, gd, d1, d2] = state[..] {
            (self.ids, self.cap_cur_gs, self.cap_cur_gd) = (ids, gs, gd);
            (self.diode_current1, self.diode_current2) = (d1, d2);
        }
    }

    fn model_buttons(&self) -> Vec<String> {
        let mut v = vec!["Create New Model".to_string()];
        if !self.model.read_only {
            v.push("Edit Model".into());
        }
        v
    }
    fn start_model_edit(&self, button: usize) -> Option<ModelEditor> {
        match button {
            0 => Some(ModelEditor::new_mosfet(&self.model)),
            1 if !self.model.read_only => Some(ModelEditor::Mosfet(self.model.clone())),
            _ => None,
        }
    }
    fn use_model(&mut self, name: &str) {
        self.model_name = name.to_string();
        self.setup();
        self.set_points();
    }
    fn update_models(&mut self) {
        self.setup();
        self.set_points();
    }

    fn class_name(&self) -> &'static str {
        match (self.jfet, self.pnp == -1) {
            (false, false) => "NMosfetElm",
            (false, true) => "PMosfetElm",
            (true, false) => "NJfetElm",
            (true, true) => "PJfetElm",
        }
    }
    fn dump_type(&self) -> i32 {
        i32::from(if self.jfet { b'j' } else { b'f' })
    }
    fn xml_dump_type(&self) -> String {
        if self.jfet { "j" } else { "f" }.into()
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        // old files carry the threshold and beta; newer ones refer to a model by name
        format!(
            "{} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.vt),
            js_number(self.beta)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", 0);
        match xml.attr("mo") {
            Some(name) => self.model_name = name.to_string(),
            None => {
                let vt0 = xml.double_attr("vt", if self.jfet { -4.0 } else { 1.5 });
                let beta0 = xml.double_attr("be", if self.jfet { 0.00125 } else { 0.02 });
                self.model = self.legacy_model(vt0, beta0);
                self.model_name = self.model.name.clone();
            }
        }
        self.setup();
        self.pnp = if self.base.flags & FLAG_PNP != 0 { -1 } else { 1 };
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.str_attr("mo", &self.model_name);
    }
    fn dump_model_xml(&self) -> Option<XmlOut> {
        (!self.model.built_in).then(|| self.model.dump_xml())
    }

    fn post_count(&self) -> usize {
        if self.has_body_terminal() { 4 } else { 3 }
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.base.point1,
            1 => self.src[0],
            2 => self.drn[0],
            _ => self.body[0],
        })
    }

    fn set_points(&mut self) {
        self.base.no_diagonal = true;
        self.base.set_points();
        if self.jfet {
            self.set_points_jfet();
        } else {
            self.set_points_mosfet();
        }
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.lastv1 = 0.0;
        self.lastv2 = 0.0;
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
        self.curcount_body1 = 0.0;
        self.curcount_body2 = 0.0;
        self.curcount_gate = 0.0;
        self.curcount_source = 0.0;
        self.curcount_drain = 0.0;
        self.cap_volt_gs = 0.0;
        self.cap_volt_gd = 0.0;
        self.cap_cur_gs = 0.0;
        self.cap_cur_gd = 0.0;
        self.geq_gs = 0.0;
        self.geq_gd = 0.0;
        self.ceq_gs = 0.0;
        self.ceq_gd = 0.0;
        self.diode_b1.reset();
        self.diode_b2.reset();
        self.jfet_diode.reset();
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        if self.jfet {
            self.draw_jfet(g, c);
        } else {
            self.draw_mosfet(g, c);
        }
    }

    fn get_current(&self) -> f64 {
        self.ids
    }
    fn get_power(&self) -> f64 {
        let v = &self.base.volts;
        let bt = self.body_terminal.min(v.len() - 1);
        self.ids * (v[2] - v[1])
            - self.diode_current1 * (v[1] - v[bt])
            - self.diode_current2 * (v[2] - v[bt])
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[2] - self.base.volts[1]
    }

    fn start_iteration(&mut self, s: &Stamper) {
        if s.time_step <= 0.0 {
            return;
        }
        if self.model.cap_gs > 0.0 {
            self.geq_gs = self.model.cap_gs / s.time_step;
            self.ceq_gs = -self.geq_gs * self.cap_volt_gs;
        }
        if self.model.cap_gd > 0.0 {
            self.geq_gd = self.model.cap_gd / s.time_step;
            self.ceq_gd = -self.geq_gd * self.cap_volt_gd;
        }
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        s.stamp_non_linear(n[1]);
        s.stamp_non_linear(n[2]);
        if self.has_gate_caps() {
            s.stamp_non_linear(n[0]);
        }
        self.body_terminal = if self.has_body_terminal() {
            3
        } else if self.pnp == -1 {
            2
        } else {
            1
        };
        if self.do_body_diode() {
            let bt = self.body_terminal;
            if self.pnp == -1 {
                self.diode_b1.stamp(s, n[1], n[bt]);
                self.diode_b2.stamp(s, n[2], n[bt]);
            } else {
                self.diode_b1.stamp(s, n[bt], n[1]);
                self.diode_b2.stamp(s, n[bt], n[2]);
            }
        }
        if self.jfet {
            if self.pnp < 0 {
                self.jfet_diode.stamp(s, n[1], n[0]);
            } else {
                self.jfet_diode.stamp(s, n[0], n[1]);
            }
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        self.calculate(s, false);
        if self.jfet {
            let v = f64::from(self.pnp) * (self.base.volts[0] - self.base.volts[1]);
            self.jfet_diode.do_step(s, v);
        }
    }

    fn calculate_current(&mut self) {
        if self.jfet {
            let v = f64::from(self.pnp) * (self.base.volts[0] - self.base.volts[1]);
            self.gate_current = f64::from(self.pnp) * self.jfet_diode.calculate_current(v);
        }
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        self.calculate(s, true);
        if self.body_terminal == 1 {
            self.diode_current1 = -self.diode_current2;
        }
        if self.body_terminal == 2 {
            self.diode_current2 = -self.diode_current1;
        }
        let v = &self.base.volts;
        if self.model.cap_gs > 0.0 && self.geq_gs > 0.0 {
            self.cap_volt_gs = v[0] - v[1];
            self.cap_cur_gs = self.geq_gs * self.cap_volt_gs + self.ceq_gs;
        }
        if self.model.cap_gd > 0.0 && self.geq_gd > 0.0 {
            self.cap_volt_gd = v[0] - v[2];
            self.cap_cur_gd = self.geq_gd * self.cap_volt_gd + self.ceq_gd;
        }
    }

    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        if self.jfet || self.has_gate_caps() {
            return true;
        }
        !(n1 == 0 || n2 == 0)
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn can_view_in_scope(&self) -> bool {
        true
    }
    fn elm_type(&self) -> Option<String> {
        Some(if self.jfet { "JFET" } else { "MOSFET" }.into())
    }
    fn scope_text(&self, _v: i32) -> Option<String> {
        let kind = if self.jfet { "JFET" } else { "MOSFET" };
        Some(crate::locale::ls(&format!("{}{kind}", if self.pnp == -1 { "p-" } else { "n-" })))
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let kind = if self.jfet { "JFET" } else { "MOSFET" };
        let pnp = f64::from(self.pnp);
        let v = &self.base.volts;
        arr.push(format!(
            "{} ({})",
            crate::locale::ls(&format!("{}{kind}", if self.pnp == -1 { "p-" } else { "n-" })),
            self.model_name
        ));
        arr.push(format!("Vt={}, \u{3b2}={}", voltage_text(pnp * self.vt), self.beta));
        arr.push(format!(
            "{} = {}",
            if self.pnp == 1 { "Ids" } else { "Isd" },
            current_text(self.ids)
        ));
        arr.push(format!("Vgs = {}", voltage_text(v[0] - v[if self.pnp == -1 { 2 } else { 1 }])));
        arr.push(format!(
            "{} = {}",
            if self.pnp == 1 { "Vds" } else { "Vsd" },
            voltage_text(v[2] - v[1])
        ));
        arr.push(crate::locale::ls(match self.mode {
            0 => "off",
            1 => "linear",
            _ => "saturation",
        }));
        arr.push(format!("gm = {}", unit_text(self.gm, "A/V")));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
        if self.show_bulk() {
            let ib = match self.body_terminal {
                1 => -self.diode_current1,
                2 => self.diode_current2,
                _ => -pnp * (self.diode_current1 + self.diode_current2),
            };
            arr.push(format!("Ib = {}", unit_text(ib, "A")));
        }
        if self.model.cap_gs > 0.0 {
            arr.push(format!("Cgs = {}", unit_text(self.model.cap_gs, "F")));
        }
        if self.model.cap_gd > 0.0 {
            arr.push(format!("Cgd = {}", unit_text(self.model.cap_gd, "F")));
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        if n == 0 {
            let models = MosfetModel::list(self.jfet);
            let selected = models.iter().position(|m| m.name == self.model_name).unwrap_or(0);
            return Some(EditInfo::choice(
                "Model",
                models.iter().map(MosfetModel::description_text).collect(),
                selected,
            ));
        }
        (n == 1).then(|| EditInfo::checkbox("Swap D/S", self.base.flags & FLAG_FLIP != 0))
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            let models = MosfetModel::list(self.jfet);
            if let Some(m) = models.get(ei.choice_index) {
                self.model = m.clone();
                self.model_name = m.name.clone();
                self.setup();
                ei.new_dialog = true;
            }
        } else if n == 1 {
            self.base.flags = ei.change_flag(self.base.flags, FLAG_FLIP);
        }
        self.alloc_nodes();
        self.set_points();
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        if self.jfet {
            return match n {
                0 => -self.gate_current,
                1 => self.gate_current + self.ids,
                _ => -self.ids,
            };
        }
        let v = &self.base.volts;
        if n == 0 {
            let mut gate = 0.0;
            if self.model.cap_gs > 0.0 && self.geq_gs > 0.0 {
                gate -= self.geq_gs * (v[0] - v[1]) + self.ceq_gs;
            }
            if self.model.cap_gd > 0.0 && self.geq_gd > 0.0 {
                gate -= self.geq_gd * (v[0] - v[2]) + self.ceq_gd;
            }
            return gate;
        }
        if n == 3 {
            return -self.diode_current1 - self.diode_current2;
        }
        if n == 1 {
            let cap = if self.model.cap_gs > 0.0 && self.geq_gs > 0.0 {
                self.geq_gs * (v[0] - v[1]) + self.ceq_gs
            } else {
                0.0
            };
            return self.ids + self.diode_current1 + cap;
        }
        let cap = if self.model.cap_gd > 0.0 && self.geq_gd > 0.0 {
            self.geq_gd * (v[0] - v[2]) + self.ceq_gd
        } else {
            0.0
        };
        -self.ids + self.diode_current2 + cap
    }

    fn on_flipped(&mut self, axis: Flip) {
        let flip = match axis {
            Flip::X => self.base.x == self.base.x2,
            Flip::Y => self.base.y == self.base.y2,
            Flip::Xy => true,
        };
        if flip {
            self.base.flags ^= FLAG_FLIP;
        }
    }

    fn shortcut(&self) -> i32 {
        match (self.jfet, self.pnp == -1) {
            (false, false) => i32::from(b'N'),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::parser::parse;
    use crate::solver::sim::Simulation;

    const HDR: &str = "$ 1 0.000005 10 50 5 50 5e-11";

    fn run(text: &str, steps: usize) -> Simulation {
        let r = parse(text, &Registry::standard());
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        sim.analyze().unwrap();
        sim.run_steps(steps).unwrap();
        sim
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    /// N-channel MOSFET with the gate at 3 V, drain through 1 k to 10 V, source grounded. Vt is
    /// 1.5 V and beta 0.02: in saturation Id = beta/2 (Vgs - Vt)^2 = 22.5 mA would pull the
    /// drain far below Vgs - Vt, so the device is in the linear region.
    #[test]
    fn mosfet_pulls_its_drain_down_when_the_gate_is_high() {
        // f from the gate post (64, 32) to (128, 32): source and drain posts at (128, 16/48)
        let text = format!(
            "{HDR}\nv 0 160 0 -64 0 0 40 10 0 0 0.5\nv -64 160 -64 32 0 0 40 3 0 0 0.5\nw -64 32 64 32 0\nr 0 -64 128 -64 0 1000\nw 128 -64 128 16 0\nf 64 32 128 32 0 1.5 0.02\nw 0 160 -64 160 0\nw 128 48 128 160 0\nw 0 160 128 160 0\n"
        );
        let sim = run(&text, 300);
        let m = sim.elements.iter().find(|e| e.dump_type() == i32::from(b'f')).unwrap();
        let v = &m.base().volts;
        let vds = (v[2] - v[1]).abs();
        let id = m.get_current().abs();
        assert!(id > 1e-3, "no channel current: {id}");
        assert!(vds < 8.0, "drain not pulled down: {vds}");
        // off when the gate is low
        let off = run(&text.replace("40 3 0 0 0.5", "40 0 0 0 0.5"), 300);
        let m = off.elements.iter().find(|e| e.dump_type() == i32::from(b'f')).unwrap();
        assert!(m.get_current().abs() < 1e-6);
    }

    #[test]
    fn legacy_lines_get_a_model_with_the_old_parameters() {
        let mut st = Tokenizer::new("2.5 0.01");
        let m = MosfetElm::from_dump(false, 0, 0, 64, 0, 0, &mut st);
        assert_eq!((m.vt, m.beta), (2.5, 0.01));
        assert!(m.model.old_style);
        let mut st = Tokenizer::new("");
        let m = MosfetElm::from_dump(false, 0, 0, 64, 0, 0, &mut st);
        // no body-diode flag in an old file: the built-in model without a body diode matches
        assert_eq!(m.model_name, "default-nodiode");
        let mut st = Tokenizer::new("");
        let j = MosfetElm::from_dump(true, 0, 0, 64, 0, 0, &mut st);
        assert_eq!(j.model_name, "default-jfet");
        assert_eq!(j.class_name(), "NJfetElm");
    }

    #[test]
    fn pmos_and_njfet_variants_are_named_and_have_the_right_posts() {
        assert_eq!(MosfetElm::new(false, true, 0, 0).class_name(), "PMosfetElm");
        assert_eq!(MosfetElm::new(true, true, 0, 0).class_name(), "PJfetElm");
        let mut body = MosfetElm::new(false, false, 0, 0);
        assert_eq!(body.post_count(), 3);
        body.model_name = "default-body".into();
        body.setup();
        assert_eq!(body.post_count(), 4);
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    #[test]
    fn jfet_conducts_with_zero_gate_voltage() {
        // n-channel JFET: Vt = -4 V, so with the gate at the source it conducts
        let text = format!(
            "{HDR}\nv 0 160 0 -64 0 0 40 10 0 0 0.5\nr 0 -64 128 -64 0 1000\nw 128 -64 128 16 0\nj 64 32 128 32 0 -4 0.00125\nw 64 32 64 48 0\nw 64 48 128 48 0\nw 128 48 128 160 0\nw 0 160 128 160 0\n"
        );
        let sim = run(&text, 300);
        let j = sim.elements.iter().find(|e| e.dump_type() == i32::from(b'j')).unwrap();
        assert!(j.get_current().abs() > 1e-4, "{}", j.get_current());
    }
}
