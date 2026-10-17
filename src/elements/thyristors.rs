// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (TriodeElm, SCRElm, DiacElm, TriacElm).
//! The vacuum triode and the thyristors: SCR, diac and triac.

use super::element::*;
use super::geometry::*;
use super::junction::Diode;
use super::models::DiodeModel;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Color, Painter};
use crate::solver::stamper::Stamper;

/// Family name used for the feature switch.
pub const FAMILY: &str = "transistors";
const ACTIVE: &str = "Active Components";

/// Registers the triode and the thyristors.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("TriodeElm", FAMILY, "Add Triode", ACTIVE, |x, y| {
            Box::new(Triode::new(x, y))
        })
        .dump(173, |a, b, c, d, f, st| Ok(Box::new(Triode::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("SCRElm", FAMILY, "Add SCR", ACTIVE, |x, y| {
            Box::new(Scr::new(x, y))
        })
        .dump(177, |a, b, c, d, f, st| Ok(Box::new(Scr::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("DiacElm", FAMILY, "Add DIAC", ACTIVE, |x, y| {
            Box::new(Diac::new(x, y))
        })
        .dump(203, |a, b, c, d, f, st| Ok(Box::new(Diac::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("TriacElm", FAMILY, "Add TRIAC", ACTIVE, |x, y| {
            Box::new(Triac::new(x, y))
        })
        .dump(206, |a, b, c, d, f, st| Ok(Box::new(Triac::from_dump(a, b, c, d, f, st)))),
    );
}

fn default_diode() -> Diode {
    let mut d = Diode::default();
    d.setup(&DiodeModel::default_model());
    d
}

// ---------------------------------------------------------------------------------------------
// Triode
// ---------------------------------------------------------------------------------------------

const TRIODE_FLIP: i32 = 1;
const TRIODE_DSIGN_FIX: i32 = 2;
const GRID_CURRENT_R: f64 = 6000.0;

/// A vacuum triode with plate, grid and cathode.
pub struct Triode {
    base: Base,
    mu: f64,
    kg1: f64,
    plate: [Point; 4],
    grid: [Point; 8],
    cath: [Point; 4],
    mid_grid: Point,
    mid_cath: Point,
    circle_r: i32,
    curcount_p: f64,
    curcount_c: f64,
    curcount_g: f64,
    current_p: f64,
    current_g: f64,
    current_c: f64,
    last: [f64; 3],
}

impl Triode {
    fn blank(base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        Triode {
            base,
            mu: 93.0,
            kg1: 680.0,
            plate: [Point::default(); 4],
            grid: [Point::default(); 8],
            cath: [Point::default(); 4],
            mid_grid: Point::default(),
            mid_cath: Point::default(),
            circle_r: 24,
            curcount_p: 0.0,
            curcount_c: 0.0,
            curcount_g: 0.0,
            current_p: 0.0,
            current_g: 0.0,
            current_c: 0.0,
            last: [0.0; 3],
        }
    }

    /// A triode being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut t = Self::blank(Base::new(x, y));
        t.base.flags |= TRIODE_DSIGN_FIX;
        t
    }

    /// A triode read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut t = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        t.mu = st.next_f64().unwrap_or(93.0);
        t.kg1 = st.next_f64().unwrap_or(680.0);
        t
    }
}

impl Element for Triode {
    impl_element_base!("TriodeElm");

    fn visual_state(&self) -> Vec<f64> {
        vec![self.current_p, self.current_g, self.current_c]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [p, g, c] = state[..] {
            (self.current_p, self.current_g, self.current_c) = (p, g, c);
        }
    }

    fn dump_type(&self) -> i32 {
        173
    }
    fn dump(&self) -> String {
        format!("{} {} {}", self.base.dump_prefix(173), js_number(self.mu), js_number(self.kg1))
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("mu", self.mu);
        out.double_attr("kg", self.kg1);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.mu = xml.double_attr("mu", self.mu);
        self.kg1 = xml.double_attr("kg", self.kg1);
    }

    fn post_count(&self) -> usize {
        3
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.plate[0],
            1 => self.grid[0],
            _ => self.cath[0],
        })
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn can_view_in_scope(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.base.curcount = 0.0;
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let mut s = if self.base.flags & TRIODE_DSIGN_FIX != 0 { self.base.dsign } else { 1 };
        if self.base.flags & TRIODE_FLIP != 0 {
            s = -s;
        }
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        self.grid[0] = p1;
        let nearw = 8 * s;
        self.plate[1] = interp_point_g(p1, p2, 1.0, f64::from(nearw));
        let farw = 32 * s;
        self.plate[0] = interp_point_g(p1, p2, 1.0, f64::from(farw));
        let (a, b) = interp_point2(p2, self.plate[1], 1.0, 18.0);
        self.plate[2] = a;
        self.plate[3] = b;
        self.circle_r = 24;
        self.grid[1] = interp_point(p1, p2, (dn - f64::from(self.circle_r)) / dn);
        for i in 0..3 {
            let f = i as f64;
            self.grid[2 + i * 2] = interp_point(self.grid[1], p2, (f * 3.0 + 1.0) / 4.5);
            self.grid[3 + i * 2] = interp_point(self.grid[1], p2, (f * 3.0 + 2.0) / 4.5);
        }
        self.mid_grid = p2;
        let cathw = 16 * s;
        self.mid_cath = interp_point_g(p1, p2, 1.0, f64::from(-nearw));
        let (c1, c2) = interp_point2(p2, self.plate[1], -1.0, f64::from(cathw));
        self.cath[1] = c1;
        self.cath[2] = c2;
        self.cath[3] = interp_point_g(p2, self.plate[1], -1.2, f64::from(-cathw));
        self.cath[0] = interp_point_g(
            p2,
            self.plate[1],
            -f64::from(farw) / f64::from(nearw),
            f64::from(cathw),
        );
        self.alloc_nodes();
    }

    fn stamp(&mut self, s: &mut Stamper) {
        for &n in &self.base.nodes.clone() {
            s.stamp_non_linear(n);
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        let mut vs = [self.base.volts[0], self.base.volts[1], self.base.volts[2]];
        for i in 1..3 {
            vs[i] = vs[i].clamp(self.last[i] - 0.5, self.last[i] + 0.5);
        }
        let vgk = vs[1] - vs[2];
        let vpk = vs[0] - vs[2];
        if (self.last[0] - vs[0]).abs() > 0.01
            || (self.last[1] - vs[1]).abs() > 0.01
            || (self.last[2] - vs[2]).abs() > 0.01
        {
            s.converged = false;
        }
        self.last = vs;
        let (ids, gm, gds);
        let ival = vgk + vpk / self.mu;
        self.current_g = 0.0;
        if vgk > 0.01 {
            s.stamp_resistor(n[1], n[2], GRID_CURRENT_R);
            self.current_g = vgk / GRID_CURRENT_R;
        } else {
            s.stamp_resistor(n[1], n[2], 1e8); // avoid a singular matrix
        }
        if ival < 0.0 {
            // should be all zero, but that makes the matrix singular
            gds = 1e-8;
            ids = vpk * gds;
            gm = 0.0;
        } else {
            ids = ival.powf(1.5) / self.kg1;
            let q = 1.5 * ival.sqrt() / self.kg1;
            gds = q;
            gm = q / self.mu;
        }
        self.current_p = ids;
        self.current_c = ids + self.current_g;
        let rs = -ids + gds * vpk + gm * vgk;
        let (p, g, c) = (n[0], n[1], n[2]);
        s.stamp_matrix(p, p, gds);
        s.stamp_matrix(p, c, -gds - gm);
        s.stamp_matrix(p, g, gm);
        s.stamp_matrix(c, p, -gds);
        s.stamp_matrix(c, c, gds + gm);
        s.stamp_matrix(c, g, -gm);
        s.stamp_right_side(p, rs);
        s.stamp_right_side(c, -rs);
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        match n {
            2 => self.current_c,
            0 => -self.current_p,
            _ => -self.current_g,
        }
    }
    fn get_current(&self) -> f64 {
        self.current_c
    }
    fn get_power(&self) -> f64 {
        let v = &self.base.volts;
        (v[0] - v[2]) * self.current_c + (v[1] - v[2]) * self.current_g
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0] - self.base.volts[2]
    }
    // the grid is not connected to the other terminals
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        !(n1 == 1 || n2 == 1)
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.set_color(Color::GRAY);
        draw_thick_circle(g, self.base.point2.x, self.base.point2.y, self.circle_r);
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, self.plate[0], 16.0);
        self.base.adjust_bbox(
            self.cath[0].x,
            self.cath[1].y,
            p2.x + self.circle_r,
            p2.y + self.circle_r,
        );
        let v = self.base.volts.clone();
        set_voltage_color(g, &self.base, c, v[0]);
        set_power_color_w(g, &self.base, c, self.current_p * (v[0] - v[2]));
        draw_thick_line(g, self.plate[0], self.plate[1]);
        draw_thick_line(g, self.plate[2], self.plate[3]);
        set_voltage_color(g, &self.base, c, v[1]);
        set_power_color_w(g, &self.base, c, self.current_g * (v[1] - v[2]));
        for i in (0..8).step_by(2) {
            draw_thick_line(g, self.grid[i], self.grid[i + 1]);
        }
        set_voltage_color(g, &self.base, c, v[2]);
        set_power_color_w(g, &self.base, c, 0.0);
        for i in 0..3 {
            draw_thick_line(g, self.cath[i], self.cath[i + 1]);
        }
        self.curcount_p = self.base.update_dot_count_for(self.current_p, self.curcount_p, c);
        self.curcount_c = self.base.update_dot_count_for(self.current_c, self.curcount_c, c);
        self.curcount_g = self.base.update_dot_count_for(self.current_g, self.curcount_g, c);
        if !self.base.creating {
            draw_dots(g, c, self.plate[0], self.mid_grid, self.curcount_p);
            draw_dots(g, c, self.mid_grid, self.mid_cath, self.curcount_c);
            let c8 = Base::add_cur_count(self.curcount_c, 8.0);
            draw_dots(g, c, self.mid_cath, self.cath[1], c8);
            draw_dots(g, c, self.cath[1], self.cath[0], c8);
            draw_dots(g, c, p1, self.mid_grid, self.curcount_g);
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let v = &self.base.volts;
        arr.push("triode".into());
        arr.push(format!("Vac = {}", voltage_text(v[0] - v[2])));
        arr.push(format!("Vgc = {}", voltage_text(v[1] - v[2])));
        arr.push(format!("Vag = {}", voltage_text(v[0] - v[1])));
        arr.push(format!("Ic = {}", unit_text(self.current_c, "A")));
        arr.push(format!("Ig = {}", unit_text(self.current_g, "A")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("mu", self.mu, 0.0, 0.0).set_dimensionless().set_positive()),
            1 => Some(EditInfo::new("kg1", self.kg1, 0.0, 0.0).set_dimensionless().set_positive()),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.mu = ei.value,
            1 => self.kg1 = ei.value,
            _ => {}
        }
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        let b = &mut self.base;
        if b.x == b.x2 {
            b.flags ^= TRIODE_FLIP;
        }
        if b.flags & TRIODE_DSIGN_FIX == 0 && b.x != b.x2 {
            b.flags ^= TRIODE_FLIP;
        }
        b.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        let b = &mut self.base;
        if b.y == b.y2 {
            b.flags ^= TRIODE_FLIP;
        }
        if b.flags & TRIODE_DSIGN_FIX == 0 && b.y != b.y2 {
            b.flags ^= TRIODE_FLIP;
        }
        b.flip_y(center2);
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.base.flags ^= TRIODE_FLIP;
        self.base.flip_xy(xmy);
        self.set_points();
    }
}

// ---------------------------------------------------------------------------------------------
// Shared geometry of the three-terminal thyristors
// ---------------------------------------------------------------------------------------------

/// Straightens a thyristor along its dominant axis and returns the direction of its gate.
fn straighten(base: &mut Base, fix_length: bool) -> i32 {
    let mut dir;
    if base.dx.abs() > base.dy.abs() {
        dir = -sign(base.dx) * sign(base.dy);
        if fix_length {
            base.dn = f64::from(base.dx.abs());
        }
        base.point2.y = base.point1.y;
    } else {
        dir = sign(base.dy) * sign(base.dx);
        if fix_length {
            base.dn = f64::from(base.dy.abs());
        }
        base.point2.x = base.point1.x;
    }
    if dir == 0 {
        dir = 1;
    }
    dir
}

/// Computes the gate stub of a thyristor; false when there is no room (the element is
/// collapsed to a point, as the original does).
fn gate_points(base: &mut Base, dir: i32) -> Option<[Point; 2]> {
    let grid = grid_size();
    let leadlen = (base.dn - 16.0) / 2.0;
    let gatelen = f64::from(grid) + leadlen % f64::from(grid);
    if leadlen < gatelen {
        base.x2 = base.x;
        base.y2 = base.y;
        return None;
    }
    let g0 = interp_point_g(base.lead2, base.point2, gatelen / leadlen, gatelen * f64::from(dir));
    let g1 = interp_point_g(base.lead2, base.point2, gatelen / leadlen, f64::from(grid * 2 * dir));
    Some([g0, g1])
}

fn arrow_polys(base: &Base) -> [Polygon; 2] {
    let (l1, l2) = (base.lead1, base.lead2);
    let mk = |i: i32| {
        let sgn = f64::from(-1 + i * 2);
        let p1 = interp_point_g(l1, l2, f64::from(i), 8.0 * sgn);
        let p2 = interp_point_g(l1, l2, f64::from(1 - i), 16.0 * sgn);
        let p3 = interp_point_g(l1, l2, f64::from(1 - i), 0.0);
        Polygon::from_points(&[p1, p2, p3])
    };
    [mk(0), mk(1)]
}

// ---------------------------------------------------------------------------------------------
// SCR
// ---------------------------------------------------------------------------------------------

const SCR_GATE_FIX: i32 = 1;

/// A silicon-controlled rectifier: anode (0), cathode (1), gate (2) and one internal node.
pub struct Scr {
    base: Base,
    diode: Diode,
    dir: i32,
    trigger_i: f64,
    holding_i: f64,
    gate_r: f64,
    a_res: f64,
    ia: f64,
    ic: f64,
    ig: f64,
    curcount: [f64; 3],
    last_vac: f64,
    last_vag: f64,
    poly: Polygon,
    cathode: [Point; 2],
    gate: [Point; 2],
}

impl Scr {
    fn blank(base: Base) -> Self {
        Scr {
            base,
            diode: default_diode(),
            dir: 1,
            trigger_i: 0.01,
            holding_i: 0.0082,
            gate_r: 50.0,
            a_res: 1.0,
            ia: 0.0,
            ic: 0.0,
            ig: 0.0,
            curcount: [0.0; 3],
            last_vac: 0.0,
            last_vag: 0.0,
            poly: Polygon::default(),
            cathode: [Point::default(); 2],
            gate: [Point::default(); 2],
        }
    }

    /// An SCR being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut s = Self::blank(Base::new(x, y));
        s.base.flags |= SCR_GATE_FIX;
        s
    }

    /// An SCR read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut s = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        if let (Ok(vac), Ok(vag)) = (st.next_f64(), st.next_f64()) {
            s.last_vac = vac;
            s.last_vag = vag;
            if let Ok(t) = st.next_f64() {
                s.trigger_i = t;
                if let Ok(h) = st.next_f64() {
                    s.holding_i = h;
                    if let Ok(g) = st.next_f64() {
                        s.gate_r = g;
                    }
                }
            }
        }
        s.alloc_nodes();
        s.base.volts[1] = -s.last_vac;
        s.base.volts[2] = -s.last_vag;
        s
    }

    fn fix_ends(&mut self) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        let pt = interp_point_g(p1, p2, 1.0, f64::from(grid_size() * self.dir));
        self.base.x2 = pt.x;
        self.base.y2 = pt.y;
    }
}

impl Element for Scr {
    impl_element_base!("SCRElm");

    fn visual_state(&self) -> Vec<f64> {
        vec![self.ia, self.ic, self.ig]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [a, c, g] = state[..] {
            (self.ia, self.ic, self.ig) = (a, c, g);
        }
    }

    fn dump_type(&self) -> i32 {
        177
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.base.dump_prefix(177),
            js_number(self.last_vac),
            js_number(self.last_vag),
            js_number(self.trigger_i),
            js_number(self.holding_i),
            js_number(self.gate_r)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("tr", self.trigger_i);
        out.double_attr("ho", self.holding_i);
        out.double_attr("gr", self.gate_r);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.trigger_i = xml.double_attr("tr", self.trigger_i);
        self.holding_i = xml.double_attr("ho", self.holding_i);
        self.gate_r = xml.double_attr("gr", self.gate_r);
    }

    fn post_count(&self) -> usize {
        3
    }
    fn internal_node_count(&self) -> usize {
        1
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.base.point1,
            1 => self.base.point2,
            _ => self.gate[1],
        })
    }
    fn non_linear(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.diode.reset();
        self.last_vag = 0.0;
        self.last_vac = 0.0;
        self.curcount = [0.0; 3];
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let fix = self.base.flags & SCR_GATE_FIX != 0;
        self.dir = straighten(&mut self.base, fix);
        self.base.calc_leads(16);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let (a, b) = interp_point2(l1, l2, 0.0, 8.0);
        let (c0, c1) = interp_point2(l1, l2, 1.0, 8.0);
        self.cathode = [c0, c1];
        self.poly = Polygon::from_points(&[a, b, l2]);
        if let Some(g) = gate_points(&mut self.base, self.dir) {
            let snapped = Point::new(snap_grid(g[1].x), snap_grid(g[1].y));
            self.gate = [g[0], snapped];
        }
        self.alloc_nodes();
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        for i in 0..4 {
            s.stamp_non_linear(n[i]);
        }
        s.stamp_resistor(n[2], n[1], self.gate_r);
        self.diode.stamp(s, n[3], n[1]);
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        let v = self.base.volts.clone();
        let vac = v[0] - v[1];
        let vag = v[0] - v[2];
        if (vac - self.last_vac).abs() > 0.01 || (vag - self.last_vag).abs() > 0.01 {
            s.converged = false;
        }
        self.last_vac = vac;
        self.last_vag = vag;
        self.diode.do_step(s, v[3] - v[1]);
        let icmult = 1.0 / self.trigger_i;
        let iamult = 1.0 / self.holding_i - icmult;
        self.a_res = if -icmult * self.ic + self.ia * iamult > 1.0 { 0.0105 } else { 10e5 };
        s.stamp_resistor(n[0], n[3], self.a_res);
    }

    fn calculate_current(&mut self) {
        let v = &self.base.volts;
        self.ig = (v[2] - v[1]) / self.gate_r;
        self.ia = (v[0] - v[3]) / self.a_res;
        self.ic = -self.ig - self.ia;
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        match n {
            0 => -self.ia,
            1 => -self.ic,
            _ => -self.ig,
        }
    }
    fn get_power(&self) -> f64 {
        let v = &self.base.volts;
        (v[0] - v[2]) * self.ia + (v[1] - v[2]) * self.ic
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, 8.0);
        self.base.adjust_bbox_pts(self.gate[0], self.gate[1]);
        let v = self.base.volts.clone();
        draw_2_leads(g, &self.base, c);
        let power = self.get_power();
        set_voltage_color(g, &self.base, c, v[0]);
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.poly);
        set_voltage_color(g, &self.base, c, v[2]);
        draw_thick_line(g, self.base.lead2, self.gate[0]);
        draw_thick_line(g, self.gate[0], self.gate[1]);
        set_voltage_color(g, &self.base, c, v[1]);
        set_power_color_w(g, &self.base, c, power);
        draw_thick_line(g, self.cathode[0], self.cathode[1]);
        let cur = [self.ia, self.ic, self.ig];
        for i in 0..3 {
            self.curcount[i] = self.base.update_dot_count_for(cur[i], self.curcount[i], c);
        }
        if !self.base.creating {
            let l2 = self.base.lead2;
            draw_dots(g, c, p1, l2, self.curcount[0]);
            draw_dots(g, c, p2, l2, self.curcount[1]);
            draw_dots(g, c, self.gate[1], self.gate[0], self.curcount[2]);
            let off = distance(self.gate[1], self.gate[0]);
            draw_dots(g, c, self.gate[0], l2, self.curcount[2] + off);
        }
        if (self.base.needs_highlight() || self.base.creating) && p1.x == p2.x && p2.y > p1.y {
            g.set_color(c.white);
            let ds = sign(self.base.dx);
            let (l1, l2) = (self.base.lead1, self.base.lead2);
            g.draw_string(
                "C",
                f64::from(l2.x + if ds < 0 { 5 } else { -15 }),
                f64::from(l2.y + 12),
            );
            g.draw_string("A", f64::from(l1.x + 5), f64::from(l1.y - 4));
            g.draw_string("G", f64::from(self.gate[0].x), f64::from(self.gate[0].y + 12));
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let v = &self.base.volts;
        arr.push("SCR".into());
        arr.push(format!("Ia = {}", current_text(self.ia)));
        arr.push(format!("Ig = {}", current_text(self.ig)));
        arr.push(format!("Vac = {}", voltage_text(v[0] - v[1])));
        arr.push(format!("Vag = {}", voltage_text(v[0] - v[2])));
        arr.push(format!("Vgc = {}", voltage_text(v[2] - v[1])));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => {
                Some(EditInfo::new("Trigger Current (A)", self.trigger_i, 0.0, 0.0).set_positive())
            }
            1 => {
                Some(EditInfo::new("Holding Current (A)", self.holding_i, 0.0, 0.0).set_positive())
            }
            2 => {
                Some(EditInfo::new("Gate Resistance (ohms)", self.gate_r, 0.0, 0.0).set_positive())
            }
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.trigger_i = ei.value,
            1 => self.holding_i = ei.value,
            2 => self.gate_r = ei.value,
            _ => {}
        }
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        self.fix_ends();
        self.base.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        self.fix_ends();
        self.base.flip_y(center2);
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.fix_ends();
        self.base.flip_xy(xmy);
        self.set_points();
    }
}

// ---------------------------------------------------------------------------------------------
// DIAC
// ---------------------------------------------------------------------------------------------

/// A diac: a two-terminal switch that conducts both ways once the breakdown voltage is passed.
pub struct Diac {
    base: Base,
    on_r: f64,
    off_r: f64,
    breakdown: f64,
    hold_current: f64,
    state: bool,
    diode1: Diode,
    diode2: Diode,
    arrows: [Polygon; 2],
    plate1: [Point; 2],
    plate2: [Point; 2],
}

impl Diac {
    fn blank(base: Base) -> Self {
        Diac {
            base,
            on_r: 500.0,
            off_r: 1e8,
            breakdown: 30.0,
            hold_current: 0.01,
            state: false,
            diode1: default_diode(),
            diode2: default_diode(),
            arrows: [Polygon::default(), Polygon::default()],
            plate1: [Point::default(); 2],
            plate2: [Point::default(); 2],
        }
    }

    /// A diac being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y))
    }

    /// A diac read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut d = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        d.on_r = st.next_f64().unwrap_or(d.on_r);
        d.off_r = st.next_f64().unwrap_or(d.off_r);
        d.breakdown = st.next_f64().unwrap_or(d.breakdown);
        d.hold_current = st.next_f64().unwrap_or(d.hold_current);
        d
    }

    fn resistance(&self) -> f64 {
        if self.state { self.on_r } else { self.off_r }
    }
}

impl Element for Diac {
    impl_element_base!("DiacElm");

    fn dump_type(&self) -> i32 {
        203
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(203),
            js_number(self.on_r),
            js_number(self.off_r),
            js_number(self.breakdown),
            js_number(self.hold_current)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("ron", self.on_r);
        out.double_attr("roff", self.off_r);
        out.double_attr("bd", self.breakdown);
        out.double_attr("hc", self.hold_current);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.on_r = xml.double_attr("ron", self.on_r);
        self.off_r = xml.double_attr("roff", self.off_r);
        self.breakdown = xml.double_attr("bd", self.breakdown);
        self.hold_current = xml.double_attr("hc", self.hold_current);
    }

    fn internal_node_count(&self) -> usize {
        2
    }
    fn non_linear(&self) -> bool {
        true
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(16);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let (a, b) = interp_point2(l1, l2, 0.0, 16.0);
        self.plate1 = [a, b];
        let (a, b) = interp_point2(l1, l2, 1.0, 16.0);
        self.plate2 = [a, b];
        for i in 0..2 {
            let sgn = f64::from(-1 + i * 2);
            let p1 = interp_point_g(l1, l2, f64::from(i), 8.0 * sgn);
            let p2 = interp_point_g(l1, l2, f64::from(1 - i), 16.0 * sgn);
            let p3 = interp_point_g(l1, l2, f64::from(1 - i), 0.0);
            self.arrows[i as usize] = Polygon::from_points(&[p1, p2, p3]);
        }
        self.alloc_nodes();
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        s.stamp_non_linear(n[0]);
        s.stamp_non_linear(n[1]);
        self.diode1.stamp(s, n[2], n[1]);
        self.diode2.stamp(s, n[1], n[3]);
    }

    fn start_iteration(&mut self, _s: &Stamper) {
        let vd = self.base.volts[0] - self.base.volts[1];
        if self.base.current.abs() < self.hold_current {
            self.state = false;
        }
        if vd.abs() > self.breakdown {
            self.state = true;
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        let v = self.base.volts.clone();
        let r = self.resistance();
        s.stamp_resistor(n[0], n[2], r);
        s.stamp_resistor(n[0], n[3], r);
        self.diode1.do_step(s, v[2] - v[1]);
        self.diode2.do_step(s, v[1] - v[3]);
    }

    fn calculate_current(&mut self) {
        let v = &self.base.volts;
        let r = self.resistance();
        self.base.current = (v[0] - v[2]) / r + (v[0] - v[3]) / r;
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, 6.0);
        let v = self.base.volts.clone();
        draw_2_leads(g, &self.base, c);
        let power = self.get_power();
        set_voltage_color(g, &self.base, c, v[0]);
        set_power_color_w(g, &self.base, c, power);
        draw_thick_line(g, self.plate1[0], self.plate1[1]);
        set_voltage_color(g, &self.base, c, v[1]);
        set_power_color_w(g, &self.base, c, power);
        draw_thick_line(g, self.plate2[0], self.plate2[1]);
        fill_polygon(g, &self.arrows[0]);
        set_voltage_color(g, &self.base, c, v[0]);
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.arrows[1]);
        do_dots(g, &mut self.base, c);
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("DIAC".into());
        basic_info(self, arr);
        arr.push(if self.state { "on" } else { "off" }.into());
        arr.push(format!("Ron = {}", unit_text(self.on_r, crate::locale::OHM)));
        arr.push(format!("Roff = {}", unit_text(self.off_r, crate::locale::OHM)));
        arr.push(format!("Vbrkdn = {}", unit_text(self.breakdown, "V")));
        arr.push(format!("Ihold = {}", unit_text(self.hold_current, "A")));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("On resistance (ohms)", self.on_r, 0.0, 0.0)),
            1 => Some(EditInfo::new("Off resistance (ohms)", self.off_r, 0.0, 0.0)),
            2 => Some(EditInfo::new("Breakdown voltage (volts)", self.breakdown, 0.0, 0.0)),
            3 => Some(EditInfo::new("Hold current (amps)", self.hold_current, 0.0, 0.0)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if ei.value <= 0.0 {
            return;
        }
        match n {
            0 => self.on_r = ei.value,
            1 => self.off_r = ei.value,
            2 => self.breakdown = ei.value,
            3 => self.hold_current = ei.value,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// TRIAC
// ---------------------------------------------------------------------------------------------

const MT1: usize = 1;
const MT2: usize = 0;
const GATE: usize = 2;
const MTI: usize = 3;

/// A triac: MT2 (post 0), MT1 (post 1), gate (post 2) and one internal node.
pub struct Triac {
    base: Base,
    diode03: Diode,
    diode30: Diode,
    state: bool,
    trigger_i: f64,
    holding_i: f64,
    gate_r: f64,
    a_res: f64,
    i1: f64,
    i2: f64,
    ig: f64,
    curcount: [f64; 3],
    arrows: [Polygon; 2],
    plate1: [Point; 2],
    plate2: [Point; 2],
    gate: [Point; 2],
}

impl Triac {
    fn blank(base: Base) -> Self {
        Triac {
            base,
            diode03: default_diode(),
            diode30: default_diode(),
            state: false,
            trigger_i: 0.01,
            holding_i: 0.0082,
            gate_r: 100.0,
            a_res: 0.0,
            i1: 0.0,
            i2: 0.0,
            ig: 0.0,
            curcount: [0.0; 3],
            arrows: [Polygon::default(), Polygon::default()],
            plate1: [Point::default(); 2],
            plate2: [Point::default(); 2],
            gate: [Point::default(); 2],
        }
    }

    /// A triac being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y))
    }

    /// A triac read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut t = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        t.trigger_i = st.next_f64().unwrap_or(t.trigger_i);
        t.holding_i = st.next_f64().unwrap_or(t.holding_i);
        t.gate_r = st.next_f64().unwrap_or(t.gate_r);
        t.state = st.next_token().is_some_and(|s| s.eq_ignore_ascii_case("true"));
        t
    }
}

impl Element for Triac {
    impl_element_base!("TriacElm");

    fn visual_state(&self) -> Vec<f64> {
        vec![self.i1, self.i2, self.ig]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [a, b, g] = state[..] {
            (self.i1, self.i2, self.ig) = (a, b, g);
        }
    }

    fn dump_type(&self) -> i32 {
        206
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(206),
            js_number(self.trigger_i),
            js_number(self.holding_i),
            js_number(self.gate_r),
            self.state
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("ti", self.trigger_i);
        out.double_attr("hi", self.holding_i);
        out.double_attr("cr", self.gate_r);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.bool_attr("st", self.state);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.trigger_i = xml.double_attr("ti", self.trigger_i);
        self.holding_i = xml.double_attr("hi", self.holding_i);
        self.gate_r = xml.double_attr("cr", self.gate_r);
        self.state = xml.bool_attr("st", self.state);
    }

    fn post_count(&self) -> usize {
        3
    }
    fn internal_node_count(&self) -> usize {
        1
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.base.point1,
            1 => self.base.point2,
            _ => self.gate[1],
        })
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn can_view_in_scope(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.diode03.reset();
        self.diode30.reset();
        self.curcount = [0.0; 3];
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let dir = straighten(&mut self.base, true);
        self.base.calc_leads(16);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let (a, b) = interp_point2(l1, l2, 0.0, 16.0);
        self.plate1 = [a, b];
        let (a, b) = interp_point2(l1, l2, 1.0, 16.0);
        self.plate2 = [a, b];
        self.arrows = arrow_polys(&self.base);
        if let Some(g) = gate_points(&mut self.base, dir) {
            self.gate = g;
        }
        self.alloc_nodes();
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        for i in [MT1, MT2, GATE, MTI] {
            s.stamp_non_linear(n[i]);
        }
        s.stamp_resistor(n[GATE], n[MT1], self.gate_r);
        self.diode03.stamp(s, n[MT2], n[MTI]);
        self.diode30.stamp(s, n[MTI], n[MT2]);
    }

    fn start_iteration(&mut self, _s: &Stamper) {
        if self.i2.abs() < self.holding_i {
            self.state = false;
        }
        if self.ig.abs() > self.trigger_i {
            self.state = true;
        }
        self.a_res = if self.state { 0.01 } else { 10e5 };
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        let v = self.base.volts.clone();
        self.diode03.do_step(s, v[MT2] - v[MTI]);
        self.diode30.do_step(s, v[MTI] - v[MT2]);
        s.stamp_resistor(n[MTI], n[MT1], self.a_res);
    }

    fn calculate_current(&mut self) {
        let v = &self.base.volts;
        // the resistance is 0 on startup
        self.i2 = if self.a_res == 0.0 { 0.0 } else { (v[MTI] - v[MT1]) / self.a_res };
        self.ig = -(v[MT1] - v[GATE]) / self.gate_r;
        self.i1 = -self.i2 - self.ig;
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        match n {
            0 => -self.i2,
            1 => -self.i1,
            _ => -self.ig,
        }
    }
    fn get_current(&self) -> f64 {
        self.i2
    }
    fn get_power(&self) -> f64 {
        let v = &self.base.volts;
        (v[MT2] - v[MT1]) * self.i2 + (v[GATE] - v[MT1]) * self.ig
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[MT2] - self.base.volts[MT1]
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, 6.0);
        self.base.adjust_bbox_pts(self.gate[0], self.gate[1]);
        let v = self.base.volts.clone();
        draw_2_leads(g, &self.base, c);
        let power = self.get_power();
        set_voltage_color(g, &self.base, c, v[0]);
        set_power_color_w(g, &self.base, c, power);
        draw_thick_line(g, self.plate1[0], self.plate1[1]);
        set_voltage_color(g, &self.base, c, v[1]);
        set_power_color_w(g, &self.base, c, power);
        draw_thick_line(g, self.plate2[0], self.plate2[1]);
        fill_polygon(g, &self.arrows[0]);
        set_voltage_color(g, &self.base, c, v[0]);
        set_power_color_w(g, &self.base, c, power);
        fill_polygon(g, &self.arrows[1]);
        set_voltage_color(g, &self.base, c, v[GATE]);
        let l2 = self.base.lead2;
        draw_thick_line(g, l2, self.gate[0]);
        draw_thick_line(g, self.gate[0], self.gate[1]);
        let cur = [self.i1, self.i2, self.ig];
        for i in 0..3 {
            self.curcount[i] = self.base.update_dot_count_for(cur[i], self.curcount[i], c);
        }
        if !self.base.creating {
            draw_dots(g, c, p1, l2, self.curcount[1]);
            draw_dots(g, c, p2, l2, self.curcount[0]);
            draw_dots(g, c, self.gate[1], self.gate[0], self.curcount[2]);
            let off = distance(self.gate[1], self.gate[0]);
            draw_dots(g, c, self.gate[0], l2, self.curcount[2] + off);
        }
        if (self.base.needs_highlight() || self.base.creating) && p1.x == p2.x && p2.y > p1.y {
            g.set_color(c.white);
            let ds = sign(self.base.dx);
            let l1 = self.base.lead1;
            g.draw_string(
                "MT1",
                f64::from(l2.x + if ds < 0 { 5 } else { -30 }),
                f64::from(l2.y + 12),
            );
            g.draw_string("MT2", f64::from(l1.x + 5), f64::from(l1.y - 4));
            g.draw_string("G", f64::from(self.gate[0].x), f64::from(self.gate[0].y + 12));
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let v = &self.base.volts;
        arr.push("TRIAC".into());
        arr.push(if self.state { "on" } else { "off" }.into());
        arr.push(format!("Vmt2mt1 = {}", voltage_text(v[MT2] - v[MT1])));
        arr.push(format!("Imt1 = {}", current_text(self.i1)));
        arr.push(format!("Imt2 = {}", current_text(self.i2)));
        arr.push(format!("Ig = {}", current_text(self.ig)));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => {
                Some(EditInfo::new("Trigger Current (A)", self.trigger_i, 0.0, 0.0).set_positive())
            }
            1 => {
                Some(EditInfo::new("Holding Current (A)", self.holding_i, 0.0, 0.0).set_positive())
            }
            2 => Some(
                EditInfo::new("Gate-MT1 Resistance (ohms)", self.gate_r, 0.0, 0.0).set_positive(),
            ),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.trigger_i = ei.value,
            1 => self.holding_i = ei.value,
            2 => self.gate_r = ei.value,
            _ => {}
        }
    }
}
