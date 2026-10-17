// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (XMLSerializer).
//! Writes circuits in the original's XML format.

use super::circuit::Circuit;
use super::xml::XmlOut;
use crate::elements::element::Element;
use crate::solver::sim::SolverType;

/// The XML tag describing one element.
pub fn element_tag(e: &dyn Element) -> XmlOut {
    let mut tag = XmlOut::new(&e.xml_dump_type());
    e.dump_xml(&mut tag);
    e.dump_xml_state(&mut tag);
    tag
}

/// Serializes just the given elements (for the clipboard) as a circuit with default settings.
pub fn write_elements_xml<'a>(elements: impl Iterator<Item = &'a dyn Element>) -> String {
    let mut root = XmlOut::new("cir");
    for e in elements {
        root.child(element_tag(e));
    }
    root.to_pretty_string()
}

/// Serializes a circuit to the XML text the original writes when saving.
pub fn write_xml(circuit: &Circuit) -> String {
    let o = &circuit.options;
    let mut root = XmlOut::new("cir");
    root.int_attr("f", o.flag_bits());
    root.double_attr("ts", o.max_time_step);
    root.double_attr("ic", o.iter_count);
    root.int_attr("cb", o.current_bar);
    root.int_attr("pb", o.power_bar);
    root.double_attr("vr", o.voltage_range);
    root.double_attr("mts", o.min_time_step);
    match o.solver {
        SolverType::Auto => {}
        SolverType::Dense => {
            root.int_attr("st", 1);
        }
        SolverType::Sparse => {
            root.int_attr("st", 2);
        }
    }
    for e in &circuit.elements {
        root.child(element_tag(e.as_ref()));
    }
    // models the elements use are written once each, before the elements that need them are read
    let mut seen: Vec<String> = Vec::new();
    // subcircuit models an element writes win over the copy kept from the file: the model
    // may have been changed since
    let mut by_instance: Vec<String> = Vec::new();
    for e in &circuit.elements {
        if let Some(model) = e.dump_model_xml() {
            let text = model.to_pretty_string();
            if !seen.contains(&text) {
                seen.push(text);
                if model.tag == "ccm" {
                    if let Some((_, name)) = model.attrs.iter().find(|(k, _)| k == "nm") {
                        by_instance.push(name.clone());
                    }
                }
                root.child(model);
            }
        }
    }
    for m in &circuit.model_xml {
        if m.tag == "ccm" && by_instance.contains(&m.string_attr("nm", "")) {
            continue;
        }
        let out = m.to_out();
        let text = out.to_pretty_string();
        if !seen.contains(&text) {
            seen.push(text);
            root.child(out);
        }
    }
    for s in &circuit.scope_xml {
        root.child(s.to_out());
    }
    for a in &circuit.adjustables {
        let mut ae = XmlOut::new("adj");
        ae.int_attr("e", a.elm as i32);
        ae.int_attr("ei", a.edit_item as i32);
        if let Some(info) = circuit.elements.get(a.elm).and_then(|e| e.get_edit_info(a.edit_item)) {
            ae.str_attr("en", &info.name);
        }
        ae.double_attr("mn", a.min_value);
        ae.double_attr("mx", a.max_value);
        ae.str_attr("st", &a.slider_text);
        if a.slider_step > 0.0 {
            ae.double_attr("stp", a.slider_step);
        }
        if let Some(s) = a.shared_slider {
            ae.int_attr("ss", s as i32);
        }
        if a.logarithmic {
            ae.int_attr("log", 1);
        }
        root.child(ae);
    }
    if let Some((t, i1, i2)) = circuit.hint {
        if t != -1 {
            let mut h = XmlOut::new("h");
            h.int_attr("t", t);
            h.int_attr("i1", i1);
            h.int_attr("i2", i2);
            root.child(h);
        }
    }
    root.to_pretty_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::registry::Registry;
    use crate::format::parser::parse;

    #[test]
    fn empty_circuit_writes_header_only() {
        let text = write_xml(&Circuit::new());
        assert!(text.starts_with("<cir f=\"2\"") || text.starts_with("<cir f=\"0\""));
        assert!(text.trim_end().ends_with("/>"));
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "reactive")]
    #[cfg(feature = "sources")]
    #[test]
    fn written_circuit_parses_back() {
        let reg = Registry::standard();
        let src = "$ 1 0.000005 10.2 50 5 50 5e-11\nr 176 80 384 80 0 10\nw 176 80 176 352 0\nc 384 352 176 352 4 0.000015 -10 -10 0\nl 384 80 384 352 0 1 0 0 0\nv 448 352 448 80 0 0 40 5 0 0 0.5\n";
        let first = parse(src, &reg);
        assert!(first.errors.is_empty(), "{:?}", first.errors);
        let xml = write_xml(&first.circuit);
        let second = parse(&xml, &reg);
        assert!(second.errors.is_empty(), "{:?}", second.errors);
        assert_eq!(second.circuit.elements.len(), 5);
        assert_eq!(write_xml(&second.circuit), xml);
    }
}
