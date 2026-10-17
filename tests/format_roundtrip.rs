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
