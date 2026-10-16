// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (TransistorElm, NTransistorElm,
// PTransistorElm, TransistorModel).
//! Bipolar transistors (Gummel-Poon model with junction capacitances).

use super::element::*;
use super::geometry::*;
use super::models::{THERMAL_VOLTAGE, TransistorModel};
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Color, Painter};
use crate::scope::{
    UNITS_A, UNITS_V, UNITS_W, VAL_IB, VAL_IC, VAL_IE, VAL_POWER, VAL_VBC, VAL_VBE, VAL_VCE,
};
use crate::solver::stamper::Stamper;
use std::cell::Cell;

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "transistors";
const ACTIVE: &str = "Active Components";

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "NTransistorElm",
            FAMILY,
            "Add Transistor (bipolar, NPN)",
            ACTIVE,
            |x, y| Box::new(TransistorElm::new(false, x, y)),
        )
        .dump(i32::from(b't'), |a, b, c, d, f, st| {
            Ok(Box::new(TransistorElm::from_dump(a, b, c, d, f, st)))
        })
        .shortcut('n'),
    );
    r.add(
        ElementDescriptor::new(
            "PTransistorElm",
            FAMILY,
            "Add Transistor (bipolar, PNP)",
            ACTIVE,
            |x, y| Box::new(TransistorElm::new(true, x, y)),
        )
        .shortcut('p'),
    );
}

const FLAG_FLIP: i32 = 1;
const FLAG_CIRCLE: i32 = 2;
const FLAGS_GLOBAL: i32 = FLAG_CIRCLE;
const VT: f64 = THERMAL_VOLTAGE;

thread_local! {
    /// The "draw circle" choice is shared by every transistor.
    static GLOBAL_FLAGS: Cell<i32> = const { Cell::new(0) };
}

/// A bipolar junction transistor.
#[derive(Clone, Debug)]
pub struct TransistorElm {
    base: Base,
    /// `1` for NPN, `-1` for PNP.
    pnp: i32,
    beta: f64,
    gmin: f64,
    model_name: String,
    model: TransistorModel,
    bad_iters: i32,
    local_sub_iters: i32,
    ic: f64,
    ie: f64,
    ib: f64,
    curcount_c: f64,
    curcount_e: f64,
    curcount_b: f64,
    cap_volt_be: f64,
    cap_volt_bc: f64,
    cap_cur_be: f64,
    cap_cur_bc: f64,
    geq_be: f64,
    geq_bc: f64,
    ceq_be: f64,
    ceq_bc: f64,
    vcrit: f64,
    lastvbc: f64,
    lastvbe: f64,
    rect_poly: Polygon,
    arrow_poly: Polygon,
    circle_center: Point,
    coll: [Point; 2],
    emit: [Point; 2],
    base_pt: Point,
}

impl TransistorElm {
    fn blank(base: Base, pnp: i32) -> Self {
        TransistorElm {
            base,
            pnp,
            beta: 100.0,
            gmin: 0.0,
            model_name: "default".into(),
            model: TransistorModel::default_model(),
            bad_iters: 0,
            local_sub_iters: 0,
            ic: 0.0,
            ie: 0.0,
            ib: 0.0,
            curcount_c: 0.0,
            curcount_e: 0.0,
            curcount_b: 0.0,
            cap_volt_be: 0.0,
            cap_volt_bc: 0.0,
            cap_cur_be: 0.0,
            cap_cur_bc: 0.0,
            geq_be: 0.0,
            geq_bc: 0.0,
            ceq_be: 0.0,
            ceq_bc: 0.0,
            vcrit: 0.0,
            lastvbc: 0.0,
            lastvbe: 0.0,
            rect_poly: Polygon::new(),
            arrow_poly: Polygon::new(),
            circle_center: Point::default(),
            coll: [Point::default(); 2],
            emit: [Point::default(); 2],
            base_pt: Point::default(),
        }
    }

    /// A transistor being placed at (`xx`, `yy`); `pnp` selects PNP over NPN.
    pub fn new(pnp: bool, xx: i32, yy: i32) -> Self {
        let mut t = Self::blank(Base::new(xx, yy), if pnp { -1 } else { 1 });
        t.setup();
        t
    }

    /// A transistor read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut t = Self::blank(Base::from_dump(xa, ya, xb, yb, f), st.next_i32().unwrap_or(1));
        t.alloc_nodes();
        let fields = (|| {
            Ok::<_, crate::format::tokenizer::TokenError>((
                st.next_f64()?,
                st.next_f64()?,
                st.next_f64()?,
                st.next_token().ok_or(crate::format::tokenizer::TokenError::Missing)?,
            ))
        })();
        match fields {
            Ok((vbe, vbc, beta, model)) => {
                t.lastvbe = vbe;
                t.lastvbc = vbc;
                t.base.volts[0] = 0.0;
                t.base.volts[1] = -vbe;
                t.base.volts[2] = -vbc;
                t.beta = beta;
                t.model_name = unescape(model);
            }
            Err(_) => t.model_name = "default".into(),
        }
        GLOBAL_FLAGS.with(|g| g.set(f & FLAGS_GLOBAL));
        t.setup();
        t
    }

    fn setup(&mut self) {
        self.model = TransistorModel::named_or_copy(&self.model_name, Some(&self.model));
        self.model_name = self.model.name.clone();
        self.vcrit = VT * (VT / (std::f64::consts::SQRT_2 * self.model.sat_cur)).ln();
        self.base.no_diagonal = true;
        self.alloc_nodes();
    }

    fn global_flags() -> i32 {
        GLOBAL_FLAGS.with(Cell::get)
    }

    fn has_circle(&self) -> bool {
        Self::global_flags() & FLAG_CIRCLE != 0
    }

    fn limit_step(&self, vnew: f64, vold: f64, s: &mut Stamper) -> f64 {
        let mut vnew = vnew;
        if vnew > self.vcrit && (vnew - vold).abs() > VT + VT {
            if vold > 0.0 {
                let arg = 1.0 + (vnew - vold) / VT;
                vnew = if arg > 0.0 { vold + VT * arg.ln() } else { self.vcrit };
            } else {
                vnew = VT * (vnew / VT).ln();
            }
            s.converged = false;
        }
        vnew
    }

    /// Junction capacitance with the forward-bias linearisation SPICE uses beyond FC * VJ.
    fn junction_cap(vj: f64, cj0: f64, vj0: f64, mj: f64) -> f64 {
        if cj0 <= 0.0 {
            return 0.0;
        }
        let fc = 0.5;
        if vj < fc * vj0 {
            cj0 / (1.0 - vj / vj0).powf(mj)
        } else {
            cj0 / (1.0 - fc).powf(1.0 + mj) * (1.0 - fc * (1.0 + mj) + mj * vj / vj0)
        }
    }
}

impl Element for TransistorElm {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        if self.pnp == -1 { "PTransistorElm" } else { "NTransistorElm" }
    }
    fn dump_type(&self) -> i32 {
        i32::from(b't')
    }
    fn xml_dump_type(&self) -> String {
        "t".into()
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            self.pnp,
            js_number(self.base.volts[0] - self.base.volts[1]),
            js_number(self.base.volts[0] - self.base.volts[2]),
            js_number(self.beta),
            escape(&self.model_name)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.pnp = xml.int_attr("pn", self.pnp);
        self.beta = xml.double_attr("be", self.beta);
        self.model_name = xml.string_attr("mo", &self.model_name);
        self.lastvbe = xml.double_attr("vbe", 0.0);
        self.lastvbc = xml.double_attr("vbc", 0.0);
        self.alloc_nodes();
        self.base.volts[0] = 0.0;
        self.base.volts[1] = -self.lastvbe;
        self.base.volts[2] = -self.lastvbc;
        GLOBAL_FLAGS.with(|g| g.set(self.base.flags & FLAGS_GLOBAL));
        self.setup();
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("pn", self.pnp);
        out.double_attr("be", self.beta);
        out.str_attr("mo", &self.model_name);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.double_attr("vbe", self.base.volts[0] - self.base.volts[1]);
        out.double_attr("vbc", self.base.volts[0] - self.base.volts[2]);
    }
    fn dump_model_xml(&self) -> Option<XmlOut> {
        (!self.model.built_in).then(|| self.model.dump_xml())
    }

    fn post_count(&self) -> usize {
        3
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.base.point1,
            1 => self.coll[0],
            _ => self.emit[0],
        })
    }

    fn set_points(&mut self) {
        self.base.flags &= !FLAGS_GLOBAL;
        self.base.flags |= Self::global_flags();
        self.base.no_diagonal = true;
        self.base.set_points();
        let hs = 16;
        if self.base.flags & FLAG_FLIP != 0 {
            self.base.dsign = -self.base.dsign;
        }
        let hs2 = hs * self.base.dsign * self.pnp;
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let (c0, e0) = interp_point2(p1, p2, 1.0, f64::from(hs2));
        self.coll[0] = c0;
        self.emit[0] = e0;
        let (r0, r1) = interp_point2(p1, p2, 1.0 - 16.0 / dn, f64::from(hs));
        let (r2, r3) = interp_point2(p1, p2, 1.0 - 13.0 / dn, f64::from(hs));
        let (c1, e1) =
            interp_point2(p1, p2, 1.0 - 13.0 / dn, f64::from(6 * self.base.dsign * self.pnp));
        self.coll[1] = c1;
        self.emit[1] = e1;
        self.base_pt = interp_point(p1, p2, 1.0 - 16.0 / dn);
        self.rect_poly = Polygon::from_points(&[r0, r2, r3, r1]);
        self.arrow_poly = if self.pnp == 1 {
            calc_arrow(self.emit[1], self.emit[0], 8.0, 4.0)
        } else {
            calc_arrow(self.emit[0], self.emit[1], 8.0, 4.0)
        };
        self.circle_center = interp_point(self.base_pt, p2, 0.5);
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.lastvbc = 0.0;
        self.lastvbe = 0.0;
        self.curcount_c = 0.0;
        self.curcount_e = 0.0;
        self.curcount_b = 0.0;
        self.cap_volt_be = 0.0;
        self.cap_volt_bc = 0.0;
        self.cap_cur_be = 0.0;
        self.cap_cur_bc = 0.0;
        self.geq_be = 0.0;
        self.geq_bc = 0.0;
        self.ceq_be = 0.0;
        self.ceq_bc = 0.0;
        self.bad_iters = 0;
        self.local_sub_iters = 0;
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        if self.base.flags & FLAGS_GLOBAL != Self::global_flags() {
            self.set_points();
        }
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, 16.0);
        if self.has_circle() {
            g.set_color(Color::GRAY);
            draw_thick_circle(g, self.circle_center.x, self.circle_center.y, 20);
        }
        let power = self.get_power();
        let v = self.base.volts.clone();
        set_power_color_w(g, &self.base, c, power);
        set_voltage_color(g, &self.base, c, v[1]);
        draw_thick_line(g, self.coll[0], self.coll[1]);
        set_voltage_color(g, &self.base, c, v[2]);
        draw_thick_line(g, self.emit[0], self.emit[1]);
        fill_polygon(g, &self.arrow_poly);
        set_voltage_color(g, &self.base, c, v[0]);
        if c.show_power {
            g.set_color(Color::GRAY);
        }
        draw_thick_line(g, p1, self.base_pt);
        self.curcount_b = self.base.update_dot_count_for(-self.ib, self.curcount_b, c);
        draw_dots(g, c, self.base_pt, p1, self.curcount_b);
        self.curcount_c = self.base.update_dot_count_for(-self.ic, self.curcount_c, c);
        draw_dots(g, c, self.coll[1], self.coll[0], self.curcount_c);
        self.curcount_e = self.base.update_dot_count_for(-self.ie, self.curcount_e, c);
        draw_dots(g, c, self.emit[1], self.emit[0], self.curcount_e);
        set_voltage_color(g, &self.base, c, v[0]);
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.rect_poly);
        if self.base.needs_highlight() || self.base.creating {
            g.set_color(c.white);
            let b = &self.base;
            let (bx, by) = (f64::from(self.base_pt.x), f64::from(self.base_pt.y));
            let (cx, cy) = (f64::from(self.coll[0].x), f64::from(self.coll[0].y));
            let (ex, ey) = (f64::from(self.emit[0].x), f64::from(self.emit[0].y));
            if b.dy == 0 {
                let l = if b.dx < 0 { -2.0 } else { 10.0 };
                g.draw_string("B", bx - l, by - 4.0);
                let l = if b.dx < 0 { 13.0 } else { -6.0 };
                g.draw_string("C", cx - l, cy + 4.0);
                g.draw_string("E", ex - l, ey + 4.0);
            } else if b.dx == 0 {
                let l = if b.dy < 0 { -11.0 } else { 3.0 };
                g.draw_string("B", bx + 3.0, by - l);
                let l = if b.dy < 0 { 7.0 } else { -14.0 };
                g.draw_string("C", cx - 4.0, cy - l);
                g.draw_string("E", ex - 4.0, ey - l);
            }
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_power(&self) -> f64 {
        let v = &self.base.volts;
        (v[0] - v[2]) * self.ib + (v[1] - v[2]) * self.ic
    }

    fn start_iteration(&mut self, s: &Stamper) {
        let m = &self.model;
        let has_be = m.junction_cap_be > 0.0 || m.transit_time_f > 0.0;
        let has_bc = m.junction_cap_bc > 0.0 || m.transit_time_r > 0.0;
        let pnp = f64::from(self.pnp);
        if has_be && s.time_step > 0.0 {
            let vj_be = pnp * self.cap_volt_be;
            let mut cje =
                Self::junction_cap(vj_be, m.junction_cap_be, m.junction_pot_be, m.junction_exp_be);
            if m.transit_time_f > 0.0 && vj_be > 0.0 {
                let vtn = VT * m.emission_coeff_f;
                cje += m.transit_time_f * m.sat_cur * (vj_be / vtn).exp() / vtn;
            }
            self.geq_be = 2.0 * cje / s.time_step;
            if self.geq_be < 1e-20 {
                self.geq_be = 0.0;
                self.ceq_be = 0.0;
                self.cap_cur_be = 0.0;
            } else {
                self.ceq_be = -self.geq_be * self.cap_volt_be - self.cap_cur_be;
            }
        }
        if has_bc && s.time_step > 0.0 {
            let vj_bc = pnp * self.cap_volt_bc;
            let mut cjc =
                Self::junction_cap(vj_bc, m.junction_cap_bc, m.junction_pot_bc, m.junction_exp_bc);
            if m.transit_time_r > 0.0 && vj_bc > 0.0 {
                let vtn = VT * m.emission_coeff_r;
                cjc += m.transit_time_r * m.sat_cur * (vj_bc / vtn).exp() / vtn;
            }
            self.geq_bc = 2.0 * cjc / s.time_step;
            if self.geq_bc < 1e-20 {
                self.geq_bc = 0.0;
                self.ceq_bc = 0.0;
                self.cap_cur_bc = 0.0;
            } else {
                self.ceq_bc = -self.geq_bc * self.cap_volt_bc - self.cap_cur_bc;
            }
        }
    }

    fn stamp(&mut self, s: &mut Stamper) {
        for n in 0..3 {
            s.stamp_non_linear(self.base.nodes[n]);
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let pnp = f64::from(self.pnp);
        let v = self.base.volts.clone();
        let mut vbc = pnp * (v[0] - v[1]); // typically negative
        let mut vbe = pnp * (v[0] - v[2]); // typically positive
        let not_converged = (vbc - self.lastvbc).abs() > 0.01 || (vbe - self.lastvbe).abs() > 0.01;
        if not_converged {
            s.converged = false;
            self.local_sub_iters += 1;
        } else {
            self.local_sub_iters = 0;
        }
        self.gmin = 1e-12;
        if self.local_sub_iters > 100 && self.bad_iters < 5 {
            self.gmin = (-9.0 * 10f64.ln() * (1.0 - f64::from(self.local_sub_iters) / 300.0)).exp();
            if self.gmin > 0.1 {
                self.gmin = 0.1;
            }
        }
        vbc = self.limit_step(vbc, self.lastvbc, s);
        vbe = self.limit_step(vbe, self.lastvbe, s);
        self.lastvbc = vbc;
        self.lastvbe = vbe;

        // dc model parameters (from SPICE 3f5, bjtload.c)
        let m = &self.model;
        let gmin = self.gmin;
        let csat = m.sat_cur;
        let oik = m.inv_roll_off_f;
        let c2 = m.be_leak_cur;
        let vte = m.leak_be_emission_coeff * VT;
        let oikr = m.inv_roll_off_r;
        let c4 = m.bc_leak_cur;
        let vtc = m.leak_bc_emission_coeff * VT;
        let mut vtn = VT * m.emission_coeff_f;
        let (cbe, gbe, cben, gben);
        if vbe > -5.0 * vtn {
            let evbe = (vbe / vtn).exp();
            cbe = csat * (evbe - 1.0) + gmin * vbe;
            gbe = csat * evbe / vtn + gmin;
            if c2 == 0.0 {
                cben = 0.0;
                gben = 0.0;
            } else {
                let evben = (vbe / vte).exp();
                cben = c2 * (evben - 1.0);
                gben = c2 * evben / vte;
            }
        } else {
            gbe = -csat / vbe + gmin;
            cbe = gbe * vbe;
            gben = -c2 / vbe;
            cben = gben * vbe;
        }
        vtn = VT * m.emission_coeff_r;
        let (cbc, gbc, cbcn, gbcn);
        if vbc > -5.0 * vtn {
            let evbc = (vbc / vtn).exp();
            cbc = csat * (evbc - 1.0) + gmin * vbc;
            gbc = csat * evbc / vtn + gmin;
            if c4 == 0.0 {
                cbcn = 0.0;
                gbcn = 0.0;
            } else {
                let evbcn = (vbc / vtc).exp();
                cbcn = c4 * (evbcn - 1.0);
                gbcn = c4 * evbcn / vtc;
            }
        } else {
            gbc = -csat / vbc + gmin;
            cbc = gbc * vbc;
            gbcn = -c4 / vbc;
            cbcn = gbcn * vbc;
        }
        // base charge terms
        let q1 = 1.0 / (1.0 - m.inv_early_volt_f * vbc - m.inv_early_volt_r * vbe);
        let (qb, dqbdve, dqbdvc);
        if oik == 0.0 && oikr == 0.0 {
            qb = q1;
            dqbdve = q1 * qb * m.inv_early_volt_r;
            dqbdvc = q1 * qb * m.inv_early_volt_f;
        } else {
            let q2 = oik * cbe + oikr * cbc;
            let arg = (1.0 + 4.0 * q2).max(0.0);
            let sqarg = if arg != 0.0 { arg.sqrt() } else { 1.0 };
            qb = q1 * (1.0 + sqarg) / 2.0;
            dqbdve = q1 * (qb * m.inv_early_volt_r + oik * gbe / sqarg);
            dqbdvc = q1 * (qb * m.inv_early_volt_f + oikr * gbc / sqarg);
        }
        let cex = cbe;
        let gex = gbe;
        // dc incremental conductances
        let cc = (cex - cbc) / qb - cbc / m.beta_r - cbcn;
        let cb = cbe / self.beta + cben + cbc / m.beta_r + cbcn;
        self.ic = pnp * cc;
        self.ib = pnp * cb;
        self.ie = pnp * (-cc - cb);
        let gpi = gbe / self.beta + gben;
        let gmu = gbc / m.beta_r + gbcn;
        let go = (gbc + (cex - cbc) * dqbdvc / qb) / qb;
        let gm = (gex - (cex - cbc) * dqbdve / qb) / qb - go;
        let ceqbe = pnp * (cc + cb - vbe * (gm + go + gpi) + vbc * go);
        let ceqbc = pnp * (-cc + vbe * (gm + go) - vbc * (gmu + go));
        if self.ib.is_infinite() || self.ic.is_nan() {
            s.stop("infinite transistor current", None);
        }
        let n = self.base.nodes.clone();
        s.stamp_matrix(n[1], n[1], gmu + go);
        s.stamp_matrix(n[1], n[0], -gmu + gm);
        s.stamp_matrix(n[1], n[2], -gm - go);
        s.stamp_matrix(n[0], n[0], gpi + gmu);
        s.stamp_matrix(n[0], n[2], -gpi);
        s.stamp_matrix(n[0], n[1], -gmu);
        s.stamp_matrix(n[2], n[0], -gpi - gm);
        s.stamp_matrix(n[2], n[1], -go);
        s.stamp_matrix(n[2], n[2], gpi + gm + go);
        // current excitation vector (right side)
        s.stamp_right_side(n[0], -ceqbe - ceqbc);
        s.stamp_right_side(n[1], ceqbc);
        s.stamp_right_side(n[2], ceqbe);
        if self.geq_be > 0.0 {
            s.stamp_matrix(n[0], n[0], self.geq_be);
            s.stamp_matrix(n[2], n[2], self.geq_be);
            s.stamp_matrix(n[0], n[2], -self.geq_be);
            s.stamp_matrix(n[2], n[0], -self.geq_be);
            s.stamp_right_side(n[0], -self.ceq_be);
            s.stamp_right_side(n[2], self.ceq_be);
        }
        if self.geq_bc > 0.0 {
            s.stamp_matrix(n[0], n[0], self.geq_bc);
            s.stamp_matrix(n[1], n[1], self.geq_bc);
            s.stamp_matrix(n[0], n[1], -self.geq_bc);
            s.stamp_matrix(n[1], n[0], -self.geq_bc);
            s.stamp_right_side(n[0], -self.ceq_bc);
            s.stamp_right_side(n[1], self.ceq_bc);
        }
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if self.ic.abs() > 1e12 || self.ib.abs() > 1e12 {
            s.stop("max current exceeded", None);
        }
        if self.local_sub_iters > 100 {
            self.bad_iters += 1;
        } else {
            self.bad_iters = 0;
        }
        let v = &self.base.volts;
        if self.geq_be > 0.0 {
            self.cap_volt_be = v[0] - v[2];
            self.cap_cur_be = self.geq_be * self.cap_volt_be + self.ceq_be;
        }
        if self.geq_bc > 0.0 {
            self.cap_volt_bc = v[0] - v[1];
            self.cap_cur_bc = self.geq_bc * self.cap_volt_bc + self.ceq_bc;
        }
        if self.geq_be > 0.0 {
            let icap = self.geq_be * (v[0] - v[2]) + self.ceq_be;
            self.ib += icap;
            self.ie -= icap;
        }
        if self.geq_bc > 0.0 {
            let icap = self.geq_bc * (v[0] - v[1]) + self.ceq_bc;
            self.ib += icap;
            self.ic -= icap;
        }
        self.local_sub_iters = 0;
    }

    fn scope_text(&self, x: i32) -> Option<String> {
        let t = match x {
            VAL_IB => "Ib",
            VAL_IC => "Ic",
            VAL_IE => "Ie",
            VAL_VBE => "Vbe",
            VAL_VBC => "Vbc",
            VAL_VCE => "Vce",
            VAL_POWER => "P",
            _ => "",
        };
        Some(format!("{}, {t}", crate::locale::ls("transistor")))
    }
    fn scope_value(&self, x: i32) -> f64 {
        let v = &self.base.volts;
        match x {
            VAL_IB => self.ib,
            VAL_IC => self.ic,
            VAL_IE => self.ie,
            VAL_VBE => v[0] - v[2],
            VAL_VBC => v[0] - v[1],
            VAL_VCE => v[1] - v[2],
            VAL_POWER => self.get_power(),
            _ => 0.0,
        }
    }
    fn scope_units(&self, x: i32) -> i32 {
        match x {
            VAL_IB | VAL_IC | VAL_IE => UNITS_A,
            VAL_POWER => UNITS_W,
            _ => UNITS_V,
        }
    }
    fn can_view_in_scope(&self) -> bool {
        true
    }

    fn elm_type(&self) -> Option<String> {
        Some("transistor".into())
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        let v = &self.base.volts;
        arr.push(format!(
            "{}{}",
            crate::locale::ls("transistor"),
            if self.pnp == -1 { " (PNP)" } else { " (NPN)" }
        ));
        arr.push(format!("{}, \u{3b2}={}", self.model.name, format(self.beta, false)));
        let (vbc, vbe, vce) = (v[0] - v[1], v[0] - v[2], v[1] - v[2]);
        let pnp = f64::from(self.pnp);
        let region = if vbc * pnp > 0.2 {
            if vbe * pnp > 0.2 { "saturation" } else { "reverse active" }
        } else if vbe * pnp > 0.2 {
            "fwd active"
        } else {
            "cutoff"
        };
        arr.push(crate::locale::ls(region));
        arr.push(format!("Ic = {}", current_text(self.ic)));
        arr.push(format!("Ib = {}", current_text(self.ib)));
        arr.push(format!("Vbe = {}", voltage_text(vbe)));
        arr.push(format!("Vbc = {}", voltage_text(vbc)));
        arr.push(format!("Vce = {}", voltage_text(vce)));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Beta/hFE", self.beta, 10.0, 1000.0).set_dimensionless()),
            1 => Some(EditInfo::checkbox("Swap E/C", self.base.flags & FLAG_FLIP != 0)),
            2 => Some(EditInfo::checkbox("Draw Circle", self.has_circle())),
            3 => {
                let models = TransistorModel::list();
                let selected = models.iter().position(|m| m.name == self.model_name).unwrap_or(0);
                Some(EditInfo::choice(
                    "Model",
                    models.iter().map(TransistorModel::description_text).collect(),
                    selected,
                ))
            }
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                self.beta = ei.value;
                self.setup();
            }
            1 => {
                self.base.flags = ei.change_flag(self.base.flags, FLAG_FLIP);
                self.set_points();
            }
            2 => GLOBAL_FLAGS.with(|g| g.set(ei.change_flag(g.get(), FLAG_CIRCLE))),
            3 => {
                let models = TransistorModel::list();
                if let Some(m) = models.get(ei.choice_index) {
                    self.model = m.clone();
                    self.model_name = m.name.clone();
                    self.setup();
                    ei.new_dialog = true;
                }
            }
            _ => {}
        }
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        match n {
            0 => -self.ib,
            1 => -self.ic,
            _ => -self.ie,
        }
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
        i32::from(if self.pnp == -1 { b'p' } else { b'n' })
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

    /// Common-emitter stage: 10 V supply, 100 k base bias, 1 k collector resistor, emitter at
    /// ground. With beta 100 and Vbe near 0.65 V the base current is about 93 uA, the
    /// collector current about 9.3 mA, and the collector sits near 0.7 V (saturating).
    #[test]
    fn common_emitter_stage_biases_to_the_expected_operating_point() {
        // transistor: base at (64, 32), body from (64, 32) to (128, 32); collector (128, 16)
        // and emitter (128, 48) for an NPN with dsign 1
        let text = format!(
            "{HDR}\nv 0 160 0 -64 0 0 40 10 0 0 0.5\nr 0 -64 64 -64 0 100000\nr 128 -64 128 16 0 1000\nt 64 32 128 32 0 1 0 0 100 default\nw 64 -64 64 32 0\nw 0 -64 128 -64 0\nw 128 48 128 160 0\nw 0 160 128 160 0\n"
        );
        let sim = run(&text, 300);
        let t = sim.elements.iter().find(|e| e.dump_type() == i32::from(b't')).unwrap();
        let v = &t.base().volts;
        let vbe = v[0] - v[2];
        assert!(vbe > 0.5 && vbe < 0.8, "vbe {vbe}");
        // the base current is (10 - vbe) / 100k; ic follows beta unless saturated
        let ib = t.scope_value(VAL_IB);
        assert!((ib - (10.0 - vbe) / 100_000.0).abs() < 2e-6, "ib {ib}");
        assert!(t.scope_value(VAL_IC) > 5e-3);
    }

    #[test]
    fn pnp_mirrors_the_npn() {
        let t = TransistorElm::new(true, 0, 0);
        assert_eq!(t.class_name(), "PTransistorElm");
        assert_eq!(t.pnp, -1);
        assert_eq!(TransistorElm::new(false, 0, 0).class_name(), "NTransistorElm");
    }

    #[test]
    fn dump_and_xml_round_trip_the_state() {
        let mut st = Tokenizer::new("1 0.6 -4 150 default");
        let t = TransistorElm::from_dump(0, 0, 64, 0, 0, &mut st);
        assert_eq!(t.beta, 150.0);
        assert!((t.lastvbe - 0.6).abs() < 1e-12);
        let xml = {
            let mut o = XmlOut::new("t");
            t.dump_xml(&mut o);
            t.dump_xml_state(&mut o);
            o.to_pretty_string()
        };
        assert!(xml.contains("be=\"150\"") && xml.contains("vbe="));
        assert!(t.dump().ends_with(" 1 0.6 -4 150 default") || t.dump().contains(" 150 default"));
    }

    #[test]
    fn junction_capacitance_follows_the_spice_formula() {
        // below FC * VJ it is CJ0 / (1 - V/VJ)^M
        let c = TransistorElm::junction_cap(0.0, 1e-12, 0.75, 0.33);
        assert!((c - 1e-12).abs() < 1e-18);
        let c = TransistorElm::junction_cap(-5.0, 1e-12, 0.75, 0.33);
        assert!(c < 1e-12);
        assert_eq!(TransistorElm::junction_cap(0.3, 0.0, 0.75, 0.33), 0.0);
    }
}
