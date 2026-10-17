// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (RelayElm, RelayCoilElm,
// RelayContactElm, RelayModel).
//! Relays: the all-in-one relay, and the coil and contact pair that are linked by a label.

use super::element::*;
use super::geometry::*;
use super::model_edit::ModelEditor;
use super::models::RelayModel;
use super::reactive::{FLAG_BACK_EULER, Inductor};
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::{TokenError, Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale;
use crate::render::painter::{Color, Painter};
use crate::solver::stamper::{NodeId, Stamper};
use std::cell::RefCell;
use std::collections::HashMap;

/// Family name used for the feature switch.
pub const FAMILY: &str = "switches";
const CAT: &str = "Switches";

/// Registers the relays.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("RelayElm", FAMILY, "Add Relay", CAT, |x, y| {
            Box::new(RelayElm::new(x, y))
        })
        .dump(178, |a, b, c, d, f, st| Ok(Box::new(RelayElm::from_dump(a, b, c, d, f, st)?))),
    );
    r.add(
        ElementDescriptor::new("RelayCoilElm", FAMILY, "Add Relay Coil", CAT, |x, y| {
            Box::new(RelayCoilElm::new(x, y))
        })
        .dump(425, |a, b, c, d, f, st| Ok(Box::new(RelayCoilElm::from_dump(a, b, c, d, f, st)?))),
    );
    r.add(
        ElementDescriptor::new("RelayContactElm", FAMILY, "Add Relay Contact", CAT, |x, y| {
            Box::new(RelayContactElm::new(x, y))
        })
        .dump(426, |a, b, c, d, f, st| {
            Ok(Box::new(RelayContactElm::from_dump(a, b, c, d, f, st)?))
        }),
    );
}

thread_local! {
    /// Position and coil type each relay coil asks its contacts to take, by label. Coil and
    /// contacts are elements of the same simulation, so they share this per-thread table.
    static LINKS: RefCell<HashMap<String, (i32, i32)>> = RefCell::new(HashMap::new());
}

/// Tells the relay contacts called `label` which position to take (used by elements that act
/// like a relay coil, such as the motor protection switch).
pub fn set_contact_link(label: &str, position: i32, kind: i32) {
    set_link(label, position, kind);
}

fn set_link(label: &str, position: i32, kind: i32) {
    LINKS.with(|l| {
        l.borrow_mut().insert(label.to_string(), (position, kind));
    });
}

fn link(label: &str) -> Option<(i32, i32)> {
    LINKS.with(|l| l.borrow().get(label).copied())
}

// ---------------------------------------------------------------------------------------------
// Relay with coil and contacts in one element
// ---------------------------------------------------------------------------------------------

const FLAG_SWAP_COIL: i32 = 1;
const FLAG_SHOW_BOX: i32 = 2;
const FLAG_BOTH_SIDES_COIL: i32 = 4;
const FLAG_FLIP: i32 = 8;
const FLAG_PULLDOWN: i32 = 16;

fn coil_style_from_flags(f: i32) -> i32 {
    if f & FLAG_SWAP_COIL != 0 {
        2
    } else if f & FLAG_BOTH_SIDES_COIL != 0 {
        0
    } else {
        1
    }
}

/// A relay with a coil and one to four changeover poles.
#[derive(Clone, Debug)]
pub struct RelayElm {
    base: Base,
    model: RelayModel,
    ind: Inductor,
    coil_current: f64,
    switch_current: Vec<f64>,
    switch_curcount: Vec<f64>,
    coil_curcount: f64,
    d_position: f64,
    i_position: i32,
    on_state: bool,
    dflip: i32,
    openhs: i32,
    sw_posts: Vec<[Point; 3]>,
    sw_poles: Vec<[Point; 3]>,
    pt_switch: Vec<Point>,
    coil_posts: [Point; 2],
    coil_leads: [Point; 2],
    outline: [Point; 4],
    current_offset: [f64; 2],
}

impl RelayElm {
    fn blank(base: Base, model: RelayModel) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        let poles = model.pole_count.max(1) as usize;
        let mut r = RelayElm {
            base,
            model,
            ind: Inductor::default(),
            coil_current: 0.0,
            switch_current: vec![0.0; poles],
            switch_curcount: vec![0.0; poles],
            coil_curcount: 0.0,
            d_position: 0.0,
            i_position: 0,
            on_state: false,
            dflip: 1,
            openhs: 16,
            sw_posts: Vec::new(),
            sw_poles: Vec::new(),
            pt_switch: Vec::new(),
            coil_posts: [Point::default(); 2],
            coil_leads: [Point::default(); 2],
            outline: [Point::default(); 4],
            current_offset: [0.0; 2],
        };
        r.ind.setup(r.model.inductance, 0.0, FLAG_BACK_EULER);
        r
    }

    /// A relay being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let r = RelayElm::blank(Base::new(xx, yy), RelayModel::default_model());
        r.post_undump()
    }

    /// A relay read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let pole_count = st.next_i32()?;
        let inductance = st.next_f64()?;
        let coil_current = st.next_f64()?;
        let r_on = st.next_f64()?;
        let r_off = st.next_f64()?;
        let on_current = st.next_f64()?;
        let coil_r = st.next_f64()?;
        let mut off_current = on_current;
        let mut switching_time = 0.0;
        let mut position = 0;
        // like the original, keep what was read before the first missing field
        if let Ok(o) = st.next_f64() {
            off_current = o;
            if let Ok(t) = st.next_f64() {
                switching_time = t;
                if let Ok(p) = st.next_i32() {
                    position = p;
                }
            }
        }
        let model = RelayModel::with_parameters(
            inductance,
            r_on,
            r_off,
            on_current,
            off_current,
            coil_r,
            switching_time,
            coil_style_from_flags(f),
            f & FLAG_SHOW_BOX != 0,
            f & FLAG_PULLDOWN != 0,
            pole_count,
        );
        let mut r = RelayElm::blank(Base::from_dump(xa, ya, xb, yb, f), model);
        r.coil_current = coil_current;
        r.i_position = position;
        r.d_position = f64::from(position);
        Ok(r.post_undump())
    }

    fn post_undump(mut self) -> Self {
        if self.i_position == 1 {
            self.on_state = true;
        }
        if self.i_position == 2 {
            self.d_position = 0.5;
        }
        self.ind.setup(self.model.inductance, self.coil_current, FLAG_BACK_EULER);
        self.setup_poles();
        self.alloc_nodes();
        self
    }

    fn poles(&self) -> usize {
        self.model.pole_count.max(1) as usize
    }

    fn setup_poles(&mut self) {
        let n = self.poles();
        if self.switch_current.len() != n {
            self.switch_current = vec![0.0; n];
            self.switch_curcount = vec![0.0; n];
        }
    }

    fn n_coil(&self, k: usize) -> usize {
        3 * self.poles() + k
    }

    fn flags_for_dump(&self) -> i32 {
        let mut f = self.base.flags & FLAG_FLIP;
        match self.model.coil_style {
            2 => f |= FLAG_SWAP_COIL,
            0 => f |= FLAG_BOTH_SIDES_COIL,
            _ => {}
        }
        if self.model.show_box {
            f |= FLAG_SHOW_BOX;
        }
        if self.model.pulldown {
            f |= FLAG_PULLDOWN;
        }
        f
    }

    /// Changes the model through `edit`; a built-in or shared model is copied first.
    fn edit_model(&mut self, edit: impl FnOnce(&mut RelayModel)) {
        if self.model.read_only || self.model.built_in {
            let name = (1..)
                .map(|n| if n == 1 { "relaymodel".to_string() } else { format!("relaymodel-{n}") })
                .find(|n| RelayModel::lookup(n).is_none())
                .unwrap_or_else(|| "relaymodel".into());
            self.model.name = name;
            self.model.built_in = false;
            self.model.read_only = false;
        }
        edit(&mut self.model);
        RelayModel::define(self.model.clone());
        self.ind.setup(self.model.inductance, self.coil_current, FLAG_BACK_EULER);
    }
}

impl Element for RelayElm {
    impl_element_base!("RelayElm");

    fn model_buttons(&self) -> Vec<String> {
        let mut v = vec!["Create New Model".to_string()];
        if !self.model.read_only {
            v.push("Edit Model".into());
        }
        v
    }
    fn start_model_edit(&self, button: usize) -> Option<ModelEditor> {
        match button {
            0 => Some(ModelEditor::new_relay(&self.model)),
            1 if !self.model.read_only => Some(ModelEditor::Relay(self.model.clone())),
            _ => None,
        }
    }
    fn use_model(&mut self, name: &str) {
        if let Some(m) = RelayModel::lookup(name) {
            self.model = m;
            self.ind.setup(self.model.inductance, self.coil_current, FLAG_BACK_EULER);
            self.setup_poles();
        }
    }
    fn update_models(&mut self) {
        let name = self.model.name.clone();
        self.use_model(&name);
    }

    fn dump_type(&self) -> i32 {
        178
    }
    fn xml_dump_type(&self) -> String {
        "rl".into()
    }
    fn dump(&self) -> String {
        let mut b = self.base.clone();
        b.flags = self.flags_for_dump();
        let m = &self.model;
        format!(
            "{} {} {} {} {} {} {} {} {} {} {}",
            b.dump_prefix(178),
            m.pole_count,
            js_number(m.inductance),
            js_number(self.coil_current),
            js_number(m.r_on),
            js_number(m.r_off),
            js_number(m.on_current),
            js_number(m.coil_r),
            js_number(m.off_current),
            js_number(m.switching_time),
            self.i_position
        )
    }
    fn dump_model_xml(&self) -> Option<XmlOut> {
        (!self.model.built_in).then(|| self.model.dump_xml())
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.str_attr("mo", &self.model.name);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.double_attr("i", self.coil_current);
        out.int_attr("ip", self.i_position);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        if let Some(mo) = xml.attr("mo") {
            self.model = RelayModel::named_or_copy(mo, Some(&self.model));
        } else if xml.attr("ix").is_none() {
            let d = RelayModel::default();
            let f = self.base.flags;
            self.model = RelayModel::with_parameters(
                xml.double_attr("in", d.inductance),
                xml.double_attr("ron", d.r_on),
                xml.double_attr("roff", d.r_off),
                xml.double_attr("on", d.on_current),
                xml.double_attr("of", d.off_current),
                xml.double_attr("coR", d.coil_r),
                xml.double_attr("sw", d.switching_time),
                coil_style_from_flags(f),
                f & FLAG_SHOW_BOX != 0,
                f & FLAG_PULLDOWN != 0,
                xml.int_attr("po", 1),
            );
        }
        self.coil_current = xml.double_attr("i", self.coil_current);
        self.i_position = xml.int_attr("ip", self.i_position);
        self.d_position = f64::from(self.i_position);
        let r =
            std::mem::replace(self, RelayElm::blank(Base::new(0, 0), RelayModel::default_model()));
        *self = r.post_undump();
    }

    fn post_count(&self) -> usize {
        2 + self.poles() * 3
    }
    fn internal_node_count(&self) -> usize {
        1
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        if n < 3 * self.poles() {
            self.sw_posts.get(n / 3).map(|p| p[n % 3])
        } else {
            self.coil_posts.get(n - 3 * self.poles()).copied()
        }
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn pre_stamp(&mut self) {
        self.model = RelayModel::named_or_copy(&self.model.name.clone(), Some(&self.model));
        self.ind.setup(self.model.inductance, self.coil_current, FLAG_BACK_EULER);
        self.setup_poles();
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.setup_poles();
        self.alloc_nodes();
        let b = &self.base;
        let dsign = b.dsign;
        self.dflip = if b.flags & FLAG_FLIP != 0 { -dsign } else { dsign };
        self.openhs = -self.dflip * 16;
        self.base.calc_leads(32);
        let (p1, p2, l1, l2, dn) =
            (self.base.point1, self.base.point2, self.base.lead1, self.base.lead2, self.base.dn);
        let hs = self.openhs;
        let poles = self.poles();
        self.sw_posts.clear();
        self.sw_poles.clear();
        for i in 0..poles {
            let i = i as i32;
            let off = |k: i32| f64::from(-hs * 3 * i + k);
            self.sw_poles.push([
                interp_point_g(l1, l2, 0.0, off(0)),
                interp_point_g(l1, l2, 1.0, off(-hs)),
                interp_point_g(l1, l2, 1.0, off(hs)),
            ]);
            self.sw_posts.push([
                interp_point_g(p1, p2, 0.0, off(0)),
                interp_point_g(p1, p2, 1.0, off(-hs)),
                interp_point_g(p1, p2, 1.0, off(hs)),
            ]);
        }
        self.pt_switch = vec![Point::default(); poles];
        let style = self.model.coil_style;
        let x = if style == 2 { 1.0 } else { 0.0 };
        let box_size = if style != 0 {
            self.coil_posts = [
                interp_point_g(p1, p2, x, f64::from(hs * 2)),
                interp_point_g(p1, p2, x, f64::from(hs * 3)),
            ];
            self.coil_leads = [
                interp_point_g(p1, p2, 0.5, f64::from(hs * 2)),
                interp_point_g(p1, p2, 0.5, f64::from(hs * 3)),
            ];
            56
        } else {
            self.coil_posts = [
                interp_point_g(p1, p2, 0.0, f64::from(hs * 2)),
                interp_point_g(p1, p2, 1.0, f64::from(hs * 2)),
            ];
            self.coil_leads = [
                interp_point_g(p1, p2, 0.5 - 16.0 / dn, f64::from(hs * 2)),
                interp_point_g(p1, p2, 0.5 + 16.0 / dn, f64::from(hs * 2)),
            ];
            40
        };
        let w = (25.0 / dn).min(0.4);
        let df = f64::from(self.dflip);
        let far = -f64::from(hs * 3 * poles as i32) - 24.0 * df;
        self.outline = [
            interp_point_g(p1, p2, 0.5 - w, -f64::from(box_size) * df),
            interp_point_g(p1, p2, 0.5 + w, -f64::from(box_size) * df),
            interp_point_g(p1, p2, 0.5 + w, far),
            interp_point_g(p1, p2, 0.5 - w, far),
        ];
        let o1 = distance(self.coil_posts[0], self.coil_leads[0]);
        self.current_offset = [o1, o1 + distance(self.coil_leads[0], self.coil_leads[1])];
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let poles = self.poles();
        let (n1, n2) = (self.n_coil(0), self.n_coil(1));
        for i in 0..2 {
            set_voltage_color(g, &self.base, c, self.base.volts[n1 + i]);
            draw_thick_line(g, self.coil_leads[i], self.coil_posts[i]);
        }
        let x = usize::from(self.model.coil_style == 2);
        set_power_color_w(
            g,
            &self.base,
            c,
            self.coil_current * (self.base.volts[n1] - self.base.volts[n2]),
        );
        draw_coil(
            g,
            &self.base,
            c,
            self.dflip * 6,
            self.coil_leads[x],
            self.coil_leads[1 - x],
            self.base.volts[n1 + x],
            self.base.volts[n2 - x],
        );
        if self.model.show_box {
            g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
            for i in 0..4 {
                draw_thick_line(g, self.outline[i], self.outline[(i + 1) % 4]);
            }
        }
        g.set_color(Color::DARK_GRAY);
        let (p1, p2) = (self.base.point1, self.base.point2);
        let (hs, df) = (self.openhs, self.dflip);
        for i in 0..poles {
            let ii = i as i32;
            let a = if i == 0 {
                let off = if self.model.coil_style == 0 { 4 } else { 0 };
                interp_point_g(p1, p2, 0.5, f64::from(hs * 2 + 5 * df - ii * hs * 3 + off))
            } else {
                let o = (f64::from(hs) * (f64::from(-ii * 3 + 3) - 0.5 + self.d_position)) as i32
                    + 5 * df;
                interp_point_g(p1, p2, 0.5, f64::from(o))
            };
            let o2 = (f64::from(hs) * (f64::from(-ii * 3) - 0.5 + self.d_position)) as i32 - 5 * df;
            let b = interp_point_g(p1, p2, 0.5, f64::from(o2));
            g.set_line_dash(4, 4);
            g.draw_line(a.x, a.y, b.x, b.y);
            g.set_line_dash(0, 0);
        }
        for p in 0..poles {
            for i in 0..3 {
                set_voltage_color(g, &self.base, c, self.base.volts[p * 3 + i]);
                draw_thick_line(g, self.sw_posts[p][i], self.sw_poles[p][i]);
            }
            self.pt_switch[p] =
                interp_point(self.sw_poles[p][1], self.sw_poles[p][2], self.d_position);
            g.set_color(Color::LIGHT_GRAY);
            draw_thick_line(g, self.sw_poles[p][0], self.pt_switch[p]);
            self.switch_curcount[p] =
                self.base.update_dot_count_for(self.switch_current[p], self.switch_curcount[p], c);
            draw_dots(g, c, self.sw_posts[p][0], self.sw_poles[p][0], self.switch_curcount[p]);
            if self.i_position != 2 {
                let k = self.i_position as usize + 1;
                draw_dots(g, c, self.sw_poles[p][k], self.sw_posts[p][k], self.switch_curcount[p]);
            }
        }
        self.coil_curcount =
            self.base.update_dot_count_for(self.coil_current, self.coil_curcount, c);
        if self.coil_curcount != 0.0 {
            let cc = self.coil_curcount;
            draw_dots(g, c, self.coil_posts[0], self.coil_leads[0], cc);
            draw_dots(
                g,
                c,
                self.coil_leads[0],
                self.coil_leads[1],
                Base::add_cur_count(cc, self.current_offset[0]),
            );
            draw_dots(
                g,
                c,
                self.coil_leads[1],
                self.coil_posts[1],
                Base::add_cur_count(cc, self.current_offset[1]),
            );
        }
        draw_posts(g, &self.base, self, c);
        self.base.set_bbox_pts(self.outline[0], self.outline[2], 0.0);
        let (c0, c1) = (self.coil_posts[0], self.coil_posts[1]);
        self.base.adjust_bbox(c0.x, c0.y, c1.x, c1.y);
        let (s0, s1) = (self.sw_posts[0][0], self.sw_posts[0][1]);
        self.base.adjust_bbox(s0.x, s0.y, s1.x, s1.y);
    }

    fn visual_state(&self) -> Vec<f64> {
        let mut v = vec![self.d_position, f64::from(self.i_position), self.coil_current];
        v.extend(&self.switch_current);
        v
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if state.len() >= 3 {
            self.d_position = state[0];
            self.i_position = state[1] as i32;
            self.coil_current = state[2];
            for (d, s) in self.switch_current.iter_mut().zip(&state[3..]) {
                *d = *s;
            }
        }
    }

    fn reset(&mut self) {
        self.base.volts.fill(0.0);
        self.base.curcount = 0.0;
        self.ind.reset();
        self.coil_current = 0.0;
        self.coil_curcount = 0.0;
        self.switch_current.fill(0.0);
        self.switch_curcount.fill(0.0);
        self.d_position = 0.0;
        self.i_position = 0;
        self.on_state = false;
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        let (c1, c2, c3) = (self.n_coil(0), self.n_coil(1), self.n_coil(2));
        self.ind.stamp(s, n[c1], n[c3]);
        s.stamp_resistor(n[c3], n[c2], self.model.coil_r);
        if self.model.pulldown {
            for p in 0..self.poles() {
                s.stamp_resistor(n[p * 3 + 1], NodeId::GROUND, self.model.r_off);
                s.stamp_resistor(n[p * 3 + 2], NodeId::GROUND, self.model.r_off);
            }
        }
    }

    fn start_iteration(&mut self, s: &Stamper) {
        let (c1, c3) = (self.n_coil(0), self.n_coil(2));
        let vd = self.base.volts[c1] - self.base.volts[c3];
        self.ind.start_iteration(s, vd);
        if self.model.switching_time == 0.0 {
            // the original's first relay model: the armature follows the coil current
            let pmult = (1.3f64 + 1.0).sqrt();
            let p = self.coil_current * pmult / self.model.on_current;
            self.d_position = ((p * p).abs() - 1.3).clamp(0.0, 1.0);
            self.i_position = if self.d_position < 0.1 {
                0
            } else if self.d_position > 0.9 {
                1
            } else {
                2
            };
            return;
        }
        let abs = self.coil_current.abs();
        let step = s.time_step / self.model.switching_time;
        if self.on_state {
            if abs < self.model.off_current {
                self.on_state = false;
                self.i_position = 2;
            } else {
                self.d_position += step;
                if self.d_position >= 1.0 {
                    self.d_position = 1.0;
                    self.i_position = 1;
                }
            }
        } else if abs > self.model.on_current {
            self.on_state = true;
            self.i_position = 2;
        } else {
            self.d_position -= step;
            if self.d_position <= 0.0 {
                self.d_position = 0.0;
                self.i_position = 0;
            }
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        self.ind.do_step(s);
        let n = self.base.nodes.clone();
        let m = &self.model;
        for p in 0..self.poles() {
            let (a, b, c) = (n[p * 3], n[p * 3 + 1], n[p * 3 + 2]);
            match self.i_position {
                0 => {
                    s.stamp_resistor(a, b, m.r_on);
                    if !m.pulldown {
                        s.stamp_resistor(a, c, m.r_off);
                    }
                }
                1 => {
                    s.stamp_resistor(a, c, m.r_on);
                    if !m.pulldown {
                        s.stamp_resistor(a, b, m.r_off);
                    }
                }
                _ => {
                    s.stamp_resistor(a, b, m.r_off);
                    s.stamp_resistor(a, c, m.r_off);
                }
            }
        }
    }

    fn calculate_current(&mut self) {
        let (c1, c3) = (self.n_coil(0), self.n_coil(2));
        let vd = self.base.volts[c1] - self.base.volts[c3];
        self.coil_current = self.ind.calculate_current(vd);
        for p in 0..self.poles() {
            self.switch_current[p] = if self.i_position == 2 {
                0.0
            } else {
                (self.base.volts[p * 3] - self.base.volts[p * 3 + 1 + self.i_position as usize])
                    / self.model.r_on
            };
        }
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        let poles = self.poles();
        if n < 3 * poles {
            let (p, k) = (n / 3, n % 3);
            if k == 0 {
                return -self.switch_current[p];
            }
            if k as i32 == 1 + self.i_position {
                return self.switch_current[p];
            }
            return 0.0;
        }
        if n == 3 * poles { -self.coil_current } else { self.coil_current }
    }

    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 / 3 == n2 / 3
    }
    fn has_ground_connection(&self, n: usize) -> bool {
        self.model.pulldown && n < self.n_coil(0)
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let name =
            if self.model.switching_time == 0.0 { "old model" } else { self.model.name.as_str() };
        arr.push(format!("{} ({name})", locale::ls("relay")));
        arr.push(locale::ls(if self.i_position == 0 { "off" } else { "on" }));
        for i in 0..self.poles() {
            arr.push(format!("I{} = {}", i + 1, current_d_text(self.switch_current[i])));
        }
        arr.push(format!("{} = {}", locale::ls("coil I"), current_d_text(self.coil_current)));
        let v = &self.base.volts;
        arr.push(format!(
            "{} = {}",
            locale::ls("coil Vd"),
            voltage_d_text(v[self.n_coil(0)] - v[self.n_coil(1)])
        ));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        let m = &self.model;
        match n {
            0 => {
                let names = RelayModel::names();
                let sel = names.iter().position(|x| *x == m.name).unwrap_or(0);
                Some(EditInfo::choice("Model", names, sel))
            }
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        let v = ei.value;
        let on = ei.checkbox.unwrap_or(false);
        match n {
            0 => {
                let names = RelayModel::names();
                if let Some(m) = names.get(ei.choice_index).and_then(|nm| RelayModel::lookup(nm)) {
                    self.model = m;
                    self.ind.setup(self.model.inductance, self.coil_current, FLAG_BACK_EULER);
                }
            }
            1 if v > 0.0 => self.edit_model(|m| m.inductance = v),
            2 if v > 0.0 => self.edit_model(|m| m.r_on = v),
            3 if v > 0.0 => self.edit_model(|m| m.r_off = v),
            4 if v > 0.0 => self.edit_model(|m| m.on_current = v),
            5 if v > 0.0 => self.edit_model(|m| m.off_current = v),
            6 if v >= 1.0 => self.edit_model(|m| m.pole_count = v as i32),
            7 if v > 0.0 => self.edit_model(|m| m.coil_r = v),
            8 if v > 0.0 => self.edit_model(|m| m.switching_time = v),
            9 => {
                let style = ei.choice_index as i32;
                self.edit_model(|m| m.coil_style = style);
            }
            10 => self.edit_model(|m| m.show_box = on),
            11 => self.edit_model(|m| m.pulldown = on),
            _ => {}
        }
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        if self.base.dx == 0 {
            self.base.flags ^= FLAG_FLIP;
        }
        self.base.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        if self.base.dy == 0 {
            self.base.flags ^= FLAG_FLIP;
        }
        self.base.flip_y(center2);
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIP;
        self.base.flip_xy(xmy);
        self.set_points();
    }
}

// ---------------------------------------------------------------------------------------------
// Relay coil
// ---------------------------------------------------------------------------------------------

const TYPE_ON_DELAY: i32 = 1;
const TYPE_OFF_DELAY: i32 = 2;
const TYPE_LATCHING: i32 = 3;
const TYPE_LATCHING_ON: i32 = 4;
const TYPE_LATCHING_OFF: i32 = 5;

fn is_latching(t: i32) -> bool {
    matches!(t, TYPE_LATCHING | TYPE_LATCHING_ON | TYPE_LATCHING_OFF)
}

/// The coil of a relay whose contacts are separate elements with the same label.
#[derive(Clone, Debug)]
pub struct RelayCoilElm {
    base: Base,
    label: String,
    inductance: f64,
    ind: Inductor,
    on_current: f64,
    off_current: f64,
    coil_r: f64,
    switching_time: f64,
    switching_time_on: f64,
    switching_time_off: f64,
    kind: i32,
    state: i32,
    switch_position: i32,
    coil_current: f64,
    coil_curcount: f64,
    avg_current: f64,
    last_transition: f64,
    coil_posts: [Point; 2],
    coil_leads: [Point; 2],
    outline: [Point; 4],
    extra: [Point; 4],
    current_offset2: f64,
}

impl RelayCoilElm {
    fn blank(base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        RelayCoilElm {
            base,
            label: "label".into(),
            inductance: 0.2,
            ind: Inductor::default(),
            on_current: 0.02,
            off_current: 0.015,
            coil_r: 20.0,
            switching_time: 5e-3,
            switching_time_on: 0.0,
            switching_time_off: 0.0,
            kind: 0,
            state: 0,
            switch_position: 0,
            coil_current: 0.0,
            coil_curcount: 0.0,
            avg_current: 0.0,
            last_transition: 0.0,
            coil_posts: [Point::default(); 2],
            coil_leads: [Point::default(); 2],
            outline: [Point::default(); 4],
            extra: [Point::default(); 4],
            current_offset2: 0.0,
        }
    }

    /// A coil being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut c = RelayCoilElm::blank(Base::new(xx, yy));
        c.ind.setup(c.inductance, 0.0, FLAG_BACK_EULER);
        c.alloc_nodes();
        c
    }

    /// A coil read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut c = RelayCoilElm::blank(Base::from_dump(xa, ya, xb, yb, f));
        c.label = unescape(st.next_token().ok_or(TokenError::Missing)?);
        c.inductance = st.next_f64()?;
        c.coil_current = st.next_f64()?;
        c.on_current = st.next_f64()?;
        c.coil_r = st.next_f64()?;
        c.off_current = st.next_f64()?;
        c.switching_time = st.next_f64()?;
        c.kind = st.next_i32()?;
        c.state = st.next_i32()?;
        c.switch_position = st.next_i32()?;
        c.ind.setup(c.inductance, c.coil_current, FLAG_BACK_EULER);
        c.alloc_nodes();
        Ok(c)
    }

    fn push_positions(&self) {
        if self.kind != TYPE_LATCHING_ON && self.kind != TYPE_LATCHING_OFF {
            set_link(&self.label, 1 - self.switch_position, self.kind);
        }
    }
}

impl Element for RelayCoilElm {
    impl_element_base!("RelayCoilElm");

    fn dump_type(&self) -> i32 {
        425
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {} {} {} {} {} {} {}",
            self.base.dump_prefix(425),
            escape(&self.label),
            js_number(self.inductance),
            js_number(self.coil_current),
            js_number(self.on_current),
            js_number(self.coil_r),
            js_number(self.off_current),
            js_number(self.switching_time),
            self.kind,
            self.state,
            self.switch_position
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.label = xml.string_attr("lb", &self.label);
        self.inductance = xml.double_attr("in", self.inductance);
        self.on_current = xml.double_attr("oc", self.on_current);
        self.coil_r = xml.double_attr("cr", self.coil_r);
        self.off_current = xml.double_attr("ofc", self.off_current);
        self.switching_time = xml.double_attr("swt", self.switching_time);
        self.kind = xml.int_attr("tp", self.kind);
        self.coil_current = xml.double_attr("ci", self.coil_current);
        self.state = xml.int_attr("st", self.state);
        self.switch_position = xml.int_attr("sp", self.switch_position);
        self.ind.setup(self.inductance, self.coil_current, FLAG_BACK_EULER);
        self.alloc_nodes();
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.str_attr("lb", &self.label);
        out.double_attr("in", self.inductance);
        out.double_attr("oc", self.on_current);
        out.double_attr("cr", self.coil_r);
        out.double_attr("ofc", self.off_current);
        out.double_attr("swt", self.switching_time);
        out.int_attr("tp", self.kind);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.double_attr("ci", self.coil_current);
        out.int_attr("st", self.state);
        out.int_attr("sp", self.switch_position);
    }

    fn post_count(&self) -> usize {
        2
    }
    fn internal_node_count(&self) -> usize {
        1
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.coil_posts.get(n).copied()
    }
    fn non_linear(&self) -> bool {
        false
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.alloc_nodes();
        let (p1, p2, dn) = (self.base.point1, self.base.point2, self.base.dn);
        let dsign = self.base.dsign;
        self.coil_posts = [p1, p2];
        let box_size = 32.0;
        let w = (12.0 / dn).min(0.4);
        self.coil_leads = [interp_point(p1, p2, 0.5 - w), interp_point(p1, p2, 0.5 + w)];
        let d = f64::from(dsign);
        self.outline[0] = interp_point_g(p1, p2, 0.5 - w, -box_size * d);
        self.outline[1] = interp_point_g(p1, p2, 0.5 + w, -box_size * d);
        self.outline[3] = interp_point_g(p1, p2, 0.5 - w, box_size * d);
        self.outline[2] = interp_point_g(p1, p2, 0.5 + w, box_size * d);
        let o1 = distance(self.coil_posts[0], self.coil_leads[0]);
        self.current_offset2 = o1 + distance(self.coil_leads[0], self.coil_leads[1]);
        let (l0, l1) = (self.coil_leads[0], self.coil_leads[1]);
        if is_latching(self.kind) {
            self.extra = [
                interp_point_g(l0, l1, 0.3, 8.0),
                interp_point_g(l0, l1, 0.3, 0.0),
                interp_point_g(l0, l1, 0.7, 0.0),
                interp_point_g(l0, l1, 0.7, -8.0),
            ];
        } else {
            self.extra[0] = self.outline[0];
            self.extra[3] = self.outline[1];
            self.extra[1] = interp_point_g(l0, l1, 0.0, -box_size + 12.0);
            self.extra[2] = interp_point_g(l0, l1, 1.0, -box_size + 12.0);
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        for i in 0..2 {
            set_voltage_color(g, &self.base, c, self.base.volts[i]);
            draw_thick_line(g, self.coil_leads[i], self.coil_posts[i]);
        }
        set_power_color_w(
            g,
            &self.base,
            c,
            self.coil_current * (self.base.volts[0] - self.base.volts[1]),
        );
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        for i in 0..4 {
            draw_thick_line(g, self.outline[i], self.outline[(i + 1) % 4]);
        }
        let e = self.extra;
        if is_latching(self.kind) {
            for i in 0..3 {
                draw_thick_line(g, e[i], e[i + 1]);
            }
            if self.kind == TYPE_LATCHING_ON || self.kind == TYPE_LATCHING_OFF {
                g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
                let t = if self.kind == TYPE_LATCHING_ON { "S" } else { "R" };
                g.draw_string(t, f64::from(e[0].x + 3), f64::from(e[0].y + 9));
            }
        } else if self.kind == TYPE_ON_DELAY {
            draw_thick_line(g, e[1], e[2]);
            draw_thick_line(g, e[0], e[2]);
            draw_thick_line(g, e[1], e[3]);
        } else if self.kind == TYPE_OFF_DELAY {
            g.fill_rect(
                f64::from(e[0].x),
                f64::from(e[0].y),
                f64::from(e[2].x - e[0].x),
                f64::from(e[2].y - e[0].y),
            );
        }
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
        let b = &self.base;
        if b.x == b.x2 {
            g.draw_string(
                &self.label,
                f64::from(self.outline[2].x + 10),
                f64::from((b.y + b.y2) / 2 + 4),
            );
        } else {
            let w = g.measure_text(&self.label);
            g.draw_string(
                &self.label,
                f64::from((b.x + b.x2) / 2) - w / 2.0,
                f64::from(self.outline[1].y + 15),
            );
        }
        self.coil_curcount =
            self.base.update_dot_count_for(self.coil_current, self.coil_curcount, c);
        if self.coil_curcount != 0.0 {
            let cc = self.coil_curcount;
            draw_dots(g, c, self.coil_posts[0], self.coil_leads[0], cc);
            draw_dots(
                g,
                c,
                self.coil_leads[1],
                self.coil_posts[1],
                Base::add_cur_count(cc, self.current_offset2),
            );
        }
        draw_posts(g, &self.base, self, c);
        self.base.set_bbox_pts(self.outline[0], self.outline[2], 0.0);
        let (c0, c1) = (self.coil_posts[0], self.coil_posts[1]);
        self.base.adjust_bbox(c0.x, c0.y, c1.x, c1.y);
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.coil_current, f64::from(self.switch_position), f64::from(self.state)]
    }
    fn set_visual_state(&mut self, s: &[f64]) {
        if let [c, p, st] = s {
            self.coil_current = *c;
            self.switch_position = *p as i32;
            self.state = *st as i32;
        }
    }

    fn reset(&mut self) {
        self.base.volts.fill(0.0);
        self.base.curcount = 0.0;
        self.ind.reset();
        self.coil_current = 0.0;
        self.coil_curcount = 0.0;
        self.avg_current = 0.0;
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        self.ind.stamp(s, n[0], n[2]);
        s.stamp_resistor(n[2], n[1], self.coil_r);
        match self.kind {
            TYPE_ON_DELAY => {
                self.switching_time_on = self.switching_time;
                self.switching_time_off = 0.0;
            }
            TYPE_OFF_DELAY => {
                self.switching_time_off = self.switching_time;
                self.switching_time_on = 0.0;
            }
            _ => {
                self.switching_time_off = self.switching_time;
                self.switching_time_on = self.switching_time;
            }
        }
        self.push_positions();
    }

    fn start_iteration(&mut self, s: &Stamper) {
        let vd = self.base.volts[0] - self.base.volts[2];
        self.ind.start_iteration(s, vd);
        let abs = self.coil_current.abs();
        let a = (-s.time_step * 1e3).exp();
        self.avg_current = a * self.avg_current + (1.0 - a) * abs;
        let old = self.switch_position;
        match self.state {
            0 => {
                if self.avg_current > self.on_current {
                    self.last_transition = s.t;
                    self.state = 1;
                }
            }
            1 => {
                if self.avg_current < self.off_current {
                    self.state = 0;
                } else if s.t - self.last_transition > self.switching_time_on {
                    self.state = 2;
                    match self.kind {
                        TYPE_LATCHING => self.switch_position = 1 - self.switch_position,
                        TYPE_LATCHING_ON => {
                            self.switch_position = 1;
                            set_link(&self.label, 0, self.kind);
                        }
                        TYPE_LATCHING_OFF => {
                            self.switch_position = 0;
                            set_link(&self.label, 1, self.kind);
                        }
                        _ => self.switch_position = 1,
                    }
                }
            }
            2 => {
                if self.avg_current < self.off_current {
                    self.last_transition = s.t;
                    self.state = 3;
                }
            }
            _ => {
                if self.avg_current > self.on_current {
                    self.state = 2;
                } else if s.t - self.last_transition > self.switching_time_off {
                    self.state = 0;
                    if !is_latching(self.kind) {
                        self.switch_position = 0;
                    }
                }
            }
        }
        if old != self.switch_position {
            self.push_positions();
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        self.ind.do_step(s);
    }
    fn calculate_current(&mut self) {
        let vd = self.base.volts[0] - self.base.volts[2];
        self.coil_current = self.ind.calculate_current(vd);
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 0 { -self.coil_current } else { self.coil_current }
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let state = if self.switch_position == 0 { " (off)" } else { " (on)" };
        arr.push(format!("{}{state}", locale::ls("relay")));
        arr.push(format!("{} = {}", locale::ls("coil I"), current_d_text(self.coil_current)));
        arr.push(format!(
            "{} = {}",
            locale::ls("coil Vd"),
            voltage_d_text(self.base.volts[0] - self.base.volts[1])
        ));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::choice(
                "Type",
                vec![
                    "Normal".into(),
                    "On Delay".into(),
                    "Off Delay".into(),
                    "Latching".into(),
                    "Latching (Set Coil)".into(),
                    "Latching (Reset Coil)".into(),
                ],
                self.kind.clamp(0, 5) as usize,
            )),
            1 => Some(EditInfo::new("Inductance (H)", self.inductance, 0.0, 0.0).set_positive()),
            2 => Some(EditInfo::new("On Current (A)", self.on_current, 0.0, 0.0).set_positive()),
            3 => Some(EditInfo::new("Off Current (A)", self.off_current, 0.0, 0.0).set_positive()),
            4 => {
                Some(EditInfo::new("Coil Resistance (ohms)", self.coil_r, 0.0, 0.0).set_positive())
            }
            5 => Some(
                EditInfo::new("Switching Time (s)", self.switching_time, 0.0, 0.0).set_positive(),
            ),
            6 => Some(EditInfo::text("Label (for linking)", &self.label)),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        let v = ei.value;
        match n {
            0 => {
                self.kind = ei.choice_index as i32;
                self.set_points();
            }
            1 if v > 0.0 => {
                self.inductance = v;
                self.ind.setup(v, self.coil_current, FLAG_BACK_EULER);
            }
            2 if v > 0.0 => self.on_current = v,
            3 if v > 0.0 => self.off_current = v,
            4 if v > 0.0 => self.coil_r = v,
            5 if v > 0.0 => self.switching_time = v,
            6 => {
                if let Some(t) = &ei.text {
                    self.label.clone_from(t);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Relay contact
// ---------------------------------------------------------------------------------------------

const FLAG_NORMALLY_CLOSED: i32 = 2;
const FLAG_IEC: i32 = 4;

/// A relay contact operated by the coil with the same label.
#[derive(Clone, Debug)]
pub struct RelayContactElm {
    base: Base,
    label: String,
    r_on: f64,
    r_off: f64,
    kind: i32,
    i_position: i32,
    openhs: i32,
    switch_current: f64,
    switch_curcount: f64,
    sw_posts: [Point; 3],
    sw_poles: [Point; 3],
    pt_switch: Point,
    extra: [Point; 5],
}

impl RelayContactElm {
    fn blank(base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        RelayContactElm {
            base,
            label: "label".into(),
            r_on: 0.05,
            r_off: 1e6,
            kind: 0,
            i_position: 0,
            openhs: 16,
            switch_current: 0.0,
            switch_curcount: 0.0,
            sw_posts: [Point::default(); 3],
            sw_poles: [Point::default(); 3],
            pt_switch: Point::default(),
            extra: [Point::default(); 5],
        }
    }

    /// A contact being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut c = RelayContactElm::blank(Base::new(xx, yy));
        c.base.flags |= FLAG_IEC;
        c
    }

    /// A contact read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut c = RelayContactElm::blank(Base::from_dump(xa, ya, xb, yb, f));
        c.label = unescape(st.next_token().ok_or(TokenError::Missing)?);
        c.r_on = st.next_f64()?;
        c.r_off = st.next_f64()?;
        if let Ok(p) = st.next_i32() {
            c.i_position = p;
        }
        Ok(c)
    }

    fn normally_closed(&self) -> bool {
        self.base.flags & FLAG_NORMALLY_CLOSED != 0
    }
    fn iec(&self) -> bool {
        self.base.flags & FLAG_IEC != 0
    }

    fn follow_coil(&mut self) {
        if let Some((pos, kind)) = link(&self.label) {
            self.i_position = if self.normally_closed() { 1 - pos } else { pos };
            self.kind = kind;
        }
    }
}

impl Element for RelayContactElm {
    impl_element_base!("RelayContactElm");

    fn dump_type(&self) -> i32 {
        426
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(426),
            escape(&self.label),
            js_number(self.r_on),
            js_number(self.r_off),
            self.i_position
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.label = xml.string_attr("lb", &self.label);
        self.r_on = xml.double_attr("ron", self.r_on);
        self.r_off = xml.double_attr("roff", self.r_off);
        self.i_position = xml.int_attr("ip", self.i_position);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.str_attr("lb", &self.label);
        out.double_attr("ron", self.r_on);
        out.double_attr("roff", self.r_off);
        out.int_attr("ip", self.i_position);
    }

    fn post_count(&self) -> usize {
        2
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.sw_posts.get(n).copied().filter(|_| n < 2)
    }
    fn non_linear(&self) -> bool {
        true
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.alloc_nodes();
        self.openhs = self.base.dsign * 16;
        self.base.calc_leads(32);
        let b = &self.base;
        let (p1, p2, l1, l2) = (b.point1, b.point2, b.lead1, b.lead2);
        let hs = f64::from(self.openhs);
        self.sw_poles = [
            interp_point_g(l1, l2, 0.0, 0.0),
            interp_point_g(l1, l2, 1.0, 0.0),
            interp_point_g(l1, l2, 1.0, hs),
        ];
        self.sw_posts = [
            interp_point_g(p1, p2, 0.0, 0.0),
            interp_point_g(p1, p2, 1.0, 0.0),
            interp_point_g(p1, p2, 1.0, hs),
        ];
        if self.iec() {
            let ds = f64::from(22 * b.dsign);
            self.extra[2] = interp_point_g(l1, l2, 0.5 - 2.0 / 32.0, ds);
            self.extra[3] = interp_point_g(l1, l2, 0.5 + 2.0 / 32.0, ds);
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        for i in 0..2 {
            set_voltage_color(g, &self.base, c, self.base.volts[i]);
            draw_thick_line(g, self.sw_posts[i], self.sw_poles[i]);
        }
        self.pt_switch =
            interp_point(self.sw_poles[1], self.sw_poles[2], f64::from(self.i_position));
        g.set_color(Color::LIGHT_GRAY);
        draw_thick_line(g, self.sw_poles[0], self.pt_switch);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
        let b = &self.base;
        if b.x == b.x2 {
            let y = self.sw_poles[usize::from(b.y >= b.y2)].y;
            g.draw_string(&self.label, f64::from(b.x + 10), f64::from(y - 5));
        } else {
            let w = g.measure_text(&self.label);
            g.draw_string(&self.label, f64::from((b.x + b.x2) / 2) - w / 2.0, f64::from(b.y + 15));
        }
        if self.iec() && (self.kind == TYPE_ON_DELAY || self.kind == TYPE_OFF_DELAY) {
            g.set_color(Color::LIGHT_GRAY);
            let (l1, l2) = (self.base.lead1, self.base.lead2);
            let lift = if self.i_position == 1 { f64::from(self.openhs / 2) } else { 0.0 };
            self.extra[0] = interp_point_g(l1, l2, 0.5 - 2.0 / 32.0, lift);
            self.extra[1] = interp_point_g(l1, l2, 0.5 + 2.0 / 32.0, lift);
            g.draw_line(self.extra[0].x, self.extra[0].y, self.extra[2].x, self.extra[2].y);
            g.draw_line(self.extra[1].x, self.extra[1].y, self.extra[3].x, self.extra[3].y);
            let dsign = self.base.dsign;
            let mut ang = -f64::from(-self.base.dy * dsign).atan2(f64::from(self.base.dx * dsign));
            let ds = f64::from(22 * dsign);
            if self.kind == TYPE_OFF_DELAY {
                ang += std::f64::consts::PI;
                self.extra[4] = interp_point_g(l1, l2, 0.5, ds + f64::from(6 * dsign));
            } else {
                self.extra[4] = interp_point_g(l1, l2, 0.5, ds - f64::from(5 * dsign));
            }
            g.begin_path();
            let pi = std::f64::consts::PI;
            g.arc(
                f64::from(self.extra[4].x),
                f64::from(self.extra[4].y),
                6.0,
                -pi / 8.0 + ang,
                pi * 9.0 / 8.0 + ang,
            );
            g.stroke();
        }
        self.switch_curcount =
            self.base.update_dot_count_for(self.switch_current, self.switch_curcount, c);
        draw_dots(g, c, self.sw_posts[0], self.sw_poles[0], self.switch_curcount);
        if self.i_position == 0 {
            draw_dots(g, c, self.sw_poles[1], self.sw_posts[1], self.switch_curcount);
        }
        draw_posts(g, &self.base, self, c);
        let (p1, p2, hs) = (self.base.point1, self.base.point2, self.openhs);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![f64::from(self.i_position), f64::from(self.kind), self.switch_current]
    }
    fn set_visual_state(&mut self, s: &[f64]) {
        if let [p, k, c] = s {
            self.i_position = *p as i32;
            self.kind = *k as i32;
            self.switch_current = *c;
        }
    }

    fn reset(&mut self) {
        self.base.volts.fill(0.0);
        self.base.curcount = 0.0;
        self.switch_current = 0.0;
        self.switch_curcount = 0.0;
        self.i_position = 0;
    }

    fn do_step(&mut self, s: &mut Stamper) {
        self.follow_coil();
        let n = &self.base.nodes;
        s.stamp_resistor(n[0], n[1], if self.i_position == 0 { self.r_on } else { self.r_off });
    }

    fn calculate_current(&mut self) {
        self.switch_current = if self.i_position == 1 {
            0.0
        } else {
            (self.base.volts[0] - self.base.volts[1 + self.i_position as usize]) / self.r_on
        };
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 0 {
            -self.switch_current
        } else if n as i32 == 1 + self.i_position {
            self.switch_current
        } else {
            0.0
        }
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let state = match self.i_position {
            0 => " (off)",
            1 => " (on)",
            _ => "",
        };
        arr.push(format!("{}{state}", locale::ls("relay")));
        arr.push(format!("I = {}", current_d_text(self.switch_current)));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("On Resistance (ohms)", self.r_on, 0.0, 0.0).set_positive()),
            1 => Some(EditInfo::new("Off Resistance (ohms)", self.r_off, 0.0, 0.0).set_positive()),
            2 => Some(EditInfo::text("Label (for linking)", &self.label)),
            3 => Some(EditInfo::checkbox("Normally Closed", self.normally_closed())),
            4 => Some(EditInfo::checkbox("IEC Symbol", self.iec())),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 if ei.value > 0.0 => self.r_on = ei.value,
            1 if ei.value > 0.0 => self.r_off = ei.value,
            2 => {
                if let Some(t) = &ei.text {
                    self.label.clone_from(t);
                }
            }
            3 => self.base.flags = ei.change_flag(self.base.flags, FLAG_NORMALLY_CLOSED),
            4 => {
                self.base.flags = ei.change_flag(self.base.flags, FLAG_IEC);
                self.set_points();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_dump_round_trips() {
        let line = "1 0.2 -0.000004999900001999959 0.05 1000000 0.02 20 0.02 0.005 0";
        let relay = RelayElm::from_dump(224, 192, 336, 192, 22, &mut Tokenizer::new(line)).unwrap();
        assert_eq!(relay.post_count(), 5);
        assert_eq!(relay.dump(), format!("178 224 192 336 192 22 {line}"));
    }

    #[test]
    fn coil_and_contact_dumps_round_trip() {
        let coil = RelayCoilElm::from_dump(
            432,
            160,
            432,
            320,
            0,
            &mut Tokenizer::new("Q1 0.2 0 0.02 20 0.015 0.005 0 0 0"),
        )
        .unwrap();
        assert_eq!(coil.dump(), "425 432 160 432 320 0 Q1 0.2 0 0.02 20 0.015 0.005 0 0 0");
        let contact = RelayContactElm::from_dump(
            848,
            416,
            848,
            576,
            6,
            &mut Tokenizer::new("abc 0.05 1000000 0"),
        )
        .unwrap();
        assert_eq!(contact.dump(), "426 848 416 848 576 6 abc 0.05 1000000 0");
    }

    #[test]
    fn contact_follows_the_coil_through_its_label() {
        let mut contact = RelayContactElm::new(0, 0);
        contact.label = "lnk-test".into();
        set_link("lnk-test", 1, 0);
        contact.follow_coil();
        assert_eq!(contact.i_position, 1);
        contact.base.flags |= FLAG_NORMALLY_CLOSED;
        contact.follow_coil();
        assert_eq!(contact.i_position, 0);
    }
}
