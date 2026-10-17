// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CustomTransformerElm).
//! A transformer with any number of coupled windings, described by a text such as `1,1:2+3`.
//!
//! The description lists the turns of each winding: `,` separates windings of the same side,
//! `:` separates the primary side from the secondary side, and `+` joins two windings in series
//! (they share a post). A negative number reverses the winding.

use super::element::*;
use super::geometry::*;
use super::reactive::FLAG_BACK_EULER;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::number::js_number;
use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::Painter;
use crate::solver::dense_lu::{lu_factor, lu_solve};
use crate::solver::stamper::Stamper;

/// Family name used for the feature switch.
pub const FAMILY: &str = "reactive";

const FLAG_FLIP: i32 = 1;
const WIDTH: i32 = 32;

/// Registers the custom transformer.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "CustomTransformerElm",
            FAMILY,
            "Add Custom Transformer",
            "Passive Components",
            |x, y| Box::new(CustomTransformer::new(x, y)),
        )
        .dump(406, |a, b, c, d, f, st| {
            Ok(Box::new(CustomTransformer::from_dump(a, b, c, d, f, st)))
        }),
    );
}

/// Splits a description into numbers and the separators `,` `:` `+`.
fn tokens(desc: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, ch) in desc.char_indices() {
        if matches!(ch, ',' | ':' | '+') {
            if start < i {
                out.push(&desc[start..i]);
            }
            out.push(&desc[i..=i]);
            start = i + 1;
        }
    }
    if start < desc.len() {
        out.push(&desc[start..]);
    }
    out
}

/// A transformer with any number of windings.
pub struct CustomTransformer {
    base: Base,
    inductance: f64,
    coupling: f64,
    description: String,
    flip: i32,
    coil_count: usize,
    node_count: usize,
    primary_coils: usize,
    coil_nodes: Vec<usize>,
    coil_inductances: Vec<f64>,
    coil_polarities: Vec<f64>,
    coil_currents: Vec<f64>,
    coil_cur_counts: Vec<f64>,
    coil_cur_sources: Vec<f64>,
    node_currents: Vec<f64>,
    node_cur_counts: Vec<f64>,
    node_points: Vec<Point>,
    node_taps: Vec<Point>,
    core: [Point; 4],
    dots: Option<Vec<Point>>,
    need_dots: bool,
    /// Inverse of the inductance matrix times the time step.
    xform: Vec<Vec<f64>>,
}

impl CustomTransformer {
    fn blank(base: Base) -> Self {
        let mut base = base;
        base.no_diagonal = true;
        CustomTransformer {
            base,
            inductance: 4.0,
            coupling: 0.999,
            description: "1,1:1".into(),
            flip: 1,
            coil_count: 0,
            node_count: 0,
            primary_coils: 0,
            coil_nodes: Vec::new(),
            coil_inductances: Vec::new(),
            coil_polarities: Vec::new(),
            coil_currents: Vec::new(),
            coil_cur_counts: Vec::new(),
            coil_cur_sources: Vec::new(),
            node_currents: Vec::new(),
            node_cur_counts: Vec::new(),
            node_points: Vec::new(),
            node_taps: Vec::new(),
            core: [Point::default(); 4],
            dots: None,
            need_dots: false,
            xform: Vec::new(),
        }
    }

    /// A transformer being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut t = Self::blank(Base::new(x, y));
        let d = t.description.clone();
        let _ = t.parse_description(&d);
        t
    }

    /// A transformer read from a text line.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut t = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        t.inductance = st.next_f64().unwrap_or(4.0);
        t.coupling = st.next_f64().unwrap_or(0.999);
        if let Some(d) = st.next_token() {
            t.description = unescape(d);
        }
        let count = st.next_i32().unwrap_or(0).max(0) as usize;
        t.coil_currents = (0..count).map(|_| st.next_f64().unwrap_or(0.0)).collect();
        let d = t.description.clone();
        let _ = t.parse_description(&d);
        t
    }

    fn trapezoidal(&self) -> bool {
        self.base.flags & FLAG_BACK_EULER == 0
    }

    /// Reads a description and sets up the windings; on a mistake says what is wrong and
    /// leaves the transformer as it was.
    fn parse_description(&mut self, desc: &str) -> Result<(), String> {
        let toks = tokens(desc);
        let mut coil_count = 0;
        let mut node_count: i32 = 0;
        for t in &toks {
            match *t {
                "+" => node_count -= 1,
                "," | ":" => {}
                _ => {
                    node_count += 2;
                    coil_count += 1;
                }
            }
        }
        if coil_count == 0 {
            return Err("no coils defined".into());
        }
        let mut coil_nodes = vec![0; coil_count];
        let mut inductances = vec![0.0; coil_count];
        let mut polarities = vec![1.0; coil_count];
        let (mut node_num, mut coil_num, mut primary) = (0usize, 0usize, 0usize);
        let (mut secondary, mut need_dots) = (false, false);
        let mut i = 0;
        loop {
            let tok = toks.get(i).copied().unwrap_or("");
            i += 1;
            let n: f64 = tok.trim().parse().map_err(|_| format!("expected number, got '{tok}'"))?;
            if n == 0.0 {
                return Err("turns ratio cannot be zero".into());
            }
            coil_nodes[coil_num] = node_num;
            inductances[coil_num] = n * n * self.inductance;
            if n < 0.0 {
                polarities[coil_num] = -1.0;
                need_dots = true;
            }
            node_num += 2;
            coil_num += 1;
            if !secondary {
                primary = coil_num;
            }
            let Some(&sep) = toks.get(i) else { break };
            i += 1;
            match sep {
                "," => {}
                "+" => node_num -= 1,
                ":" => {
                    if secondary {
                        return Err("only one ':' separator allowed".into());
                    }
                    secondary = true;
                }
                other => return Err(format!("unexpected '{other}'")),
            }
        }
        let node_count = node_count as usize;
        self.coil_count = coil_count;
        self.node_count = node_count;
        self.coil_nodes = coil_nodes;
        self.coil_inductances = inductances;
        self.coil_polarities = polarities;
        self.primary_coils = primary;
        self.need_dots = need_dots;
        if self.coil_currents.len() != coil_count {
            self.coil_currents = vec![0.0; coil_count];
        }
        self.coil_cur_counts = vec![0.0; coil_count];
        self.coil_cur_sources = vec![0.0; coil_count];
        self.node_currents = vec![0.0; node_count];
        self.node_cur_counts = vec![0.0; node_count];
        self.node_points = vec![Point::default(); node_count];
        self.node_taps = vec![Point::default(); node_count];
        self.xform.clear();
        self.alloc_nodes();
        self.set_points();
        Ok(())
    }

    fn reparse(&mut self) {
        let d = self.description.clone();
        let _ = self.parse_description(&d);
    }
}

impl Element for CustomTransformer {
    impl_element_base!("CustomTransformerElm");

    fn dump_type(&self) -> i32 {
        406
    }
    fn dump(&self) -> String {
        let mut s = format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(406),
            js_number(self.inductance),
            js_number(self.coupling),
            escape(&self.description),
            self.coil_count
        );
        for c in &self.coil_currents {
            s += &format!(" {}", js_number(*c));
        }
        s
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("in", self.inductance);
        out.double_attr("cc", self.coupling);
        out.str_attr("ds", &self.description);
        out.int_attr("nc", self.coil_count as i32);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        let s: Vec<String> = self.coil_currents.iter().map(|c| js_number(*c)).collect();
        out.str_attr("ci", &s.join(" "));
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.inductance = xml.double_attr("in", self.inductance);
        self.coupling = xml.double_attr("cc", self.coupling);
        self.description = xml.string_attr("ds", &self.description);
        let count = xml.int_attr("nc", self.coil_count as i32).max(0) as usize;
        self.coil_currents = vec![0.0; count];
        if let Some(ci) = xml.attr("ci") {
            for (slot, t) in self.coil_currents.iter_mut().zip(ci.split_whitespace()) {
                *slot = t.parse().unwrap_or(0.0);
            }
        }
        self.reparse();
    }

    fn post_count(&self) -> usize {
        self.node_count
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.node_points.get(n).copied()
    }

    fn set_points(&mut self) {
        self.base.set_points();
        if self.node_count == 0 {
            return;
        }
        self.base.point2.y = self.base.point1.y;
        self.flip = if self.base.flags & FLAG_FLIP != 0 { -1 } else { 1 };
        let (p1, p2) = (self.base.point1, self.base.point2);
        let primary_nodes = if self.primary_coils == self.coil_count {
            self.node_count
        } else {
            self.coil_nodes[self.primary_coils]
        };
        let dn = f64::from((p1.x - p2.x).abs());
        self.base.dn = dn;
        let ce = 0.5 - 12.0 / dn;
        let cd = 0.5 - 2.0 / dn;
        let mut max_width = 0.0f64;
        let flip = f64::from(self.flip);
        for step in 0..2 {
            let mut c = 0;
            let mut offset = 0.0f64;
            for i in 0..self.node_count {
                if i == primary_nodes {
                    offset = 0.0;
                }
                if step == 1 {
                    if i == primary_nodes - 1 || i == self.node_count - 1 {
                        offset = max_width;
                    }
                    let (f_post, f_tap) =
                        if i < primary_nodes { (0.0, ce) } else { (1.0, 1.0 - ce) };
                    self.node_points[i] = interp_point_g(p1, p2, f_post, -offset * flip);
                    self.node_taps[i] = interp_point_g(p1, p2, f_tap, -offset * flip);
                }
                max_width = max_width.max(offset);
                if c < self.coil_count && self.coil_nodes[c] == i {
                    c += 1;
                    offset += f64::from(WIDTH);
                } else {
                    offset += 16.0;
                }
            }
        }
        for i in [0, 2] {
            let h = if i == 2 { -max_width * flip } else { 0.0 };
            self.core[i] = interp_point_g(p1, p2, cd, h);
            self.core[i + 1] = interp_point_g(p1, p2, 1.0 - cd, h);
        }
        self.dots = if self.need_dots {
            let dotp = (7.0 / f64::from(WIDTH)).abs();
            Some(
                (0..self.coil_count)
                    .map(|i| {
                        let n = self.coil_nodes[i];
                        let f = if self.coil_polarities[i] > 0.0 { dotp } else { 1.0 - dotp };
                        let side = if i < self.primary_coils { -7.0 } else { 7.0 };
                        interp_point_g(self.node_taps[n], self.node_taps[n + 1], f, side)
                    })
                    .collect(),
            )
        } else {
            None
        };
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.coil_currents.iter_mut().for_each(|c| *c = 0.0);
        self.coil_cur_sources.iter_mut().for_each(|c| *c = 0.0);
        self.coil_cur_counts.iter_mut().for_each(|c| *c = 0.0);
        self.base.volts.iter_mut().for_each(|c| *c = 0.0);
        self.node_currents.iter_mut().for_each(|c| *c = 0.0);
        self.node_cur_counts.iter_mut().for_each(|c| *c = 0.0);
    }

    fn visual_state(&self) -> Vec<f64> {
        self.coil_currents.iter().chain(&self.node_currents).copied().collect()
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        let n = self.coil_currents.len();
        if state.len() == n + self.node_currents.len() {
            self.coil_currents.copy_from_slice(&state[..n]);
            self.node_currents.copy_from_slice(&state[n..]);
        }
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.coil_count;
        let mut m = vec![vec![0.0; n]; n];
        for i in 0..n {
            m[i][i] = self.coil_inductances[i];
            for j in 0..i {
                let v = self.coupling
                    * (self.coil_inductances[i] * self.coil_inductances[j]).sqrt()
                    * self.coil_polarities[i]
                    * self.coil_polarities[j];
                m[i][j] = v;
                m[j][i] = v;
            }
        }
        // invert by solving for each unit vector
        let mut lu = m.clone();
        let mut ipvt = vec![0usize; n];
        let mut inv = vec![vec![0.0; n]; n];
        if lu_factor(&mut lu, n, &mut ipvt) {
            for col in 0..n {
                let mut b = vec![0.0; n];
                b[col] = 1.0;
                lu_solve(&lu, n, &ipvt, &mut b);
                for row in 0..n {
                    inv[row][col] = b[row];
                }
            }
        }
        let ts = if self.trapezoidal() { s.time_step / 2.0 } else { s.time_step };
        let nodes = self.base.nodes.clone();
        for i in 0..n {
            for j in 0..n {
                inv[i][j] *= ts;
                let (ni, nj) = (self.coil_nodes[i], self.coil_nodes[j]);
                if i == j {
                    s.stamp_conductance(nodes[ni], nodes[ni + 1], inv[i][i]);
                } else {
                    s.stamp_vccs(nodes[ni], nodes[ni + 1], nodes[nj], nodes[nj + 1], inv[i][j]);
                }
            }
        }
        self.xform = inv;
        for node in &nodes {
            s.stamp_right_side_var(*node);
        }
    }

    fn start_iteration(&mut self, _s: &Stamper) {
        for i in 0..self.coil_count {
            let mut val = self.coil_currents[i];
            if self.trapezoidal() && !self.xform.is_empty() {
                for j in 0..self.coil_count {
                    let n = self.coil_nodes[j];
                    let vd = self.base.volts[n] - self.base.volts[n + 1];
                    val += vd * self.xform[i][j];
                }
            }
            self.coil_cur_sources[i] = val;
        }
    }

    fn do_step(&mut self, s: &mut Stamper) {
        for i in 0..self.coil_count {
            let n = self.coil_nodes[i];
            s.stamp_current_source(
                self.base.nodes[n],
                self.base.nodes[n + 1],
                self.coil_cur_sources[i],
            );
        }
    }

    fn calculate_current(&mut self) {
        self.node_currents.iter_mut().for_each(|c| *c = 0.0);
        for i in 0..self.coil_count {
            let mut val = self.coil_cur_sources[i];
            if !self.xform.is_empty() {
                for j in 0..self.coil_count {
                    let n = self.coil_nodes[j];
                    let vd = self.base.volts[n] - self.base.volts[n + 1];
                    val += vd * self.xform[i][j];
                }
            }
            self.coil_currents[i] = val;
            let ni = self.coil_nodes[i];
            self.node_currents[ni] += val;
            self.node_currents[ni + 1] -= val;
        }
    }

    fn get_current_into_node(&self, n: usize) -> f64 {
        -self.node_currents.get(n).copied().unwrap_or(0.0)
    }

    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        self.coil_nodes.iter().any(|&c| (n1 == c && n2 == c + 1) || (n2 == c && n1 == c + 1))
    }
    fn get_matrix_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        if self.node_count == 0 {
            return;
        }
        let v = self.base.volts.clone();
        for i in 0..self.node_count {
            set_voltage_color(g, &self.base, c, v[i]);
            draw_thick_line(g, self.node_points[i], self.node_taps[i]);
        }
        for i in 0..self.coil_count {
            let n = self.coil_nodes[i];
            set_voltage_color(g, &self.base, c, v[n]);
            set_power_color_w(g, &self.base, c, self.coil_currents[i] * (v[n] - v[n + 1]));
            let hs = if i >= self.primary_coils { -6 * self.flip } else { 6 * self.flip };
            draw_coil(
                g,
                &self.base,
                c,
                hs,
                self.node_taps[n],
                self.node_taps[n + 1],
                v[n],
                v[n + 1],
            );
            if let Some(dots) = &self.dots {
                g.set_color(if self.base.needs_highlight() {
                    c.select_color
                } else {
                    c.light_gray
                });
                g.fill_oval(dots[i].x - 2, dots[i].y - 2, 5, 5);
            }
        }
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        for i in 0..2 {
            draw_thick_line(g, self.core[i], self.core[i + 2]);
        }
        for i in 0..self.coil_count {
            self.coil_cur_counts[i] =
                self.base.update_dot_count_for(self.coil_currents[i], self.coil_cur_counts[i], c);
            let ni = self.coil_nodes[i];
            draw_dots(g, c, self.node_taps[ni], self.node_taps[ni + 1], self.coil_cur_counts[i]);
        }
        for i in 0..self.node_count {
            self.node_cur_counts[i] =
                self.base.update_dot_count_for(self.node_currents[i], self.node_cur_counts[i], c);
            draw_dots(g, c, self.node_points[i], self.node_taps[i], self.node_cur_counts[i]);
        }
        draw_posts(g, &self.base, self, c);
        let (a, b) = (self.node_points[0], self.node_points[self.node_count - 1]);
        self.base.set_bbox_pts(a, b, 0.0);
        self.base.adjust_bbox_pts(self.core[0], self.core[3]);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("transformer (custom)".into());
        arr.push(format!("L = {}", unit_text(self.inductance, "H")));
        for i in 0..self.coil_count {
            if 2 + i * 2 >= 10 {
                break;
            }
            let ni = self.coil_nodes[i];
            let v = &self.base.volts;
            arr.push(format!("Vd{} = {}", i + 1, voltage_text(v[ni] - v[ni + 1])));
            arr.push(format!("I{} = {}", i + 1, current_text(self.coil_currents[i])));
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(
                EditInfo::new("Base Inductance (H)", self.inductance, 0.01, 5.0).set_positive(),
            ),
            1 => Some(EditInfo::text("Description", &self.description)),
            2 => Some(
                EditInfo::new("Coupling Coefficient", self.coupling, 0.0, 1.0)
                    .set_dimensionless()
                    .set_positive(),
            ),
            3 => Some(EditInfo::checkbox("Trapezoidal Approximation", self.trapezoidal())),
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 if ei.value > 0.0 => {
                self.inductance = ei.value;
                self.reparse();
            }
            1 => {
                if let Some(s) = ei.text.clone() {
                    if s != self.description {
                        match self.parse_description(&s) {
                            Ok(()) => self.description = s,
                            Err(e) => {
                                ei.error = Some(e);
                                self.reparse();
                            }
                        }
                    }
                }
            }
            2 => {
                if ei.value > 0.0 && ei.value < 1.0 {
                    self.coupling = ei.value;
                    self.reparse();
                } else {
                    ei.error = Some("must be > 0 and < 1".into());
                }
            }
            3 => {
                self.base.flags = ei.change_flag_inverted(self.base.flags, FLAG_BACK_EULER);
                self.reparse();
            }
            _ => {}
        }
    }

    fn flip_x(&mut self, center2: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIP;
        self.base.flip_x(center2);
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, _count: usize) {
        self.base.flags ^= FLAG_FLIP;
        self.base.flip_y(center2);
        self.set_points();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptions_split_into_numbers_and_separators() {
        assert_eq!(tokens("1,2:3+4"), ["1", ",", "2", ":", "3", "+", "4"]);
        assert!(tokens("").is_empty());
    }

    #[test]
    fn descriptions_are_checked() {
        let mut t = CustomTransformer::new(16, 16);
        assert!(t.parse_description("1:2").is_ok());
        assert_eq!((t.coil_count, t.node_count, t.primary_coils), (2, 4, 1));
        // windings in series share a post
        assert!(t.parse_description("1+1:2").is_ok());
        assert_eq!((t.coil_count, t.node_count), (3, 5));
        assert!(t.parse_description("").is_err());
        assert!(t.parse_description("1:x").unwrap_err().starts_with("expected number"));
        assert_eq!(t.parse_description("0:1").unwrap_err(), "turns ratio cannot be zero");
        assert_eq!(t.parse_description("1:2:3").unwrap_err(), "only one ':' separator allowed");
    }
}
