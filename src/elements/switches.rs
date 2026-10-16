// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (SwitchElm).
//! Switches.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::{TokenError, Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Painter, TextAlign};
use crate::solver::path::{PathKind, Validator};
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "switches";

/// Flag: draw the IEC symbol.
pub const FLAG_IEC: i32 = 2;
/// Flag: the dump carries a label.
pub const FLAG_LABEL: i32 = 4;
/// Half height of the open blade.
const OPEN_HS: i32 = 16;
/// Extra pixels around the blade that still count as a click on the switch.
const CLICK_SLACK: i32 = 6;

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("SwitchElm", FAMILY, "Add Switch", "Passive Components", |x, y| {
            Box::new(SwitchElm::new(x, y))
        })
        .dump(i32::from(b's'), |a, b, c, d, f, st| {
            Ok(Box::new(SwitchElm::from_dump(a, b, c, d, f, st)?))
        })
        .icon("switch")
        .shortcut('s'),
    );
    r.add(ElementDescriptor::new(
        "PushSwitchElm",
        FAMILY,
        "Add Push Switch",
        "Passive Components",
        |x, y| Box::new(SwitchElm::momentary(x, y)),
    ));
    r.add(
        ElementDescriptor::new(
            "Switch2Elm",
            FAMILY,
            "Add SPDT Switch",
            "Passive Components",
            |x, y| Box::new(Switch2Elm::new(x, y)),
        )
        .dump(i32::from(b'S'), |a, b, c, d, f, st| {
            Ok(Box::new(Switch2Elm::from_dump(a, b, c, d, f, st)?))
        })
        .icon("spdt")
        .shortcut('S'),
    );
}

/// A single-pole single-throw switch.
#[derive(Clone, Debug)]
pub struct SwitchElm {
    base: Base,
    /// Returns to its rest position when released.
    pub momentary: bool,
    /// 0 is closed, 1 is open.
    pub position: i32,
    /// Number of positions a click cycles through.
    pub pos_count: i32,
    /// On resistance in ohms; zero makes the switch an ideal wire.
    pub resistance: f64,
    /// Label linking switches that toggle together.
    pub label: Option<String>,
    /// Keyboard shortcut character.
    pub key_shortcut: Option<String>,
    extra: Vec<Point>,
}

impl SwitchElm {
    /// A switch being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        SwitchElm {
            base: Base::new(xx, yy),
            momentary: false,
            position: 0,
            pos_count: 2,
            resistance: 0.0,
            label: None,
            key_shortcut: None,
            extra: Vec::new(),
        }
    }

    /// A push switch being placed: open until pressed, then back to open on release.
    pub fn momentary(xx: i32, yy: i32) -> Self {
        let mut sw = SwitchElm::new(xx, yy);
        sw.momentary = true;
        sw.position = 1;
        sw
    }

    /// A switch read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        Self::from_dump_as(xa, ya, xb, yb, f, st, false)
    }

    /// Reads a switch; `logic_input` swaps the meaning of the words `true` and `false`, as a
    /// logic input stores them the other way round.
    pub fn from_dump_as(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
        logic_input: bool,
    ) -> Result<Self, TokenError> {
        let mut sw = SwitchElm {
            base: Base::from_dump(xa, ya, xb, yb, f),
            momentary: false,
            position: 0,
            pos_count: 2,
            resistance: 0.0,
            label: None,
            key_shortcut: None,
            extra: Vec::new(),
        };
        let tok = st.next_token().ok_or(TokenError::Missing)?;
        sw.position = match tok {
            "true" => i32::from(!logic_input),
            "false" => i32::from(logic_input),
            other => other.trim().parse().map_err(|_| TokenError::Invalid(other.to_string()))?,
        };
        let mm = st.next_token().ok_or(TokenError::Missing)?;
        sw.momentary = mm.eq_ignore_ascii_case("true");
        if f & FLAG_LABEL != 0 {
            sw.label = st.next_token().map(unescape);
        }
        Ok(sw)
    }

    /// The fields after the common prefix: position, momentary flag and, when set, the label.
    pub fn dump_fields(&self) -> String {
        let mut s = format!("{} {}", self.position, self.momentary);
        if self.base.flags & FLAG_LABEL != 0 {
            if let Some(l) = &self.label {
                s.push(' ');
                s.push_str(&escape(l));
            }
        }
        s
    }

    /// Flips the switch to its other position.
    pub fn simple_toggle(&mut self) {
        self.position += 1;
        if self.position >= self.pos_count {
            self.position = 0;
        }
    }

    fn use_iec(&self) -> bool {
        self.base.flags & FLAG_IEC != 0
    }
}

impl Element for SwitchElm {
    impl_element_base!("SwitchElm");

    fn dump_type(&self) -> i32 {
        i32::from(b's')
    }

    fn dump(&self) -> String {
        format!("{} {}", self.base.dump_prefix(self.dump_type()), self.dump_fields())
    }

    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.position = xml.int_attr("p", self.position);
        self.momentary = xml.bool_attr("mm", self.momentary);
        self.label = xml.attr("lab").map(str::to_string).or(self.label.take());
        self.key_shortcut = xml.attr("key").map(str::to_string).or(self.key_shortcut.take());
        self.resistance = xml.double_attr("r", 0.0);
    }

    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        if self.position != 0 {
            out.int_attr("p", self.position);
        }
        if self.momentary {
            out.bool_attr("mm", true);
        }
        if let Some(l) = &self.label {
            out.str_attr("lab", l);
        }
        if let Some(k) = &self.key_shortcut {
            out.str_attr("key", k);
        }
        if self.resistance != 0.0 {
            out.double_attr("r", self.resistance);
        }
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(32);
        self.extra.clear();
        if self.use_iec() {
            let (l1, l2) = (self.base.lead1, self.base.lead2);
            let at = |f: f64, g: f64| interp_point_g(l1, l2, f, g);
            self.extra = vec![
                at(0.5, f64::from(OPEN_HS / 2)),
                at(0.5, 24.0),
                at(0.4, 24.0),
                at(0.6, 24.0),
                at(0.5, 19.0),
                at(0.4, 16.0),
                at(0.5, 13.0),
            ];
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let hs1 = if self.position == 1 { 0 } else { 2 };
        let hs2 = if self.position == 1 { OPEN_HS } else { 2 };
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(OPEN_HS));
        draw_2_leads(g, &self.base, c);
        if self.position == 0 {
            do_dots(g, &mut self.base, c);
        }
        let highlight = self.base.needs_highlight();
        if !highlight {
            g.set_color(c.white);
        }
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let ps = interp_point_g(l1, l2, 0.0, f64::from(hs1));
        let ps2 = interp_point_g(l1, l2, 1.0, f64::from(hs2));
        draw_thick_line(g, ps, ps2);

        if let Some(label) = self.label.clone() {
            g.set_color(if highlight { c.select_color } else { c.white });
            let b = &self.base;
            if b.dy.abs() > b.dx.abs() {
                let ly = if b.y < b.y2 { l1.y } else { l2.y };
                g.draw_string(&label, f64::from(b.x + 10), f64::from(ly - 5));
            } else {
                g.save();
                g.set_text_align(TextAlign::Center);
                let ty = if b.x2 > b.x { b.y + 15 } else { b.y - 15 };
                g.draw_string(&label, f64::from((b.x + b.x2) / 2), f64::from(ty));
                g.restore();
            }
        }

        if self.use_iec() && self.extra.len() == 7 {
            let e = self.extra.clone();
            g.draw_line(e[2].x, e[2].y, e[3].x, e[3].y);
            g.set_line_dash(3, 3);
            let hs = if self.position == 1 { OPEN_HS / 2 } else { 2 };
            let e0 = interp_point_g(l1, l2, 0.5, f64::from(hs));
            if self.momentary {
                g.draw_line(e[1].x, e[1].y, e0.x, e0.y);
            } else {
                g.draw_line(e[6].x, e[6].y, e0.x, e0.y);
                g.draw_line(e[1].x, e[1].y, e[4].x, e[4].y);
            }
            g.set_line_dash(0, 0);
            if !self.momentary {
                g.draw_line(e[4].x, e[4].y, e[5].x, e[5].y);
                g.draw_line(e[6].x, e[6].y, e[5].x, e[5].y);
            }
        }
        draw_posts(g, &self.base, self, c);
    }

    fn calculate_current(&mut self) {
        self.base.current = if self.position == 1 {
            0.0
        } else if self.resistance > 0.0 {
            (self.base.volts[0] - self.base.volts[1]) / self.resistance
        } else {
            self.base.current
        };
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if self.position == 0 && self.resistance > 0.0 {
            s.stamp_resistor(self.base.nodes[0], self.base.nodes[1], self.resistance);
        }
    }

    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        self.position == 0
    }
    fn is_wire_equivalent(&self) -> bool {
        self.position == 0 && self.resistance == 0.0
    }
    fn is_removable_wire(&self) -> bool {
        self.position == 0 && self.resistance == 0.0
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(if self.momentary { "push switch (SPST)" } else { "switch (SPST)" }.into());
        if self.position == 1 {
            arr.push("open".into());
            arr.push(format!("Vd = {}", voltage_d_text(self.get_voltage_diff())));
        } else {
            arr.push("closed".into());
            arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
            arr.push(format!("I = {}", current_d_text(self.get_current())));
        }
    }

    fn elm_type(&self) -> Option<String> {
        Some("switch (SPST)".into())
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::checkbox("Momentary Switch", self.momentary)),
            1 => Some(EditInfo::checkbox("IEC Symbol", self.use_iec())),
            2 => Some(EditInfo::text("Label (for linking)", self.label.as_deref().unwrap_or(""))),
            3 => Some(EditInfo::text(
                "Keyboard Shortcut",
                self.key_shortcut.as_deref().unwrap_or(""),
            )),
            4 => Some(
                EditInfo::new("On Resistance (ohms)", self.resistance, 0.0, 0.0).set_non_negative(),
            ),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.momentary = ei.checkbox.unwrap_or(false),
            1 => {
                self.base.flags = ei.change_flag(self.base.flags, FLAG_IEC);
                self.set_points();
            }
            2 => {
                let t = ei.text.clone().unwrap_or_default();
                if t.is_empty() {
                    self.label = None;
                    self.base.flags &= !FLAG_LABEL;
                } else {
                    self.label = Some(t);
                    self.base.flags |= FLAG_LABEL;
                }
            }
            3 => {
                let t = ei.text.clone().unwrap_or_default();
                let t = t.trim();
                self.key_shortcut = t.chars().next().map(|ch| ch.to_lowercase().to_string());
            }
            4 => self.resistance = ei.value,
            _ => {}
        }
    }

    fn shortcut(&self) -> i32 {
        i32::from(b's')
    }

    fn click_rect(&self) -> Option<Rect> {
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let blade = interp_point_g(l1, l2, 0.0, f64::from(OPEN_HS));
        let (x0, x1) = (l1.x.min(l2.x).min(blade.x), l1.x.max(l2.x).max(blade.x));
        let (y0, y1) = (l1.y.min(l2.y).min(blade.y), l1.y.max(l2.y).max(blade.y));
        // a few pixels of slack make the blade easier to hit than the original's exact box
        Some(Rect::new(
            x0 - CLICK_SLACK,
            y0 - CLICK_SLACK,
            x1 - x0 + 2 * CLICK_SLACK + 1,
            y1 - y0 + 2 * CLICK_SLACK + 1,
        ))
    }
    fn simple_toggle(&mut self) {
        SwitchElm::simple_toggle(self);
    }
    fn link_label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    fn toggle_key(&self) -> Option<&str> {
        self.key_shortcut.as_deref()
    }
    fn is_momentary(&self) -> bool {
        self.momentary
    }
}

/// Flag: the centre position is off (only for two throws).
pub const FLAG_CENTER_OFF: i32 = 1;
/// Flag: the element was mirrored, used to keep linked switches in step.
pub const FLAG_FLIPPED: i32 = 8;

/// A single-pole multi-throw switch (SPDT by default).
#[derive(Clone, Debug)]
pub struct Switch2Elm {
    inner: SwitchElm,
    /// Group number that makes several switches move together; 0 for none.
    pub link: i32,
    /// Number of throw positions.
    pub throw_count: usize,
    swposts: Vec<Point>,
    swpoles: Vec<Point>,
}

impl Switch2Elm {
    /// A switch being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut inner = SwitchElm::new(xx, yy);
        inner.base.no_diagonal = true;
        Switch2Elm { inner, link: 0, throw_count: 2, swposts: Vec::new(), swpoles: Vec::new() }
    }

    /// A switch read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut inner = SwitchElm::from_dump(xa, ya, xb, yb, f, st)?;
        inner.base.no_diagonal = true;
        let link = st.next_i32()?;
        let throw_count = st.next_i32().ok().and_then(|n| usize::try_from(n).ok()).unwrap_or(2);
        Ok(Switch2Elm {
            inner,
            link,
            throw_count: throw_count.max(2),
            swposts: Vec::new(),
            swpoles: Vec::new(),
        })
    }

    fn center_off(&self) -> bool {
        self.inner.base.flags & FLAG_CENTER_OFF != 0 && self.throw_count == 2
    }

    fn pos_count(&self) -> i32 {
        if self.center_off() { 3 } else { self.throw_count as i32 }
    }

    fn is_center_off_now(&self) -> bool {
        self.inner.position == 2 && self.center_off()
    }

    fn flipped(&self) -> bool {
        self.inner.base.flags & FLAG_FLIPPED != 0
    }

    fn throw_node(&self) -> usize {
        usize::try_from(self.inner.position).unwrap_or(0) + 1
    }
}

impl Element for Switch2Elm {
    fn base(&self) -> &Base {
        &self.inner.base
    }
    fn base_mut(&mut self) -> &mut Base {
        &mut self.inner.base
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn class_name(&self) -> &'static str {
        "Switch2Elm"
    }

    fn dump_type(&self) -> i32 {
        i32::from(b'S')
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {}",
            self.inner.base.dump_prefix(self.dump_type()),
            self.inner.dump_fields(),
            self.link,
            self.throw_count
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.inner.undump_xml(xml);
        self.link = xml.int_attr("li", self.link);
        self.throw_count =
            usize::try_from(xml.int_attr("th", self.throw_count as i32)).unwrap_or(2).max(2);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        self.inner.dump_xml(out);
        out.int_attr("li", self.link);
        out.int_attr("th", self.throw_count as i32);
    }

    fn post_count(&self) -> usize {
        1 + self.throw_count
    }
    fn voltage_source_count(&self) -> usize {
        if self.is_center_off_now() || self.inner.resistance > 0.0 { 0 } else { 1 }
    }

    fn get_post(&self, n: usize) -> Option<Point> {
        if n == 0 { Some(self.inner.base.point1) } else { self.swposts.get(n - 1).copied() }
    }

    fn set_points(&mut self) {
        self.inner.base.no_diagonal = true;
        self.inner.base.set_points();
        self.inner.base.calc_leads(32);
        let b = &self.inner.base;
        let n = self.throw_count;
        let mut posts = Vec::with_capacity(n);
        let mut poles = Vec::with_capacity(n + 1);
        for i in 0..n {
            let mut hs = -OPEN_HS * (i as i32 - (n as i32 - 1) / 2);
            if n == 2 && i == 0 {
                hs = OPEN_HS;
            }
            poles.push(interp_point_g(b.lead1, b.lead2, 1.0, f64::from(hs)));
            posts.push(interp_point_g(b.point1, b.point2, 1.0, f64::from(hs)));
        }
        poles.push(b.lead2); // the centre-off position
        self.swposts = posts;
        self.swpoles = poles;
        self.alloc_nodes();
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        if self.swposts.is_empty() {
            return;
        }
        let (p1, p2) = (self.inner.base.point1, self.inner.base.point2);
        let last = self.throw_count - 1;
        self.inner.base.set_bbox_pts(p1, p2, f64::from(OPEN_HS));
        self.inner.base.adjust_bbox_pts(self.swposts[0], self.swposts[last]);
        let b = &self.inner.base;
        set_voltage_color(g, b, c, b.volts[0]);
        draw_thick_line(g, b.point1, b.lead1);
        for i in 0..self.throw_count {
            set_voltage_color(g, b, c, b.volts[i + 1]);
            draw_thick_line(g, self.swpoles[i], self.swposts[i]);
        }
        if !b.needs_highlight() {
            g.set_color(c.white);
        }
        let pos = usize::try_from(self.inner.position).unwrap_or(0).min(self.swpoles.len() - 1);
        draw_thick_line(g, b.lead1, self.swpoles[pos]);
        self.inner.base.update_dot_count(c);
        let b = &self.inner.base;
        draw_dots(g, c, b.point1, b.lead1, b.curcount);
        if !self.is_center_off_now() {
            draw_dots(g, c, self.swpoles[pos], self.swposts[pos.min(last)], b.curcount);
        }
        draw_posts(g, &self.inner.base, self, c);
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 0 {
            -self.inner.base.current
        } else if n == self.throw_node() {
            self.inner.base.current
        } else {
            0.0
        }
    }

    fn calculate_current(&mut self) {
        let b = &self.inner.base;
        let node = self.throw_node();
        self.inner.base.current = if self.is_center_off_now() {
            0.0
        } else if self.inner.resistance > 0.0 {
            (b.volts[0] - b.volts.get(node).copied().unwrap_or(0.0)) / self.inner.resistance
        } else {
            b.current
        };
    }

    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.inner.base.volt_source = Some(v);
    }

    fn voltage_source_nodes(&self) -> Option<(NodeId, NodeId)> {
        let b = &self.inner.base;
        (!self.is_center_off_now()).then(|| (b.nodes[0], b.nodes[self.throw_node()]))
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if self.is_center_off_now() {
            return;
        }
        let (n0, n1) = (self.inner.base.nodes[0], self.inner.base.nodes[self.throw_node()]);
        if self.inner.resistance > 0.0 {
            s.stamp_resistor(n0, n1, self.inner.resistance);
        } else if let Some(vs) = self.inner.base.volt_source {
            s.set_vs_nodes(vs, n0, n1);
            s.stamp_voltage_source(n0, n1, vs, 0.0);
        }
    }

    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        if self.is_center_off_now() {
            return false;
        }
        let t = self.throw_node();
        (n1 == 0 && n2 == t) || (n1 == t && n2 == 0)
    }
    fn is_wire_equivalent(&self) -> bool {
        self.inner.resistance == 0.0
    }
    fn is_removable_wire(&self) -> bool {
        false
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        if self.is_center_off_now() || self.inner.resistance > 0.0 {
            return true;
        }
        let (n0, n1) = (self.inner.base.nodes[0], self.inner.base.nodes[self.throw_node()]);
        if v.find_path(PathKind::Voltage, idx, n0, n1) {
            // a loop of ideal sources and wires: give the switch a small resistance
            self.inner.resistance = 0.001;
            return false;
        }
        true
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let kind = if self.link == 0 { "S" } else { "D" };
        let throws =
            if self.throw_count > 2 { format!("{}T", self.throw_count) } else { "DT".to_string() };
        arr.push(format!("switch ({kind}P{throws})"));
        arr.push(format!("I = {}", current_d_text(self.get_current())));
    }
    fn elm_type(&self) -> Option<String> {
        Some("switch (SPDT)".into())
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            1 => Some(
                EditInfo::new("Group Number (for linking)", f64::from(self.link), 0.0, 100.0)
                    .set_dimensionless()
                    .disallow_sliders(),
            ),
            2 => Some(
                EditInfo::new("# of Throws", self.throw_count as f64, 2.0, 10.0)
                    .set_dimensionless()
                    .disallow_sliders(),
            ),
            _ => self.inner.get_edit_info(n),
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            1 => self.link = ei.value as i32,
            2 => {
                if ei.value >= 2.0 {
                    self.throw_count = ei.value as usize;
                }
                if self.throw_count > 2 {
                    self.inner.momentary = false;
                }
                self.inner.position = self.inner.position.min(self.pos_count() - 1);
                self.set_points();
            }
            _ => self.inner.set_edit_value(n, ei),
        }
    }

    fn click_rect(&self) -> Option<Rect> {
        let (first, last) = (self.swpoles.first()?, self.swpoles.get(self.throw_count - 1)?);
        let l1 = self.inner.base.lead1;
        let xs = [l1.x, first.x, last.x];
        let ys = [l1.y, first.y, last.y];
        let (x0, x1) = (*xs.iter().min()?, *xs.iter().max()?);
        let (y0, y1) = (*ys.iter().min()?, *ys.iter().max()?);
        Some(Rect::new(
            x0 - CLICK_SLACK,
            y0 - CLICK_SLACK,
            x1 - x0 + 2 * CLICK_SLACK + 1,
            y1 - y0 + 2 * CLICK_SLACK + 1,
        ))
    }
    fn simple_toggle(&mut self) {
        self.inner.position += 1;
        if self.inner.position >= self.pos_count() {
            self.inner.position = 0;
        }
    }
    fn link_label(&self) -> Option<&str> {
        self.inner.label.as_deref()
    }
    fn is_momentary(&self) -> bool {
        self.inner.momentary
    }
    fn toggle_key(&self) -> Option<&str> {
        self.inner.key_shortcut.as_deref()
    }
    fn link_group(&self) -> i32 {
        self.link
    }
    fn switch_state(&self) -> Option<(i32, i32, bool)> {
        Some((self.inner.position, self.pos_count(), self.flipped()))
    }
    fn set_switch_position(&mut self, pos: i32) {
        self.inner.position = pos;
    }
    fn on_flipped(&mut self, _axis: Flip) {
        self.inner.position = self.pos_count() - 1 - self.inner.position;
        self.inner.base.flags ^= FLAG_FLIPPED;
    }

    fn shortcut(&self) -> i32 {
        i32::from(b'S')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn switch(line: &str, flags: i32) -> SwitchElm {
        let mut st = Tokenizer::new(line);
        SwitchElm::from_dump(0, 0, 64, 0, flags, &mut st).unwrap()
    }

    #[test]
    fn reads_boolean_and_numeric_positions() {
        assert_eq!(switch("1 false", 0).position, 1);
        assert_eq!(switch("true false", 0).position, 1);
        assert_eq!(switch("false true", 0).position, 0);
        assert!(switch("0 true", 0).momentary);
    }

    #[test]
    fn closed_switch_is_a_wire_and_open_one_is_not() {
        let mut s = switch("0 false", 0);
        assert!(s.is_wire_equivalent());
        s.simple_toggle();
        assert!(!s.is_wire_equivalent());
        assert!(!s.get_connection(0, 1));
        s.simple_toggle();
        assert_eq!(s.position, 0);
    }

    fn spdt_circuit(position: i32) -> crate::solver::sim::Simulation {
        use crate::elements::registry::Registry;
        use crate::format::parser::parse;
        let text = format!(
            "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 10 0 0 0.5\nw 0 64 64 64 0\nS 64 64 128 64 0 {position} false 0 2\nr 128 48 192 48 0 1000\nr 128 80 192 80 0 2000\nw 0 0 192 0 0\nw 192 0 192 48 0\nw 192 48 192 80 0\n"
        );
        let r = parse(&text, &Registry::standard());
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let mut sim = crate::solver::sim::Simulation::new(r.circuit.elements, Default::default());
        sim.analyze().unwrap();
        sim.run_steps(3).unwrap();
        sim
    }

    #[test]
    fn spdt_routes_the_source_to_the_selected_throw() {
        let sim = spdt_circuit(0);
        assert!((sim.elements[3].get_current().abs() - 10.0 / 1000.0).abs() < 1e-9);
        assert!(sim.elements[4].get_current().abs() < 1e-9);
        let sim = spdt_circuit(1);
        assert!(sim.elements[3].get_current().abs() < 1e-9);
        assert!((sim.elements[4].get_current().abs() - 10.0 / 2000.0).abs() < 1e-9);
    }

    #[test]
    fn spdt_dump_round_trips_and_toggles_between_positions() {
        let mut st = Tokenizer::new("0 false 3 2");
        let mut s = Switch2Elm::from_dump(64, 64, 128, 64, 0, &mut st).unwrap();
        assert_eq!((s.link, s.throw_count), (3, 2));
        assert_eq!(s.dump(), "S 64 64 128 64 0 0 false 3 2");
        assert_eq!(s.post_count(), 3);
        s.set_points();
        assert_eq!(s.get_post(1), Some(Point::new(128, 48)));
        assert_eq!(s.get_post(2), Some(Point::new(128, 80)));
        s.simple_toggle();
        assert_eq!(s.inner.position, 1);
        s.simple_toggle();
        assert_eq!(s.inner.position, 0);
    }

    #[test]
    fn linked_spdt_switches_move_together_and_mirror_when_flipped() {
        use crate::app::session::Session;
        use crate::elements::registry::Registry;
        let text = "$ 1 0.000005 10 50 5 50 5e-11\nS 0 0 64 0 0 0 false 7 2\nS 0 64 64 64 0 0 false 7 2\nr 64 0 64 64 0 100\n";
        let mut s = Session::load(text, &Registry::standard()).unwrap();
        s.sim.elements[1].on_flipped(Flip::X);
        s.view = crate::render::scene::View::default();
        assert!(s.press_at(40.0, 0.0));
        let pos = |s: &Session, i: usize| s.sim.elements[i].switch_state().unwrap().0;
        assert_eq!(pos(&s, 0), 1);
        // the second switch is flipped, so it takes the opposite position
        assert_eq!(pos(&s, 1), 0);
    }

    #[test]
    fn spdt_switches_of_the_three_way_example_are_solved() {
        // the unselected throws sit in other matrices; the source must follow the selected one
        use crate::elements::registry::Registry;
        use crate::format::parser::parse;
        let text = std::fs::read_to_string("assets/circuits/3way.txt").unwrap();
        let r = parse(&text, &Registry::standard());
        let mut sim = crate::solver::sim::Simulation::new(
            r.circuit.elements,
            r.circuit.options.sim_settings(),
        );
        sim.analyze().unwrap();
        sim.run_steps(50).unwrap();
        assert!(sim.elements.iter().any(|e| e.get_current().abs() > 1e-6));
    }

    #[test]
    fn push_switch_starts_open_and_is_momentary() {
        let s = SwitchElm::momentary(0, 0);
        assert!(s.momentary && s.position == 1);
        assert!(!s.is_wire_equivalent());
    }

    #[test]
    fn click_area_covers_the_blade() {
        let mut s = switch("0 false", 0);
        s.base.set_position(0, 0, 64, 0);
        s.set_points();
        let r = s.click_rect().unwrap();
        assert!(r.contains(32, 0));
        assert!(r.contains(32, -12));
        assert!(!r.contains(32, 40));
    }

    #[test]
    fn label_round_trips_through_the_text_dump() {
        let s = switch("0 false a\\sb", FLAG_LABEL);
        assert_eq!(s.label.as_deref(), Some("a b"));
        assert!(s.dump().ends_with("0 false a\\sb"));
    }
}
