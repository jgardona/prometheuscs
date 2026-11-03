// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Property tests: what the parser reads, the writer writes back, and the parser reads again
//! unchanged (guarantees G2 and G6: no precision lost in numbers).

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::format::writer::write_xml;
use proptest::prelude::*;

fn positive() -> impl Strategy<Value = f64> {
    prop_oneof![
        (1u32..1_000_000).prop_map(|n| f64::from(n) / 1000.0),
        (-12i32..4, 1u32..1000).prop_map(|(e, m)| f64::from(m) * 10f64.powi(e)),
        any::<f64>().prop_filter("finite and positive", |v| v.is_finite() && *v > 1e-300),
    ]
}

proptest! {
    #[test]
    fn header_options_survive_write_and_read(
        flags in 0i32..256,
        ts in positive(),
        ic in positive(),
        cb in 0i32..101,
        vr in positive(),
        pb in 0i32..101,
        mts in positive(),
    ) {
        let text = format!("$ {flags} {ts} {ic} {cb} {vr} {pb} {mts}\n");
        let first = parse(&text, &Registry::standard());
        prop_assert!(first.errors.is_empty(), "{:?}", first.errors);
        let xml = write_xml(&first.circuit);
        let second = parse(&xml, &Registry::standard());
        prop_assert!(second.errors.is_empty(), "{:?}", second.errors);
        prop_assert_eq!(&first.circuit.options, &second.circuit.options);
        prop_assert_eq!(first.circuit.options.max_time_step, ts);
        prop_assert_eq!(first.circuit.options.iter_count, ic);
        prop_assert_eq!(first.circuit.options.voltage_range, vr);
        prop_assert_eq!(first.circuit.options.min_time_step, mts);
        prop_assert_eq!(write_xml(&second.circuit), xml);
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "reactive")]
    #[test]
    fn generic_element_lines_survive_write_and_read(
        x in -2000i32..2000,
        y in -2000i32..2000,
        dx in -512i32..512,
        dy in -512i32..512,
        flags in 0i32..16,
        value in positive(),
        kind in 0usize..4,
    ) {
        let (x2, y2) = (x + dx, y + dy);
        let line = match kind {
            0 => format!("r {x} {y} {x2} {y2} {flags} {value}"),
            1 => format!("c {x} {y} {x2} {y2} {flags} {value} 0 0 0"),
            2 => format!("l {x} {y} {x2} {y2} {flags} {value} 0 0"),
            _ => format!("w {x} {y} {x2} {y2} {flags}"),
        };
        let text = format!("$ 1 0.000005 10 50 5 50 5e-11\n{line}\n");
        let first = parse(&text, &Registry::standard());
        prop_assert!(first.errors.is_empty(), "{line}: {:?}", first.errors);
        prop_assert_eq!(first.circuit.elements.len(), 1);
        let e = &first.circuit.elements[0];
        prop_assert_eq!((e.base().x, e.base().y, e.base().x2, e.base().y2), (x, y, x2, y2));
        prop_assert_eq!(e.base().flags, flags);
        let xml = write_xml(&first.circuit);
        let second = parse(&xml, &Registry::standard());
        prop_assert!(second.errors.is_empty(), "{xml}: {:?}", second.errors);
        let f = &second.circuit.elements[0];
        prop_assert_eq!((f.base().x, f.base().y, f.base().x2, f.base().y2), (x, y, x2, y2));
        prop_assert_eq!(write_xml(&second.circuit), xml);
    }
}

/// Reads `body` (elements in the 5.x XML format), writes it and reads it back: the written text
/// must contain each of `want` and survive a second round unchanged.
fn xml_round_trip(body: &str, want: &[&str]) {
    let text = format!(
        "<cir f=\"1\" ts=\"0.000005\" ic=\"10\" cb=\"50\" pb=\"50\" vr=\"5\" mts=\"5e-11\">\n{body}\n</cir>\n"
    );
    let first = parse(&text, &Registry::standard());
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    let xml = write_xml(&first.circuit);
    for w in want {
        assert!(xml.contains(w), "{w} missing from:\n{xml}");
    }
    let second = parse(&xml, &Registry::standard());
    assert!(second.errors.is_empty(), "{:?}", second.errors);
    assert_eq!(write_xml(&second.circuit), xml);
}

#[cfg(feature = "analog")]
#[test]
fn two_terminal_controlled_sources_keep_their_expressions() {
    xml_round_trip(
        r#"<cv x="0 64 0 0" f="0" ex="2*v(in)"/>
<ci x="64 64 64 0" f="0" ex="didt(ia)/1000+v(&quot;a,b&quot;)"/>"#,
        &[r#"<cv x="0 64 0 0" f="0" ex="2*v(in)""#, r#"<ci x="64 64 64 0""#, "didt(ia)/1000"],
    );
}

#[cfg(feature = "outputs")]
#[test]
fn meter_labels_are_written_only_when_set() {
    xml_round_trip(
        r#"<p x="0 0 0 64" f="3" me="0" sc="0" re="0" lb="vd"/>
<Ammeter x="64 0 128 0" f="3" me="0" sc="0" lb="ia"/>
<p x="0 128 0 192" f="3" me="0" sc="0" re="0"/>
<Ammeter x="64 128 128 128" f="3" me="0" sc="0"/>"#,
        &[r#"lb="vd""#, r#"lb="ia""#],
    );
    let text = "<cir f=\"1\" ts=\"0.000005\">\n<p x=\"0 0 0 64\" f=\"3\" me=\"0\" sc=\"0\" re=\"0\"/>\n</cir>\n";
    let xml = write_xml(&parse(text, &Registry::standard()).circuit);
    assert!(!xml.contains("lb="), "{xml}");
}

proptest! {
    #[test]
    fn expressions_with_references_survive_write_and_read(
        name in "[A-Za-z][A-Za-z0-9_]{0,6}",
        k in 1u32..1000,
        form in 0usize..4,
    ) {
        let ex = match form {
            0 => format!("{k}*v({name})"),
            1 => format!("i({name})/{k}"),
            2 => format!("dvdt({name})+{k}"),
            _ => format!("didt({name})*v({name}, x{k})"),
        };
        let text = format!(
            "<cir f=\"1\" ts=\"0.000005\">\n<cv x=\"0 64 0 0\" f=\"0\" ex=\"{ex}\"/>\n</cir>\n"
        );
        let first = parse(&text, &Registry::standard());
        prop_assert!(first.errors.is_empty(), "{:?}", first.errors);
        let xml = write_xml(&first.circuit);
        let quoted = format!("ex=\"{ex}\"");
        prop_assert!(xml.contains(&quoted), "{}", xml);
        prop_assert_eq!(write_xml(&parse(&xml, &Registry::standard()).circuit), xml);
    }
}

/// A subcircuit library (`<cir sublib="1">`, spec 006 user story 5) holds models only: it
/// round-trips them, and loading it adds them without touching the circuit.
#[cfg(feature = "subcircuits")]
#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[test]
fn a_subcircuit_library_round_trips_its_models_and_leaves_the_circuit_alone() {
    use prometheuscs::app::session::Session;
    use prometheuscs::app::subcircuits::{ModelScope, is_subcircuit_library};
    use prometheuscs::elements::subcircuit;
    let rc = "$ 1 0.000005 10 50 5 50 5e-11\n207 64 96 32 96 0 in\nr 64 96 128 96 0 1000\nc 128 96 128 160 0 1e-6 0\nw 128 96 192 96 0\n207 192 96 224 96 0 out\ng 128 160 128 176 0\n";
    let reg = Registry::standard();
    let mut s = Session::load(rc, &reg).unwrap();
    for name in ["liba", "libb"] {
        let mut m = s.start_subcircuit().unwrap();
        m.name = name.into();
        s.finish_subcircuit(m, ModelScope::Session).unwrap();
    }
    let names = vec!["liba".to_string(), "libb".to_string()];
    let text = s.library_text(&names).unwrap();
    assert!(is_subcircuit_library(&text), "{text}");
    assert!(text.starts_with(r#"<cir sublib="1">"#), "{text}");
    assert!(!is_subcircuit_library(&s.circuit_xml()));
    for n in &names {
        subcircuit::remove(n);
    }
    // loading it into another circuit adds both models and changes none of its elements
    let mut other = Session::load(rc, &reg).unwrap();
    let before = other.circuit_xml();
    assert_eq!(other.load_subcircuits_only(&text).unwrap(), names);
    assert!(names.iter().all(|n| subcircuit::lookup(n).is_some()));
    assert_eq!(other.sim.elements.len(), 6);
    assert_eq!(other.take_loaded_library(), Some(names.clone()));
    // they are kept for the session, not in the circuit's file
    assert_eq!(other.circuit_xml(), before);
    // and written again, they give the same library
    assert_eq!(other.library_text(&names).unwrap(), text);
    // a library opened as a file also only adds its models
    let path = std::env::temp_dir().join(format!("prometheuscs-lib-{}.xml", std::process::id()));
    std::fs::write(&path, &text).unwrap();
    for n in &names {
        subcircuit::remove(n);
    }
    other.open_file(&path).unwrap();
    assert_eq!(other.circuit_xml(), before);
    assert!(names.iter().all(|n| subcircuit::lookup(n).is_some()));
    let _ = std::fs::remove_file(path);
}
