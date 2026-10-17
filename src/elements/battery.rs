// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (BatteryElm).
//! A battery: drawn like a DC source, modelled as a voltage source whose value follows the
//! state of charge through a table, behind an ohmic resistor and a polarization network.
//!
//! ```text
//! (-) -- Vsrc -- nodeA -- R0 -- nodeB -- (R1 parallel C1) -- (+)
//! ```

use super::element::*;
use super::geometry::*;
use super::registry::{ElementDescriptor, Registry};
use super::units::*;
use crate::format::xml::{XmlNode, XmlOut};
use crate::impl_element_base;
use crate::render::painter::Painter;
use crate::solver::sim::dc_analysis;
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Family name used for the feature switch.
pub const FAMILY: &str = "sources";

const FLAG_SHOW_VOLTAGE: i32 = 1;
const FLAG_SHOW_SOC: i32 = 2;

const BT_ALKALINE: i32 = 0;
const BT_LITHIUM: i32 = 1;
const BT_CUSTOM: i32 = -1;
const TYPE_NAMES: [&str; 5] =
    ["Alkaline 1.5V", "Lithium-Ion", "NiMH 1.2V", "NiCd 1.2V", "Lead-Acid"];
const TYPE_TABLES: [&str; 5] = [
    "0=0.8\n10=0.95\n20=1.05\n40=1.18\n60=1.28\n80=1.38\n90=1.43\n100=1.55\n",
    "0=3.00\n5=3.30\n10=3.45\n20=3.55\n30=3.62\n40=3.68\n50=3.73\n60=3.79\n70=3.87\n80=3.97\n90=4.08\n95=4.15\n100=4.20\n",
    "0=1.00\n10=1.15\n20=1.20\n50=1.25\n80=1.30\n90=1.33\n100=1.40\n",
    "0=1.00\n10=1.15\n20=1.20\n50=1.22\n80=1.25\n90=1.28\n100=1.35\n",
    "0=1.75\n10=1.90\n20=1.95\n50=2.05\n80=2.10\n90=2.12\n100=2.15\n",
];
/// Capacity (Ah), R0, R1 and C1 applied when the user switches to a preset type.
const TYPE_DEFAULTS: [[f64; 4]; 5] = [
    [2.5, 0.15, 0.25, 1500.0],
    [3.0, 0.025, 0.020, 2000.0],
    [2.0, 0.030, 0.040, 1800.0],
    [1.0, 0.020, 0.025, 1200.0],
    [10.0, 0.008, 0.012, 5000.0],
];

/// Registers the battery.
pub fn register(r: &mut Registry) {
    r.add(ElementDescriptor::new(
        "BatteryElm",
        FAMILY,
        "Add Battery",
        "Inputs and Sources",
        |x, y| Box::new(Battery::new(x, y)),
    ));
}

/// A battery with state of charge.
pub struct Battery {
    base: Base,
    battery_type: i32,
    r0: f64,
    r1: f64,
    c1: f64,
    capacity_ah: f64,
    /// 0 to 1, used when resetting.
    initial_soc: f64,
    /// 0 to 1.
    soc: f64,
    comp_resistance: f64,
    cap_volt_diff: f64,
    cap_current: f64,
    cur_source_value: f64,
    soc_table_text: String,
    /// Pairs of (state of charge in percent, volts), sorted.
    soc_table: Vec<(f64, f64)>,
    vs: Option<VsId>,
}

fn clamp_soc(s: f64) -> f64 {
    s.clamp(0.0, 1.0)
}

impl Battery {
    /// A battery being placed.
    pub fn new(x: i32, y: i32) -> Self {
        let mut base = Base::new(x, y);
        base.flags |= FLAG_SHOW_VOLTAGE | FLAG_SHOW_SOC;
        let mut b = Battery {
            base,
            battery_type: BT_LITHIUM,
            r0: 0.01,
            r1: 0.02,
            c1: 2000.0,
            capacity_ah: 2.0,
            initial_soc: 1.0,
            soc: 1.0,
            comp_resistance: 0.0,
            cap_volt_diff: 0.0,
            cap_current: 0.0,
            cur_source_value: 0.0,
            soc_table_text: TYPE_TABLES[BT_LITHIUM as usize].into(),
            soc_table: Vec::new(),
            vs: None,
        };
        b.parse_soc_table(None);
        b
    }

    /// Reads the `soc = volts` lines; mistakes are reported through `error` when it is given.
    fn parse_soc_table(&mut self, mut error: Option<&mut Option<String>>) {
        let mut table = Vec::new();
        for line in self.soc_table_text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some((a, b)) = line.split_once('=') else {
                if let Some(e) = error.as_deref_mut() {
                    *e = Some(format!("missing =: {line}"));
                }
                continue;
            };
            match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
                (Ok(pct), Ok(v)) => table.push((pct, v)),
                _ => {
                    if let Some(e) = error.as_deref_mut() {
                        *e = Some(format!("bad line: {line}"));
                    }
                }
            }
        }
        table.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.soc_table = table;
    }

    fn interp_table(&self, pct: f64) -> f64 {
        let t = &self.soc_table;
        match t.len() {
            0 => return 3.7,
            1 => return t[0].1,
            _ => {}
        }
        if pct <= t[0].0 {
            return t[0].1;
        }
        let last = t[t.len() - 1];
        if pct >= last.0 {
            return last.1;
        }
        for w in t.windows(2) {
            let (a, b) = (w[0], w[1]);
            if pct >= a.0 && pct <= b.0 {
                if b.0 == a.0 {
                    return a.1;
                }
                return a.1 + (pct - a.0) / (b.0 - a.0) * (b.1 - a.1);
            }
        }
        last.1
    }

    /// Open-circuit voltage at a state of charge (below 0% the table is extrapolated).
    fn voltage_for_soc(&self, soc: f64) -> f64 {
        let pct = soc * 100.0;
        if pct < 0.0 {
            let v0 = self.interp_table(0.0);
            let v10 = self.interp_table(10.0);
            let slope = (v10 - v0) / 10.0;
            return v0 + slope * 3.0 * pct;
        }
        self.interp_table(pct)
    }

    fn soc_text(&self) -> String {
        format!("{}%", (self.soc * 100.0).round())
    }

    fn type_name(&self) -> &'static str {
        usize::try_from(self.battery_type)
            .ok()
            .and_then(|i| TYPE_NAMES.get(i))
            .copied()
            .unwrap_or("Custom")
    }

    /// Picks the table of a type, falling back to lithium-ion.
    fn load_type_table(&mut self) {
        self.soc_table_text = usize::try_from(self.battery_type)
            .ok()
            .and_then(|i| TYPE_TABLES.get(i))
            .copied()
            .unwrap_or(TYPE_TABLES[BT_LITHIUM as usize])
            .into();
    }
}

impl Element for Battery {
    impl_element_base!("BatteryElm");

    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.double_attr("r0", self.r0);
        out.double_attr("r1", self.r1);
        out.double_attr("c1", self.c1);
        out.double_attr("cap", self.capacity_ah);
        out.double_attr("isoc", self.initial_soc);
        out.int_attr("bt", self.battery_type);
        if self.battery_type == BT_CUSTOM && !self.soc_table_text.is_empty() {
            out.text = Some(self.soc_table_text.clone());
        }
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        out.double_attr("soc", self.soc);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.r0 = xml.double_attr("r0", 0.01);
        self.r1 = xml.double_attr("r1", 0.02);
        self.c1 = xml.double_attr("c1", 2000.0);
        self.capacity_ah = xml.double_attr("cap", 2.0);
        self.initial_soc = clamp_soc(xml.double_attr("isoc", 1.0));
        // over-discharge is modelled, so the state itself is only capped at 100%
        self.soc = xml.double_attr("soc", self.initial_soc).min(1.0);
        self.battery_type = xml.int_attr("bt", BT_LITHIUM);
        if self.battery_type == BT_CUSTOM {
            self.soc_table_text = xml.text.clone();
            if self.soc_table_text.is_empty() {
                self.soc_table_text = TYPE_TABLES[BT_LITHIUM as usize].into();
            }
        } else {
            self.load_type_table();
        }
        self.parse_soc_table(None);
    }

    fn voltage_source_count(&self) -> usize {
        1
    }
    fn internal_node_count(&self) -> usize {
        2
    }
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.vs = Some(v);
        self.base.volt_source = Some(v);
    }
    fn voltage_source_nodes(&self) -> Option<(NodeId, NodeId)> {
        let n = &self.base.nodes;
        Some((n[0], n[2]))
    }
    fn non_linear(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.soc = clamp_soc(self.initial_soc);
        self.cap_volt_diff = 0.0;
        self.cap_current = 0.0;
        self.cur_source_value = 0.0;
        self.base.curcount = 0.0;
        self.base.current = 0.0;
    }

    fn stamp(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        if let Some(vs) = self.vs {
            s.stamp_voltage_source_var(n[0], n[2], vs);
        }
        s.stamp_resistor(n[2], n[3], self.r0);
        s.stamp_resistor(n[3], n[1], self.r1);
        self.comp_resistance = if dc_analysis() {
            // finding the operating point: the capacitor becomes a 100M resistor
            1e8
        } else {
            // trapezoidal companion model between nodes 3 and 1
            s.time_step / (2.0 * self.c1)
        };
        s.stamp_resistor(n[3], n[1], self.comp_resistance);
        s.stamp_right_side_var(n[3]);
        s.stamp_right_side_var(n[1]);
    }

    fn start_iteration(&mut self, _s: &Stamper) {
        self.cur_source_value = if dc_analysis() {
            0.0
        } else {
            -self.cap_volt_diff / self.comp_resistance - self.cap_current
        };
    }

    fn do_step(&mut self, s: &mut Stamper) {
        let n = self.base.nodes.clone();
        if let Some(vs) = self.vs {
            s.update_voltage_source(vs, self.voltage_for_soc(self.soc));
        }
        s.stamp_current_source(n[3], n[1], self.cur_source_value);
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        self.cap_volt_diff = self.base.volts[3] - self.base.volts[1];
        if self.comp_resistance > 0.0 {
            self.cap_current = self.cap_volt_diff / self.comp_resistance + self.cur_source_value;
        }
        // coulomb counting: the current is positive when the battery discharges
        if self.capacity_ah > 0.0 && !dc_analysis() {
            self.soc -= self.base.current * s.time_step / (3600.0 * self.capacity_ah);
            if self.soc > 1.0 {
                self.soc = 1.0;
            }
        }
    }

    fn visual_state(&self) -> Vec<f64> {
        vec![self.soc]
    }
    fn set_visual_state(&mut self, state: &[f64]) {
        if let Some(&soc) = state.first() {
            self.soc = soc;
        }
    }

    fn set_points(&mut self) {
        self.base.set_points();
        self.base.calc_leads(8);
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        let (x, y, x2, y2) = (self.base.x, self.base.y, self.base.x2, self.base.y2);
        self.base.set_bbox(x, y, x2, y2);
        draw_2_leads(g, &self.base, c);
        let (l1, l2) = (self.base.lead1, self.base.lead2);
        let power = self.get_power();
        set_voltage_color(g, &self.base, c, self.base.volts[0]);
        set_power_color_w(g, &self.base, c, power);
        let (a, b) = interp_point2(l1, l2, 0.0, 10.0);
        draw_thick_line(g, a, b);
        set_voltage_color(g, &self.base, c, self.base.volts[1]);
        set_power_color_w(g, &self.base, c, power);
        let hs = 16;
        let (p1, p2) = (self.base.point1, self.base.point2);
        self.base.set_bbox_pts(p1, p2, f64::from(hs));
        let (a, b) = interp_point2(l1, l2, 1.0, f64::from(hs));
        draw_thick_line(g, a, b);
        if self.base.dx == 0 || self.base.dy == 0 {
            let volts = short_unit_text(self.voltage_for_soc(self.soc), "V");
            let s = match (
                self.base.flags & FLAG_SHOW_VOLTAGE != 0,
                self.base.flags & FLAG_SHOW_SOC != 0,
            ) {
                (true, true) => Some(format!("{volts} {}", self.soc_text())),
                (true, false) => Some(volts),
                (false, true) => Some(self.soc_text()),
                (false, false) => None,
            };
            draw_values(g, &self.base, c, s.as_deref(), f64::from(hs), false, false);
        }
        self.base.update_dot_count(c);
        if !self.base.creating {
            draw_dots(g, c, p1, p2, self.base.curcount);
        }
        draw_posts(g, &self.base, self, c);
    }

    fn get_power(&self) -> f64 {
        -self.get_voltage_diff() * self.base.current
    }
    fn get_voltage_diff(&self) -> f64 {
        self.base.volts[1] - self.base.volts[0]
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(format!(
            "{} ({})",
            crate::locale::ls("battery"),
            crate::locale::ls(self.type_name())
        ));
        arr.push(format!("I = {}", current_text(self.base.current)));
        arr.push(format!("Vd = {}", voltage_text(self.get_voltage_diff())));
        arr.push(format!("SOC = {}", self.soc_text()));
        arr.push(format!("P = {}", unit_text(self.get_power(), "W")));
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => {
                let mut names: Vec<String> = TYPE_NAMES.iter().map(|s| (*s).to_string()).collect();
                names.push("Custom".into());
                let selected = if self.battery_type == BT_CUSTOM {
                    TYPE_NAMES.len()
                } else {
                    self.battery_type as usize
                };
                Some(EditInfo::choice("Battery Type", names, selected))
            }
            1 => Some(EditInfo::number("Capacity (Ah)", self.capacity_ah).set_positive()),
            2 => Some(
                EditInfo::new("Initial State of Charge (%)", self.initial_soc * 100.0, 0.0, 100.0)
                    .set_dimensionless(),
            ),
            3 => Some(EditInfo::number("R0, Ohmic Resistance (ohms)", self.r0).set_positive()),
            4 => {
                Some(EditInfo::number("R1, Polarization Resistance (ohms)", self.r1).set_positive())
            }
            5 => Some(EditInfo::number("C1, Polarization Capacitance (F)", self.c1).set_positive()),
            6 => Some(EditInfo::checkbox("Show Voltage", self.base.flags & FLAG_SHOW_VOLTAGE != 0)),
            7 => Some(EditInfo::checkbox(
                "Show State of Charge",
                self.base.flags & FLAG_SHOW_SOC != 0,
            )),
            8 if self.battery_type == BT_CUSTOM => {
                Some(EditInfo::text_area("SOC(%) = Voltage Table", &self.soc_table_text))
            }
            _ => None,
        }
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 if ei.choice.is_some() => {
                let old = self.battery_type;
                self.battery_type = if ei.choice_index >= TYPE_NAMES.len() {
                    BT_CUSTOM
                } else {
                    ei.choice_index as i32
                };
                if self.battery_type != BT_CUSTOM {
                    self.load_type_table();
                    if self.battery_type != old {
                        let d = TYPE_DEFAULTS[self.battery_type as usize];
                        self.capacity_ah = d[0];
                        self.r0 = d[1];
                        self.r1 = d[2];
                        self.c1 = d[3];
                    }
                } else if old != BT_CUSTOM {
                    self.soc_table_text = usize::try_from(old)
                        .ok()
                        .and_then(|i| TYPE_TABLES.get(i))
                        .copied()
                        .unwrap_or(TYPE_TABLES[BT_ALKALINE as usize])
                        .into();
                }
                self.parse_soc_table(None);
                if self.battery_type != old {
                    ei.new_dialog = true;
                }
            }
            1 => self.capacity_ah = ei.value,
            2 => self.initial_soc = ei.value.clamp(0.0, 100.0) * 0.01,
            3 => self.r0 = ei.value,
            4 => self.r1 = ei.value,
            5 => self.c1 = ei.value,
            6 if ei.checkbox.is_some() => {
                self.base.flags = ei.change_flag(self.base.flags, FLAG_SHOW_VOLTAGE);
            }
            7 if ei.checkbox.is_some() => {
                self.base.flags = ei.change_flag(self.base.flags, FLAG_SHOW_SOC);
            }
            8 => {
                if let Some(t) = &ei.text {
                    self.soc_table_text = t.clone();
                }
                let mut error = None;
                self.parse_soc_table(Some(&mut error));
                ei.error = error;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voltage_follows_the_table_and_extrapolates_below_zero() {
        let mut b = Battery::new(16, 16);
        assert!((b.voltage_for_soc(1.0) - 4.2).abs() < 1e-12);
        assert!((b.voltage_for_soc(0.0) - 3.0).abs() < 1e-12);
        // halfway between 50% (3.73) and 60% (3.79)
        assert!((b.voltage_for_soc(0.55) - 3.76).abs() < 1e-9);
        assert!(b.voltage_for_soc(-0.1) < 3.0);
        b.soc_table_text = "bad".into();
        let mut e = None;
        b.parse_soc_table(Some(&mut e));
        assert_eq!(e.as_deref(), Some("missing =: bad"));
    }
}
