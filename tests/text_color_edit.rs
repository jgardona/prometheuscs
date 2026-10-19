// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The text element color is edited with one color field.

use prometheuscs::elements::element::Element;
use prometheuscs::elements::outputs::TextElm;

fn set(e: &mut TextElm, hex: &str) {
    let mut ei = e.get_edit_info(3).unwrap();
    ei.text = Some(hex.into());
    e.set_edit_value(3, &mut ei);
}

#[test]
fn the_default_color_shows_light_gray() {
    let e = TextElm::new(0, 0);
    let ei = e.get_edit_info(3).unwrap();
    assert!(ei.is_color);
    assert_eq!(ei.text.as_deref(), Some("#c0c0c0"));
}

#[test]
fn a_malformed_stored_color_shows_light_gray() {
    let mut e = TextElm::new(0, 0);
    e.color = Some("zzz".into());
    assert_eq!(e.get_edit_info(3).unwrap().text.as_deref(), Some("#c0c0c0"));
}

#[test]
fn a_chosen_color_is_stored_and_the_default_clears_it() {
    let mut e = TextElm::new(0, 0);
    set(&mut e, "#336699");
    assert_eq!(e.color.as_deref(), Some("#336699"));
    set(&mut e, "#c0c0c0");
    assert_eq!(e.color, None);
}

#[test]
fn an_unparseable_choice_changes_nothing() {
    let mut e = TextElm::new(0, 0);
    set(&mut e, "#336699");
    set(&mut e, "oops");
    assert_eq!(e.color.as_deref(), Some("#336699"));
}
