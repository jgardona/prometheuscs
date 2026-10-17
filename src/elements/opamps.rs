// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (OpAmpRealElm).
//! Op amps built from their transistors (the LM741 and the two LM324 models) and the
//! optocoupler.

use super::chip::{FLAG_FLIP_X, FLAG_FLIP_Y};
use super::composite::{ChildNet, FLAG_ESCAPE, patch_child};
use super::element::*;
use super::geometry::*;
use super::models::DiodeModel;
use super::registry::{ElementDescriptor, Registry};
use super::units::{current_text, voltage_text};
use crate::format::number::js_number;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::forward_to_net;
use crate::impl_element_base;
use crate::render::painter::Painter;

/// Family name used for the feature switch.
pub const FAMILY: &str = "subcircuits";

// from https://commons.wikimedia.org/wiki/File:OpAmpTransistorLevel_Colored_Labeled.svg
const MODEL_741: &str = "NTransistorElm 3 8 9\nNTransistorElm 2 8 10\nPTransistorElm 11 12 9\nPTransistorElm 11 13 10\nNTransistorElm 14 12 1\nNTransistorElm 14 13 5\nNTransistorElm 12 7 14\nPTransistorElm 8 8 7\nPTransistorElm 8 11 7\nNTransistorElm 17 11 16\nNTransistorElm 17 17 4\nPTransistorElm 18 18 7\nPTransistorElm 18 20 7\nNTransistorElm 20 7 25\nNTransistorElm 13 22 24\nNTransistorElm 21 20 22\nNTransistorElm 25 20 6\nNTransistorElm 24 22 23\nPTransistorElm 22 4 15\nNTransistorElm 23 13 4\nCapacitorElm 13 20\nResistorElm 15 6\nResistorElm 6 25\nResistorElm 4 1\nResistorElm 4 14\nResistorElm 4 5\nResistorElm 4 16\nResistorElm 4 24\nResistorElm 4 23\nResistorElm 17 18\nResistorElm 22 21\nResistorElm 21 20";
const MODEL_324: &str = "TransistorElm 1 2 3\nCurrentElm 4 3\nTransistorElm 2 2 5\nTransistorElm 2 6 5\nCapacitorElm 6 7\nCurrentElm 4 8\nCurrentElm 4 7\nTransistorElm 8 4 9\nTransistorElm 7 4 10\nTransistorElm 10 4 11\nTransistorElm 11 7 12\nResistorElm 11 12\nTransistorElm 7 5 12\nCurrentElm 12 5\nTransistorElm 6 5 8\nResistorElm 9 5\nTransistorElm 9 7 5\nTransistorElm 13 6 3";
const DUMP_324: &str = "0 -1 -0 0 10000/0 0.000006/0 1 0 0 100/0 1 0 0 100/0 1e-11 0/0 0.000004/0 0.0001/0 1 0 0 100/0 1 0 0 100/0 1 0 0 100/0 1 0 0 100/0 25/0 -1 0 0 100/0 0.00005/0 -1 0 0 100/0 10000/0 1 0 0 100/0 -1 0 0 10000";
const MODEL_324V2: &str = "ResistorElm 4 6\nCurrentElm 4 7\nResistorElm 4 29\nResistorElm 8 30\nResistorElm 9 31\nTransistorElm 30 29 31 \nResistorElm 4 32\nResistorElm 2 33\nResistorElm 10 34\nTransistorElm 33 32 34 \nResistorElm 9 35\nResistorElm 9 36\nResistorElm 11 37\nTransistorElm 36 35 37 \nResistorElm 10 38\nResistorElm 10 39\nResistorElm 11 40\nTransistorElm 39 38 40 \nResistorElm 12 41\nTransistorElm 13 41 4 \nResistorElm 13 42\nTransistorElm 13 42 4 \nResistorElm 4 43\nTransistorElm 12 43 14 \nResistorElm 3 44\nTransistorElm 14 44 6 \nResistorElm 15 45\nTransistorElm 6 45 4 \nResistorElm 3 46\nTransistorElm 15 46 16 \nResistorElm 3 47\nTransistorElm 16 47 17 \nResistorElm 17 16\nResistorElm 5 17\nResistorElm 4 48\nTransistorElm 15 48 5 \nResistorElm 15 49\nTransistorElm 17 49 5 \nCurrentElm 18 3\nCurrentElm 19 3\nCurrentElm 20 3\nResistorElm 11 50\nTransistorElm 18 50 3 \nResistorElm 14 51\nTransistorElm 19 51 3 \nResistorElm 5 52\nTransistorElm 7 52 4 \nResistorElm 15 53\nTransistorElm 20 53 3 \nCapacitorElm 21 22\nResistorElm 12 21\nResistorElm 12 15\nVCVSElm 3 0 23 8\nVoltageElm 23 1\nCurrentElm 3 4\nResistorElm 4 3\nResistorElm 12 54\nTransistorElm 9 54 11 \nResistorElm 13 55\nTransistorElm 10 55 11 \nCapacitorElm 12 13\nCapacitorElm 6 15\nCapacitorElm 3 24\nResistorElm 11 24\nCapacitorElm 1 2\nCapacitorElm 2 0\nCapacitorElm 1 0\nVCVSElm 15 0 22 0\nCapacitorElm 5 0\nResistorElm 25 56\nTransistorElm 25 56 0 \nVCCSElm 27 0 4 3\nCurrentElm 0 25\nVoltageElm 25 26\nResistorElm 0 26\nVCVSElm 28 26 27 0\nResistorElm 0 27\nVoltageElm 28 0\nResistorElm 0 28";
const DUMP_324V2: &str = "0 40000/0 5e-7/0 380/0 1700/0 5/0 -1 0 0 306 xlm324v2-qpi/0 380/0 1700/0 5/0 -1 0 0 300 xlm324v2-qpa/0 380/0 1700/0 5/0 -1 0 0 306 xlm324v2-qpi/0 380/0 1700/0 5/0 -1 0 0 306 xlm324v2-qpi/0 25/0 1 0 0 100 xlm324v2-qnq/0 25/0 1 0 0 100 xlm324v2-qnq/0 300/0 -1 0 0 100 xlm324v2-qpq/0 25/0 1 0 0 100 xlm324v2-qnq/0 25/0 1 0 0 100 xlm324v2-qnq/0 25/0 1 0 0 100 xlm324v2-qnq/0 25/0 1 0 0 100 xlm324v2-qnq/0 40000/0 18/0 300/0 -1 0 0 100 xlm324v2-qpq/0 25/0 1 0 0 100 xlm324v2-qnq/0 1.2e-7/0 6e-8/0 0.000001/0 300/0 -1 0 0 100 xlm324v2-qpq/0 300/0 -1 0 0 100 xlm324v2-qpq/0 25/0 1 0 0 100 xlm324v2-qnq/0 300/0 -1 0 0 100 xlm324v2-qpq/2 4.8e-12 0 0/0 3/0 3000000000/0 2 -0.00001*(a-b)/0 0 0 -0.00156/0 0.000005/0 450000/0 300/0 -1 0 0 100 xlm324v2-qpq/0 300/0 -1 0 0 100 xlm324v2-qpq/2 8e-12 0 0/2 1e-12 0 0/2 1e-13 0 0/0 300000/2 2.3e-13 0 0/2 7.9e-13 0 0/2 7.9e-13 0 0/0 2 2*(a-b)/2 5e-14 0 0/0 25/0 1 0 0 100 xlm324v2-qnq/0 2 0.0003*(a-b)/0 0.001/0 0 0 -0.25/0 1000000/0 2 1*(a-b)/0 1000000/0 0 0 -0.55/0 1000000";

const NODES_741: [i32; 5] = [2, 3, 6, 7, 4];
const NODES_324: [i32; 5] = [1, 13, 12, 4, 5];
const NODES_324V2: [i32; 5] = [2, 1, 5, 3, 4];
const RESISTANCES_741: [f64; 11] =
    [50.0, 25.0, 1e3, 50e3, 1e3, 5e3, 50e3, 50.0, 39e3, 7500.0, 4500.0];

const TYPE_741: i32 = 0;
const TYPE_324: i32 = 1;
const TYPE_324V2: i32 = 2;
const DEFAULT_CURRENT_LIMIT: f64 = 0.0231;
const FLAG_SWAP: i32 = 2;
const OP_HEIGHT: i32 = 16;
const OP_WIDTH: i32 = 32;

/// A transistor-level op amp (LM741, LM324).
pub struct OpAmpReal {
    base: Base,
    net: ChildNet,
    model_type: i32,
    /// V/us.
    slew_rate: f64,
    current_limit: f64,
    /// Voltage of the compensation capacitor (kept across model rebuilds and in the files).
    cap_value: f64,
    curcounts: [f64; 5],
    in1: [Point; 2],
    in2: [Point; 2],
    text_p: [Point; 2],
    rail1: [Point; 2],
    rail2: [Point; 2],
    triangle: Polygon,
}

impl OpAmpReal {
    fn blank(base: Base) -> Self {
        let mut a = OpAmpReal {
            base,
            net: ChildNet::default(),
            model_type: TYPE_741,
            slew_rate: 0.6,
            current_limit: DEFAULT_CURRENT_LIMIT,
            cap_value: 0.0,
            curcounts: [0.0; 5],
            in1: [Point::default(); 2],
            in2: [Point::default(); 2],
            text_p: [Point::default(); 2],
            rail1: [Point::default(); 2],
            rail2: [Point::default(); 2],
            triangle: Polygon::default(),
        };
        a.base.no_diagonal = true;
        a
    }

    /// An op amp being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut a = Self::blank(Base::new(x, y));
        // the registry makes one at the origin to learn its tag: the parts need the registry
        if x != 0 || y != 0 {
            a.init_model();
        }
        a
    }

    /// An op amp read from a text line: slew rate, capacitor voltage, current limit, model.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let mut a = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        a.slew_rate = st.next_f64().unwrap_or(0.6);
        a.cap_value = st.next_f64().unwrap_or(0.0);
        if let (Ok(cl), Ok(mt)) = (st.next_f64(), st.next_i32()) {
            a.current_limit = cl;
            a.model_type = mt;
        }
        a.init_model();
        a
    }

    /// The compensation capacitor is the only child that holds charge.
    fn cap_index(&self) -> Option<usize> {
        match self.model_type {
            TYPE_741 => Some(20),
            TYPE_324 => Some(4),
            _ => None,
        }
    }

    fn cap_voltage(&self) -> f64 {
        let Some(c) = self.cap_index().and_then(|i| self.net.children.get(i)) else { return 0.0 };
        let mut out = XmlOut::new("c");
        c.dump_xml_state(&mut out);
        out.attrs.iter().find(|(k, _)| k == "vd").and_then(|(_, v)| v.parse().ok()).unwrap_or(0.0)
    }

    fn patch(&mut self, index: usize, attrs: &[(&str, String)]) {
        if let Some(c) = self.net.children.get_mut(index) {
            patch_child(c.as_mut(), attrs);
        }
    }

    /// Builds the parts for the current model, slew rate and current limit.
    fn init_model(&mut self) {
        self.base.flags |= FLAG_ESCAPE;
        let current_mult = self.current_limit / DEFAULT_CURRENT_LIMIT;
        let beta = ("be", (current_mult * 100.0).to_string());
        match self.model_type {
            TYPE_324 => {
                let tokens: Vec<&str> = DUMP_324.split('/').collect();
                self.net = ChildNet::from_model_tokens(MODEL_324, &NODES_324, &tokens);
                self.patch(
                    4,
                    &[
                        ("c", (10e-12 / (self.slew_rate / 0.55)).to_string()),
                        ("vd", self.cap_value.to_string()),
                    ],
                );
                self.patch(11, &[("r", (25.0 / current_mult).to_string())]);
                for i in [9, 10, 12, 16] {
                    self.patch(i, std::slice::from_ref(&beta));
                }
            }
            TYPE_324V2 => {
                let tokens: Vec<&str> = DUMP_324V2.split('/').collect();
                self.net = ChildNet::from_model_tokens(MODEL_324V2, &NODES_324V2, &tokens);
            }
            _ => {
                self.net = ChildNet::from_model_string(MODEL_741, &NODES_741, None, true);
                self.patch(
                    20,
                    &[
                        ("c", (30e-12 / (self.slew_rate / 0.6)).to_string()),
                        ("vd", self.cap_value.to_string()),
                    ],
                );
                for (i, r) in RESISTANCES_741.iter().enumerate() {
                    let mut r = *r;
                    if i < 2 {
                        r /= current_mult;
                    }
                    self.patch(21 + i, &[("r", r.to_string())]);
                }
                self.patch(13, std::slice::from_ref(&beta));
                self.patch(18, &[beta]);
            }
        }
        self.curcounts = [0.0; 5];
        self.set_points();
    }
}

impl Element for OpAmpReal {
    impl_element_base!("OpAmpRealElm");
    forward_to_net!();

    fn dump_type(&self) -> i32 {
        409
    }
    fn dump(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.base.dump_prefix(409),
            js_number(self.slew_rate),
            js_number(self.cap_voltage()),
            js_number(self.current_limit),
            self.model_type
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("slr", self.slew_rate);
        out.double_attr("cl", self.current_limit);
        out.int_attr("mt", self.model_type);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.double_attr("vd", self.cap_voltage());
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.slew_rate = xml.double_attr("slr", self.slew_rate);
        self.current_limit = xml.double_attr("cl", self.current_limit);
        self.model_type = xml.int_attr("mt", self.model_type);
        self.cap_value = xml.double_attr("vd", 0.0);
        self.init_model();
    }

    fn post_count(&self) -> usize {
        5
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        Some(match n {
            0 => self.in1[0],
            1 => self.in2[0],
            2 => self.base.point2,
            3 => self.rail1[0],
            _ => self.rail2[0],
        })
    }
    fn can_flip_x(&self) -> bool {
        self.base.dy == 0
    }
    fn can_flip_y(&self) -> bool {
        self.base.dx == 0
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let mut ww = OP_WIDTH;
        if f64::from(ww) > self.base.dn / 2.0 {
            ww = (self.base.dn / 2.0) as i32;
        }
        self.base.calc_leads(ww * 2);
        let hs = OP_HEIGHT * self.base.dsign;
        let hsswap = if self.base.flags & FLAG_SWAP != 0 { -hs } else { hs };
        let (p1, p2, l1, l2) =
            (self.base.point1, self.base.point2, self.base.lead1, self.base.lead2);
        let (a, b) = interp_point2(p1, p2, 0.0, f64::from(hsswap));
        let (c, d) = interp_point2(l1, l2, 0.0, f64::from(hsswap));
        self.in1 = [a, c];
        self.in2 = [b, d];
        let (t0, t1) = interp_point2(l1, l2, 0.2, f64::from(hsswap));
        self.text_p = [t0, t1];
        // rails: ideally in the middle but they may have to move to fit the grid
        let rail_pos = 0.5 - ((self.base.dn / 2.0) % f64::from(grid_size())) / f64::from(ww * 2);
        let (r1, r2) = interp_point2(l1, l2, rail_pos, f64::from(hs) * 2.0 * (1.0 - rail_pos));
        let (r3, r4) = interp_point2(l1, l2, rail_pos, f64::from(hs * 2));
        self.rail1 = [r3, r1];
        self.rail2 = [r4, r2];
        let (tr0, tr1) = interp_point2(l1, l2, 0.0, f64::from(hs * 2));
        self.triangle = Polygon::from_points(&[tr0, tr1, l2]);
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
        self.curcounts = [0.0; 5];
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(OP_HEIGHT * 2));
        let v = self.base.volts.clone();
        let lines = [
            (self.in1[0], self.in1[1]),
            (self.in2[0], self.in2[1]),
            (self.base.lead2, p2),
            (self.rail1[0], self.rail1[1]),
            (self.rail2[0], self.rail2[1]),
        ];
        for (i, (a, b)) in lines.iter().enumerate() {
            set_voltage_color(g, &self.base, c, v[i]);
            draw_thick_line(g, *a, *b);
        }
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        let power = self.net.power();
        set_power_color_w(g, &self.base, c, power);
        draw_thick_polygon(g, &self.triangle);
        g.set_font_size(14);
        let t = self.text_p;
        draw_centered_text(g, &mut self.base, "-", t[0].x, t[0].y - 2, true);
        draw_centered_text(g, &mut self.base, "+", t[1].x, t[1].y, true);
        for i in 0..5 {
            let cur = self.net.current_into_node(i);
            self.curcounts[i] = self.base.update_dot_count_for(cur, self.curcounts[i], c);
        }
        draw_dots(g, c, self.in1[1], self.in1[0], self.curcounts[0]);
        draw_dots(g, c, self.in2[1], self.in2[0], self.curcounts[1]);
        draw_dots(g, c, self.base.lead2, p2, self.curcounts[2]);
        // drawn the other way so the dots line up on the off-grid segments
        draw_dots(g, c, self.rail1[0], self.rail1[1], -self.curcounts[3]);
        draw_dots(g, c, self.rail2[0], self.rail2[1], -self.curcounts[4]);
        draw_posts(g, &self.base, self, c);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let kind = if self.model_type == TYPE_741 { "LM741" } else { "LM324" };
        let v = &self.base.volts;
        arr.push(format!("op-amp ({kind})"));
        arr.push(format!("V+ = {}", voltage_text(v[1])));
        arr.push(format!("V- = {}", voltage_text(v[0])));
        arr.push(format!("Vout = {}", voltage_text(v[2])));
        arr.push(format!("Iout = {}", current_text(self.net.current_into_node(2))));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => {
                let (items, selected) = if self.model_type == TYPE_324 {
                    (vec!["LM741", "LM324, old", "LM324, fixed"], self.model_type)
                } else {
                    (vec!["LM741", "LM324"], i32::from(self.model_type != TYPE_741))
                };
                Some(EditInfo::choice(
                    "Model",
                    items.into_iter().map(String::from).collect(),
                    selected as usize,
                ))
            }
            1 => Some(EditInfo::checkbox("Swap Inputs", self.base.flags & FLAG_SWAP != 0)),
            _ if self.model_type == TYPE_324V2 => None,
            2 => Some(EditInfo::number("Slew Rate (V/usec)", self.slew_rate)),
            3 => Some(EditInfo::number("Output Current Limit (A)", self.current_limit)),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                let mut t = ei.choice_index as i32;
                if self.model_type != TYPE_324 && t == 1 {
                    t = TYPE_324V2;
                }
                self.model_type = t;
                self.cap_value = 0.0;
                self.init_model();
                ei.new_dialog = true;
            }
            1 => {
                self.base.flags = ei.change_flag(self.base.flags, FLAG_SWAP);
                self.set_points();
            }
            2 => {
                self.slew_rate = ei.value;
                self.cap_value = self.cap_voltage();
                self.init_model();
            }
            3 => {
                self.current_limit = ei.value;
                self.cap_value = self.cap_voltage();
                self.init_model();
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------
// Optocoupler
// ---------------------------------------------------------------------------------------

const OPTO_MODEL: &str = "DiodeElm 6 1\nCCCSElm 1 2 3 4\nNTransistorElm 3 4 5";
const OPTO_NODES: [i32; 4] = [6, 2, 4, 5];
const CSPC: i32 = 16;
const CSPC2: i32 = CSPC * 2;
const DEFAULT_LED_MODEL: &str = "default-optocoupler-led";

/// The current transfer expression of the original (about 100% CTR) scaled by `ctr`, from
/// http://www.cel.com/pdf/appnotes/an3017.pdf
fn transfer_expression(ctr: f64) -> String {
    format!(
        "{}*max(0,min(.0001, select(i-.003, (-80000000000*(i)^5+800000000*(i)^4-3000000*(i)^3+5177.2*(i)^2+.2453*(i)-.00005)*1.04/700, (9000000*(i)^5-998113*(i)^4+42174*(i)^3-861.32*(i)^2+9.0836*(i)-.0078)*.945/700)))",
        js_number(ctr)
    )
}

/// An LED driving a phototransistor through a current-controlled current source.
pub struct Optocoupler {
    base: Base,
    net: ChildNet,
    /// Current transfer ratio scale (1 = 100%).
    ctr: f64,
    led_model: String,
    posts: [Point; 4],
    stubs: [Point; 4],
    rect: Polygon,
    curcounts: [f64; 4],
}

impl Optocoupler {
    fn blank(base: Base) -> Self {
        let mut o = Optocoupler {
            base,
            net: ChildNet::default(),
            ctr: 1.0,
            led_model: DEFAULT_LED_MODEL.into(),
            posts: [Point::default(); 4],
            stubs: [Point::default(); 4],
            rect: Polygon::default(),
            curcounts: [0.0; 4],
        };
        o.base.no_diagonal = true;
        o
    }

    /// An optocoupler being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut o = Self::blank(Base::new(x, y));
        // the registry makes one at the origin to learn its tag: the parts need the registry
        if x != 0 || y != 0 {
            o.init();
        }
        o
    }

    /// An optocoupler read from a text line (the parts' state is not stored).
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, _st: &mut Tokenizer<'_>) -> Self {
        let mut o = Self::blank(Base::from_dump(xa, ya, xb, yb, f));
        o.led_model = "default".into();
        o.init();
        o
    }

    /// Builds the parts and sets the expression, LED model and gain of the transistor.
    fn init(&mut self) {
        self.base.flags |= FLAG_ESCAPE;
        self.net = ChildNet::from_model_string(OPTO_MODEL, &OPTO_NODES, None, true);
        let led = self.led_model.clone();
        let expr = transfer_expression(self.ctr);
        if let Some(d) = self.net.children.get_mut(0) {
            patch_child(d.as_mut(), &[("mo", led)]);
        }
        if let Some(c) = self.net.children.get_mut(1) {
            patch_child(c.as_mut(), &[("ex", expr)]);
        }
        if let Some(t) = self.net.children.get_mut(2) {
            patch_child(t.as_mut(), &[("be", "700".into())]);
        }
        self.curcounts = [0.0; 4];
        self.set_points();
    }

    fn flipped_x(&self) -> bool {
        self.base.flags & FLAG_FLIP_X != 0
    }
    fn flipped_y(&self) -> bool {
        self.base.flags & FLAG_FLIP_Y != 0
    }

    /// Places pin `n` the way the chips do.
    fn set_pin(&mut self, n: usize, p: (i32, i32), d: (i32, i32), da: (i32, i32), s: (i32, i32)) {
        let pos = (n % 2) as i32;
        let ((mut px, mut py), (mut dx, mut dy), (mut dax, mut day), (mut sx, mut sy)) =
            (p, d, da, s);
        if self.flipped_x() {
            dx = -dx;
            dax = -dax;
            px += CSPC2;
            sx = -sx;
        }
        if self.flipped_y() {
            dy = -dy;
            day = -day;
            py += CSPC2;
            sy = -sy;
        }
        let xa = px + CSPC2 * dx * pos + sx;
        let ya = py + CSPC2 * dy * pos + sy;
        self.posts[n] = Point::new(xa + dax * CSPC2, ya + day * CSPC2);
        self.stubs[n] = Point::new(xa + dax * CSPC, ya + day * CSPC);
    }
}

impl Element for Optocoupler {
    impl_element_base!("OptocouplerElm");
    forward_to_net!();

    fn dump_type(&self) -> i32 {
        407
    }
    fn dump(&self) -> String {
        format!("{}{}", self.base.dump_prefix(407), self.net.dump_children())
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("ctr", self.ctr);
        out.str_attr("dmo", &self.led_model);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.str_attr("dmo", &self.led_model);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.ctr = xml.double_attr("ctr", self.ctr);
        // a state restore (which has no "ix") keeps the model; a definition falls back to "default"
        let fallback =
            if xml.attr("ix").is_some() { self.led_model.clone() } else { "default".into() };
        self.led_model = xml.string_attr("dmo", &fallback);
        self.init();
    }

    fn post_count(&self) -> usize {
        4
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.posts.get(n).copied()
    }

    fn set_points(&mut self) {
        self.base.set_points();
        let (x, y) = (self.base.x, self.base.y);
        let x0 = x + CSPC2;
        let y0 = y;
        let xr = x0 - CSPC;
        let yr = y0 - CSPC / 2;
        let xs = 2 * CSPC2;
        let ys = 2 * CSPC2 - CSPC;
        self.rect = Polygon::from_points(&[
            Point::new(xr, yr),
            Point::new(xr + xs, yr),
            Point::new(xr + xs, yr + ys),
            Point::new(xr, yr + ys),
        ]);
        self.base.set_bbox(xr, yr, xr + xs, yr + ys);
        self.set_pin(0, (x0, y0), (0, 1), (-1, 0), (0, 0));
        self.set_pin(1, (x0, y0), (0, 1), (-1, 0), (0, 0));
        self.set_pin(2, (x0, y0), (0, 1), (1, 0), (xs - CSPC2, 0));
        self.set_pin(3, (x0, y0), (0, 1), (1, 0), (xs - CSPC2, 0));
        let dx = if self.flipped_x() { -1 } else { 1 };
        let flip_y = self.flipped_y();
        let posts = self.posts;
        let midp = (posts[2].y + posts[3].y) / 2;
        if self.net.children.len() == 3 {
            self.net.children[0].base_mut().set_position(
                posts[0].x + 32 * dx,
                posts[0].y,
                posts[1].x + 32 * dx,
                posts[1].y,
            );
            // `Base::set_position` only does the generic geometry; the part's own `set_points`
            // computes its terminals and the polygons it draws (the original's `setPosition`
            // is virtual and does both)
            self.net.children[0].set_points();
            let t = &mut self.net.children[2];
            // the transistor's own "swap E/C" flag
            let flags = t.base().flags;
            t.base_mut().flags = if flip_y { flags | 1 } else { flags & !1 };
            t.base_mut().set_position(posts[2].x - 40 * dx, midp, posts[2].x - 24 * dx, midp);
            t.set_points();
            if let (Some(a), Some(b)) =
                (self.net.children[0].get_post(0), self.net.children[0].get_post(1))
            {
                self.stubs[0] = a;
                self.stubs[1] = b;
            }
            if let (Some(c), Some(e)) =
                (self.net.children[2].get_post(1), self.net.children[2].get_post(2))
            {
                self.stubs[2] = c;
                self.stubs[3] = e;
            }
        }
        self.alloc_nodes();
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
        self.curcounts = [0.0; 4];
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        g.set_color(if self.base.needs_highlight() { c.select_color } else { c.light_gray });
        draw_thick_polygon(g, &self.rect);
        for i in 0..4 {
            set_voltage_color(g, &self.base, c, self.base.volts[i]);
            draw_thick_line(g, self.posts[i], self.stubs[i]);
            let cur = -self.net.current_into_node(i);
            self.curcounts[i] = self.base.update_dot_count_for(cur, self.curcounts[i], c);
            draw_dots(g, c, self.posts[i], self.stubs[i], self.curcounts[i]);
        }
        self.net.children[0].draw(g, c);
        self.net.children[2].draw(g, c);
        draw_posts(g, &self.base, self, c);
        // the little arrows from the LED to the transistor
        g.set_color(c.light_gray);
        let dx = if self.flipped_x() { -1 } else { 1 };
        let sx = self.stubs[0].x + 2 * dx;
        let sy = (self.stubs[0].y + self.stubs[1].y) / 2;
        for i in 0..2 {
            let y = sy + i * 10 - 5;
            let (p1, p2) = (Point::new(sx, y), Point::new(sx + 20 * dx, y));
            fill_polygon(g, &calc_arrow(p1, p2, 5.0, 2.0));
            g.draw_line(sx + 10 * dx, y, sx + 15 * dx, y);
        }
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push("optocoupler".into());
        arr.push(format!("CTR Scale = {}", js_number(self.ctr)));
        arr.push(format!("Iin = {}", current_text(self.net.current_into_node(0))));
        arr.push(format!("Iout = {}", current_text(self.net.current_into_node(2))));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => Some(EditInfo::new("CTR Scale", self.ctr, 0.0, 0.0).set_dimensionless()),
            1 => {
                let models = DiodeModel::list(false);
                let selected = models.iter().position(|m| m.name == self.led_model).unwrap_or(0);
                Some(EditInfo::choice(
                    "LED Model",
                    models.iter().map(DiodeModel::description_text).collect(),
                    selected,
                ))
            }
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        if n == 0 && ei.value > 0.0 {
            self.ctr = ei.value;
            self.init();
        }
        if n == 1 {
            if let Some(m) = DiodeModel::list(false).get(ei.choice_index) {
                self.led_model.clone_from(&m.name);
                self.init();
            }
        }
    }

    fn flip_x(&mut self, center2: i32, count: usize) {
        self.base.flags ^= FLAG_FLIP_X;
        if count != 1 {
            let xs = 3 * CSPC2;
            self.base.x = center2 - self.base.x - xs;
            self.base.x2 = center2 - self.base.x2;
        }
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, count: usize) {
        self.base.flags ^= FLAG_FLIP_Y;
        if count != 1 {
            let ys = CSPC2;
            self.base.y = center2 - self.base.y - ys;
            self.base.y2 = center2 - self.base.y2;
        }
        self.set_points();
    }
}

/// Registers the elements of this module.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "OptocouplerElm",
            FAMILY,
            "Add Optocoupler",
            "Active Components",
            |x, y| Box::new(Optocoupler::new(x, y)),
        )
        .dump(407, |a, b, c, d, f, st| Ok(Box::new(Optocoupler::from_dump(a, b, c, d, f, st)))),
    );
    r.add(
        ElementDescriptor::new(
            "OpAmpRealElm",
            FAMILY,
            "Add Op Amp (real)",
            "Active Components",
            |x, y| Box::new(OpAmpReal::new(x, y)),
        )
        .dump(409, |a, b, c, d, f, st| Ok(Box::new(OpAmpReal::from_dump(a, b, c, d, f, st)))),
    );
}
