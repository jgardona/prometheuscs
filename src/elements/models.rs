// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (DiodeModel).
//! Device models shared by elements and stored in circuit files.
//!
//! Models live in per-thread libraries: a circuit file defines the models it uses, elements
//! refer to them by name, and an element looks its model up whenever the circuit is analysed.
//! The interface thread and the simulation worker each hold their own copy, filled by reading
//! the same circuit text.

use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use std::cell::RefCell;
use std::collections::HashMap;

/// Electron thermal voltage at SPICE's default temperature of 27 C (300.15 K).
pub const THERMAL_VOLTAGE: f64 = 0.025865;
/// Flag of a diode model made from a forward voltage instead of an emission coefficient.
pub const DIODE_FLAGS_SIMPLE: i32 = 1;

/// A diode model: saturation current, series resistance, emission coefficient and breakdown.
#[derive(Clone, Debug, PartialEq)]
pub struct DiodeModel {
    /// Name elements refer to the model by.
    pub name: String,
    /// Model flags ([`DIODE_FLAGS_SIMPLE`]).
    pub flags: i32,
    /// Saturation current in amperes.
    pub saturation_current: f64,
    /// Series resistance in ohms.
    pub series_resistance: f64,
    /// Emission coefficient.
    pub emission_coefficient: f64,
    /// Reverse breakdown voltage (0 for none).
    pub breakdown_voltage: f64,
    /// Forward current used by the simple model editor.
    pub forward_current: f64,
    /// Forward voltage used by the simple model editor.
    pub forward_voltage: f64,
    /// Short description shown beside the name in lists.
    pub description: Option<&'static str>,
    /// Part of the program; never written to files.
    pub built_in: bool,
    /// Hidden from model lists.
    pub internal: bool,
    /// Made from a bare forward-drop value in an old file.
    pub old_style: bool,
    /// The user may not edit it.
    pub read_only: bool,
    /// Voltage increase that raises the current by a factor of e.
    pub vscale: f64,
    /// `1 / vscale`.
    pub vdcoef: f64,
    /// Forward voltage drop at 1 A.
    pub fwdrop: f64,
}

impl DiodeModel {
    /// A model with the given saturation current, series resistance, emission coefficient,
    /// breakdown voltage and description.
    pub fn new(sc: f64, sr: f64, ec: f64, bv: f64, description: Option<&'static str>) -> Self {
        let mut m = DiodeModel {
            name: String::new(),
            flags: 0,
            saturation_current: sc,
            series_resistance: sr,
            emission_coefficient: ec,
            breakdown_voltage: bv,
            forward_current: 0.0,
            forward_voltage: 0.0,
            description,
            built_in: false,
            internal: false,
            old_style: false,
            read_only: false,
            vscale: 0.0,
            vdcoef: 0.0,
            fwdrop: 0.0,
        };
        m.update();
        m
    }

    /// Recomputes the values derived from the parameters.
    pub fn update(&mut self) {
        self.vscale = self.emission_coefficient * THERMAL_VOLTAGE;
        self.vdcoef = 1.0 / self.vscale;
        self.fwdrop = (1.0 / self.saturation_current + 1.0).ln()
            * self.emission_coefficient
            * THERMAL_VOLTAGE;
    }

    /// True for a model described by forward voltage and current.
    pub fn is_simple(&self) -> bool {
        self.flags & DIODE_FLAGS_SIMPLE != 0
    }

    /// Name and description as shown in lists.
    pub fn description_text(&self) -> String {
        match self.description {
            Some(d) => format!("{} ({})", self.name, crate::locale::ls(d)),
            None => self.name.clone(),
        }
    }

    fn built_in(name: &str, mut m: DiodeModel) -> DiodeModel {
        m.name = name.to_string();
        m.read_only = true;
        m.built_in = true;
        m
    }

    /// Reads a model from the fields of a `34` line (after the type code).
    pub fn undump(st: &mut Tokenizer<'_>) -> Option<DiodeModel> {
        let name = unescape(st.next_token()?);
        let mut m = DiodeModel::lookup(&name).unwrap_or_else(|| {
            let mut m = DiodeModel::new(1e-14, 0.0, 1.0, 0.0, None);
            m.name = name.clone();
            m
        });
        m.flags = st.next_i32().ok()?;
        m.saturation_current = st.next_f64().ok()?;
        m.series_resistance = st.next_f64().ok()?;
        m.emission_coefficient = st.next_f64().ok()?;
        m.breakdown_voltage = st.next_f64().ok()?;
        if let Ok(fc) = st.next_f64() {
            m.forward_current = fc;
        }
        m.update();
        Some(m)
    }

    /// Reads a model from a `dm` tag.
    pub fn undump_xml(xml: &XmlNode) -> Option<DiodeModel> {
        let name = xml.attr("nm")?.to_string();
        let mut m = DiodeModel::lookup(&name).unwrap_or_else(|| {
            let mut m = DiodeModel::new(1e-14, 0.0, 1.0, 0.0, None);
            m.name = name.clone();
            m
        });
        m.flags = xml.int_attr("f", m.flags);
        m.saturation_current = xml.double_attr("is", m.saturation_current);
        m.series_resistance = xml.double_attr("rs", m.series_resistance);
        m.emission_coefficient = xml.double_attr("n", m.emission_coefficient);
        m.breakdown_voltage = xml.double_attr("bv", m.breakdown_voltage);
        m.forward_current = xml.double_attr("fi", m.forward_current);
        m.update();
        Some(m)
    }

    /// The model as the XML tag files carry it.
    pub fn dump_xml(&self) -> XmlOut {
        let mut o = XmlOut::new("dm");
        o.str_attr("nm", &self.name);
        o.int_attr("f", self.flags);
        o.double_attr("is", self.saturation_current);
        o.double_attr("rs", self.series_resistance);
        o.double_attr("n", self.emission_coefficient);
        o.double_attr("bv", self.breakdown_voltage);
        if self.forward_current > 0.0 {
            o.double_attr("fi", self.forward_current);
        }
        o
    }

    /// The model as a text line.
    pub fn dump(&self) -> String {
        use crate::format::number::js_number;
        format!(
            "34 {} {} {} {} {} {} {}",
            escape(&self.name),
            self.flags,
            js_number(self.saturation_current),
            js_number(self.series_resistance),
            js_number(self.emission_coefficient),
            js_number(self.breakdown_voltage),
            js_number(self.forward_current)
        )
    }

    /// Sets the emission coefficient from the forward voltage and current of a simple model.
    pub fn set_emission_coefficient(&mut self) {
        if self.forward_current > 0.0 && self.forward_voltage > 0.0 {
            self.emission_coefficient = (self.forward_voltage
                / (self.forward_current / self.saturation_current + 1.0).ln())
                / THERMAL_VOLTAGE;
        }
        self.series_resistance = 0.0;
        self.update();
    }

    /// Sets the forward voltage of a simple model from its current and parameters.
    pub fn set_forward_voltage(&mut self) {
        if self.forward_current == 0.0 {
            self.forward_current = 1.0;
        }
        self.forward_voltage = self.emission_coefficient
            * THERMAL_VOLTAGE
            * (self.forward_current / self.saturation_current + 1.0).ln();
    }

    // ---- the library -----------------------------------------------------------------------

    /// The model called `name`, if the library has it.
    pub fn lookup(name: &str) -> Option<DiodeModel> {
        DIODE_MODELS.with(|l| l.borrow().get(name).cloned())
    }

    /// The default model.
    pub fn default_model() -> DiodeModel {
        Self::lookup("default").unwrap_or_else(|| {
            let mut m = DiodeModel::new(1.714_352_819_280_888_3e-7, 0.0, 2.0, 0.0, None);
            m.name = "default".into();
            m
        })
    }

    /// The model called `name`; when there is none, a copy of `old` under that name, or the
    /// default model when there is no `old` either.
    pub fn named_or_copy(name: &str, old: Option<&DiodeModel>) -> DiodeModel {
        if let Some(m) = Self::lookup(name) {
            return m;
        }
        match old {
            None => Self::default_model(),
            Some(old) => {
                let mut m = old.clone();
                m.name = name.to_string();
                m.built_in = false;
                m.read_only = false;
                m.internal = false;
                m.update();
                Self::define(m.clone());
                m
            }
        }
    }

    /// Adds or replaces a model in the library.
    pub fn define(m: DiodeModel) {
        DIODE_MODELS.with(|l| {
            l.borrow_mut().insert(m.name.clone(), m);
        });
    }

    /// The model for an old file that stores only a forward drop and a breakdown voltage; made
    /// on first use and found again by its name.
    pub fn with_parameters(fwdrop: f64, zvoltage: f64) -> DiodeModel {
        const EMCOEF: f64 = 2.0;
        let existing = DIODE_MODELS.with(|l| {
            l.borrow()
                .values()
                .find(|m| {
                    (m.fwdrop - fwdrop).abs() < 1e-8
                        && m.series_resistance == 0.0
                        && (m.breakdown_voltage - zvoltage).abs() < 1e-8
                        && m.emission_coefficient == EMCOEF
                })
                .cloned()
        });
        if let Some(m) = existing {
            return m;
        }
        let vscale = EMCOEF * THERMAL_VOLTAGE;
        let vdcoef = 1.0 / vscale;
        let leakage = 1.0 / ((fwdrop * vdcoef).exp() - 1.0);
        let mut name = format!("fwdrop={}", crate::format::number::js_number(fwdrop));
        if zvoltage != 0.0 {
            name = format!("{name} zvoltage={}", crate::format::number::js_number(zvoltage));
        }
        let mut m = DiodeModel::new(leakage, 0.0, EMCOEF, zvoltage, None);
        m.name = name;
        m.read_only = true;
        m.old_style = true;
        Self::define(m.clone());
        m
    }

    /// Models to offer in a list, sorted by name; only models with a breakdown voltage when
    /// `zener` is set.
    pub fn list(zener: bool) -> Vec<DiodeModel> {
        let mut v: Vec<DiodeModel> = DIODE_MODELS.with(|l| {
            l.borrow()
                .values()
                .filter(|m| !m.internal && !(zener && m.breakdown_voltage == 0.0))
                .cloned()
                .collect()
        });
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
}

fn builtin_diode_models() -> HashMap<String, DiodeModel> {
    let b = DiodeModel::built_in;
    let mut v: Vec<DiodeModel> = vec![
        b("spice-default", DiodeModel::new(1e-14, 0.0, 1.0, 0.0, None)),
        b("default", DiodeModel::new(1.714_352_819_280_888_3e-7, 0.0, 2.0, 0.0, None)),
        b("default-zener", DiodeModel::new(1.714_352_819_280_888_3e-7, 0.0, 2.0, 5.6, None)),
        internal(b(
            "old-default-led",
            DiodeModel::new(2.234_990_700_667_192_7e-18, 0.0, 2.0, 0.0, None),
        )),
        b("default-led", DiodeModel::new(93.2e-12, 0.042, 3.73, 0.0, None)),
        b("default-optocoupler-led", DiodeModel::new(1.714e-7, 0.0, 4.077, 0.0, None)),
        b("1N5711", DiodeModel::new(315e-9, 2.8, 2.03, 70.0, Some("Schottky"))),
        b("1N5712", DiodeModel::new(680e-12, 12.0, 1.003, 20.0, Some("Schottky"))),
        b("BAT85", DiodeModel::new(2.076e-7, 2.326, 1.023, 33.0, Some("Schottky"))),
        internal(b("1N34", DiodeModel::new(200e-12, 84e-3, 2.19, 60.0, Some("germanium")))),
        b("1N4004", DiodeModel::new(18.8e-9, 28.6e-3, 2.0, 400.0, Some("general purpose"))),
        b("1N4148", DiodeModel::new(4.352e-9, 0.6458, 1.906, 75.0, Some("switching"))),
        internal(b("x2n2646-emitter", DiodeModel::new(2.13e-11, 0.0, 1.8, 0.0, None))),
        internal(b("~tl431ed-d_ed", DiodeModel::new(1e-14, 0.0, 1.0, 5.0, None))),
        internal(b("~lm317-dz", DiodeModel::new(1e-14, 0.0, 1.0, 0.0, None))),
    ];
    // the two internal models of TL431 and LM317 carry a breakdown voltage instead
    for m in &mut v {
        if m.name == "~lm317-dz" {
            m.breakdown_voltage = 6.3;
            m.update();
        }
    }
    v.into_iter().map(|m| (m.name.clone(), m)).collect()
}

fn internal(mut m: DiodeModel) -> DiodeModel {
    m.internal = true;
    m
}

thread_local! {
    static DIODE_MODELS: RefCell<HashMap<String, DiodeModel>> = RefCell::new(builtin_diode_models());
}

// ---------------------------------------------------------------------------------------------
// Transistor models
// ---------------------------------------------------------------------------------------------

/// A Gummel-Poon bipolar transistor model.
#[derive(Clone, Debug, PartialEq)]
pub struct TransistorModel {
    /// Name elements refer to the model by.
    pub name: String,
    /// Short description shown beside the name in lists.
    pub description: Option<String>,
    /// Model flags.
    pub flags: i32,
    /// Transport saturation current (IS).
    pub sat_cur: f64,
    /// Inverse forward knee current (1 / IKF).
    pub inv_roll_off_f: f64,
    /// Base-emitter leakage saturation current (ISE).
    pub be_leak_cur: f64,
    /// Base-emitter leakage emission coefficient (NE).
    pub leak_be_emission_coeff: f64,
    /// Inverse reverse knee current (1 / IKR).
    pub inv_roll_off_r: f64,
    /// Base-collector leakage saturation current (ISC).
    pub bc_leak_cur: f64,
    /// Base-collector leakage emission coefficient (NC).
    pub leak_bc_emission_coeff: f64,
    /// Forward emission coefficient (NF).
    pub emission_coeff_f: f64,
    /// Reverse emission coefficient (NR).
    pub emission_coeff_r: f64,
    /// Inverse forward Early voltage (1 / VAF).
    pub inv_early_volt_f: f64,
    /// Inverse reverse Early voltage (1 / VAR).
    pub inv_early_volt_r: f64,
    /// Reverse beta (BR).
    pub beta_r: f64,
    /// Zero-bias base-emitter junction capacitance (CJE).
    pub junction_cap_be: f64,
    /// Zero-bias base-collector junction capacitance (CJC).
    pub junction_cap_bc: f64,
    /// Base-emitter built-in potential (VJE).
    pub junction_pot_be: f64,
    /// Base-collector built-in potential (VJC).
    pub junction_pot_bc: f64,
    /// Base-emitter grading coefficient (MJE).
    pub junction_exp_be: f64,
    /// Base-collector grading coefficient (MJC).
    pub junction_exp_bc: f64,
    /// Forward transit time (TF).
    pub transit_time_f: f64,
    /// Reverse transit time (TR).
    pub transit_time_r: f64,
    /// Part of the program; never written to files.
    pub built_in: bool,
    /// Hidden from model lists.
    pub internal: bool,
    /// The user may not edit it.
    pub read_only: bool,
}

impl TransistorModel {
    /// A model with all SPICE parameters at the values of an unconfigured model.
    fn blank() -> Self {
        TransistorModel {
            name: String::new(),
            description: None,
            flags: 0,
            sat_cur: 0.0,
            inv_roll_off_f: 0.0,
            be_leak_cur: 0.0,
            leak_be_emission_coeff: 0.0,
            inv_roll_off_r: 0.0,
            bc_leak_cur: 0.0,
            leak_bc_emission_coeff: 0.0,
            emission_coeff_f: 0.0,
            emission_coeff_r: 0.0,
            inv_early_volt_f: 0.0,
            inv_early_volt_r: 0.0,
            beta_r: 0.0,
            junction_cap_be: 0.0,
            junction_cap_bc: 0.0,
            junction_pot_be: 0.75,
            junction_pot_bc: 0.75,
            junction_exp_be: 0.33,
            junction_exp_bc: 0.33,
            transit_time_f: 0.0,
            transit_time_r: 0.0,
            built_in: false,
            internal: false,
            read_only: false,
        }
    }

    /// A model described by `description` with saturation current `sc` and the usual defaults.
    pub fn new(description: &str, sc: f64) -> Self {
        TransistorModel {
            description: Some(description.to_string()),
            sat_cur: sc,
            emission_coeff_f: 1.0,
            emission_coeff_r: 1.0,
            leak_be_emission_coeff: 1.5,
            leak_bc_emission_coeff: 2.0,
            beta_r: 1.0,
            ..Self::blank()
        }
    }

    /// Name and description as shown in lists.
    pub fn description_text(&self) -> String {
        match &self.description {
            Some(d) if d != &self.name => format!("{} ({})", self.name, crate::locale::ls(d)),
            _ => self.name.clone(),
        }
    }

    fn undump_fields(&mut self, st: &mut Tokenizer<'_>) -> Option<()> {
        self.flags = st.next_i32().ok()?;
        self.sat_cur = st.next_f64().ok()?;
        self.inv_roll_off_f = st.next_f64().ok()?;
        self.be_leak_cur = st.next_f64().ok()?;
        self.leak_be_emission_coeff = st.next_f64().ok()?;
        self.inv_roll_off_r = st.next_f64().ok()?;
        self.bc_leak_cur = st.next_f64().ok()?;
        self.leak_bc_emission_coeff = st.next_f64().ok()?;
        self.emission_coeff_f = st.next_f64().ok()?;
        self.emission_coeff_r = st.next_f64().ok()?;
        self.inv_early_volt_f = st.next_f64().ok()?;
        self.inv_early_volt_r = st.next_f64().ok()?;
        self.beta_r = st.next_f64().ok()?;
        // the capacitance parameters and the transit times are optional in older files
        if let (Ok(a), Ok(b), Ok(c), Ok(d), Ok(e), Ok(f)) = (
            st.next_f64(),
            st.next_f64(),
            st.next_f64(),
            st.next_f64(),
            st.next_f64(),
            st.next_f64(),
        ) {
            self.junction_cap_be = a;
            self.junction_pot_be = b;
            self.junction_exp_be = c;
            self.junction_cap_bc = d;
            self.junction_pot_bc = e;
            self.junction_exp_bc = f;
        }
        if let (Ok(tf), Ok(tr)) = (st.next_f64(), st.next_f64()) {
            self.transit_time_f = tf;
            self.transit_time_r = tr;
        }
        Some(())
    }

    /// Reads a model from the fields of a `32` line (after the type code).
    pub fn undump(st: &mut Tokenizer<'_>) -> Option<TransistorModel> {
        let name = unescape(st.next_token()?);
        let mut m = TransistorModel::lookup(&name).unwrap_or_else(|| {
            let mut m = TransistorModel::blank();
            m.name = name.clone();
            m
        });
        m.undump_fields(st)?;
        Some(m)
    }

    /// Reads a model from a `tm` tag.
    pub fn undump_xml(xml: &XmlNode) -> Option<TransistorModel> {
        let name = xml.attr("nm")?.to_string();
        let mut m = TransistorModel::lookup(&name).unwrap_or_else(|| {
            let mut m = TransistorModel::blank();
            m.name = name.clone();
            m
        });
        let d = |n: &str, v: f64| xml.double_attr(n, v);
        m.flags = xml.int_attr("f", m.flags);
        m.sat_cur = d("is", m.sat_cur);
        m.inv_roll_off_f = d("ikf", m.inv_roll_off_f);
        m.be_leak_cur = d("ise", m.be_leak_cur);
        m.leak_be_emission_coeff = d("ne", m.leak_be_emission_coeff);
        m.inv_roll_off_r = d("ikr", m.inv_roll_off_r);
        m.bc_leak_cur = d("isc", m.bc_leak_cur);
        m.leak_bc_emission_coeff = d("nc", m.leak_bc_emission_coeff);
        m.emission_coeff_f = d("nf", m.emission_coeff_f);
        m.emission_coeff_r = d("nr", m.emission_coeff_r);
        m.inv_early_volt_f = d("vaf", m.inv_early_volt_f);
        m.inv_early_volt_r = d("var", m.inv_early_volt_r);
        m.beta_r = d("br", m.beta_r);
        m.junction_cap_be = d("cje", m.junction_cap_be);
        m.junction_pot_be = d("vje", m.junction_pot_be);
        m.junction_exp_be = d("mje", m.junction_exp_be);
        m.junction_cap_bc = d("cjc", m.junction_cap_bc);
        m.junction_pot_bc = d("vjc", m.junction_pot_bc);
        m.junction_exp_bc = d("mjc", m.junction_exp_bc);
        m.transit_time_f = d("tf", m.transit_time_f);
        m.transit_time_r = d("tr", m.transit_time_r);
        Some(m)
    }

    /// The model as the XML tag files carry it.
    pub fn dump_xml(&self) -> XmlOut {
        let mut o = XmlOut::new("tm");
        o.str_attr("nm", &self.name);
        o.int_attr("f", self.flags);
        o.double_attr("is", self.sat_cur);
        o.double_attr("ikf", self.inv_roll_off_f);
        o.double_attr("ise", self.be_leak_cur);
        o.double_attr("ne", self.leak_be_emission_coeff);
        o.double_attr("ikr", self.inv_roll_off_r);
        o.double_attr("isc", self.bc_leak_cur);
        o.double_attr("nc", self.leak_bc_emission_coeff);
        o.double_attr("nf", self.emission_coeff_f);
        o.double_attr("nr", self.emission_coeff_r);
        o.double_attr("vaf", self.inv_early_volt_f);
        o.double_attr("var", self.inv_early_volt_r);
        o.double_attr("br", self.beta_r);
        for (tag, v) in [
            ("cje", self.junction_cap_be),
            ("vje", self.junction_pot_be),
            ("mje", self.junction_exp_be),
            ("cjc", self.junction_cap_bc),
            ("vjc", self.junction_pot_bc),
            ("mjc", self.junction_exp_bc),
        ] {
            // the defaults are not written, keeping files of simple models short
            let default = match tag {
                "vje" | "vjc" => 0.75,
                "mje" | "mjc" => 0.33,
                _ => 0.0,
            };
            if v != default {
                o.double_attr(tag, v);
            }
        }
        if self.transit_time_f != 0.0 {
            o.double_attr("tf", self.transit_time_f);
        }
        if self.transit_time_r != 0.0 {
            o.double_attr("tr", self.transit_time_r);
        }
        o
    }

    /// The model called `name`, if the library has it.
    pub fn lookup(name: &str) -> Option<TransistorModel> {
        TRANSISTOR_MODELS.with(|l| l.borrow().get(name).cloned())
    }

    /// The default model.
    pub fn default_model() -> TransistorModel {
        Self::lookup("default").unwrap_or_else(|| {
            let mut m = TransistorModel::new("default", 1e-13);
            m.name = "default".into();
            m
        })
    }

    /// The model called `name`; when there is none, a copy of `old` under that name, or the
    /// default model when there is no `old` either.
    pub fn named_or_copy(name: &str, old: Option<&TransistorModel>) -> TransistorModel {
        if let Some(m) = Self::lookup(name) {
            return m;
        }
        match old {
            None => Self::default_model(),
            Some(old) => {
                let mut m = old.clone();
                m.name = name.to_string();
                m.built_in = false;
                m.read_only = false;
                m.internal = false;
                Self::define(m.clone());
                m
            }
        }
    }

    /// Adds or replaces a model in the library.
    pub fn define(m: TransistorModel) {
        TRANSISTOR_MODELS.with(|l| {
            l.borrow_mut().insert(m.name.clone(), m);
        });
    }

    /// Models to offer in a list, sorted by name.
    pub fn list() -> Vec<TransistorModel> {
        let mut v: Vec<TransistorModel> = TRANSISTOR_MODELS
            .with(|l| l.borrow().values().filter(|m| !m.internal).cloned().collect());
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
}

fn builtin_transistor_models() -> HashMap<String, TransistorModel> {
    let mut map = HashMap::new();
    for (name, sc) in [("default", 1e-13), ("spice-default", 1e-16)] {
        let mut m = TransistorModel::new(name, sc);
        m.name = name.to_string();
        m.read_only = true;
        m.built_in = true;
        map.insert(name.to_string(), m);
    }
    // models used inside the built-in LM324, TL431 and LM317 subcircuits
    for line in [
        "xlm324v2-qpi 0 1.01e-16 333.3333333333333 0 1.5 0 0 2 1 1 0.0034482758620689655 0 1",
        "xlm324v2-qpa 0 1.01e-16 333.3333333333333 0 1.5 0 0 2 1 1 0.004081632653061225 0 1",
        "xlm324v2-qnq 0 1e-16 200 0 1.5 0 0 2 1 1 0 0 1",
        "xlm324v2-qpq 0 1e-16 333.3333333333333 0 1.5 0 0 2 1 1 0 0 1",
        "~tl431ed-qn_ed 0 1e-16 0 0 1.5 0 0 2 1 1 0.0125 0.02 1",
        "~tl431ed-qn_ed-A1.2 0 1.2e-16 0 0 1.5 0 0 2 1 1 0.0125 0.02 1",
        "~tl431ed-qn_ed-A2.2 0 2.2000000000000002e-16 0 0 1.5 0 0 2 1 1 0.0125 0.02 1",
        "~tl431ed-qn_ed-A0.5 0 5e-17 0 0 1.5 0 0 2 1 1 0.0125 0.02 1",
        "~tl431ed-qp_ed 0 1e-16 0 0 1.5 0 0 2 1 1 0.014285714285714285 0.025 1",
        "~tl431ed-qn_ed-A5 0 5e-16 0 0 1.5 0 0 2 1 1 0.0125 0.02 1",
        "~lm317-qpl-A0.1 0 1e-17 0 0 1.5 0 0 2 1 1 0.02 0 1",
        "~lm317-qnl-A0.2 0 2e-17 0 0 1.5 0 0 2 1 1 0.01 0 1",
        "~lm317-qpl-A0.2 0 2e-17 0 0 1.5 0 0 2 1 1 0.02 0 1",
        "~lm317-qnl-A2 0 2e-16 0 0 1.5 0 0 2 1 1 0.01 0 1",
        "~lm317-qpl-A2 0 2e-16 0 0 1.5 0 0 2 1 1 0.02 0 1",
        "~lm317-qnl-A5 0 5e-16 0 0 1.5 0 0 2 1 1 0.01 0 1",
        "~lm317-qnl-A50 0 5e-15 0 0 1.5 0 0 2 1 1 0.01 0 1",
    ] {
        let mut st = Tokenizer::new(line);
        let Some(name) = st.next_token() else { continue };
        let mut m = TransistorModel::blank();
        m.name = unescape(name);
        if m.undump_fields(&mut st).is_some() {
            m.built_in = true;
            m.internal = true;
            map.insert(m.name.clone(), m);
        }
    }
    map
}

thread_local! {
    static TRANSISTOR_MODELS: RefCell<HashMap<String, TransistorModel>> =
        RefCell::new(builtin_transistor_models());
}

// ---------------------------------------------------------------------------------------------
// MOSFET and JFET models
// ---------------------------------------------------------------------------------------------

/// Flag of a field-effect model that describes a JFET.
pub const MOSFET_FLAG_JFET: i32 = 1;

/// A square-law field-effect transistor model (MOSFET or JFET).
#[derive(Clone, Debug, PartialEq)]
pub struct MosfetModel {
    /// Name elements refer to the model by.
    pub name: String,
    /// Short description shown beside the name in lists.
    pub description: Option<String>,
    /// Model flags ([`MOSFET_FLAG_JFET`]).
    pub flags: i32,
    /// Threshold voltage.
    pub threshold: f64,
    /// Transconductance parameter in A/V^2.
    pub beta: f64,
    /// Channel-length modulation in 1/V (0 for ideal).
    pub lambda: f64,
    /// Gate-source capacitance in farads (0 to disable).
    pub cap_gs: f64,
    /// Gate-drain capacitance in farads (0 to disable).
    pub cap_gd: f64,
    /// Draw the bulk connection.
    pub show_bulk: bool,
    /// Draw a simple digital-logic symbol (only without a bulk connection).
    pub digital_symbol: bool,
    /// Simulate the parasitic body diode (only with a bulk connection).
    pub body_diode: bool,
    /// Expose the body as a fourth terminal (only with a body diode).
    pub body_terminal: bool,
    /// Draw the body diode symbol (only with a body diode).
    pub show_body_diode_symbol: bool,
    /// Part of the program; never written to files.
    pub built_in: bool,
    /// The user may not edit it.
    pub read_only: bool,
    /// Hidden from model lists.
    pub internal: bool,
    /// Made from the bare threshold and beta of an old file.
    pub old_style: bool,
}

impl MosfetModel {
    fn plain() -> Self {
        MosfetModel {
            name: String::new(),
            description: None,
            flags: 0,
            threshold: 1.5,
            beta: 0.02,
            lambda: 0.0,
            cap_gs: 0.0,
            cap_gd: 0.0,
            show_bulk: true,
            digital_symbol: false,
            body_diode: true,
            body_terminal: false,
            show_body_diode_symbol: false,
            built_in: false,
            read_only: false,
            internal: false,
            old_style: false,
        }
    }

    /// True for a JFET model.
    pub fn is_jfet(&self) -> bool {
        self.flags & MOSFET_FLAG_JFET != 0
    }

    /// Name and description as shown in lists.
    pub fn description_text(&self) -> String {
        match &self.description {
            Some(d) if d != &self.name => format!("{} ({})", self.name, crate::locale::ls(d)),
            _ => self.name.clone(),
        }
    }

    /// Reads a model from an `mm` tag.
    pub fn undump_xml(xml: &XmlNode) -> Option<MosfetModel> {
        let name = xml.attr("nm")?.to_string();
        let mut m = MosfetModel::lookup(&name).unwrap_or_else(|| {
            let mut m = MosfetModel::plain();
            m.name = name.clone();
            m
        });
        m.flags = xml.int_attr("f", m.flags);
        m.threshold = xml.double_attr("vt", m.threshold);
        m.beta = xml.double_attr("be", m.beta);
        m.lambda = xml.double_attr("la", m.lambda);
        m.cap_gs = xml.double_attr("cgs", m.cap_gs);
        m.cap_gd = xml.double_attr("cgd", m.cap_gd);
        m.show_bulk = xml.int_attr("sb", i32::from(m.show_bulk)) != 0;
        m.digital_symbol = xml.int_attr("dsy", i32::from(m.digital_symbol)) != 0;
        m.body_diode = xml.int_attr("bd", i32::from(m.body_diode)) != 0;
        m.body_terminal = xml.int_attr("bt", i32::from(m.body_terminal)) != 0;
        m.show_body_diode_symbol = xml.int_attr("sbd", i32::from(m.show_body_diode_symbol)) != 0;
        Some(m)
    }

    /// The model as the XML tag files carry it.
    pub fn dump_xml(&self) -> XmlOut {
        let mut o = XmlOut::new("mm");
        o.str_attr("nm", &self.name);
        o.int_attr("f", self.flags);
        o.double_attr("vt", self.threshold);
        o.double_attr("be", self.beta);
        if self.lambda != 0.0 {
            o.double_attr("la", self.lambda);
        }
        if self.cap_gs != 0.0 {
            o.double_attr("cgs", self.cap_gs);
        }
        if self.cap_gd != 0.0 {
            o.double_attr("cgd", self.cap_gd);
        }
        o.int_attr("sb", i32::from(self.show_bulk));
        o.int_attr("dsy", i32::from(self.digital_symbol));
        o.int_attr("bd", i32::from(self.body_diode));
        o.int_attr("bt", i32::from(self.body_terminal));
        o.int_attr("sbd", i32::from(self.show_body_diode_symbol));
        o
    }

    /// The model called `name`, if the library has it.
    pub fn lookup(name: &str) -> Option<MosfetModel> {
        MOSFET_MODELS.with(|l| l.borrow().get(name).cloned())
    }

    /// The default model for MOSFETs or JFETs.
    pub fn default_model(jfet: bool) -> MosfetModel {
        let name = if jfet { "default-jfet" } else { "default" };
        Self::lookup(name).unwrap_or_else(|| {
            let mut m = MosfetModel::plain();
            m.name = name.into();
            m
        })
    }

    /// The model called `name`; when there is none, a copy of `old` under that name, or the
    /// default model when there is no `old` either.
    pub fn named_or_copy(name: &str, old: Option<&MosfetModel>, jfet: bool) -> MosfetModel {
        if let Some(m) = Self::lookup(name) {
            return m;
        }
        match old {
            None => Self::default_model(jfet),
            Some(old) => {
                let mut m = old.clone();
                m.name = name.to_string();
                m.built_in = false;
                m.read_only = false;
                m.internal = false;
                m.old_style = false;
                Self::define(m.clone());
                m
            }
        }
    }

    /// Adds or replaces a model in the library.
    pub fn define(m: MosfetModel) {
        MOSFET_MODELS.with(|l| {
            l.borrow_mut().insert(m.name.clone(), m);
        });
    }

    /// The model for an old file that stores only a threshold and a beta (and a few drawing
    /// flags); an existing model with the same parameters is reused.
    pub fn with_parameters(
        vt: f64,
        beta: f64,
        jfet: bool,
        show_bulk: bool,
        body_diode: bool,
        body_terminal: bool,
        digital_symbol: bool,
        show_body_diode_symbol: bool,
    ) -> MosfetModel {
        let existing = MOSFET_MODELS.with(|l| {
            l.borrow()
                .values()
                .find(|m| {
                    m.is_jfet() == jfet
                        && (m.threshold - vt).abs() < 1e-15
                        && (m.beta - beta).abs() < 1e-15
                        && m.lambda == 0.0
                        && m.cap_gs == 0.0
                        && m.cap_gd == 0.0
                        && m.show_bulk == show_bulk
                        && m.body_diode == body_diode
                        && m.body_terminal == body_terminal
                        && m.digital_symbol == digital_symbol
                        && m.show_body_diode_symbol == show_body_diode_symbol
                })
                .cloned()
        });
        if let Some(m) = existing {
            return m;
        }
        let base_name = if jfet { "old-jfet" } else { "old-mosfet" };
        let mut name = base_name.to_string();
        let mut num = 2;
        while Self::lookup(&name).is_some() {
            name = format!("{base_name}-{num}");
            num += 1;
        }
        let mut m = MosfetModel::plain();
        m.name = name;
        m.threshold = vt;
        m.beta = beta;
        m.show_bulk = show_bulk;
        m.body_diode = body_diode;
        m.body_terminal = body_terminal;
        m.digital_symbol = digital_symbol;
        m.show_body_diode_symbol = show_body_diode_symbol;
        if jfet {
            m.flags |= MOSFET_FLAG_JFET;
        }
        m.old_style = true;
        Self::define(m.clone());
        m
    }

    /// Models to offer in a list for MOSFETs or JFETs, sorted by name.
    pub fn list(jfet: bool) -> Vec<MosfetModel> {
        let mut v: Vec<MosfetModel> = MOSFET_MODELS.with(|l| {
            l.borrow().values().filter(|m| !m.internal && m.is_jfet() == jfet).cloned().collect()
        });
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
}

fn builtin_mosfet_models() -> HashMap<String, MosfetModel> {
    let named = |name: &str, vt: f64, beta: f64| {
        let mut m = MosfetModel::plain();
        m.name = name.to_string();
        m.description = Some(name.to_string());
        m.threshold = vt;
        m.beta = beta;
        m.read_only = true;
        m.built_in = true;
        m
    };
    let mut v = vec![named("default", 1.5, 0.02)];
    let mut m = named("default-nodiode", 1.5, 0.02);
    m.body_diode = false;
    v.push(m);
    let mut m = named("default-body", 1.5, 0.02);
    m.body_terminal = true;
    v.push(m);
    let mut m = named("default-digital", 1.5, 0.02);
    m.show_bulk = false;
    m.digital_symbol = true;
    m.body_diode = false;
    v.push(m);
    let mut m = named("default-jfet", -4.0, 0.00125);
    m.flags |= MOSFET_FLAG_JFET;
    m.show_bulk = false;
    m.body_diode = false;
    v.push(m);
    v.into_iter().map(|m| (m.name.clone(), m)).collect()
}

thread_local! {
    static MOSFET_MODELS: RefCell<HashMap<String, MosfetModel>> =
        RefCell::new(builtin_mosfet_models());
}

// ---------------------------------------------------------------------------------------------
// Reading models out of circuit files
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// Relay models
// ---------------------------------------------------------------------------------------------

/// A relay model: coil and contact parameters shared by relays.
#[derive(Clone, Debug, PartialEq)]
pub struct RelayModel {
    /// Name elements refer to the model by.
    pub name: String,
    /// Model flags.
    pub flags: i32,
    /// Coil inductance in henries.
    pub inductance: f64,
    /// Contact resistance when closed.
    pub r_on: f64,
    /// Contact resistance when open.
    pub r_off: f64,
    /// Coil current at which the relay operates.
    pub on_current: f64,
    /// Coil current at which the relay releases.
    pub off_current: f64,
    /// Coil resistance.
    pub coil_r: f64,
    /// Time the armature takes to move (0 selects the original's old model).
    pub switching_time: f64,
    /// 0 coil across both sides, 1 on side 1, 2 on side 2.
    pub coil_style: i32,
    /// Number of poles.
    pub pole_count: i32,
    /// Draw the box around the coil.
    pub show_box: bool,
    /// Connect the open contacts to ground through the off resistance.
    pub pulldown: bool,
    /// Part of the program; never written to files.
    pub built_in: bool,
    /// The user may not edit it.
    pub read_only: bool,
}

impl Default for RelayModel {
    fn default() -> Self {
        RelayModel {
            name: String::new(),
            flags: 0,
            inductance: 0.2,
            r_on: 0.05,
            r_off: 1e6,
            on_current: 0.02,
            off_current: 0.015,
            coil_r: 20.0,
            switching_time: 5e-3,
            coil_style: 0,
            pole_count: 1,
            show_box: true,
            pulldown: true,
            built_in: false,
            read_only: false,
        }
    }
}

fn builtin_relay_models() -> HashMap<String, RelayModel> {
    let mut v = HashMap::new();
    for (name, poles) in [("default", 1), ("default-2-poles", 2), ("default-3-poles", 3)] {
        let m = RelayModel {
            name: name.into(),
            pole_count: poles,
            built_in: true,
            read_only: true,
            ..RelayModel::default()
        };
        v.insert(name.to_string(), m);
    }
    v
}

thread_local! {
    static RELAY_MODELS: RefCell<HashMap<String, RelayModel>> = RefCell::new(builtin_relay_models());
}

impl RelayModel {
    /// The model called `name`, if the library has it.
    pub fn lookup(name: &str) -> Option<RelayModel> {
        RELAY_MODELS.with(|l| l.borrow().get(name).cloned())
    }

    /// The default model.
    pub fn default_model() -> RelayModel {
        Self::lookup("default").unwrap_or_default()
    }

    /// Adds or replaces a model in the library.
    pub fn define(m: RelayModel) {
        RELAY_MODELS.with(|l| {
            l.borrow_mut().insert(m.name.clone(), m);
        });
    }

    /// The model called `name`; when there is none, a copy of `old` under that name, or the
    /// default model when there is no `old` either.
    pub fn named_or_copy(name: &str, old: Option<&RelayModel>) -> RelayModel {
        if let Some(m) = Self::lookup(name) {
            return m;
        }
        match old {
            None => Self::default_model(),
            Some(old) => {
                let mut m = old.clone();
                m.name = name.to_string();
                m.built_in = false;
                m.read_only = false;
                Self::define(m.clone());
                m
            }
        }
    }

    /// The model for an old file that stores the parameters in each relay; made on first use
    /// and found again by its values.
    pub fn with_parameters(
        inductance: f64,
        r_on: f64,
        r_off: f64,
        on_current: f64,
        off_current: f64,
        coil_r: f64,
        switching_time: f64,
        coil_style: i32,
        show_box: bool,
        pulldown: bool,
        pole_count: i32,
    ) -> RelayModel {
        let same = |a: f64, b: f64| (a - b).abs() < 1e-15;
        let existing = RELAY_MODELS.with(|l| {
            let l = l.borrow();
            let mut names: Vec<&String> = l.keys().collect();
            names.sort();
            names
                .into_iter()
                .map(|n| &l[n])
                .find(|m| {
                    same(m.inductance, inductance)
                        && same(m.r_on, r_on)
                        && same(m.r_off, r_off)
                        && same(m.on_current, on_current)
                        && same(m.off_current, off_current)
                        && same(m.coil_r, coil_r)
                        && same(m.switching_time, switching_time)
                        && m.coil_style == coil_style
                        && m.show_box == show_box
                        && m.pulldown == pulldown
                        && m.pole_count == pole_count
                })
                .cloned()
        });
        if let Some(m) = existing {
            return m;
        }
        let name = (1..)
            .map(|n| if n == 1 { "old-relay".to_string() } else { format!("old-relay-{n}") })
            .find(|n| Self::lookup(n).is_none())
            .unwrap_or_else(|| "old-relay".into());
        let m = RelayModel {
            name,
            inductance,
            r_on,
            r_off,
            on_current,
            off_current,
            coil_r,
            switching_time,
            coil_style,
            pole_count,
            show_box,
            pulldown,
            ..RelayModel::default()
        };
        Self::define(m.clone());
        m
    }

    /// Reads a `rlm` tag of the XML format.
    pub fn undump_xml(xml: &XmlNode) -> Option<RelayModel> {
        let name = xml.attr("nm")?.to_string();
        let mut m = Self::lookup(&name)
            .unwrap_or_else(|| RelayModel { name: name.clone(), ..Self::default() });
        m.flags = xml.int_attr("f", m.flags);
        m.inductance = xml.double_attr("in", m.inductance);
        m.r_on = xml.double_attr("ron", m.r_on);
        m.r_off = xml.double_attr("rof", m.r_off);
        m.on_current = xml.double_attr("on", m.on_current);
        m.off_current = xml.double_attr("of", m.off_current);
        m.coil_r = xml.double_attr("coR", m.coil_r);
        m.switching_time = xml.double_attr("sw", m.switching_time);
        m.coil_style = xml.int_attr("cs", m.coil_style);
        m.pole_count = xml.int_attr("po", m.pole_count);
        m.show_box = xml.int_attr("sb", i32::from(m.show_box)) != 0;
        m.pulldown = xml.int_attr("pd", i32::from(m.pulldown)) != 0;
        Some(m)
    }

    /// The model as the XML tag files carry it.
    pub fn dump_xml(&self) -> XmlOut {
        let mut o = XmlOut::new("rlm");
        o.str_attr("nm", &self.name);
        o.int_attr("f", self.flags);
        o.double_attr("in", self.inductance);
        o.double_attr("ron", self.r_on);
        o.double_attr("rof", self.r_off);
        o.double_attr("on", self.on_current);
        o.double_attr("of", self.off_current);
        o.double_attr("coR", self.coil_r);
        o.double_attr("sw", self.switching_time);
        o.int_attr("cs", self.coil_style);
        if self.pole_count != 1 {
            o.int_attr("po", self.pole_count);
        }
        if self.show_box {
            o.int_attr("sb", 1);
        }
        if self.pulldown {
            o.int_attr("pd", 1);
        }
        o
    }

    /// Names of the models in the library, sorted, for model lists.
    pub fn names() -> Vec<String> {
        RELAY_MODELS.with(|l| {
            let mut v: Vec<String> = l.borrow().keys().cloned().collect();
            v.sort();
            v
        })
    }
}

/// Reads a model line of the text format that carries type code `code`. Returns true when a
/// model library took it.
pub fn undump_text_model(code: i32, st: &mut Tokenizer<'_>) -> bool {
    match code {
        34 => DiodeModel::undump(st).map(DiodeModel::define).is_some(),
        32 => TransistorModel::undump(st).map(TransistorModel::define).is_some(),
        _ => false,
    }
}

/// Reads a custom logic model line (`!`) of the text format. Returns true when it was taken.
#[cfg(feature = "digital")]
pub fn undump_text_custom_logic(st: &mut Tokenizer<'_>) -> bool {
    super::custom_logic::define_from_text(st)
}

/// Without the digital family there are no custom logic models to read.
#[cfg(not(feature = "digital"))]
pub fn undump_text_custom_logic(_st: &mut Tokenizer<'_>) -> bool {
    false
}

/// Reads a model tag of the XML format. Returns true when a model library took it.
pub fn undump_xml_model(node: &XmlNode) -> bool {
    match node.tag.as_str() {
        "dm" => DiodeModel::undump_xml(node).map(DiodeModel::define).is_some(),
        "tm" => TransistorModel::undump_xml(node).map(TransistorModel::define).is_some(),
        "mm" => MosfetModel::undump_xml(node).map(MosfetModel::define).is_some(),
        "rlm" => RelayModel::undump_xml(node).map(RelayModel::define).is_some(),
        #[cfg(feature = "subcircuits")]
        "ccm" => super::subcircuit::define_from_xml(node),
        #[cfg(feature = "digital")]
        "clm" => super::custom_logic::define_from_xml(node),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_matches_the_original() {
        let m = DiodeModel::default_model();
        assert_eq!(m.emission_coefficient, 2.0);
        assert!((m.vscale - 2.0 * THERMAL_VOLTAGE).abs() < 1e-15);
        // the default diode drops about 0.8 V at 1 A
        assert!((m.fwdrop - 0.8059).abs() < 1e-3, "{}", m.fwdrop);
    }

    #[test]
    fn model_lines_define_models_and_round_trip() {
        let mut st = Tokenizer::new("my\\sled 0 1e-12 2 3.5 0 0");
        assert!(undump_text_model(34, &mut st));
        let m = DiodeModel::lookup("my led").unwrap();
        assert_eq!(m.emission_coefficient, 3.5);
        let xml = m.dump_xml().to_pretty_string();
        let node = crate::format::xml::parse(&xml).unwrap();
        let back = DiodeModel::undump_xml(&node).unwrap();
        assert_eq!(back, m);
        assert!(m.dump().starts_with("34 my\\sled 0 "));
    }

    #[test]
    fn parameters_of_old_files_make_a_named_model_once() {
        let a = DiodeModel::with_parameters(0.805_904_783, 0.0);
        let b = DiodeModel::with_parameters(0.805_904_783, 0.0);
        assert_eq!(a.name, b.name);
        // that is the default diode's forward drop, so the built-in default is found instead
        assert_eq!(a.name, "default");
        let z = DiodeModel::with_parameters(0.7, 5.1);
        assert!(z.name.contains("zvoltage=5.1"));
    }

    #[test]
    fn named_or_copy_copies_the_old_model_for_a_missing_name() {
        let old = DiodeModel::default_model();
        let m = DiodeModel::named_or_copy("brand-new", Some(&old));
        assert_eq!(m.name, "brand-new");
        assert_eq!(m.saturation_current, old.saturation_current);
        assert!(DiodeModel::lookup("brand-new").is_some());
        assert_eq!(DiodeModel::named_or_copy("missing", None).name, "default");
    }

    #[test]
    fn transistor_models_load_from_lines_and_tags() {
        let d = TransistorModel::default_model();
        assert_eq!(d.sat_cur, 1e-13);
        assert_eq!(d.beta_r, 1.0);
        let mut st = Tokenizer::new("mytr 0 1e-14 0 0 1.5 0 0 2 1 1 0.01 0 2");
        assert!(undump_text_model(32, &mut st));
        let m = TransistorModel::lookup("mytr").unwrap();
        assert_eq!(m.beta_r, 2.0);
        let xml = m.dump_xml().to_pretty_string();
        let back = TransistorModel::undump_xml(&crate::format::xml::parse(&xml).unwrap()).unwrap();
        assert_eq!(back, m);
        assert!(TransistorModel::list().iter().all(|m| !m.internal));
        assert!(TransistorModel::lookup("~lm317-qnl-A5").unwrap().internal);
    }

    #[test]
    fn zener_list_only_offers_models_with_breakdown() {
        assert!(DiodeModel::list(true).iter().all(|m| m.breakdown_voltage > 0.0));
        assert!(DiodeModel::list(false).iter().any(|m| m.name == "1N4148"));
        assert!(DiodeModel::list(false).iter().all(|m| !m.internal));
    }
}
