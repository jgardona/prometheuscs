// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (OpAmpElm, OpAmpSwapElm).
//! Analog building blocks: the ideal op-amp and its relatives.

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::Painter;
use crate::solver::path::Validator;
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name, also the Cargo feature that enables these elements.
pub const FAMILY: &str = "analog";
const BLOCKS: &str = "Active Building Blocks";

/// Registers the elements of this family.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "OpAmpElm",
            FAMILY,
            "Add Op Amp (ideal, - on top)",
            BLOCKS,
            |x, y| Box::new(OpAmpElm::new(x, y)),
        )
        .dump(i32::from(b'a'), |a, b, c, d, f, st| {
            Ok(Box::new(OpAmpElm::from_dump(a, b, c, d, f, st)))
        })
        .shortcut('a'),
    );
    r.add(
        ElementDescriptor::new(
            "OpAmpSwapElm",
            FAMILY,
            "Add Op Amp (ideal, + on top)",
            BLOCKS,
            |x, y| {
                let mut e = OpAmpElm::new(x, y);
                e.base.flags |= FLAG_SWAP;
                Box::new(e)
            },
        )
        .shortcut('A'),
    );
}

const FLAG_SWAP: i32 = 1;
const FLAG_SMALL: i32 = 2;
const FLAG_LOWGAIN: i32 = 4;
const FLAG_GAIN: i32 = 8;

/// An ideal op-amp with limited output range and finite gain.
#[derive(Clone, Debug)]
pub struct OpAmpElm {
    base: Base,
    opsize: i32,
    opheight: i32,
    opwidth: i32,
    /// Highest output voltage.
    pub max_out: f64,
    /// Lowest output voltage.
    pub min_out: f64,
    /// Open-loop gain.
    pub gain: f64,
    gbw: f64,
    lastvd: f64,
    in1p: [Point; 2],
    in2p: [Point; 2],
    textp: [Point; 2],
    triangle: Polygon,
}

impl OpAmpElm {
    fn blank(base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        OpAmpElm {
            base,
            opsize: 2,
            opheight: 16,
            opwidth: 26,
            max_out: 15.0,
            min_out: -15.0,
            gain: 100_000.0,
            gbw: 1e6,
            lastvd: 0.0,
            in1p: [Point::default(); 2],
            in2p: [Point::default(); 2],
            textp: [Point::default(); 2],
            triangle: Polygon::new(),
        }
    }

    /// An op-amp being placed at (`xx`, `yy`).
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut e = Self::blank(Base::new(xx, yy));
        e.base.flags = FLAG_GAIN;
        e.set_size(2);
        e
    }

    /// An op-amp read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut e = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        e.alloc_nodes();
        let fields = (|| {
            Ok::<_, crate::format::tokenizer::TokenError>((
                st.next_f64()?,
                st.next_f64()?,
                st.next_f64()?,
                st.next_f64()?,
                st.next_f64()?,
                st.next_f64()?,
            ))
        })();
        if let Ok((max_out, min_out, gbw, v0, v1, gain)) = fields {
            e.max_out = max_out;
            e.min_out = min_out;
            e.gbw = gbw;
            e.base.volts[0] = v0;
            e.base.volts[1] = v1;
            e.gain = gain;
        }
        e.set_size(if f & FLAG_SMALL != 0 { 1 } else { 2 });
        e.set_gain();
        e
    }

    fn set_gain(&mut self) {
        if self.base.flags & FLAG_GAIN != 0 {
            return;
        }
        self.gain = if self.base.flags & FLAG_LOWGAIN != 0 { 1000.0 } else { 100_000.0 };
    }

    fn set_size(&mut self, s: i32) {
        self.opsize = s;
        self.opheight = 8 * s;
        self.opwidth = 13 * s;
        self.base.flags = (self.base.flags & !FLAG_SMALL) | if s == 1 { FLAG_SMALL } else { 0 };
    }
}

impl Element for OpAmpElm {
    impl_element_base!("OpAmpElm");

    fn dump_type(&self) -> i32 {
        i32::from(b'a')
    }
    fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "{} {} {} {} {} {} {}",
            self.base.dump_prefix(self.dump_type()),
            js_number(self.max_out),
            js_number(self.min_out),
            js_number(self.gbw),
            js_number(self.base.volts.first().copied().unwrap_or(0.0)),
            js_number(self.base.volts.get(1).copied().unwrap_or(0.0)),
            js_number(self.gain)
        )
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        // the size might have changed, so the flags start from zero
        self.base.flags = xml.int_attr("f", 0);
        self.max_out = xml.double_attr("ma", self.max_out);
        self.min_out = xml.double_attr("mi", self.min_out);
        self.gain = xml.double_attr("ga", self.gain);
        self.set_size(if self.base.flags & FLAG_SMALL != 0 { 1 } else { 2 });
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("ma", self.max_out);
        out.double_attr("mi", self.min_out);
        out.double_attr("ga", self.gain);
    }

    fn post_count(&self) -> usize {
        3
    }
    fn voltage_source_count(&self) -> usize {
        1
    }
    fn non_linear(&self) -> bool {
        true
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.in1p[0],
            1 => self.in2p[0],
            _ => self.base.point2,
        })
    }

    fn set_points(&mut self) {
        self.base.no_diagonal = true;
        self.base.set_points();
        if self.base.dn > 150.0 && self.base.creating {
            self.set_size(2);
        }
        let mut ww = self.opwidth;
        if f64::from(ww) > self.base.dn / 2.0 {
            ww = (self.base.dn / 2.0) as i32;
        }
        self.base.calc_leads(ww * 2);
        let mut hs = self.opheight * self.base.dsign;
        if self.base.flags & FLAG_SWAP != 0 {
            hs = -hs;
        }
        let hsf = f64::from(hs);
        let b = &self.base;
        let (a0, a1) = interp_point2(b.point1, b.point2, 0.0, hsf);
        let (c0, c1) = interp_point2(b.lead1, b.lead2, 0.0, hsf);
        let (t0, t1) = interp_point2(b.lead1, b.lead2, 0.2, hsf);
        let (g0, g1) = interp_point2(b.lead1, b.lead2, 0.0, hsf * 2.0);
        self.in1p = [a0, c0];
        self.in2p = [a1, c1];
        self.textp = [t0, t1];
        self.triangle = Polygon::from_points(&[g0, g1, b.lead2]);
        self.alloc_nodes();
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(self.opheight * 2));
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        draw_thick_line(g, self.in1p[0], self.in1p[1]);
        set_voltage_color(g, &self.base, c, self.base.volts[1]);
        draw_thick_line(g, self.in2p[0], self.in2p[1]);
        set_voltage_color(g, &self.base, c, self.base.volts[2]);
        draw_thick_line(g, self.base.lead2, p2);
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        set_power_color_w(g, &self.base, c, self.get_power());
        draw_thick_polygon(g, &self.triangle);
        g.set_font_size(if self.opsize == 2 { 14 } else { 10 });
        let (t0, t1) = (self.textp[0], self.textp[1]);
        draw_centered_text(g, &mut self.base, "-", t0.x, t0.y - 2, true);
        draw_centered_text(g, &mut self.base, "+", t1.x, t1.y, true);
        self.base.update_dot_count(c);
        draw_dots(g, c, p2, self.base.lead2, self.base.curcount);
        draw_posts(g, &self.base, self, c);
    }

    fn get_power(&self) -> f64 {
        self.base.volts[2] * self.base.current
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if let Some(vs) = self.base.volt_source {
            s.stamp_non_linear(vs);
            s.stamp_matrix(self.base.nodes[2], vs, 1.0);
        }
    }
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base.volt_source = Some(v);
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let Some(vs) = self.base.volt_source else { return };
        let v = &self.base.volts;
        let vd = v[1] - v[0];
        let midpoint = (self.max_out + self.min_out) * 0.5;
        if (self.lastvd - vd).abs() > 0.1 || v[2] > self.max_out + 0.1 || v[2] < self.min_out - 0.1
        {
            s.converged = false;
        }
        let max_adj = self.max_out - midpoint;
        let min_adj = self.min_out - midpoint;
        let (x, dx);
        if vd >= max_adj / self.gain && (self.lastvd >= 0.0 || (s.random() * 4.0) as i32 == 1) {
            dx = 1e-4;
            x = self.max_out - dx * max_adj / self.gain;
        } else if vd <= min_adj / self.gain
            && (self.lastvd <= 0.0 || (s.random() * 4.0) as i32 == 1)
        {
            dx = 1e-4;
            x = self.min_out - dx * min_adj / self.gain;
        } else {
            dx = self.gain;
            x = midpoint;
        }
        let n = &self.base.nodes;
        s.stamp_matrix(vs, n[0], dx);
        s.stamp_matrix(vs, n[1], -dx);
        s.stamp_matrix(vs, n[2], 1.0);
        s.stamp_right_side(vs, x);
        self.lastvd = vd;
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        validate_rail_node(v, idx, self.base.nodes[2])
    }
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    fn has_ground_connection(&self, n1: usize) -> bool {
        n1 == 2
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[2] - self.base.volts[1]
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 2 { -self.base.current } else { 0.0 }
    }
    fn voltage_source_nodes(&self) -> Option<(NodeId, NodeId)> {
        None
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let v = &self.base.volts;
        arr.push("op-amp".into());
        arr.push(format!("V+ = {}", voltage_text(v[1])));
        arr.push(format!("V- = {}", voltage_text(v[0])));
        let vo = v[2].min(self.max_out).max(self.min_out);
        arr.push(format!("Vout = {}", voltage_text(vo)));
        arr.push(format!("Iout = {}", current_text(-self.base.current)));
        arr.push(format!(
            "range = {} to {}",
            voltage_text(self.min_out),
            voltage_text(self.max_out)
        ));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("Max Output (V)", self.max_out, 1.0, 20.0)),
            1 => Some(EditInfo::new("Min Output (V)", self.min_out, -20.0, 0.0)),
            2 => Some(EditInfo::new("Gain", self.gain, 10.0, 1_000_000.0).set_positive()),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => self.max_out = ei.value,
            1 => self.min_out = ei.value,
            2 => self.gain = ei.value,
            _ => {}
        }
    }

    fn on_flipped(&mut self, axis: Flip) {
        let swap = match axis {
            Flip::X => self.base.dx == 0,
            Flip::Y => self.base.dy == 0,
            Flip::Xy => true,
        };
        if swap {
            self.base.flags ^= FLAG_SWAP;
        }
    }

    fn shortcut(&self) -> i32 {
        i32::from(b'a')
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

    /// Non-inverting amplifier: Vin 1 V on the + input, gain 1 + R2/R1 = 3.
    #[test]
    fn non_inverting_amplifier_has_gain_one_plus_r2_over_r1() {
        // op-amp (128, 0)-(192, 0): inputs at (128, -16) '-' and (128, 16) '+'; output (192, 0)
        let text = format!(
            "{HDR}\nv 64 80 64 16 0 0 40 1 0 0 0.5\nw 64 16 128 16 0\nw 64 80 192 80 0\na 128 0 192 0 0 15 -15 1000000 0 0 100000\nw 192 0 192 -32 0\nr 192 -32 128 -32 0 2000\nw 128 -32 128 -16 0\nr 128 -16 128 48 0 1000\nw 128 48 192 48 0\nw 192 48 192 80 0\n"
        );
        let sim = run(&text, 300);
        let amp = sim.elements.iter().find(|e| e.class_name() == "OpAmpElm").unwrap();
        let out = amp.base().volts[2];
        assert!((out - 3.0).abs() < 0.01, "{out}");
    }

    #[test]
    fn output_saturates_at_the_supply_limits() {
        let text = format!(
            "{HDR}\nv 64 80 64 16 0 0 40 1 0 0 0.5\nw 64 16 128 16 0\nw 64 80 128 -16 0\na 128 0 192 0 0 5 -5 1000000 0 0 100000\n"
        );
        let sim = run(&text, 100);
        let out = sim.elements[3].base().volts[2];
        assert!((out - 5.0).abs() < 0.1, "{out}");
    }

    #[test]
    fn swap_flag_exchanges_the_inputs() {
        let mut a = OpAmpElm::new(0, 0);
        a.base.set_position(0, 0, 64, 0);
        a.set_points();
        let top = a.get_post(0).unwrap();
        a.on_flipped(Flip::Y);
        a.set_points();
        assert_eq!(a.get_post(0).unwrap().y, -top.y);
    }

    #[test]
    fn dump_round_trips_the_text_fields() {
        let mut st = Tokenizer::new("15 -15 1000000 0 0 100000");
        let a = OpAmpElm::from_dump(0, 0, 64, 0, 0, &mut st);
        assert_eq!(a.dump(), "a 0 0 64 0 0 15 -15 1000000 0 0 100000");
    }
}
