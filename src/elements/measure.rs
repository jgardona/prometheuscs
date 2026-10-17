// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (AmmeterElm, LineElm).
//! Measurement and annotation elements: the ammeter and the plain line.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::{Color, Painter, TextBaseline};
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name used for the feature switch.
pub const FAMILY: &str = "outputs";

const FLAG_SHOW_CURRENT: i32 = 1;
const FLAG_CIRCLE: i32 = 2;
const CIRCLE_SIZE: i32 = 12;
const CAT: &str = "Outputs and Labels";

/// Registers the measurement elements.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("StopTriggerElm", FAMILY, "Add Stop Trigger", CAT, |x, y| {
            Box::new(StopTrigger::new(x, y))
        })
        .dump(408, |a, b, c, d, f, st| Ok(Box::new(StopTrigger::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("DataRecorderElm", FAMILY, "Add Data Export", CAT, |x, y| {
            Box::new(DataRecorder::new(x, y))
        })
        .dump(210, |a, b, c, d, f, st| Ok(Box::new(DataRecorder::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("TestPointElm", FAMILY, "Add Test Point", CAT, |x, y| {
            Box::new(TestPoint::new(x, y))
        })
        .dump(368, |a, b, c, d, f, st| Ok(Box::new(TestPoint::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("WattmeterTrueElm", FAMILY, "Add Wattmeter", CAT, |x, y| {
            Box::new(Wattmeter::new(x, y, true))
        })
        .hidden(),
    );
    r.add(
        ElementDescriptor::new("WattmeterElm", FAMILY, "Add Wattmeter (old)", CAT, |x, y| {
            Box::new(Wattmeter::new(x, y, false))
        })
        .dump(420, |a, b, c, d, f, st| Ok(Box::new(Wattmeter::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new(
            "AmmeterElm",
            FAMILY,
            "Add Ammeter",
            "Outputs and Labels",
            |x, y| Box::new(Ammeter::new(x, y)),
        )
        .dump(370, |a, b, c, d, f, st| Ok(Box::new(Ammeter::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new("LineElm", FAMILY, "Add Line", "Outputs and Labels", |x, y| {
            Box::new(Line::new(x, y))
        })
        .dump(423, |a, b, c, d, f, _| Ok(Box::new(Line::from_dump(a, b, c, d, f)))),
    );
}

/// An ammeter: a zero-volt source whose current is shown (or its RMS value).
pub struct Ammeter {
    base: Base,
    /// 0 current, 1 RMS current.
    meter: i32,
    scale: i32,
    zero_count: i32,
    rms: f64,
    total: f64,
    count: f64,
    max_i: f64,
    min_i: f64,
    increasing: bool,
    decreasing: bool,
    selected_value: f64,
    center: Point,
    arrow: Polygon,
}

impl Ammeter {
    fn blank(base: Base) -> Self {
        Ammeter {
            base,
            meter: 0,
            scale: 0,
            zero_count: 0,
            rms: 0.0,
            total: 0.0,
            count: 0.0,
            max_i: 0.0,
            min_i: 0.0,
            increasing: true,
            decreasing: true,
            selected_value: 0.0,
            center: Point::default(),
            arrow: Polygon::default(),
        }
    }

    /// An ammeter being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut a = Self::blank(Base::new(x, y));
        a.base.flags = FLAG_SHOW_CURRENT | FLAG_CIRCLE;
        a
    }

    /// An ammeter read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut a = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        a.meter = st.next_i32().unwrap_or(0);
        a.scale = st.next_i32().unwrap_or(0);
        a
    }

    fn as_circle(&self) -> bool {
        self.base.flags & FLAG_CIRCLE != 0
    }

    fn finish_cycle(&mut self) {
        self.total /= self.count;
        self.rms = self.total.sqrt();
        if self.rms.is_nan() {
            self.rms = 0.0;
        }
        self.count = 0.0;
        self.total = 0.0;
    }
}

impl Element for Ammeter {
    impl_element_base!("AmmeterElm");

    fn dump_type(&self) -> i32 {
        370
    }
    fn dump(&self) -> String {
        format!("{} {} {}", self.base.dump_prefix(370), self.meter, self.scale)
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("me", self.meter);
        out.int_attr("sc", self.scale);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.meter = xml.int_attr("me", self.meter);
        self.scale = xml.int_attr("sc", self.scale);
    }

    fn voltage_source_count(&self) -> usize {
        1
    }
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base.volt_source = Some(v);
    }
    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.base.volt_source {
            let n = &self.base.nodes;
            s.set_vs_nodes(vs, n[0], n[1]);
            s.stamp_voltage_source(n[0], n[1], vs, 0.0);
        }
    }
    fn voltage_source_nodes(&self) -> Option<(NodeId, NodeId)> {
        let n = &self.base.nodes;
        Some((n[0], n[1]))
    }
    // not optimised out even though it is a wire: the current is needed every step
    fn is_wire_equivalent(&self) -> bool {
        true
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let (p1, p2) = (self.base.point1, self.base.point2);
        let mid = interp_point(p1, p2, 0.6);
        self.center = interp_point(p1, p2, 0.5);
        self.arrow = calc_arrow(p1, mid, 14.0, 7.0);
    }

    fn step_finished(&mut self, _s: &mut Stamper) {
        let current = self.base.current;
        self.count += 1.0;
        self.total += current * current;
        if current > self.max_i && self.increasing {
            self.max_i = current;
            self.increasing = true;
            self.decreasing = false;
        }
        if current < self.max_i && self.increasing {
            self.min_i = current;
            self.increasing = false;
            self.decreasing = true;
            self.finish_cycle();
        }
        if current < self.min_i && self.decreasing {
            self.min_i = current;
            self.increasing = false;
            self.decreasing = true;
        }
        if current > self.min_i && self.decreasing {
            self.max_i = current;
            self.increasing = true;
            self.decreasing = false;
            self.finish_cycle();
        }
        if current == 0.0 {
            self.zero_count += 1;
            if self.zero_count > 5 {
                self.total = 0.0;
                self.rms = 0.0;
                self.max_i = 0.0;
                self.min_i = 0.0;
            }
        } else {
            self.zero_count = 0;
        }
        self.selected_value = if self.meter == 1 { self.rms } else { current };
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let v0 = self.base.volts[0];
        set_voltage_color(g, &self.base, c, v0);
        let mut width = 4.0;
        let (p1, p2) = (self.base.point1, self.base.point2);
        if self.as_circle() {
            self.base.calc_leads(CIRCLE_SIZE * 2);
            let (l1, l2) = (self.base.lead1, self.base.lead2);
            set_voltage_color(g, &self.base, c, v0);
            draw_thick_line(g, p1, l1);
            draw_thick_line(g, l2, p2);
            g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
            draw_thick_circle(g, self.center.x, self.center.y, CIRCLE_SIZE);
            let (cx, cy) = (self.center.x, self.center.y);
            draw_centered_text(g, &mut self.base, "A", cx, cy, true);
            g.set_color(c.white);
            g.set_font_size(c.value_font_size);
            let len = f64::from(CIRCLE_SIZE * 2);
            let dn = self.base.dn;
            let mut plus = interp_point_g(
                p1,
                p2,
                (dn / 2.0 - len / 2.0 - 4.0) / dn,
                f64::from(-10 * self.base.dsign),
            );
            if self.base.y2 > self.base.y {
                plus.y += 4;
            }
            if self.base.y > self.base.y2 {
                plus.y += 3;
            }
            let w = g.measure_text("+") as i32;
            g.draw_string("+", f64::from(plus.x - w / 2), f64::from(plus.y));
            width = f64::from(CIRCLE_SIZE);
        } else {
            draw_thick_line(g, p1, p2);
            fill_polygon(g, &self.arrow);
        }
        do_dots(g, &mut self.base, c);
        self.base.set_bbox_pts(p1, p2, width);
        let scale = Scale::from_index(self.scale);
        let s = match self.meter {
            1 => unit_text_with_scale(self.rms, "A(rms)", scale, false),
            _ => unit_text_with_scale(self.base.current, "A", scale, false),
        };
        draw_values(g, &self.base, c, Some(&s), width, false, false);
        draw_posts(g, &self.base, self, c);
    }

    fn get_power(&self) -> f64 {
        0.0
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("Ammeter".into());
        arr.push(match self.meter {
            1 => format!("Irms = {}", unit_text(self.rms, "A")),
            _ => format!("I = {}", unit_text(self.base.current, "A")),
        });
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => {
                let mut e = EditInfo::choice(
                    "Value",
                    vec!["Current".into(), "RMS Current".into()],
                    self.meter.max(0) as usize,
                );
                e.value = self.selected_value;
                Some(e)
            }
            1 => Some(EditInfo::choice(
                "Scale",
                vec!["Auto".into(), "A".into(), "mA".into(), format!("{}A", crate::locale::MU)],
                self.scale.max(0) as usize,
            )),
            2 => Some(EditInfo::checkbox("Circular Symbol", self.as_circle())),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.meter = ei.choice_index as i32,
            1 => self.scale = ei.choice_index as i32,
            2 => self.base.flags = ei.change_flag(self.base.flags, FLAG_CIRCLE),
            _ => {}
        }
    }
}

/// A plain line drawn on the circuit.
pub struct Line {
    base: Base,
}

impl Line {
    /// A line being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut base = Base::new(x, y);
        base.x2 = x;
        base.y2 = y;
        base.init_bounding_box();
        Line { base }
    }

    /// A line read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32) -> Self {
        Line { base: Base::from_dump(xa, ya, xb, yb, f) }
    }
}

impl Element for Line {
    impl_element_base!("LineElm");

    fn dump_type(&self) -> i32 {
        423
    }
    fn post_count(&self) -> usize {
        0
    }
    fn is_graphic(&self) -> bool {
        true
    }
    fn num_handles(&self) -> usize {
        2
    }
    fn drag(&mut self, xx: i32, yy: i32) {
        self.base.x2 = xx;
        self.base.y2 = yy;
        self.set_points();
    }
    fn creation_failed(&self) -> bool {
        let b = &self.base;
        f64::from((b.x - b.x2).pow(2) + (b.y - b.y2).pow(2)).sqrt() < 16.0
    }
    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.set_color(if self.base.needs_highlight() { c.select_color } else { Color::GRAY });
        let b = &mut self.base;
        let (x, y, x2, y2) = (b.x, b.y, b.x2, b.y2);
        b.set_bbox(x, y, x2, y2);
        g.draw_line(x, y, x2, y2);
    }
    fn mouse_distance(&self, gx: i32, gy: i32) -> i32 {
        let d2 = self.base.line_distance_sq(gx, gy);
        if d2 <= 100 { d2 } else { -1 }
    }
    fn shortcut(&self) -> i32 {
        0
    }
}

// ---------------------------------------------------------------------------------------------
// Wattmeters
// ---------------------------------------------------------------------------------------------

/// A wattmeter with a current coil and a voltage coil. The "old" one measures the voltage with a
/// second zero-volt source; the new one with a large resistor.
pub struct Wattmeter {
    base: Base,
    new_style: bool,
    width: i32,
    /// 0 instantaneous power, 1 average power.
    meter: i32,
    sources: [Option<VsId>; 2],
    currents: [f64; 2],
    curcounts: [f64; 2],
    posts: [Point; 4],
    inner: [Point; 4],
    rect: [Point; 4],
    center: Point,
    max_text_len: i32,
    // old-style average: whole cycles between rising crossings of the running mean
    avg_power: f64,
    total_energy: f64,
    cycle_time: f64,
    last_cycle_time: f64,
    run_energy: f64,
    run_time: f64,
    zero_time: f64,
    peak: f64,
    trough: f64,
    was_above_mid: bool,
    have_full_cycle: bool,
    // new-style average: between the extremes of the power
    total_power: f64,
    count: f64,
    zero_count: i32,
    max_p: f64,
    min_p: f64,
    increasing: bool,
    decreasing: bool,
}

impl Wattmeter {
    fn blank(base: Base, new_style: bool) -> Self {
        Wattmeter {
            base,
            new_style,
            width: 0,
            meter: 0,
            sources: [None; 2],
            currents: [0.0; 2],
            curcounts: [0.0; 2],
            posts: [Point::default(); 4],
            inner: [Point::default(); 4],
            rect: [Point::default(); 4],
            center: Point::default(),
            max_text_len: 5,
            avg_power: 0.0,
            total_energy: 0.0,
            cycle_time: 0.0,
            last_cycle_time: 0.0,
            run_energy: 0.0,
            run_time: 0.0,
            zero_time: 0.0,
            peak: 0.0,
            trough: 0.0,
            was_above_mid: false,
            have_full_cycle: false,
            total_power: 0.0,
            count: 0.0,
            zero_count: 0,
            max_p: 0.0,
            min_p: 0.0,
            increasing: true,
            decreasing: true,
        }
    }

    /// A wattmeter being placed (`new_style` picks the resistor-based voltage coil).
    pub fn new(x: i32, y: i32, new_style: bool) -> Self {
        Self::blank(Base::new(x, y), new_style)
    }

    /// An old-style wattmeter read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut w = Self::blank(Base::from_dump(xa, ya, xb, yb, f), false);
        w.width = st.next_i32().unwrap_or(0);
        w.meter = st.next_i32().unwrap_or(0);
        w
    }

    fn voltage(&self) -> f64 {
        let v = &self.base.volts;
        if self.new_style { v[1] - v[0] } else { v[2] - v[0] }
    }

    fn power(&self) -> f64 {
        self.voltage() * self.currents[1]
    }

    fn update_old_average(&mut self, dt: f64) {
        let p = self.power();
        self.cycle_time += dt;
        self.total_energy += p * dt;
        self.run_time += dt;
        self.run_energy += p * dt;
        let mid = self.run_energy / self.run_time;
        self.peak = self.peak.max(p);
        self.trough = self.trough.min(p);
        let band = (self.peak - self.trough) * 0.05 + self.peak.abs() * 1e-9;
        let above = if self.was_above_mid { p > mid - band } else { p > mid + band };
        if above && !self.was_above_mid {
            if self.have_full_cycle {
                self.avg_power = self.total_energy / self.cycle_time;
                if self.avg_power.is_nan() {
                    self.avg_power = 0.0;
                }
                self.last_cycle_time = self.cycle_time;
            } else {
                self.have_full_cycle = true;
            }
            self.total_energy = 0.0;
            self.cycle_time = 0.0;
        } else if self.last_cycle_time > 0.0 && self.cycle_time > self.last_cycle_time * 8.0 {
            self.avg_power = self.total_energy / self.cycle_time;
            if self.avg_power.is_nan() {
                self.avg_power = 0.0;
            }
            self.total_energy = 0.0;
            self.cycle_time = 0.0;
        }
        self.was_above_mid = above;
        // constant power never crosses its own mean: report the running mean until a period
        // has been measured
        if self.last_cycle_time == 0.0 {
            self.avg_power = mid;
        }
        if p == 0.0 {
            self.zero_time += dt;
            if self.last_cycle_time > 0.0 && self.zero_time > self.last_cycle_time * 1.5 {
                self.avg_power = 0.0;
                self.total_energy = 0.0;
                self.cycle_time = 0.0;
            }
        } else {
            self.zero_time = 0.0;
        }
    }

    fn update_new_average(&mut self) {
        let p = self.power();
        self.count += 1.0;
        self.total_power += p;
        let mut cycle_end = false;
        if p > self.max_p && self.increasing {
            self.max_p = p;
            self.increasing = true;
            self.decreasing = false;
        }
        if p < self.max_p && self.increasing {
            self.min_p = p;
            self.increasing = false;
            self.decreasing = true;
            cycle_end = true;
        }
        if p < self.min_p && self.decreasing {
            self.min_p = p;
            self.increasing = false;
            self.decreasing = true;
        }
        if p > self.min_p && self.decreasing {
            self.max_p = p;
            self.increasing = true;
            self.decreasing = false;
            cycle_end = true;
        }
        if cycle_end {
            self.avg_power = self.total_power / self.count;
            if self.avg_power.is_nan() {
                self.avg_power = 0.0;
            }
            self.count = 0.0;
            self.total_power = 0.0;
        }
        if p == 0.0 {
            self.zero_count += 1;
            if self.zero_count > 5 {
                self.total_power = 0.0;
                self.avg_power = 0.0;
                self.max_p = 0.0;
                self.min_p = 0.0;
            }
        } else {
            self.zero_count = 0;
        }
    }
}

impl Element for Wattmeter {
    impl_element_base!();

    fn class_name(&self) -> &'static str {
        if self.new_style { "WattmeterTrueElm" } else { "WattmeterElm" }
    }
    fn dump_type(&self) -> i32 {
        if self.new_style { 0 } else { 420 }
    }
    fn xml_dump_type(&self) -> String {
        if self.new_style { "WattmeterTrue".into() } else { "Wattmeter".into() }
    }
    fn dump(&self) -> String {
        format!("{} {} {}", self.base.dump_prefix(420), self.width, self.meter)
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("w", self.width);
        out.int_attr("meter", self.meter);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.width = xml.int_attr("w", self.width);
        self.meter = xml.int_attr("meter", self.meter);
    }

    fn post_count(&self) -> usize {
        4
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.posts.get(n).copied()
    }
    fn voltage_source_count(&self) -> usize {
        if self.new_style { 1 } else { 2 }
    }
    fn can_flip_x(&self) -> bool {
        false
    }
    fn can_flip_y(&self) -> bool {
        false
    }
    fn can_view_in_scope(&self) -> bool {
        true
    }

    fn drag(&mut self, xx: i32, yy: i32) {
        let (xx, mut yy) = (snap_grid(xx), snap_grid(yy));
        let (mut xx, b) = (xx, &mut self.base);
        let w1 = grid_size().max((yy - b.y).abs());
        let w2 = grid_size().max((xx - b.x).abs());
        if w1 > w2 {
            xx = b.x;
            self.width = w2;
        } else {
            yy = b.y;
            self.width = w1;
        }
        b.x2 = xx;
        b.y2 = yy;
        self.set_points();
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let b = &self.base;
        let (p1, p2, dn) = (b.point1, b.point2, b.dn);
        let ds = if b.dy == 0 { sign(b.dx) } else { -sign(b.dy) };
        let w = self.width;
        let p3 = interp_point_g(p1, p2, 0.0, f64::from(-w * ds));
        let p4 = interp_point_g(p1, p2, 1.0, f64::from(-w * ds));
        let sep = grid_size();
        let fsep = f64::from(sep);
        let p5 = interp_point(p1, p2, fsep / dn);
        let p6 = interp_point(p1, p2, 1.0 - fsep / dn);
        let p7 = interp_point(p3, p4, fsep / dn);
        let p8 = interp_point(p3, p4, 1.0 - fsep / dn);
        // the lower-numbered posts are on the bottom, so that unconnected ones are grounded
        // automatically; the new style swaps the potential coil posts
        if self.new_style {
            self.posts = [p4, p3, p1, p2];
            self.inner = [p8, p7, p5, p6];
        } else {
            self.posts = [p3, p4, p1, p2];
            self.inner = [p7, p8, p5, p6];
        }
        let r1 = interp_point_g(p1, p2, fsep / dn, f64::from(ds * sep));
        let r2 = interp_point_g(p1, p2, 1.0 - fsep / dn, f64::from(ds * sep));
        let r3 = interp_point_g(p1, p2, fsep / dn, f64::from(-ds * (sep + w)));
        let r4 = interp_point_g(p1, p2, 1.0 - fsep / dn, f64::from(-ds * (sep + w)));
        self.rect = [r1, r2, r4, r3];
        self.center = interp_point(r1, r4, 0.5);
        self.max_text_len = ((r1.x - r4.x).abs() - 5).max(5);
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.currents = [0.0; 2];
    }

    fn set_voltage_source(&mut self, j: usize, vs: VsId) {
        if let Some(s) = self.sources.get_mut(j) {
            *s = Some(vs);
        }
    }
    fn voltage_source_nodes_at(&self, j: usize) -> Option<(NodeId, NodeId)> {
        let n = &self.base.nodes;
        if self.new_style { (j == 0).then(|| (n[2], n[3])) } else { Some((n[j * 2], n[j * 2 + 1])) }
    }
    fn set_current(&mut self, vs: VsId, c: f64) {
        if self.new_style {
            self.currents[1] = c;
        } else {
            self.currents[usize::from(self.sources[0] != Some(vs))] = c;
        }
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        if self.new_style {
            // the current coil is a zero-volt source, the voltage coil a large resistor
            if let Some(vs) = self.sources[0] {
                s.stamp_voltage_source(n[2], n[3], vs, 0.0);
            }
            s.stamp_resistor(n[0], n[1], 1e8);
        } else {
            for j in 0..2 {
                if let Some(vs) = self.sources[j] {
                    s.stamp_voltage_source(n[j * 2], n[j * 2 + 1], vs, 0.0);
                }
            }
        }
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if self.new_style {
            self.update_new_average();
        } else {
            self.update_old_average(s.time_step);
        }
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        if n.is_multiple_of(2) { -self.currents[n / 2] } else { self.currents[n / 2] }
    }
    fn get_current(&self) -> f64 {
        self.currents[1]
    }
    fn get_voltage_diff(&self) -> f64 {
        self.voltage()
    }
    fn get_power(&self) -> f64 {
        self.power()
    }
    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        n1 / 2 == n2 / 2
    }
    fn has_ground_connection(&self, _n1: usize) -> bool {
        false
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.avg_power]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let Some(v) = state.first() {
            self.avg_power = *v;
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        for i in 0..2 {
            self.curcounts[i] =
                self.base.update_dot_count_for(self.currents[i], self.curcounts[i], c);
        }
        let mut flip = 1.0;
        for i in 0..4 {
            set_voltage_color(g, &self.base, c, self.base.volts[i]);
            draw_thick_line(g, self.posts[i], self.inner[i]);
            if !self.new_style || i >= 2 {
                draw_dots(g, c, self.posts[i], self.inner[i], self.curcounts[i / 2] * flip);
            }
            if self.new_style {
                // M L / C V, the standard terminal marks, just inside the box edge
                let label = ["V", "C", "M", "L"][i];
                g.set_color(if self.base.needs_highlight() { c.select_color } else { c.white });
                let w = g.measure_text(label) as i32;
                let lp = interp_point_g(self.posts[i], self.inner[i], 1.5, 0.0);
                g.draw_string(label, f64::from(lp.x - w / 2), f64::from(lp.y + 4));
            }
            flip = -flip;
        }
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        draw_thick_polygon(g, &Polygon::from_points(&self.rect));
        if self.new_style {
            let (r0, r2) = (self.rect[0], self.rect[2]);
            self.base.set_bbox(r0.x, r0.y, r2.x, r2.y);
        } else {
            let (a, b) = (self.posts[0], self.posts[3]);
            self.base.set_bbox(a.x, a.y, b.x, b.y);
        }
        draw_posts(g, &self.base, self, c);
        let text = match (self.new_style, self.meter) {
            (_, 1) => unit_text(self.avg_power, if self.new_style { "W" } else { "W(avg)" }),
            _ => unit_text(self.power(), "W"),
        };
        g.save();
        let mut fsize = 15;
        let mut w;
        loop {
            g.set_font_size(fsize);
            w = g.measure_text(&text) as i32;
            if w < self.max_text_len || fsize <= 4 {
                break;
            }
            fsize -= 1;
        }
        g.set_color(c.white);
        g.set_text_baseline(crate::render::painter::TextBaseline::Middle);
        g.draw_string(&text, f64::from(self.center.x - w / 2), f64::from(self.center.y));
        g.restore();
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(if self.new_style { "wattmeter" } else { "wattmeter (old)" }.into());
        basic_info(self, arr);
        if self.new_style {
            arr.push(format!("P = {}", unit_text(self.power(), "W")));
            arr.push(format!("P = {}", unit_text(self.avg_power, "W")));
        } else {
            arr.push(format!("P = {}", unit_text(self.power(), "W")));
            if self.meter == 1 {
                arr.push(format!("Pavg = {}", unit_text(self.avg_power, "W")));
            }
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        (n == 0).then(|| {
            let names = if self.new_style {
                vec!["Instantaneous Power".into(), "Average Power".into()]
            } else {
                vec!["Instantaneous".into(), "Average".into()]
            };
            EditInfo::choice("Value", names, self.meter.max(0) as usize)
        })
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if n == 0 {
            self.meter = ei.choice_index as i32;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Stop trigger
// ---------------------------------------------------------------------------------------------

/// A one-terminal probe that stops the simulation when its voltage crosses a level.
pub struct StopTrigger {
    base: Base,
    trigger_voltage: f64,
    /// 0 stops at or above the voltage, 1 at or below it.
    kind: i32,
    delay: f64,
    required_duration: f64,
    count: i32,
    triggered: bool,
    stopped: bool,
    condition_active: bool,
    duration_met: bool,
    trigger_time: f64,
    condition_start: f64,
    trigger_count: i32,
    /// Seconds until a triggered stop happens, for the info panel.
    remaining: f64,
}

impl StopTrigger {
    fn blank(base: Base) -> Self {
        StopTrigger {
            base,
            trigger_voltage: 1.0,
            kind: 0,
            delay: 0.0,
            required_duration: 0.0,
            count: 1,
            triggered: false,
            stopped: false,
            condition_active: false,
            duration_met: false,
            trigger_time: 0.0,
            condition_start: 0.0,
            trigger_count: 0,
            remaining: 0.0,
        }
    }

    /// A trigger being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y))
    }

    /// A trigger read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut t = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        t.trigger_voltage = st.next_f64().unwrap_or(1.0);
        t.kind = st.next_i32().unwrap_or(0);
        t.delay = st.next_f64().unwrap_or(0.0);
        t
    }
}

impl Element for StopTrigger {
    impl_element_base!("StopTriggerElm");

    fn dump_type(&self) -> i32 {
        408
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {}",
            self.base.dump_prefix(408),
            js_number(self.trigger_voltage),
            self.kind,
            js_number(self.delay)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("tv", self.trigger_voltage);
        out.int_attr("tp", self.kind);
        out.double_attr("dl", self.delay);
        out.int_attr("ct", self.count);
        out.double_attr("rd", self.required_duration);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.trigger_voltage = xml.double_attr("tv", self.trigger_voltage);
        self.kind = xml.int_attr("tp", self.kind);
        self.delay = xml.double_attr("dl", self.delay);
        self.count = xml.int_attr("ct", 1).max(1);
        self.required_duration = xml.double_attr("rd", 0.0);
    }

    fn post_count(&self) -> usize {
        1
    }
    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.triggered = false;
        self.condition_active = false;
        self.duration_met = false;
        self.trigger_count = 0;
    }
    fn set_points(&mut self) {
        self.base.set_points();
        let b = &mut self.base;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - 8.0 / b.dn);
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        self.stopped = false;
        let v = self.base.volts[0];
        let condition = (self.kind == 0 && v >= self.trigger_voltage)
            || (self.kind == 1 && v <= self.trigger_voltage);
        if !self.condition_active && condition {
            self.condition_active = true;
            self.condition_start = s.t;
            self.duration_met = false;
        }
        if self.condition_active
            && condition
            && !self.duration_met
            && s.t - self.condition_start >= self.required_duration
        {
            self.duration_met = true;
            self.trigger_count += 1;
            if !self.triggered && self.trigger_count >= self.count {
                self.triggered = true;
                self.trigger_time = s.t;
            }
        }
        if self.condition_active && !condition {
            self.condition_active = false;
        }
        self.remaining = self.trigger_time + self.delay - s.t;
        if self.triggered && s.t >= self.trigger_time + self.delay {
            self.triggered = false;
            self.trigger_count = 0;
            self.stopped = true;
            s.pause_requested = true;
        }
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![
            if self.stopped { 1.0 } else { 0.0 },
            if self.triggered { 1.0 } else { 0.0 },
            f64::from(self.trigger_count),
            self.remaining,
        ]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [stopped, triggered, count, remaining] = state[..] {
            self.stopped = stopped != 0.0;
            self.triggered = triggered != 0.0;
            self.trigger_count = count as i32;
            self.remaining = remaining;
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        let selected = self.base.needs_highlight() || self.stopped;
        g.set_font_size(14);
        g.set_color(if selected { c.select_color } else { c.white });
        let (p1, l1) = (self.base.point1, self.base.lead1);
        self.base.set_bbox_pts(p1, l1, 0.0);
        draw_labeled_node(g, &mut self.base, c, &crate::locale::ls("trigger"), p1, l1);
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        if selected {
            g.set_color(c.select_color);
        }
        draw_thick_line(g, p1, l1);
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("stop trigger".into());
        arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
        arr.push(format!("Vtrigger = {}", voltage_text(self.trigger_voltage)));
        let mut state = if self.triggered {
            format!("stopping in {}", unit_text(self.remaining, "s"))
        } else if self.stopped {
            "stopped".to_string()
        } else {
            "waiting".to_string()
        };
        if !self.stopped && self.count > 1 {
            state += &format!(" ({}/{})", self.trigger_count.min(self.count), self.count);
        }
        arr.push(state);
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::number("Voltage", self.trigger_voltage).set_unit_step()),
            1 => Some(EditInfo::choice(
                "Trigger Type",
                vec![">=".into(), "<=".into()],
                self.kind.max(0) as usize,
            )),
            2 => Some(EditInfo::number("Delay (s)", self.delay)),
            3 => Some(EditInfo::number("Required Duration (s)", self.required_duration)),
            4 => Some(
                EditInfo::number("Required Count", f64::from(self.count))
                    .set_dimensionless()
                    .set_positive(),
            ),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.trigger_voltage = ei.value,
            1 => self.kind = ei.choice_index as i32,
            2 => self.delay = ei.value,
            3 => self.required_duration = ei.value.max(0.0),
            4 => self.count = (ei.value as i32).max(1),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------
// Test point
// ---------------------------------------------------------------------------------------

const TP_VOL: i32 = 0;
const TP_RMS: i32 = 1;
const TP_MAX: i32 = 2;
const TP_MIN: i32 = 3;
const TP_P2P: i32 = 4;
const TP_BIN: i32 = 5;
const TP_FRQ: i32 = 6;
const TP_PER: i32 = 7;
const TP_PWI: i32 = 8;
const TP_DUT: i32 = 9;
const TP_AVG: i32 = 10;
const TP_FLAG_LABEL: i32 = 1;
/// The measurements the edit dialog offers, in its order.
const TP_CHOICES: [i32; 7] = [TP_VOL, TP_RMS, TP_AVG, TP_MAX, TP_MIN, TP_P2P, TP_BIN];

/// A one-terminal probe that shows its voltage or a measurement made on it, and gives that
/// measurement to the scopes.
pub struct TestPoint {
    base: Base,
    meter: i32,
    label: String,
    zero_count: i32,
    rms_v: f64,
    total: f64,
    count: f64,
    avg_v: f64,
    total_v: f64,
    max_v: f64,
    last_max_v: f64,
    min_v: f64,
    last_min_v: f64,
    frequency: f64,
    period: f64,
    binary_level: f64,
    pulse_width: f64,
    duty_cycle: f64,
    selected_value: f64,
    last_step_count: u64,
    increasing: bool,
    decreasing: bool,
    started: bool,
    period_start: f64,
    pulse_start: f64,
}

impl TestPoint {
    fn blank(base: Base) -> Self {
        TestPoint {
            base,
            meter: TP_VOL,
            label: "TP".into(),
            zero_count: 0,
            rms_v: 0.0,
            total: 0.0,
            count: 0.0,
            avg_v: 0.0,
            total_v: 0.0,
            max_v: 0.0,
            last_max_v: 0.0,
            min_v: 0.0,
            last_min_v: 0.0,
            frequency: 0.0,
            period: 0.0,
            binary_level: 0.0,
            pulse_width: 0.0,
            duty_cycle: 0.0,
            selected_value: 0.0,
            last_step_count: 0,
            increasing: true,
            decreasing: true,
            started: false,
            period_start: 0.0,
            pulse_start: 0.0,
        }
    }

    /// A test point being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y))
    }

    /// A test point read from a text line: the measurement, then the label if flagged.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut t = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        t.meter = st.next_i32().unwrap_or(TP_VOL);
        if f & TP_FLAG_LABEL != 0 {
            if let Some(l) = st.next_token() {
                t.label = crate::format::tokenizer::unescape(l);
            }
        }
        t
    }

    /// A finished cycle: the averages are taken and started again.
    fn close_cycle(&mut self) {
        self.total /= self.count;
        self.rms_v = self.total.sqrt();
        if self.rms_v.is_nan() {
            self.rms_v = 0.0;
        }
        self.avg_v = self.total_v / self.count;
        if self.avg_v.is_nan() {
            self.avg_v = 0.0;
        }
        self.count = 0.0;
        self.total = 0.0;
        self.total_v = 0.0;
    }

    fn value_of(&self, meter: i32) -> f64 {
        match meter {
            TP_VOL => self.base.volts[0],
            TP_RMS => self.rms_v,
            TP_AVG => self.avg_v,
            TP_MAX => self.last_max_v,
            TP_MIN => self.last_min_v,
            TP_P2P => self.last_max_v - self.last_min_v,
            TP_BIN => self.binary_level,
            TP_FRQ => self.frequency,
            TP_PER => self.period,
            TP_PWI => self.pulse_width,
            TP_DUT => self.duty_cycle,
            _ => 0.0,
        }
    }

    fn meter_line(&self, m: i32) -> String {
        let v = self.value_of(m);
        match m {
            TP_VOL => format!("V = {}", unit_text(v, "V")),
            TP_RMS => format!("V(rms) = {}", unit_text(v, "V")),
            TP_AVG => format!("V(avg) = {}", unit_text(v, "V")),
            TP_MAX => format!("Vmax = {}", unit_text(v, "Vpk")),
            TP_MIN => format!("Vmin = {}", unit_text(v, "Vmin")),
            TP_P2P => format!("Vp2p = {}", unit_text(v, "Vp2p")),
            TP_BIN => format!("Binary:{}", js_number(v)),
            TP_FRQ => format!("Freq = {}", unit_text(v, "Hz")),
            TP_PER => format!("Period = {}", unit_text(v, "s")),
            TP_PWI => format!("Pulse width = {}", unit_text(v, "s")),
            TP_DUT => format!("Duty cycle = {}", js_number(v)),
            _ => String::new(),
        }
    }

    fn shown_text(&self) -> String {
        let v = self.value_of(self.meter);
        match self.meter {
            TP_VOL => unit_text(v, "V"),
            TP_RMS => unit_text(v, "V(rms)"),
            TP_AVG => unit_text(v, "V(avg)"),
            TP_MAX => unit_text(v, "Vpk"),
            TP_MIN => unit_text(v, "Vmin"),
            TP_P2P => unit_text(v, "Vp2p"),
            TP_BIN => js_number(v),
            TP_FRQ => unit_text(v, "Hz"),
            TP_PWI => unit_text(v, "s"),
            TP_DUT => js_number(v),
            _ => self.label.clone(),
        }
    }
}

impl Element for TestPoint {
    impl_element_base!("TestPointElm");

    fn dump_type(&self) -> i32 {
        368
    }
    fn dump(&self) -> String {
        let mut s = format!("{} {}", self.base.dump_prefix(368), self.meter);
        if self.base.flags & TP_FLAG_LABEL != 0 {
            s += &format!(" {}", crate::format::tokenizer::escape(&self.label));
        }
        s
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("me", self.meter);
        if self.label != "TP" {
            out.str_attr("lb", &self.label);
        }
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.meter = xml.int_attr("me", self.meter);
        self.label = xml.string_attr("lb", "TP");
    }

    fn post_count(&self) -> usize {
        1
    }
    fn set_points(&mut self) {
        self.base.set_points();
        self.base.lead1 = Point::default();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.zero_count = 0;
        self.rms_v = 0.0;
        self.total = 0.0;
        self.count = 0.0;
        self.avg_v = 0.0;
        self.total_v = 0.0;
        self.max_v = 0.0;
        self.last_max_v = 0.0;
        self.min_v = 0.0;
        self.last_min_v = 0.0;
        self.binary_level = 0.0;
        self.period = 0.0;
        self.pulse_width = 0.0;
        self.duty_cycle = 0.0;
        self.selected_value = 0.0;
        self.period_start = 0.0;
        self.pulse_start = 0.0;
        self.increasing = true;
        self.decreasing = true;
        self.started = false;
        self.last_step_count = 0;
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if s.time_step_count == self.last_step_count {
            return;
        }
        self.last_step_count = s.time_step_count;
        let v = self.base.volts[0];
        self.count += 1.0;
        self.total += v * v;
        self.total_v += v;
        // fixed 2.5 V threshold, as in the original (it assumes about 5 V logic)
        self.binary_level = if v < 2.5 { 0.0 } else { 1.0 };
        if !self.started {
            // prime the tracking with the first sample instead of stale defaults
            self.started = true;
            self.max_v = v;
            self.min_v = v;
            self.increasing = true;
            self.decreasing = false;
            self.period_start = s.t;
            self.pulse_start = s.t;
        }
        // going up: track the maximum
        if v > self.max_v && self.increasing {
            self.max_v = v;
            self.increasing = true;
            self.decreasing = false;
        }
        if v < self.max_v && self.increasing {
            // direction change: the waveform starts going down
            self.last_max_v = self.max_v;
            let period_length = s.t - self.period_start;
            self.period_start = s.t;
            self.period = period_length;
            self.pulse_width = s.t - self.pulse_start;
            self.duty_cycle = self.pulse_width / period_length;
            self.min_v = v;
            self.increasing = false;
            self.decreasing = true;
            self.close_cycle();
        }
        if v < self.min_v && self.decreasing {
            self.min_v = v;
            self.increasing = false;
            self.decreasing = true;
        }
        if v > self.min_v && self.decreasing {
            self.last_min_v = self.min_v;
            self.pulse_start = s.t;
            self.max_v = v;
            self.increasing = true;
            self.decreasing = false;
            self.close_cycle();
        }
        // zero the readings if the voltage stays at 0 for a while
        if v == 0.0 {
            self.zero_count += 1;
            if self.zero_count > 5 {
                self.total = 0.0;
                self.rms_v = 0.0;
                self.avg_v = 0.0;
                self.max_v = 0.0;
                self.min_v = 0.0;
            }
        } else {
            self.zero_count = 0;
        }
        self.selected_value = self.value_of(self.meter);
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![
            self.rms_v,
            self.avg_v,
            self.last_max_v,
            self.last_min_v,
            self.binary_level,
            self.period,
            self.pulse_width,
            self.duty_cycle,
            self.selected_value,
        ]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [rms, avg, max, min, bin, per, pw, duty, sel] = state[..] {
            self.rms_v = rms;
            self.avg_v = avg;
            self.last_max_v = max;
            self.last_min_v = min;
            self.binary_level = bin;
            self.period = per;
            self.pulse_width = pw;
            self.duty_cycle = duty;
            self.selected_value = sel;
        }
    }

    fn scope_value(&self, _x: i32) -> f64 {
        self.selected_value
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        let selected = self.base.needs_highlight();
        g.set_font_size(14);
        g.set_color(if selected { c.select_color } else { c.white });
        let (p1, p2) = (self.base.point1, self.base.point2);
        let tp_width = g.measure_text("TP") as i32;
        self.base.lead1 = interp_point(p1, p2, 1.0 - (f64::from(tp_width / 2 + 8)) / self.base.dn);
        let l1 = self.base.lead1;
        self.base.set_bbox_pts(p1, l1, 0.0);
        let value = self.shown_text();
        // the label above (or beside) the reading
        let w1 = g.measure_text(&self.label) as i32;
        let w2 = g.measure_text(&value) as i32;
        let spacing = 14;
        let wmax = w1.max(w2);
        let h = g.font_size();
        g.set_text_baseline(TextBaseline::Middle);
        let (mut x, mut y) = (l1.x, l1.y);
        if p1.y != l1.y {
            x -= wmax / 2;
            y += (l1.y - p1.y).signum() * h;
            if l1.y < p1.y {
                y -= spacing - 4;
            }
        } else if l1.x > p1.x {
            x += 4;
        } else {
            x -= 4 + wmax;
        }
        g.draw_string(&self.label, f64::from(x + (wmax - w1) / 2), f64::from(y));
        g.draw_string(&value, f64::from(x + (wmax - w2) / 2), f64::from(y + spacing));
        self.base.adjust_bbox(x, y - h / 2, x + wmax, y + spacing + h / 2);
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        if selected {
            g.set_color(c.select_color);
        }
        draw_thick_line(g, p1, l1);
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("Test Point".into());
        arr.push(self.meter_line(self.meter));
        for m in [TP_VOL, TP_MAX, TP_MIN, TP_RMS, TP_AVG, TP_P2P, TP_PER, TP_PWI, TP_DUT] {
            if m != self.meter {
                arr.push(self.meter_line(m));
            }
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => {
                let names = [
                    "Voltage",
                    "RMS Voltage",
                    "Average Voltage",
                    "Max Voltage",
                    "Min Voltage",
                    "P2P Voltage",
                    "Binary Value",
                ];
                let selected = TP_CHOICES.iter().position(|&m| m == self.meter).unwrap_or(0);
                Some(EditInfo::choice(
                    "Value",
                    names.iter().map(|s| (*s).to_string()).collect(),
                    selected,
                ))
            }
            1 => Some(EditInfo::text("Label", &self.label)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.meter = TP_CHOICES.get(ei.choice_index).copied().unwrap_or(TP_VOL),
            1 => {
                if let Some(t) = &ei.text {
                    self.label = t.clone();
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------
// Data recorder
// ---------------------------------------------------------------------------------------

/// A one-terminal probe that keeps the last voltages it saw, one per time step, so they can be
/// written to a file.
pub struct DataRecorder {
    base: Base,
    data_count: usize,
    data: Vec<f64>,
    data_ptr: usize,
    full: bool,
    last_step_count: u64,
    time_step: f64,
}

impl DataRecorder {
    fn blank(base: Base, count: usize) -> Self {
        let mut r = DataRecorder {
            base,
            data_count: 0,
            data: Vec::new(),
            data_ptr: 0,
            full: false,
            last_step_count: 0,
            time_step: 5e-6,
        };
        r.set_data_count(count);
        r
    }

    /// A recorder being placed.
    pub fn new(x: i32, y: i32) -> Self {
        Self::blank(Base::new(x, y), 10240)
    }

    /// A recorder read from a text line.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let count = st.next_i32().unwrap_or(10240).max(1) as usize;
        Self::blank(Base::from_dump(xa, ya, xb, yb, f), count)
    }

    fn set_data_count(&mut self, count: usize) {
        self.data_count = count.max(1);
        self.data = vec![0.0; self.data_count];
        self.data_ptr = 0;
        self.full = false;
    }

    /// The recorded values, oldest first.
    pub fn recorded(&self) -> Vec<f64> {
        if self.full {
            (0..self.data_count).map(|i| self.data[(i + self.data_ptr) % self.data_count]).collect()
        } else {
            self.data[..self.data_ptr].to_vec()
        }
    }

    /// The recording as the original's export file: a comment with the time step, then one
    /// value per line.
    pub fn export_text(&self) -> String {
        let mut s = format!("# time step = {} sec\n", js_number(self.time_step));
        for v in self.recorded() {
            s += &js_number(v);
            s.push('\n');
        }
        s
    }
}

impl Element for DataRecorder {
    impl_element_base!("DataRecorderElm");

    fn dump_type(&self) -> i32 {
        210
    }
    fn dump(&self) -> String {
        format!("{} {}", self.base.dump_prefix(210), self.data_count)
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.int_attr("dc", self.data_count as i32);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        let count = xml.int_attr("dc", self.data_count as i32).max(1) as usize;
        self.set_data_count(count);
    }

    fn post_count(&self) -> usize {
        1
    }
    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.data_ptr = 0;
        self.full = false;
        self.last_step_count = 0;
    }
    fn set_points(&mut self) {
        self.base.set_points();
        let b = &mut self.base;
        b.lead1 = interp_point(b.point1, b.point2, 1.0 - 8.0 / b.dn);
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        if self.last_step_count == s.time_step_count {
            return;
        }
        self.time_step = s.time_step;
        self.data[self.data_ptr] = self.base.volts[0];
        self.data_ptr += 1;
        self.last_step_count = s.time_step_count;
        if self.data_ptr >= self.data_count {
            self.data_ptr = 0;
            self.full = true;
        }
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.data_ptr as f64, if self.full { 1.0 } else { 0.0 }]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [ptr, full] = state[..] {
            self.data_ptr = (ptr as usize).min(self.data_count - 1);
            self.full = full != 0.0;
        }
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.save();
        let selected = self.base.needs_highlight();
        g.set_font_size(14);
        g.set_color(if selected { c.select_color } else { c.white });
        let (p1, l1) = (self.base.point1, self.base.lead1);
        self.base.set_bbox_pts(p1, l1, 0.0);
        draw_labeled_node(g, &mut self.base, c, &crate::locale::ls("export"), p1, l1);
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        if selected {
            g.set_color(c.select_color);
        }
        draw_thick_line(g, p1, l1);
        draw_posts(g, &self.base, self, c);
        g.restore();
    }

    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[0]
    }
    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("data export".into());
        arr.push(format!("V = {}", voltage_text(self.base.volts[0])));
        let held = if self.full { self.data_count } else { self.data_ptr };
        arr.push(format!("{held}/{}", self.data_count));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::number("# of Data Points", self.data_count as f64)
                    .set_dimensionless()
                    .set_positive(),
            ),
            1 => Some(EditInfo::text("Export File (path)", "").save_file("data.txt")),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 if ei.value > 0.0 => self.set_data_count(ei.value as usize),
            1 => {
                let path = ei.text.as_deref().map(str::trim).unwrap_or_default();
                if !path.is_empty() {
                    if let Err(e) = std::fs::write(path, self.export_text()) {
                        ei.error = Some(format!("{path}: {e}"));
                    }
                }
            }
            _ => {}
        }
    }
}
