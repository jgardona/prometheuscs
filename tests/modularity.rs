// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Adding a component touches one module and one registration (SC-012): the dummy element below
//! is defined and registered entirely in this file, and no file under `src/` knows about it.

use prometheuscs::app::commands::{Cmd, all_commands};
use prometheuscs::elements::element::{Base, Element};
use prometheuscs::elements::registry::{ElementDescriptor, Registry};
use prometheuscs::format::parser::parse;
use prometheuscs::format::writer::write_xml;
use prometheuscs::impl_element_base;

/// A two-terminal element with one parameter, the least a new component needs.
#[derive(Clone, Debug)]
struct DummyElm {
    base: Base,
    gain: f64,
}

impl Element for DummyElm {
    impl_element_base!("DummyElm");

    fn dump_type(&self) -> i32 {
        9001
    }
    fn dump(&self) -> String {
        format!("{} {}", self.base.dump_prefix(self.dump_type()), self.gain)
    }
    fn undump_xml(&mut self, xml: &prometheuscs::format::xml::XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.gain = xml.double_attr("g", self.gain);
    }
    fn dump_xml(&self, out: &mut prometheuscs::format::xml::XmlOut) {
        prometheuscs::elements::element::default_dump_xml(&self.base, out);
        out.double_attr("g", self.gain);
    }
}

fn registry_with_dummy() -> Registry {
    let mut r = Registry::standard();
    r.add(
        ElementDescriptor::new("DummyElm", "test", "Add Dummy", "Test Parts", |x, y| {
            Box::new(DummyElm { base: Base::new(x, y), gain: 1.0 })
        })
        .dump(9001, |a, b, c, d, f, st| {
            Ok(Box::new(DummyElm { base: Base::from_dump(a, b, c, d, f), gain: st.next_f64()? }))
        })
        .shortcut('d'),
    );
    r
}

#[test]
fn the_dummy_element_registers_without_errors() {
    let r = registry_with_dummy();
    assert!(r.errors().is_empty(), "{:?}", r.errors());
    assert!(r.contains("DummyElm"));
    assert!(r.families().contains(&"test"));
}

#[test]
fn the_factory_creates_it_for_placement_and_for_files() {
    let r = registry_with_dummy();
    let placed = r.create("DummyElm", 16, 32).expect("placement");
    assert_eq!(placed.class_name(), "DummyElm");
    assert_eq!((placed.base().x, placed.base().y), (16, 32));
    let parsed = parse("$ 1 0.000005 10 50 5 50 5e-11\n9001 0 0 64 0 0 2.5\n", &r);
    assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
    assert_eq!(parsed.circuit.elements.len(), 1);
    assert_eq!(parsed.circuit.elements[0].class_name(), "DummyElm");
}

#[test]
fn the_draw_menu_lists_it_in_its_category() {
    let r = registry_with_dummy();
    let group = r.menu().into_iter().find(|g| g.category == "Test Parts").expect("category");
    assert_eq!(group.items, vec!["DummyElm"]);
    assert!(all_commands(&r).contains(&Cmd::Place("DummyElm".into())));
}

#[test]
fn it_survives_a_save_and_load_round_trip() {
    let r = registry_with_dummy();
    let parsed = parse("$ 1 0.000005 10 50 5 50 5e-11\n9001 0 0 64 0 0 2.5\n", &r);
    let xml = write_xml(&parsed.circuit);
    assert!(xml.contains("Dummy") && xml.contains("g=\"2.5\""), "{xml}");
    let again = parse(&xml, &r);
    assert!(again.errors.is_empty(), "{:?}", again.errors);
    assert_eq!(write_xml(&again.circuit), xml);
}

#[test]
fn the_standard_registry_does_not_know_it() {
    let r = Registry::standard();
    assert!(!r.contains("DummyElm"));
    assert!(r.menu().iter().all(|g| g.category != "Test Parts"));
}
