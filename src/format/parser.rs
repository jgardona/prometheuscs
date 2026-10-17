// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CircuitLoader, XMLDeserializer).
//! Reads circuits in the original's text format and XML format.

use super::circuit::{Adjustable, Circuit};
use super::tokenizer::Tokenizer;
use super::xml::{self, XmlNode};
use crate::elements::registry::Registry;
use crate::error::LoadError;

/// A parsed circuit together with the problems found while reading it. Loading continues
/// after a bad line, so \`circuit\` holds everything that could be read.
pub struct ParseResult {
    /// What was read.
    pub circuit: Circuit,
    /// Problems, in file order.
    pub errors: Vec<LoadError>,
}

/// Reads a circuit from text (the XML format when the text starts with \`<\`).
pub fn parse(text: &str, registry: &Registry) -> ParseResult {
    if text.trim_start().starts_with('<') {
        parse_xml(text, registry)
    } else {
        parse_text(text, registry)
    }
}

fn parse_text(text: &str, registry: &Registry) -> ParseResult {
    let mut circuit = Circuit::new();
    let mut errors = Vec::new();
    for (idx, raw) in text.split(['\n', '\r']).enumerate() {
        let line_no = idx + 1;
        let mut st = Tokenizer::for_circuit_line(raw);
        let Some(type_tok) = st.next_token() else { continue };
        let first = type_tok.chars().next().unwrap_or(' ');
        let malformed = |message: String| LoadError::Malformed {
            line: line_no,
            type_code: type_tok.to_string(),
            message,
        };
        match first {
            'o' => {
                circuit.scope_lines.push(raw.to_string());
                continue;
            }
            'h' => {
                let h = (|| {
                    Ok::<_, super::tokenizer::TokenError>((
                        st.next_i32()?,
                        st.next_i32()?,
                        st.next_i32()?,
                    ))
                })();
                match h {
                    Ok(h) => circuit.hint = Some(h),
                    Err(e) => errors.push(malformed(e.to_string())),
                }
                continue;
            }
            '$' => {
                if let Err(e) = read_options(&mut st, &mut circuit) {
                    errors.push(malformed(e.to_string()));
                }
                continue;
            }
            '!' => {
                if !crate::elements::models::undump_text_custom_logic(&mut st) {
                    circuit.model_lines.push(raw.to_string());
                }
                continue;
            }
            '.' => {
                circuit.model_lines.push(raw.to_string());
                continue;
            }
            '%' | '?' | 'B' => continue, // ignored (filter-specific)
            _ => {}
        }
        let code = if first.is_ascii_digit() {
            match type_tok.parse::<i32>() {
                Ok(n) => n,
                Err(_) => {
                    errors.push(malformed("invalid type code".into()));
                    continue;
                }
            }
        } else {
            first as i32
        };
        match code {
            34 | 32 => {
                if !crate::elements::models::undump_text_model(code, &mut st) {
                    circuit.model_lines.push(raw.to_string());
                }
                continue;
            }
            38 => {
                if let Some(a) = read_adjustable(&mut st) {
                    circuit.adjustables.push(a);
                }
                continue;
            }
            _ => {}
        }
        let coords = (|| {
            Ok::<_, super::tokenizer::TokenError>((
                st.next_i32()?,
                st.next_i32()?,
                st.next_i32()?,
                st.next_i32()?,
                st.next_i32()?,
            ))
        })();
        let (x1, y1, x2, y2, f) = match coords {
            Ok(c) => c,
            Err(e) => {
                errors.push(malformed(e.to_string()));
                continue;
            }
        };
        match registry.create_from_dump(code, x1, y1, x2, y2, f, &mut st) {
            None => errors
                .push(LoadError::UnknownType { line: line_no, type_code: type_tok.to_string() }),
            Some(Err(e)) => errors.push(malformed(e.to_string())),
            Some(Ok(mut elm)) => {
                elm.set_points();
                circuit.elements.push(elm);
            }
        }
    }
    ParseResult { circuit, errors }
}

fn read_options(
    st: &mut Tokenizer<'_>,
    circuit: &mut Circuit,
) -> Result<(), super::tokenizer::TokenError> {
    let o = &mut circuit.options;
    o.set_flag_bits(st.next_i32()?);
    o.max_time_step = st.next_f64()?;
    o.iter_count = st.next_f64()?;
    o.current_bar = st.next_i32()?;
    o.voltage_range = st.next_f64()?;
    // these two were added later; old files stop before them
    if let (Ok(pb), Ok(mts)) = (st.next_i32(), st.next_f64()) {
        o.power_bar = pb;
        o.min_time_step = mts;
    }
    Ok(())
}

fn read_adjustable(st: &mut Tokenizer<'_>) -> Option<Adjustable> {
    let e = st.next_i32().ok()?;
    if e < 0 {
        return None;
    }
    let mut flags = 0;
    let mut ei = st.next_token()?.to_string();
    // the first versions did not dump a flags field; it is marked with an F prefix
    if let Some(rest) = ei.strip_prefix('F') {
        flags = rest.parse().ok()?;
        ei = st.next_token()?.to_string();
    }
    let edit_item = ei.parse().ok()?;
    let min_value = st.next_f64().ok()?;
    let max_value = st.next_f64().ok()?;
    let mut shared_slider = None;
    if flags & 1 != 0 {
        let ano = st.next_i32().ok()?;
        shared_slider = usize::try_from(ano).ok();
    }
    let slider_text = unescape_text(st.next_token()?);
    let slider_step = st.next_f64().unwrap_or(0.0);
    Some(Adjustable {
        elm: e as usize,
        edit_item,
        min_value,
        max_value,
        slider_text,
        slider_step,
        logarithmic: flags & 2 != 0,
        shared_slider,
    })
}

/// Decodes the text escaping used for names inside text lines (`\s` for space and so on).
pub fn unescape_text(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('s') => out.push(' '),
            Some('p') => out.push('+'),
            Some('q') => out.push('='),
            Some('a') => out.push('&'),
            Some('\\') => out.push('\\'),
            Some(o) => out.push(o),
            None => {}
        }
    }
    out
}

fn parse_xml(text: &str, registry: &Registry) -> ParseResult {
    let mut circuit = Circuit::new();
    let mut errors = Vec::new();
    let root = match xml::parse(text) {
        Ok(r) => r,
        Err(e) => return ParseResult { circuit, errors: vec![e] },
    };
    let o = &mut circuit.options;
    o.set_flag_bits(root.int_attr("f", 0));
    o.max_time_step = root.double_attr("ts", o.max_time_step);
    o.iter_count = root.double_attr("ic", o.iter_count);
    o.current_bar = root.int_attr("cb", o.current_bar);
    o.voltage_range = root.double_attr("vr", o.voltage_range);
    o.power_bar = root.int_attr("pb", o.power_bar);
    o.min_time_step = root.double_attr("mts", o.min_time_step);
    o.solver = match root.int_attr("st", 0) {
        1 => crate::solver::sim::SolverType::Dense,
        2 => crate::solver::sim::SolverType::Sparse,
        _ => crate::solver::sim::SolverType::Auto,
    };
    // models first, so the elements that use them can be built whatever the file's order
    let is_model = |t: &str| matches!(t, "dm" | "rlm" | "tm" | "mm" | "clm" | "ccm");
    for node in root.children.iter().filter(|n| is_model(&n.tag)) {
        read_xml_child(node, registry, &mut circuit, &mut errors);
    }
    for node in root.children.iter().filter(|n| !is_model(&n.tag)) {
        read_xml_child(node, registry, &mut circuit, &mut errors);
    }
    ParseResult { circuit, errors }
}

fn read_xml_child(
    node: &XmlNode,
    registry: &Registry,
    circuit: &mut Circuit,
    errors: &mut Vec<LoadError>,
) {
    match node.tag.as_str() {
        "o" => circuit.scope_xml.push(node.clone()),
        "dm" | "rlm" | "tm" | "mm" | "clm" | "ccm" => {
            let defined = crate::elements::models::undump_xml_model(node);
            // a subcircuit model belongs to the circuit even when nothing uses it yet: keep it
            // so that saving writes it again
            let keep = !defined
                || (node.tag == "ccm"
                    && !circuit.model_xml.iter().any(|m| {
                        m.tag == "ccm" && m.string_attr("nm", "") == node.string_attr("nm", "")
                    }));
            if keep {
                circuit.model_xml.push(node.clone());
            }
        }
        "h" => {
            circuit.hint =
                Some((node.int_attr("t", -1), node.int_attr("i1", 0), node.int_attr("i2", 0)))
        }
        "adj" => {
            let e = node.int_attr("e", -1);
            if e >= 0 {
                circuit.adjustables.push(Adjustable {
                    elm: e as usize,
                    edit_item: node.int_attr("ei", 0).max(0) as usize,
                    min_value: node.double_attr("mn", 1.0),
                    max_value: node.double_attr("mx", 1000.0),
                    slider_text: node.string_attr("st", ""),
                    slider_step: node.double_attr("stp", 0.0),
                    logarithmic: node.int_attr("log", 0) != 0,
                    shared_slider: usize::try_from(node.int_attr("ss", -1)).ok(),
                });
            }
        }
        "test" | "switchevent" | "scopedata" => {}
        tag => {
            let Some(x) = node.attr("x") else { return };
            let Some(d) = registry.get_by_xml_tag(tag) else {
                errors.push(LoadError::UnknownType { line: 0, type_code: tag.to_string() });
                return;
            };
            let Some(mut elm) = registry.create(d.id, 0, 0) else { return };
            elm.undump_xml(node);
            let parts: Vec<i32> = x.split_whitespace().filter_map(|p| p.parse().ok()).collect();
            if let [x1, y1, x2, y2] = parts[..] {
                elm.base_mut().set_position(x1, y1, x2, y2);
            }
            elm.set_points();
            circuit.elements.push(elm);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescapes_text_fields() {
        assert_eq!(unescape_text("a\\sb\\\\c"), "a b\\c");
    }

    #[test]
    fn empty_input_gives_an_empty_circuit() {
        let r = parse("", &Registry::new());
        assert!(r.circuit.elements.is_empty() && r.errors.is_empty());
    }

    #[test]
    fn unknown_types_are_reported_with_their_line_number() {
        let r = parse("$ 1 5e-6 10 50 5 50 5e-11\nzz 0 0 8 8 0\n", &Registry::new());
        assert_eq!(r.errors.len(), 1);
        assert!(matches!(&r.errors[0], LoadError::UnknownType { line: 2, .. }));
        assert!(r.circuit.options.show_dots);
    }
}
