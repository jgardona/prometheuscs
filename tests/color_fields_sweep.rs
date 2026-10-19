// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! No element edits a color with a text or number box: every color field is a color chooser.

use prometheuscs::elements::registry::Registry;

#[test]
fn every_color_field_is_a_color_chooser() {
    let reg = Registry::standard();
    let mut offenders = Vec::new();
    for id in reg.ids() {
        let Some(e) = reg.create(id, 0, 0) else { continue };
        for n in 0.. {
            let Some(ei) = e.get_edit_info(n) else { break };
            let name = ei.name.to_lowercase();
            let colorish = ["color", "colour", "red value", "green value", "blue value"]
                .iter()
                .any(|w| name.contains(w));
            if (colorish || ei.is_color) && !ei.is_color {
                offenders.push(format!("{id} field {n}: {}", ei.name));
            }
            if ei.is_color {
                assert!(ei.text.as_deref().is_some_and(|t| t.starts_with('#')), "{id} field {n}");
            }
        }
    }
    assert!(offenders.is_empty(), "color fields that are not choosers: {offenders:?}");
}
