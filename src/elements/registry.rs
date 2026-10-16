// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (ElementFactory, CirSim.register).
//! Declarative registry of element types.
//!
//! Each element family declares its elements in one `register` function. Factory dispatch, the
//! Draw menu model, toolbar quick-insert entries and the component inventory are all derived
//! from these descriptors, so adding or removing a component never touches other modules.

use super::element::Element;
use crate::format::tokenizer::{TokenError, Tokenizer};
use std::collections::HashMap;

/// Creates a freshly placed element with one post at the given grid position.
pub type CreateFn = fn(i32, i32) -> Box<dyn Element>;

/// Creates an element from a file entry: `x1 y1 x2 y2 flags` then element-specific fields.
pub type FromDumpFn =
    fn(i32, i32, i32, i32, i32, &mut Tokenizer<'_>) -> Result<Box<dyn Element>, TokenError>;

/// Declares one component type to the system.
#[derive(Clone)]
pub struct ElementDescriptor {
    /// Unique stable identifier: the original class name (for example `ResistorElm`).
    pub id: &'static str,
    /// File-format type code(s), when elements of this type can be loaded from files.
    pub dump_code: Option<i32>,
    /// English label of the Draw menu entry (for example `Add Resistor`).
    pub display_name: &'static str,
    /// Draw menu submenu; empty for entries at the top level of the menu.
    pub category: &'static str,
    /// Icon asset name, if the element has a toolbar icon.
    pub icon: Option<&'static str>,
    /// Key that inserts the element.
    pub shortcut: Option<char>,
    /// Whether the element has a button in the placement toolbar.
    pub quick_insert: bool,
    /// Whether the element appears in the Draw menu.
    pub in_menu: bool,
    /// Owning family (also the Cargo feature name).
    pub family: &'static str,
    /// Placement constructor.
    pub create: CreateFn,
    /// File constructor.
    pub from_dump: Option<FromDumpFn>,
}

impl ElementDescriptor {
    /// Starts a descriptor for an element created by `create` with a Draw menu entry.
    pub fn new(
        id: &'static str,
        family: &'static str,
        display_name: &'static str,
        category: &'static str,
        create: CreateFn,
    ) -> Self {
        ElementDescriptor {
            id,
            dump_code: None,
            display_name,
            category,
            icon: None,
            shortcut: None,
            quick_insert: false,
            in_menu: true,
            family,
            create,
            from_dump: None,
        }
    }

    /// Sets the file-format code and loader.
    pub fn dump(mut self, code: i32, from_dump: FromDumpFn) -> Self {
        self.dump_code = Some(code);
        self.from_dump = Some(from_dump);
        self
    }

    /// Sets the toolbar icon.
    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the insert shortcut key.
    pub fn shortcut(mut self, key: char) -> Self {
        self.shortcut = Some(key);
        self
    }

    /// Adds a placement toolbar button.
    pub fn quick(mut self) -> Self {
        self.quick_insert = true;
        self
    }

    /// Hides the element from the Draw menu (it can still be loaded and placed by code).
    pub fn hidden(mut self) -> Self {
        self.in_menu = false;
        self
    }
}

/// Error registering an element.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// Two descriptors use the same identifier.
    #[error("duplicate element id `{0}`")]
    DuplicateId(&'static str),
    /// Two descriptors use the same file-format code.
    #[error("duplicate dump code {0} (`{1}` and `{2}`)")]
    DuplicateDumpCode(i32, &'static str, &'static str),
}

/// One Draw menu submenu with its entries in registration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuGroup {
    /// Submenu title; empty for entries at the top of the menu.
    pub category: &'static str,
    /// Descriptor ids of the entries.
    pub items: Vec<&'static str>,
}

/// All known element types.
#[derive(Default, Clone)]
pub struct Registry {
    descriptors: Vec<ElementDescriptor>,
    by_id: HashMap<&'static str, usize>,
    by_dump: HashMap<i32, usize>,
    by_xml_tag: HashMap<String, usize>,
    errors: Vec<RegistryError>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// The registry with every element family compiled into this build.
    pub fn standard() -> Self {
        let mut r = Registry::new();
        super::register_all(&mut r);
        r
    }

    /// Adds a descriptor.
    pub fn register(&mut self, d: ElementDescriptor) -> Result<(), RegistryError> {
        if self.by_id.contains_key(d.id) {
            return Err(RegistryError::DuplicateId(d.id));
        }
        if let Some(code) = d.dump_code {
            if let Some(&other) = self.by_dump.get(&code) {
                return Err(RegistryError::DuplicateDumpCode(
                    code,
                    self.descriptors[other].id,
                    d.id,
                ));
            }
            self.by_dump.insert(code, self.descriptors.len());
        }
        // the first element to claim an XML tag owns it (dump classes are registered first)
        let tag = (d.create)(0, 0).xml_dump_type();
        self.by_xml_tag.entry(tag).or_insert(self.descriptors.len());
        self.by_id.insert(d.id, self.descriptors.len());
        self.descriptors.push(d);
        Ok(())
    }

    /// Adds a descriptor; a duplicate is recorded (see [`Registry::errors`]) instead of
    /// replacing the earlier entry, so the registry test can report it.
    pub fn add(&mut self, d: ElementDescriptor) {
        if let Err(e) = self.register(d) {
            self.errors.push(e);
        }
    }

    /// Registration problems found by [`Registry::add`].
    pub fn errors(&self) -> &[RegistryError] {
        &self.errors
    }

    /// All descriptors in registration order.
    pub fn descriptors(&self) -> &[ElementDescriptor] {
        &self.descriptors
    }

    /// Looks up a descriptor by identifier.
    pub fn get(&self, id: &str) -> Option<&ElementDescriptor> {
        self.by_id.get(id).map(|&i| &self.descriptors[i])
    }

    /// Looks up a descriptor by file-format code.
    pub fn get_by_dump(&self, code: i32) -> Option<&ElementDescriptor> {
        self.by_dump.get(&code).map(|&i| &self.descriptors[i])
    }

    /// Looks up a descriptor by the tag name used in XML circuit files.
    pub fn get_by_xml_tag(&self, tag: &str) -> Option<&ElementDescriptor> {
        self.by_xml_tag.get(tag).map(|&i| &self.descriptors[i])
    }

    /// True when an element with this identifier is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// Identifiers of all registered elements.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.descriptors.iter().map(|d| d.id)
    }

    /// Creates a freshly placed element by identifier.
    pub fn create(&self, id: &str, x: i32, y: i32) -> Option<Box<dyn Element>> {
        let d = self.get(id)?;
        let mut elm = (d.create)(x, y);
        elm.alloc_nodes();
        Some(elm)
    }

    /// Creates an element from a file entry by type code. Returns `None` for unknown codes.
    pub fn create_from_dump(
        &self,
        code: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        flags: i32,
        st: &mut Tokenizer<'_>,
    ) -> Option<Result<Box<dyn Element>, TokenError>> {
        let d = self.get_by_dump(code)?;
        let from_dump = d.from_dump?;
        Some(from_dump(x1, y1, x2, y2, flags, st).map(|mut elm| {
            elm.alloc_nodes();
            elm
        }))
    }

    /// The Draw menu model: groups in order of first appearance, items in registration order.
    pub fn menu(&self) -> Vec<MenuGroup> {
        let mut groups: Vec<MenuGroup> = Vec::new();
        for d in self.descriptors.iter().filter(|d| d.in_menu) {
            match groups.iter_mut().find(|g| g.category == d.category) {
                Some(g) => g.items.push(d.id),
                None => groups.push(MenuGroup { category: d.category, items: vec![d.id] }),
            }
        }
        groups
    }

    /// Descriptors that have a placement toolbar button.
    pub fn quick_insert(&self) -> impl Iterator<Item = &ElementDescriptor> {
        self.descriptors.iter().filter(|d| d.quick_insert)
    }

    /// Families that contributed at least one element.
    pub fn families(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for d in &self.descriptors {
            if !out.contains(&d.family) {
                out.push(d.family);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::element::Base;

    struct Dummy {
        base: Base,
    }

    impl Element for Dummy {
        crate::impl_element_base!("DummyElm");
    }

    fn make(x: i32, y: i32) -> Box<dyn Element> {
        Box::new(Dummy { base: Base::new(x, y) })
    }

    fn load(
        a: i32,
        b: i32,
        c: i32,
        d: i32,
        f: i32,
        _st: &mut Tokenizer<'_>,
    ) -> Result<Box<dyn Element>, TokenError> {
        Ok(Box::new(Dummy { base: Base::from_dump(a, b, c, d, f) }))
    }

    fn descriptor(id: &'static str, code: i32) -> ElementDescriptor {
        ElementDescriptor::new(id, "test", "Add Dummy", "Test", make).dump(code, load)
    }

    #[test]
    fn registers_and_creates_elements() {
        let mut r = Registry::new();
        r.add(descriptor("DummyElm", 900));
        assert!(r.contains("DummyElm"));
        let e = r.create("DummyElm", 8, 16).unwrap();
        assert_eq!((e.base().x, e.base().y), (8, 16));
        assert_eq!(e.base().nodes.len(), 2);
        let mut st = Tokenizer::new("");
        let loaded = r.create_from_dump(900, 1, 2, 3, 4, 5, &mut st).unwrap().unwrap();
        assert_eq!(loaded.base().flags, 5);
        assert!(r.create_from_dump(901, 0, 0, 0, 0, 0, &mut st).is_none());
    }

    #[test]
    fn rejects_duplicate_ids_and_codes() {
        let mut r = Registry::new();
        r.add(descriptor("A", 1));
        assert_eq!(r.register(descriptor("A", 2)), Err(RegistryError::DuplicateId("A")));
        assert_eq!(
            r.register(descriptor("B", 1)),
            Err(RegistryError::DuplicateDumpCode(1, "A", "B"))
        );
    }

    #[test]
    fn menu_groups_follow_registration_order() {
        let mut r = Registry::new();
        r.add(descriptor("A", 1));
        r.add(ElementDescriptor::new("B", "test", "Add B", "", make));
        r.add(descriptor("C", 3));
        let menu = r.menu();
        assert_eq!(menu[0], MenuGroup { category: "Test", items: vec!["A", "C"] });
        assert_eq!(menu[1], MenuGroup { category: "", items: vec!["B"] });
        assert_eq!(r.families(), vec!["test"]);
    }
}
