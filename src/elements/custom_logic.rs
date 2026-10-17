// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CustomLogicElm, CustomLogicModel).
//! Custom logic: a chip whose behaviour is a table of rules written by the user.
//!
//! The rules belong to a named model shared by all chips that use it. Each rule is
//! `inputs = outputs`: a digit is a level, `?` ignores the pin, `+` and `-` are edges, a letter
//! remembers a level and uses it again (on the right it copies the remembered level) and `_`
//! on the right leaves an output floating.

use super::chip::*;
use super::element::*;
use super::registry::{ElementDescriptor, Registry};
use crate::format::tokenizer::{Tokenizer, escape, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::solver::stamper::{NodeId, Stamper};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Family name used for the feature switch.
pub const FAMILY: &str = "digital";

/// A named set of rules.
#[derive(Clone, Debug, Default)]
pub struct ClModel {
    /// Name chips refer to.
    pub name: String,
    /// Model flags.
    pub flags: i32,
    /// Input pin names.
    pub inputs: Vec<String>,
    /// Output pin names.
    pub outputs: Vec<String>,
    /// Text shown in the info panel.
    pub info_text: String,
    /// The rules as written.
    pub rules: String,
    rules_left: Vec<String>,
    rules_right: Vec<String>,
    /// True when some rule leaves an output floating.
    pub tri_state: bool,
}

fn split_list(s: &str) -> Vec<String> {
    s.split(',').map(str::to_string).collect()
}

impl ClModel {
    fn blank(name: &str) -> ClModel {
        ClModel {
            name: name.into(),
            inputs: split_list("A,B"),
            outputs: split_list("C,D"),
            info_text: if name == "default" { "custom logic".into() } else { name.into() },
            ..ClModel::default()
        }
    }

    /// Parses the rules text. Returns a message naming the first line that is wrong; the rules
    /// parsed before it are kept, as in the original.
    pub fn parse_rules(&mut self) -> Option<String> {
        self.rules_left.clear();
        self.rules_right.clear();
        self.tri_state = false;
        let text = self.rules.clone();
        for (i, line) in text.split('\n').enumerate() {
            let s = line.to_lowercase();
            let s = s.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let compact: String = s.chars().filter(|c| *c != ' ').collect();
            let parts: Vec<&str> = compact.split('=').collect();
            if parts.len() != 2 {
                return Some(format!("error on line {}", i + 1));
            }
            let (left, right) = (parts[0], parts[1]);
            let (ni, no) = (self.inputs.len(), self.outputs.len());
            if left.len() < ni {
                return Some(format!("must have >= {ni} digits on left side (line {})", i + 1));
            }
            if left.len() > ni + no {
                return Some(format!(
                    "must have <= {} digits on left side (line {})",
                    ni + no,
                    i + 1
                ));
            }
            if right.len() != no {
                return Some(format!("must have {no} digits on right side (line {})", i + 1));
            }
            let mut used = [false; 26];
            let mut new_left = String::new();
            for x in left.chars() {
                if matches!(x, '?' | '+' | '-' | '0' | '1') {
                    new_left.push(x);
                    continue;
                }
                if !x.is_ascii_lowercase() {
                    return Some(format!("error on line {}", i + 1));
                }
                let k = (x as u8 - b'a') as usize;
                // a letter that appears again is capitalised so it compares instead of saving
                if used[k] {
                    new_left.push(x.to_ascii_uppercase());
                } else {
                    used[k] = true;
                    new_left.push(x);
                }
            }
            if right.contains('_') {
                self.tri_state = true;
            }
            self.rules_left.push(new_left);
            self.rules_right.push(right.to_string());
        }
        None
    }

    /// The `clm` tag of the model.
    pub fn to_xml(&self) -> XmlOut {
        let mut o = XmlOut::new("clm");
        o.str_attr("nm", &self.name);
        o.int_attr("f", self.flags);
        o.str_attr("in", &self.inputs.join(","));
        o.str_attr("o", &self.outputs.join(","));
        o.str_attr("if", &self.info_text);
        o.text = Some(self.rules.clone());
        o
    }

    fn apply(&mut self, flags: i32, inputs: &str, outputs: &str, info: &str, rules: &str) {
        self.flags = flags;
        self.inputs = split_list(inputs);
        self.outputs = split_list(outputs);
        self.info_text = info.into();
        self.rules = rules.into();
        let _ = self.parse_rules();
    }
}

type Shared = Rc<RefCell<ClModel>>;

thread_local! {
    static MODELS: RefCell<HashMap<String, Shared>> = RefCell::new(HashMap::new());
    static LAST_MODEL: RefCell<String> = RefCell::new("default".into());
}

/// The model called `name`; one with no rules is made when there is none.
pub fn model_with_name(name: &str) -> Shared {
    MODELS.with(|m| {
        m.borrow_mut()
            .entry(name.to_string())
            .or_insert_with(|| Rc::new(RefCell::new(ClModel::blank(name))))
            .clone()
    })
}

/// Like [`model_with_name`], but a new model starts as a copy of `old`.
fn model_or_copy(name: &str, old: &Shared) -> Shared {
    MODELS.with(|m| {
        m.borrow_mut()
            .entry(name.to_string())
            .or_insert_with(|| {
                let mut c = old.borrow().clone();
                c.name = name.into();
                c.info_text = name.into();
                Rc::new(RefCell::new(c))
            })
            .clone()
    })
}

/// Defines a model from a `clm` tag.
pub fn define_from_xml(node: &XmlNode) -> bool {
    let name = node.string_attr("nm", "");
    if name.is_empty() {
        return false;
    }
    let m = model_with_name(&name);
    m.borrow_mut().apply(
        node.int_attr("f", 0),
        &node.string_attr("in", ""),
        &node.string_attr("o", ""),
        &node.string_attr("if", ""),
        node.text.trim(),
    );
    true
}

/// Defines a model from a `!` line of the text format (the fields after the `!`).
pub fn define_from_text(st: &mut Tokenizer<'_>) -> bool {
    let Some(name) = st.next_token().map(unescape) else { return false };
    let flags = st.next_i32().unwrap_or(0);
    let mut field = || st.next_token().map(unescape).unwrap_or_default();
    let (inputs, outputs, info, rules) = (field(), field(), field(), field());
    model_with_name(&name).borrow_mut().apply(flags, &inputs, &outputs, &info, &rules);
    true
}

/// The logic of a custom logic chip.
#[derive(Clone, Debug)]
pub struct CustomLogic {
    model_name: String,
    model: Shared,
    last_values: Vec<bool>,
    pattern: [bool; 26],
    high_z: Vec<bool>,
}

impl CustomLogic {
    fn new() -> Self {
        let name = LAST_MODEL.with(|l| l.borrow().clone());
        CustomLogic {
            model: model_with_name(&name),
            model_name: name,
            last_values: Vec::new(),
            pattern: [false; 26],
            high_z: Vec::new(),
        }
    }

    fn tri_state(&self) -> bool {
        self.model.borrow().tri_state
    }
    fn input_count(&self) -> usize {
        self.model.borrow().inputs.len()
    }
    fn output_count(&self) -> usize {
        self.model.borrow().outputs.len()
    }
}

impl ChipLogic for CustomLogic {
    fn class_name(&self) -> &'static str {
        "CustomLogicElm"
    }
    fn dump_type(&self) -> i32 {
        208
    }
    fn xml_name(&self) -> Option<&'static str> {
        Some("cl")
    }
    fn chip_name(&self) -> &'static str {
        "custom logic"
    }

    fn setup_pins(&mut self, c: &mut ChipCore) {
        let m = self.model.borrow();
        let (ni, no) = (m.inputs.len(), m.outputs.len());
        c.size_y = ni.max(no).max(1) as i32;
        c.size_x = 2;
        let mut pins = Vec::with_capacity(ni + no);
        for (i, name) in m.inputs.iter().enumerate() {
            let mut p = Pin::new(i as i32, SIDE_W, name);
            p.fix_name();
            pins.push(p);
        }
        for (i, name) in m.outputs.iter().enumerate() {
            let mut p = Pin::output(i as i32, SIDE_E, name);
            p.fix_name();
            pins.push(p);
        }
        c.pins = pins;
        self.last_values = vec![false; ni + no];
        self.high_z = vec![false; ni + no];
    }

    fn internal_nodes(&self) -> usize {
        if self.tri_state() { self.output_count() } else { 0 }
    }
    fn non_linear(&self) -> bool {
        self.tri_state()
    }

    fn execute(&mut self, c: &mut ChipCore, _s: &Stamper) {
        let ni = self.input_count();
        let m = self.model.borrow();
        for (rl, rr) in m.rules_left.iter().zip(&m.rules_right) {
            let mut matched = true;
            for (j, x) in rl.chars().enumerate() {
                let Some(pin) = c.pins.get(j) else {
                    matched = false;
                    break;
                };
                let ok = match x {
                    '0' | '1' => pin.value == (x == '1'),
                    '?' => true,
                    '+' => pin.value && !self.last_values[j],
                    '-' => !pin.value && self.last_values[j],
                    'a'..='z' => {
                        self.pattern[(x as u8 - b'a') as usize] = pin.value;
                        true
                    }
                    'A'..='Z' => self.pattern[(x as u8 - b'A') as usize] == pin.value,
                    _ => true,
                };
                if !ok {
                    matched = false;
                    break;
                }
            }
            if !matched {
                continue;
            }
            for (j, x) in rr.chars().enumerate() {
                let k = j + ni;
                if k >= c.pins.len() {
                    break;
                }
                self.high_z[k] = false;
                match x {
                    'a'..='z' => c.pins[k].value = self.pattern[(x as u8 - b'a') as usize],
                    '_' => self.high_z[k] = true,
                    _ => c.pins[k].value = x == '1',
                }
            }
            break;
        }
        for (j, p) in c.pins.iter().enumerate() {
            self.last_values[j] = p.value;
        }
    }

    fn voltage_source_node(&self, c: &ChipCore, j: usize) -> Option<usize> {
        // a chip with floating outputs drives an internal node behind a switchable resistor
        self.tri_state().then(|| c.pins.len() + j)
    }

    fn stamp(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let tri = self.tri_state();
        let (ni, posts) = (self.input_count(), c.pins.len());
        for i in ni..posts {
            let node = if tri { c.base.nodes[posts + i - ni] } else { c.base.nodes[i] };
            if let Some(vs) = c.pins[i].volt_source {
                s.stamp_voltage_source_var(NodeId::GROUND, node, vs);
            }
            if tri {
                s.stamp_non_linear(node);
                s.stamp_non_linear(c.base.nodes[i]);
            }
        }
        true
    }

    fn do_step(&mut self, c: &mut ChipCore, s: &mut Stamper) -> bool {
        let tri = self.tri_state();
        let (ni, posts) = (self.input_count(), c.pins.len());
        let hv = c.high_voltage;
        for i in ni..posts {
            let node = if tri { c.base.nodes[posts + i - ni] } else { c.base.nodes[i] };
            if let Some(vs) = c.pins[i].volt_source {
                s.update_voltage_source(vs, if c.pins[i].value { hv } else { 0.0 });
            }
            if tri {
                s.stamp_resistor(node, c.base.nodes[i], if self.high_z[i] { 1e8 } else { 1e-3 });
            }
        }
        true
    }

    fn validate(
        &mut self,
        _c: &mut ChipCore,
        _idx: usize,
        _v: &mut crate::solver::path::Validator<'_>,
    ) -> Option<bool> {
        self.tri_state().then_some(true)
    }

    fn read_extra(&mut self, c: &mut ChipCore, st: &mut Tokenizer<'_>) {
        if let Some(t) = st.next_token() {
            self.model_name = unescape(t);
            self.model = model_with_name(&self.model_name);
        }
        self.setup_pins(c);
        c.base.alloc_nodes(c.pins.len() + self.internal_nodes());
        let threshold = c.threshold();
        for i in 0..c.pins.len() {
            if c.pins[i].output {
                let v = st.next_f64().unwrap_or(0.0);
                c.base.volts[i] = v;
                c.pins[i].value = v > threshold;
            }
        }
    }
    fn write_extra(&self, c: &ChipCore) -> String {
        let mut s = format!(" {}", escape(&self.model_name));
        for (i, p) in c.pins.iter().enumerate() {
            if p.output {
                s += &format!(" {}", crate::format::number::js_number(c.base.volts[i]));
            }
        }
        s
    }
    fn read_xml(&mut self, _c: &mut ChipCore, xml: &XmlNode) {
        if let Some(name) = xml.attr("mo") {
            self.model_name = name.to_string();
            self.model = model_with_name(name);
        }
    }
    fn write_xml(&self, _c: &ChipCore, out: &mut XmlOut) {
        out.str_attr("mo", &self.model_name);
    }
    fn model_xml(&self) -> Option<XmlOut> {
        Some(self.model.borrow().to_xml())
    }

    fn info_extra(&self, _c: &ChipCore, arr: &mut Vec<String>) {
        if let Some(first) = arr.first_mut() {
            first.clone_from(&self.model.borrow().info_text);
        }
    }

    fn chip_edit_info(&self, _c: &ChipCore, n: usize) -> Option<EditInfo> {
        let m = self.model.borrow();
        match n {
            0 => Some(EditInfo::text("Model Name", &self.model_name)),
            1 => Some(EditInfo::text("Inputs", &m.inputs.join(","))),
            2 => Some(EditInfo::text("Outputs", &m.outputs.join(","))),
            3 => Some(EditInfo::text("Info Text", &m.info_text)),
            4 => Some(EditInfo::text_area("Definition", &m.rules)),
            _ => None,
        }
    }
    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, n: usize, ei: &mut EditInfo) {
        let Some(text) = ei.text.clone() else { return };
        match n {
            0 => {
                if text != self.model_name {
                    self.model = model_or_copy(&text, &self.model);
                    LAST_MODEL.with(|l| l.borrow_mut().clone_from(&text));
                    self.model_name = text;
                }
            }
            1..=4 => {
                let mut m = self.model.borrow_mut();
                match n {
                    1 => m.inputs = split_list(&text),
                    2 => m.outputs = split_list(&text),
                    3 => m.info_text = text,
                    _ => m.rules = text,
                }
                if let Some(err) = m.parse_rules() {
                    ei.error = Some(err);
                }
            }
            _ => {}
        }
    }
}

/// Registers the custom logic chip.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "CustomLogicElm",
            FAMILY,
            "Add Custom Logic",
            "Digital Chips",
            |x, y| Box::new(ChipElm::new(CustomLogic::new(), x, y)),
        )
        .dump(208, |a, b, c, d, f, st| {
            Ok(Box::new(ChipElm::from_dump(CustomLogic::new(), a, b, c, d, f, st)))
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_are_validated_and_repeated_letters_compare() {
        let mut m = ClModel::blank("t");
        m.rules = "a?=a0\n# comment\nab = 11\nb+ = 1_".into();
        assert!(m.parse_rules().is_none());
        assert!(m.tri_state);
        assert_eq!(m.rules_left, vec!["a?", "ab", "b+"]);
        m.rules = "aa=11".into();
        assert!(m.parse_rules().is_none());
        assert_eq!(m.rules_left, vec!["aA"]);
        m.rules = "a=1".into();
        assert!(m.parse_rules().unwrap().contains("left side"));
    }

    #[test]
    fn text_models_define_and_xml_round_trips() {
        let mut st = Tokenizer::new("inv 0 A Y inverter 0\\q1\\n1\\q0");
        assert!(define_from_text(&mut st));
        let m = model_with_name("inv");
        assert_eq!(m.borrow().outputs, vec!["Y"]);
        assert_eq!(m.borrow().rules_left.len(), 2);
        let xml = m.borrow().to_xml().to_pretty_string();
        let node = crate::format::xml::parse(&xml).unwrap();
        assert!(define_from_xml(&node));
        assert_eq!(model_with_name("inv").borrow().rules, "0=1\n1=0");
    }
}
