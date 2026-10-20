// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (PotElm).
//! Potentiometer: a resistor with a movable wiper driven by a slider in the side panel.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::{TokenError, Tokenizer};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::locale;
use crate::render::painter::Painter;
use crate::solver::stamper::Stamper;

const FAMILY: &str = "passive";
const FLAG_SHOW_VALUES: i32 = 1;
const FLAG_FLIP: i32 = 2;
const FLAG_FLIP_OFFSET: i32 = 4;

/// Registers the potentiometer.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "PotElm",
            FAMILY,
            "Add Potentiometer",
            "Passive Components",
            |x, y| Box::new(PotElm::new(x, y)),
        )
        .dump(174, |a, b, c, d, f, st| Ok(Box::new(PotElm::from_dump(a, b, c, d, f, st)?))),
    );
}

/// A potentiometer with a slider.
#[derive(Clone, Debug)]
pub struct PotElm {
    base: Base,
    /// Resistance end to end.
    pub max_resistance: f64,
    /// Wiper position, 0 to 1.
    pub position: f64,
    /// Text above the slider.
    pub slider_text: String,
    /// Group number: pots with the same number share a slider.
    pub link: i32,
    resistance1: f64,
    resistance2: f64,
    current1: f64,
    current2: f64,
    current3: f64,
    curcount1: f64,
    curcount2: f64,
    curcount3: f64,
    post3: Point,
    corner2: Point,
    arrow_point: Point,
    midpoint: Point,
    arrow1: Point,
    arrow2: Point,
}

impl PotElm {
    fn blank(base: Base) -> Self {
        PotElm {
            base,
            max_resistance: 1000.0,
            position: 0.5,
            slider_text: "Resistance".into(),
            link: 0,
            resistance1: 0.0,
            resistance2: 0.0,
            current1: 0.0,
            current2: 0.0,
            current3: 0.0,
            curcount1: 0.0,
            curcount2: 0.0,
            curcount3: 0.0,
            post3: Point::default(),
            corner2: Point::default(),
            arrow_point: Point::default(),
            midpoint: Point::default(),
            arrow1: Point::default(),
            arrow2: Point::default(),
        }
    }

    /// A potentiometer being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut base = Base::new(xx, yy);
        base.flags = FLAG_SHOW_VALUES;
        PotElm::blank(base)
    }

    /// A potentiometer read from a file.
    pub fn from_dump(
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Result<Self, TokenError> {
        let mut p = PotElm::blank(Base::from_dump(xa, ya, xb, yb, f));
        p.max_resistance = st.next_f64()?;
        p.position = st.next_f64()?;
        let mut text = st.next_token().ok_or(TokenError::Missing)?.to_string();
        while let Some(t) = st.next_token() {
            text.push(' ');
            text.push_str(t);
        }
        p.slider_text = text.replace("%2b", "+").replace("%2B", "+");
        Ok(p)
    }

    fn slider_position(&self) -> i32 {
        (((self.position - 0.005) / 0.0099).round() as i32).clamp(0, 100)
    }
}

impl Element for PotElm {
    impl_element_base!("PotElm");

    fn dump_type(&self) -> i32 {
        174
    }
    fn xml_dump_type(&self) -> String {
        "pt".into()
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {} {}",
            self.base.dump_prefix(174),
            js_number(self.max_resistance),
            js_number(self.position),
            self.slider_text.replace('+', "%2b")
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.max_resistance = xml.double_attr("ma", self.max_resistance);
        self.position = xml.double_attr("po", self.position);
        self.slider_text = xml.string_attr("sl", &self.slider_text);
        self.link = xml.int_attr("li", 0);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("ma", self.max_resistance);
        out.double_attr("po", self.position);
        out.str_attr("sl", &self.slider_text);
        if self.link != 0 {
            out.int_attr("li", self.link);
        }
    }

    fn post_count(&self) -> usize {
        3
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        match n {
            0 => Some(self.base.point1),
            1 => Some(self.base.point2),
            2 => Some(self.post3),
            _ => None,
        }
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let grid = grid_size();
        let b = &mut self.base;
        let (dx, dy) = (b.dx, b.dy);
        let mut offset = 0;
        if (dx.abs() > dy.abs()) != (b.flags & FLAG_FLIP != 0) {
            let len = 2 * grid * dx.signum() * ((dx.abs() + 2 * grid - 1) / (2 * grid));
            b.point2.x = b.point1.x + len;
            offset = if dx < 0 { dy } else { -dy };
            b.point2.y = b.point1.y;
        } else {
            let len = 2 * grid * dy.signum() * ((dy.abs() + 2 * grid - 1) / (2 * grid));
            if dy != 0 {
                b.point2.y = b.point1.y + len;
                offset = if dy > 0 { dx } else { -dx };
                b.point2.x = b.point1.x;
            }
        }
        if offset == 0 {
            offset = if b.flags & FLAG_FLIP_OFFSET != 0 { -grid } else { grid };
        }
        b.dn = distance(b.point1, b.point2);
        let body_len = 32;
        b.calc_leads(body_len);
        self.position = f64::from(self.slider_position()) * 0.0099 + 0.005;
        let soff = ((self.position - 0.5) * f64::from(body_len)) as i32;
        let (p1, p2) = (self.base.point1, self.base.point2);
        let dn = self.base.dn;
        let frac = f64::from(soff) / dn + 0.5;
        self.post3 = interp_point_g(p1, p2, 0.5, f64::from(offset));
        self.corner2 = interp_point_g(p1, p2, frac, f64::from(offset));
        self.arrow_point = interp_point_g(p1, p2, frac, f64::from(8 * offset.signum()));
        self.midpoint = interp_point(p1, p2, frac);
        let clen = f64::from(offset.abs() - 8);
        let (a1, a2) = interp_point2(self.corner2, self.arrow_point, (clen - 8.0) / clen, 8.0);
        self.arrow1 = a1;
        self.arrow2 = a2;
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let segments = 16;
        let hs = if c.euro_resistors { 6 } else { 8 };
        let (v1, v2, v3) = (self.base.volts[0], self.base.volts[1], self.base.volts[2]);
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
        self.base.adjust_bbox(self.post3.x, self.post3.y, self.post3.x, self.post3.y);
        draw_2_leads(g, &self.base, c);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let segf = 1.0 / f64::from(segments);
        let divide = (f64::from(segments) * self.position) as i32;
        let volt_at = |i: i32| {
            if i >= divide {
                v3 + (v2 - v3) * f64::from(i - divide) / f64::from(segments - divide)
            } else {
                v1 + (v3 - v1) * f64::from(i) / f64::from(divide)
            }
        };
        if c.euro_resistors {
            set_voltage_color(g, &self.base, c, v1);
            let (a, b) = interp_point2(l1, l2, 0.0, f64::from(hs));
            draw_thick_line(g, a, b);
            for i in 0..segments {
                set_voltage_color(g, &self.base, c, volt_at(i));
                let (a, b) = interp_point2(l1, l2, f64::from(i) * segf, f64::from(hs));
                let (a2, b2) = interp_point2(l1, l2, f64::from(i + 1) * segf, f64::from(hs));
                draw_thick_line(g, a, a2);
                draw_thick_line(g, b, b2);
            }
            let (a, b) = interp_point2(l1, l2, 1.0, f64::from(hs));
            draw_thick_line(g, a, b);
        } else {
            let mut ox = 0;
            for i in 0..segments {
                let nx = match i & 3 {
                    0 => 1,
                    2 => -1,
                    _ => 0,
                };
                set_voltage_color(g, &self.base, c, volt_at(i));
                let a = interp_point_g(l1, l2, f64::from(i) * segf, f64::from(hs * ox));
                let b = interp_point_g(l1, l2, f64::from(i + 1) * segf, f64::from(hs * nx));
                draw_thick_line(g, a, b);
                ox = nx;
            }
        }
        set_voltage_color(g, &self.base, c, v3);
        draw_thick_line(g, self.post3, self.corner2);
        draw_thick_line(g, self.corner2, self.arrow_point);
        draw_thick_line(g, self.arrow1, self.arrow_point);
        draw_thick_line(g, self.arrow2, self.arrow_point);
        self.curcount1 = self.base.update_dot_count_for(self.current1, self.curcount1, c);
        self.curcount2 = self.base.update_dot_count_for(self.current2, self.curcount2, c);
        self.curcount3 = self.base.update_dot_count_for(self.current3, self.curcount3, c);
        if !self.base.creating {
            draw_dots(g, c, p1, self.midpoint, self.curcount1);
            draw_dots(g, c, p2, self.midpoint, self.curcount2);
            draw_dots(g, c, self.post3, self.corner2, self.curcount3);
            let extra = Base::add_cur_count(self.curcount3, distance(self.post3, self.corner2));
            draw_dots(g, c, self.corner2, self.midpoint, extra);
        }
        draw_posts(g, &self.base, self, c);
        if c.show_values && self.resistance1 > 0.0 && self.base.flags & FLAG_SHOW_VALUES != 0 {
            let ohm = if c.show_ohm { locale::OHM } else { "" };
            let vertical = l1.x == l2.x;
            let reverse_y = self.post3.x < l1.x && vertical;
            let reverse_x = self.post3.y < l1.y && !vertical;
            let rev = (vertical && l1.y < l2.y) || (l1.y == l2.y && l1.x > l2.x);
            let s1 = short_unit_text(if rev { self.resistance2 } else { self.resistance1 }, ohm);
            let s2 = short_unit_text(if rev { self.resistance1 } else { self.resistance2 }, ohm);
            g.save();
            g.set_font_size(c.value_font_size);
            g.set_color(c.white);
            let ya = g.font_size() / 2;
            let ap = self.arrow_point;
            let (a1, a2) = (self.arrow1, self.arrow2);
            let w1 = g.measure_text(&s1) as i32;
            if vertical {
                let x = if reverse_y { ap.x - 2 - w1 } else { ap.x + 2 };
                g.draw_string(&s1, f64::from(x), f64::from(a1.y.max(a2.y) + 5 + ya));
            } else {
                let y = if reverse_x { ap.y - 4 } else { ap.y + 4 + ya };
                g.draw_string(&s1, f64::from(a1.x.min(a2.x) - 2 - w1), f64::from(y));
            }
            let w2 = g.measure_text(&s2) as i32;
            if vertical {
                let x = if reverse_y { ap.x - 2 - w2 } else { ap.x + 2 };
                g.draw_string(&s2, f64::from(x), f64::from(a1.y.min(a2.y) - 3));
            } else {
                let y = if reverse_x { ap.y - 4 } else { ap.y + 4 + ya };
                g.draw_string(&s2, f64::from(a1.x.max(a2.x) + 2), f64::from(y));
            }
            g.restore();
        }
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.current1, self.current2, self.current3, self.resistance1, self.resistance2]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let [i1, i2, i3, r1, r2] = *state {
            (self.current1, self.current2, self.current3) = (i1, i2, i3);
            (self.resistance1, self.resistance2) = (r1, r2);
        }
    }

    fn reset(&mut self) {
        self.curcount1 = 0.0;
        self.curcount2 = 0.0;
        self.curcount3 = 0.0;
        self.base.volts.fill(0.0);
        self.base.curcount = 0.0;
    }

    fn calculate_current(&mut self) {
        if self.resistance1 == 0.0 {
            return;
        }
        let v = &self.base.volts;
        self.current1 = (v[0] - v[2]) / self.resistance1;
        self.current2 = (v[1] - v[2]) / self.resistance2;
        self.current3 = -self.current1 - self.current2;
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        match n {
            0 => -self.current1,
            1 => -self.current2,
            _ => -self.current3,
        }
    }

    fn stamp(&mut self, s: &mut Stamper) {
        self.resistance1 = self.max_resistance * self.position;
        self.resistance2 = self.max_resistance * (1.0 - self.position);
        let n = &self.base.nodes;
        s.stamp_resistor(n[0], n[2], self.resistance1);
        s.stamp_resistor(n[2], n[1], self.resistance2);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("potentiometer".into());
        arr.push(format!("Vd = {}", voltage_d_text(self.get_voltage_diff())));
        arr.push(format!("R1 = {}", unit_text(self.resistance1, locale::OHM)));
        arr.push(format!("R2 = {}", unit_text(self.resistance2, locale::OHM)));
        arr.push(format!("I1 = {}", current_d_text(self.current1)));
        arr.push(format!("I2 = {}", current_d_text(self.current2)));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Resistance (ohms)", self.max_resistance, 0.0, 0.0)),
            1 => Some(EditInfo {
                text: Some(self.slider_text.clone()),
                ..EditInfo::text("Slider Text", "")
            }),
            2 => Some(EditInfo::checkbox("Show Values", self.base.flags & FLAG_SHOW_VALUES != 0)),
            3 => {
                Some(EditInfo::new("Group Number (for linking)", f64::from(self.link), -1.0, -1.0))
            }
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.max_resistance = ei.value,
            1 => {
                if let Some(t) = &ei.text {
                    self.slider_text.clone_from(t);
                }
            }
            2 => self.base.flags = ei.change_flag(self.base.flags, FLAG_SHOW_VALUES),
            3 => self.link = ei.value as i32,
            _ => {}
        }
    }

    fn slider(&self) -> Option<ElementSlider> {
        Some(ElementSlider { text: self.slider_text.clone(), value: self.slider_position() })
    }
    fn set_slider_value(&mut self, v: i32) {
        self.position = f64::from(v.clamp(0, 100)) * 0.0099 + 0.005;
        Element::set_points(self);
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIP_OFFSET;
        self.base.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIP_OFFSET;
        self.base.flip_y(center2);
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        if self.base.dx.abs() == self.base.dy.abs() {
            self.base.flags ^= FLAG_FLIP;
        }
        self.base.flags ^= FLAG_FLIP_OFFSET;
        self.base.flip_xy(xmy);
        self.set_points();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_round_trips() {
        let mut st = Tokenizer::new("1000 0.5 X Resistance");
        let p = PotElm::from_dump(0, 0, 64, 0, 0, &mut st).unwrap();
        assert_eq!(p.slider_text, "X Resistance");
        assert_eq!(p.dump(), "0 0 0 64 0 0 1000 0.5 X Resistance".replacen("0 0 0", "174 0 0", 1));
    }

    #[test]
    fn visual_state_carries_currents_and_resistances_to_the_drawing_copy() {
        let mut p = PotElm::new(0, 0);
        (p.current1, p.current2, p.current3, p.resistance1, p.resistance2) =
            (0.01, 0.02, -0.03, 500.0, 600.0);
        let mut copy = PotElm::new(0, 0);
        copy.set_visual_state(&p.visual_state());
        assert_eq!(copy.visual_state(), p.visual_state());
        assert_eq!(copy.resistance2, 600.0);
    }

    #[test]
    fn slider_maps_to_position() {
        let mut p = PotElm::new(0, 0);
        p.set_slider_value(100);
        assert!((p.position - 0.995).abs() < 1e-12);
        assert_eq!(p.slider().unwrap().value, 100);
    }
}
