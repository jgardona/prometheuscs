// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CompositeElm, CustomCompositeElm,
// CustomCompositeModel and CustomCompositeChipElm).
//! Subcircuits: a named model (`ccm` tag) made of ordinary elements, and instances of it (`cc`
//! tag) that show up as one chip.
//!
//! An instance owns private copies of the model's elements. The model numbers its nodes; the
//! external ones become the chip's pins and the others are internal nodes of the instance. The
//! solver sees one element, and the instance forwards node assignment, stamping and results to
//! its children.

use super::chip::*;
use super::composite::{ChildNet, FLAG_ESCAPE, children_from_xml};
use super::element::*;
use super::geometry::Point;
use super::registry::{ElementDescriptor, Registry};
use super::units::voltage_text;
use crate::format::tokenizer::{Tokenizer, unescape};
use crate::format::xml::{XmlNode, XmlOut};
use crate::forward_to_net;
use crate::impl_element_base;
use crate::render::painter::Painter;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

/// Family name used for the feature switch.
pub const FAMILY: &str = "subcircuits";

const FLAG_SMALL_INSTANCE: i32 = 2;
const MODEL_SHOW_LABEL: i32 = 1;
/// How many subcircuits may be built inside one another.
const MAX_NESTING: usize = 16;
/// How many elements one instance may be made of, counting the parts of its parts. A model
/// with several instances of itself would otherwise multiply for ever.
const MAX_PARTS: usize = 50_000;
const DEFAULT_MODEL: &str = r#"<ccm nm="default" f="0" sx="1" sy="1"><ext nm="gnd" nd="1" ps="0" sd="2"/><g nn="1" f="0"/></ccm>"#;

/// One pin of a subcircuit model.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtPin {
    /// Pin label.
    pub name: String,
    /// Node number inside the model.
    pub node: i32,
    /// Position along the side.
    pub pos: i32,
    /// Side of the chip (0 north, 1 south, 2 west, 3 east).
    pub side: i32,
    /// Number of bits when the pin is a bus.
    pub bus_width: i32,
    /// Index of this pin inside its bus.
    pub bus_z: i32,
}

/// A subcircuit definition.
#[derive(Clone, Debug, Default)]
pub struct CompositeModel {
    /// Name instances refer to.
    pub name: String,
    /// Model flags (bit 0: show the name on the chip).
    pub flags: i32,
    /// Chip width in pin spacings.
    pub size_x: i32,
    /// Chip height in pin spacings.
    pub size_y: i32,
    /// Pins, in post order.
    pub ext: Vec<ExtPin>,
    /// The model's elements as XML tags (with a `nn` attribute listing their nodes).
    pub elms: Vec<XmlNode>,
}

fn is_model_tag(tag: &str) -> bool {
    matches!(tag, "dm" | "rlm" | "tm" | "mm" | "clm" | "ccm")
}

impl CompositeModel {
    /// Reads a `ccm` tag. Models embedded in it are defined on the way.
    pub fn from_xml(node: &XmlNode) -> CompositeModel {
        let bcs = node.int_attr("bcs", 0) != 0;
        let mut m = CompositeModel {
            name: node.string_attr("nm", ""),
            flags: node.int_attr("f", 0),
            size_x: node.int_attr("sx", 1),
            size_y: node.int_attr("sy", 1),
            ..CompositeModel::default()
        };
        for child in &node.children {
            if child.tag == "ext" {
                let name = child.string_attr("nm", "");
                let node_no = child.int_attr("nd", 0);
                let pos = child.int_attr("ps", 0);
                let side = child.int_attr("sd", 0).clamp(0, 3);
                let bw = child.int_attr("bw", 1);
                if bcs && bw > 1 {
                    for j in 0..bw {
                        m.ext.push(ExtPin {
                            name: name.clone(),
                            node: node_no + j,
                            pos,
                            side,
                            bus_width: bw,
                            bus_z: j,
                        });
                    }
                } else {
                    m.ext.push(ExtPin {
                        name,
                        node: node_no,
                        pos,
                        side,
                        bus_width: bw,
                        bus_z: child.int_attr("bz", 0),
                    });
                }
            } else {
                if is_model_tag(&child.tag) {
                    super::models::undump_xml_model(child);
                }
                m.elms.push(child.clone());
            }
        }
        m
    }

    fn bus_nodes_consecutive(&self) -> bool {
        self.ext
            .iter()
            .enumerate()
            .all(|(i, e)| e.bus_z <= 0 || (i > 0 && e.node == self.ext[i - 1].node + 1))
    }

    /// The `ccm` tag of the model, with the models its instances need written inside it so a
    /// file carries everything it uses.
    pub fn to_xml(&self) -> XmlOut {
        let mut seen = HashSet::new();
        self.to_xml_inner(&mut seen)
    }

    fn to_xml_inner(&self, seen: &mut HashSet<String>) -> XmlOut {
        seen.insert(self.name.clone());
        let mut o = XmlOut::new("ccm");
        o.str_attr("nm", &self.name);
        o.int_attr("f", self.flags);
        o.int_attr("sx", self.size_x);
        o.int_attr("sy", self.size_y);
        let bcs = self.bus_nodes_consecutive();
        if bcs {
            o.int_attr("bcs", 1);
        }
        for e in &self.ext {
            if bcs && e.bus_z > 0 {
                continue;
            }
            let mut x = XmlOut::new("ext");
            x.str_attr("nm", &e.name);
            x.int_attr("nd", e.node);
            x.int_attr("ps", e.pos);
            x.int_attr("sd", e.side);
            if e.bus_width > 1 {
                x.int_attr("bw", e.bus_width);
                if !bcs {
                    x.int_attr("bz", e.bus_z);
                }
            }
            o.child(x);
        }
        for el in &self.elms {
            if el.tag == "ccm" {
                seen.insert(el.string_attr("nm", ""));
            }
        }
        for el in &self.elms {
            if el.tag != "cc" {
                continue;
            }
            let name = el.string_attr("mo", "");
            if seen.contains(&name) {
                continue;
            }
            if let Some(inner) = lookup(&name) {
                if !inner.is_builtin() {
                    o.child(inner.to_xml_inner(seen));
                }
            }
        }
        for el in &self.elms {
            o.child(el.to_out());
        }
        o
    }

    /// True for the model that is always there.
    pub fn is_builtin(&self) -> bool {
        self.name == "default"
    }

    /// True when the chip shows the model's name.
    pub fn show_label(&self) -> bool {
        self.flags & MODEL_SHOW_LABEL != 0
    }
}

thread_local! {
    static MODELS: RefCell<HashMap<String, CompositeModel>> = RefCell::new(HashMap::new());
    static LAST_MODEL: RefCell<String> = RefCell::new("default".to_string());
    /// How many subcircuits are being built one inside another right now.
    static NESTING: Cell<usize> = const { Cell::new(0) };
    /// How many more elements the subcircuit being built may have, parts of parts included.
    static BUDGET: Cell<usize> = const { Cell::new(0) };
}

/// Defines (or replaces) a model.
pub fn define(model: CompositeModel) {
    MODELS.with(|m| m.borrow_mut().insert(model.name.clone(), model));
}

/// Defines a model from a `ccm` tag.
pub fn define_from_xml(node: &XmlNode) -> bool {
    let model = CompositeModel::from_xml(node);
    if model.name.is_empty() {
        return false;
    }
    define(model);
    true
}

/// Finds a model by name; `default` is always available.
pub fn lookup(name: &str) -> Option<CompositeModel> {
    if let Some(m) = MODELS.with(|m| m.borrow().get(name).cloned()) {
        return Some(m);
    }
    if name == "default" {
        let node = crate::format::xml::parse(DEFAULT_MODEL).ok()?;
        let m = CompositeModel::from_xml(&node);
        define(m.clone());
        return Some(m);
    }
    None
}

/// Forgets a model (the built-in one stays).
pub fn remove(name: &str) {
    if name != "default" {
        MODELS.with(|m| m.borrow_mut().remove(name));
    }
}

/// Makes the next instance placed use model `name`.
pub fn set_last_model(name: &str) {
    LAST_MODEL.with(|l| *l.borrow_mut() = name.to_string());
}

/// Names of all models, sorted.
pub fn model_names() -> Vec<String> {
    let _ = lookup("default");
    MODELS.with(|m| {
        let mut v: Vec<String> = m.borrow().keys().cloned().collect();
        v.sort();
        v
    })
}

/// The chip that draws an instance.
#[derive(Clone, Debug, Default)]
struct SubChip {
    pins: Vec<ExtPin>,
    size_x: i32,
    size_y: i32,
    label: Option<String>,
}

impl ChipLogic for SubChip {
    fn class_name(&self) -> &'static str {
        "CustomCompositeChipElm"
    }
    fn dump_type(&self) -> i32 {
        0
    }
    fn setup_pins(&mut self, c: &mut ChipCore) {
        c.size_x = self.size_x;
        c.size_y = self.size_y;
        c.pins = self
            .pins
            .iter()
            .map(|e| {
                let mut p = Pin::new(e.pos, e.side.clamp(0, 3) as usize, &e.name);
                p.fix_name();
                p.bus_width = e.bus_width;
                p.bus_z = e.bus_z;
                p
            })
            .collect();
    }
    fn is_digital(&self) -> bool {
        false
    }
    fn voltage_source_count(&self, _c: &ChipCore) -> usize {
        0
    }
    fn draw_label(&self, g: &mut dyn Painter, _c: &mut ChipCore, ctx: &DrawCtx, x: i32, y: i32) {
        if let Some(l) = &self.label {
            draw_centered_label(g, ctx, l, x, y, 12);
        }
    }
}

/// An instance of a subcircuit model.
pub struct CustomComposite {
    base: Base,
    model_name: String,
    model: Option<CompositeModel>,
    chip: ChipElm<SubChip>,
    net: ChildNet,
    high_voltage: f64,
}

impl CustomComposite {
    fn empty_chip() -> ChipElm<SubChip> {
        ChipElm::new(SubChip::default(), 0, 0)
    }

    fn with_base(base: Base, name: &str) -> Self {
        let mut c = CustomComposite {
            base,
            model_name: name.to_string(),
            model: None,
            chip: Self::empty_chip(),
            net: ChildNet::default(),
            high_voltage: 0.0,
        };
        c.base.flags |= FLAG_ESCAPE;
        c.base.no_diagonal = true;
        c
    }

    /// An instance being placed. It starts with the last model used; the origin is reserved
    /// for the registry's probe element, which must not build any children.
    pub fn new(x: i32, y: i32) -> Self {
        let mut c = Self::with_base(Base::new(x, y), "default");
        if x != 0 || y != 0 {
            c.model_name = LAST_MODEL.with(|l| l.borrow().clone());
            c.update_models();
        }
        c
    }

    /// An instance read from a text line: the model name follows the common fields.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32, st: &mut Tokenizer<'_>) -> Self {
        let name = st.next_token().map_or_else(|| "default".to_string(), unescape);
        let mut c = Self::with_base(Base::from_dump(xa, ya, xb, yb, f), &name);
        c.update_models();
        c
    }

    fn post_total(&self) -> usize {
        self.model.as_ref().map_or(0, |m| m.ext.len())
    }

    /// Looks the model up again and rebuilds the children.
    pub fn update_models(&mut self) {
        // a model that holds an instance of itself (or of a model that does) would build
        // instances for ever; a few levels are plenty, and the parts have a total allowance
        let depth = NESTING.with(|n| {
            n.set(n.get() + 1);
            n.get()
        });
        if depth == 1 {
            BUDGET.with(|b| b.set(MAX_PARTS));
        }
        self.build_from_model(depth);
        NESTING.with(|n| n.set(n.get() - 1));
    }

    fn build_from_model(&mut self, depth: usize) {
        self.model = lookup(&self.model_name).filter(|m| {
            let affordable = BUDGET.with(|b| match b.get().checked_sub(m.elms.len()) {
                Some(left) => {
                    b.set(left);
                    true
                }
                None => false,
            });
            depth <= MAX_NESTING && affordable
        });
        self.net = ChildNet::default();
        if let Some(m) = &self.model {
            let (children, nodes) = build_children(m);
            self.net.children = children;
            self.net.child_nodes = nodes;
            self.net.ext_nodes = m.ext.iter().map(|e| e.node).collect();
        }
        self.net.set_high_voltage(self.high_voltage);
        self.net.build_links();
        self.alloc_nodes();
        self.set_points();
    }

    fn label(&self) -> Option<String> {
        match &self.model {
            None => Some("?".into()),
            Some(m) if m.show_label() => Some(m.name.clone()),
            Some(_) => None,
        }
    }
}

/// Creates the children of a model.
fn build_children(model: &CompositeModel) -> (Vec<Box<dyn Element>>, Vec<Vec<i32>>) {
    children_from_xml(&model.elms)
}

impl Element for CustomComposite {
    impl_element_base!("CustomCompositeElm");

    fn update_models(&mut self) {
        CustomComposite::update_models(self);
    }
    fn subcircuit_model_name(&self) -> Option<String> {
        self.model.as_ref().filter(|m| !m.is_builtin()).map(|m| m.name.clone())
    }
    forward_to_net!();

    fn dump_type(&self) -> i32 {
        410
    }
    fn xml_dump_type(&self) -> String {
        "cc".into()
    }
    fn dump(&self) -> String {
        format!(
            "{} {}",
            self.base.dump_prefix(410),
            crate::format::tokenizer::escape(&self.model_name)
        )
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.base, out);
        out.str_attr("mo", &self.model_name);
        if self.high_voltage != 0.0 {
            out.double_attr("hv", self.high_voltage);
        }
    }
    fn dump_model_xml(&self) -> Option<XmlOut> {
        self.model.as_ref().filter(|m| !m.is_builtin()).map(CompositeModel::to_xml)
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        self.net.dump_state(out);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        if xml.attr("mo").is_some() || xml.attr("ix").is_none() {
            self.model_name = xml.string_attr("mo", &self.model_name);
            self.high_voltage = xml.double_attr("hv", 0.0);
            self.update_models();
        }
        self.net.undump_state(xml);
    }

    fn post_count(&self) -> usize {
        self.post_total()
    }
    fn post_width(&self, n: usize) -> usize {
        self.chip.post_width(n)
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.chip.get_post(n)
    }

    fn set_points(&mut self) {
        let b = &self.base;
        let mut flags = b.flags;
        let logic = match &self.model {
            Some(m) => SubChip {
                pins: m.ext.clone(),
                size_x: m.size_x,
                size_y: m.size_y,
                label: self.label(),
            },
            None => SubChip { pins: Vec::new(), size_x: 2, size_y: 2, label: self.label() },
        };
        if let Some(m) = &self.model {
            if b.x2 - b.x > m.size_x * 16 && b.creating {
                flags &= !FLAG_SMALL_INSTANCE;
            }
        }
        self.base.flags = flags;
        let mut chip = ChipElm::new(logic, self.base.x, self.base.y);
        let cb = &mut chip.core.base;
        cb.x2 = self.base.x2;
        cb.y2 = self.base.y2;
        cb.flags = flags & (FLAG_FLIP_X | FLAG_FLIP_Y | FLAG_FLIP_XY);
        cb.creating = self.base.creating;
        chip.core.set_size(if flags & FLAG_SMALL_INSTANCE != 0 { 1 } else { 2 });
        chip.set_points();
        self.base.bbox = chip.core.base.bbox;
        self.base.point1 = Point::new(self.base.x, self.base.y);
        self.base.point2 = Point::new(self.base.x2, self.base.y2);
        self.chip = chip;
    }

    fn reset(&mut self) {
        self.base.volts.iter_mut().for_each(|v| *v = 0.0);
        self.net.reset();
    }

    fn draw(&mut self, g: &mut dyn Painter, ctx: &DrawCtx) {
        for i in 0..self.post_total() {
            let v = self.base.volts.get(i).copied().unwrap_or(0.0);
            let cur = self.get_current_into_node(i);
            if let Some(slot) = self.chip.core.base.volts.get_mut(i) {
                *slot = v;
            }
            self.chip.core.pins[i].current = cur;
        }
        self.chip.core.base.selected = self.base.selected;
        self.chip.core.base.highlight = self.base.highlight;
        self.chip.draw(g, ctx);
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        let Some(m) = &self.model else {
            arr.push(format!("subcircuit (missing: {})", self.model_name));
            return;
        };
        arr.push(format!("subcircuit ({})", m.name));
        let threshold = self.chip.core.threshold();
        for (i, e) in m.ext.iter().enumerate() {
            if arr.len() >= 12 {
                break;
            }
            if e.bus_z > 0 {
                continue;
            }
            if e.bus_width > 1 {
                let mut value = 0_i64;
                for j in 0..e.bus_width as usize {
                    if self.base.volts.get(i + j).copied().unwrap_or(0.0) > threshold {
                        value |= 1 << j;
                    }
                }
                arr.push(format!("{} = {value} / 0x{value:X}", e.name));
            } else {
                arr.push(format!("{} = {}", e.name, voltage_text(self.base.volts[i])));
            }
        }
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        match n {
            0 => {
                let names = model_names();
                let sel = names.iter().position(|s| *s == self.model_name).unwrap_or(0);
                let label = if self.model.is_some() {
                    "Model Name".to_string()
                } else {
                    format!("Model not found: {}", self.model_name)
                };
                Some(EditInfo::choice(&label, names, sel))
            }
            1 if self.model.is_some() => Some(
                EditInfo::new("High Logic Voltage (0=default)", self.high_voltage, 0.0, 10.0)
                    .set_unit_step(),
            ),
            _ => None,
        }
    }
    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        match n {
            0 => {
                let names = model_names();
                if let Some(name) = names.get(ei.choice_index) {
                    self.model_name = name.clone();
                    LAST_MODEL.with(|l| *l.borrow_mut() = name.clone());
                    self.update_models();
                }
            }
            1 => {
                self.high_voltage = ei.value;
                self.net.set_high_voltage(self.high_voltage);
            }
            _ => {}
        }
    }

    fn num_handles(&self) -> usize {
        0
    }
    fn drag(&mut self, xx: i32, yy: i32) {
        let yy = (yy + 7) & !15;
        let size_x = self.model.as_ref().map_or(2, |m| m.size_x);
        let cspc2 = self.chip.core.cspc2;
        let b = &mut self.base;
        if xx < b.x {
            b.x2 = b.x;
            b.y2 = b.y;
        } else {
            b.y = yy;
            b.y2 = yy;
            b.x2 = ((xx + 7) & !15).min(b.x + (size_x + 1) * cspc2);
        }
        self.set_points();
    }
    fn flip_x(&mut self, center2: i32, count: usize) {
        self.base.flags ^= FLAG_FLIP_X;
        if count != 1 {
            let xs = (self.chip.core.flipped_size_x + 1) * self.chip.core.cspc2;
            self.base.x = center2 - self.base.x - xs;
            self.base.x2 = center2 - self.base.x2;
        }
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, count: usize) {
        self.base.flags ^= FLAG_FLIP_Y;
        if count != 1 {
            let ys = (self.chip.core.flipped_size_y - 1) * self.chip.core.cspc2;
            self.base.y = center2 - self.base.y - ys;
            self.base.y2 = center2 - self.base.y2;
        }
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, count: usize) {
        self.base.flags ^= FLAG_FLIP_XY;
        let fx = self.base.flags & FLAG_FLIP_X != 0;
        let fy = self.base.flags & FLAG_FLIP_Y != 0;
        if fx != fy {
            self.base.flags ^= FLAG_FLIP_X | FLAG_FLIP_Y;
        }
        if count != 1 {
            self.base.x += self.chip.core.cspc2;
            self.base.flip_xy(xmy);
            self.base.x -= self.chip.core.cspc2;
        }
        self.set_points();
    }
}

/// Registers the subcircuit instance.
pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new(
            "CustomCompositeElm",
            FAMILY,
            "Add Subcircuit Instance",
            "Subcircuits",
            |x, y| Box::new(CustomComposite::new(x, y)),
        )
        .dump(410, |a, b, c, d, f, st| Ok(Box::new(CustomComposite::from_dump(a, b, c, d, f, st)))),
    );
}

#[cfg(test)]
mod tests {
    use super::super::composite::parse_nodes;
    use super::*;

    #[test]
    fn node_lists_split_on_spaces_and_plus() {
        assert_eq!(parse_nodes("1 2+3\t4"), vec![1, 2, 3, 4]);
        assert!(parse_nodes("").is_empty());
    }

    #[test]
    fn models_round_trip_through_xml() {
        let text = r#"<ccm nm="m" f="1" sx="2" sy="3"><ext nm="A" nd="1" ps="0" sd="2"/><ext nm="Y" nd="2" ps="0" sd="3"/><r nn="1 2" x="0 0 16 0" f="0" r="100"/></ccm>"#;
        let node = crate::format::xml::parse(text).unwrap();
        let m = CompositeModel::from_xml(&node);
        assert_eq!((m.ext.len(), m.elms.len(), m.size_y), (2, 1, 3));
        let again = CompositeModel::from_xml(
            &crate::format::xml::parse(&m.to_xml().to_pretty_string()).unwrap(),
        );
        assert_eq!(again.ext, m.ext);
        assert_eq!(again.elms, m.elms);
    }

    #[test]
    fn bus_pins_expand_when_nodes_are_consecutive() {
        let text = r#"<ccm nm="b" f="0" sx="2" sy="4" bcs="1"><ext nm="D" nd="5" ps="0" sd="2" bw="3"/></ccm>"#;
        let m = CompositeModel::from_xml(&crate::format::xml::parse(text).unwrap());
        assert_eq!(m.ext.iter().map(|e| e.node).collect::<Vec<_>>(), vec![5, 6, 7]);
        assert_eq!(m.ext[2].bus_z, 2);
        let out = m.to_xml().to_pretty_string();
        assert!(out.contains("bcs=\"1\"") && out.matches("<ext").count() == 1);
    }

    #[cfg(feature = "passive")]
    #[test]
    fn default_model_is_a_ground_pin() {
        let m = lookup("default").unwrap();
        assert_eq!(m.ext.len(), 1);
        let c = CustomComposite::new(16, 16);
        assert_eq!(c.net.children.len(), 1);
        assert_eq!(c.post_count(), 1);
    }
}
