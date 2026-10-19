// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The LED color is edited with one color field; the other LED fields keep their positions.

use prometheuscs::elements::diodes::{DiodeElm, DiodeKind};
use prometheuscs::elements::element::Element;

fn led() -> DiodeElm {
    DiodeElm::new(DiodeKind::Led, 0, 0)
}

fn color_text(e: &DiodeElm) -> String {
    e.get_edit_info(0).unwrap().text.unwrap()
}

#[test]
fn the_led_offers_one_color_field_first() {
    let ei = led().get_edit_info(0).unwrap();
    assert!(ei.is_color);
    assert_eq!(ei.text.as_deref(), Some("#ff0000"));
}

#[test]
fn the_old_green_and_blue_slots_are_hidden_and_the_rest_keep_their_index() {
    let e = led();
    assert!(e.get_edit_info(1).unwrap().hidden);
    assert!(e.get_edit_info(2).unwrap().hidden);
    assert!(e.get_edit_info(3).unwrap().name.starts_with("Max Brightness Current"));
    assert!(e.get_edit_info(4).is_some());
}

#[test]
fn a_new_color_is_stored_and_read_back() {
    let mut e = led();
    let mut ei = e.get_edit_info(0).unwrap();
    ei.text = Some("#00ff00".into());
    e.set_edit_value(0, &mut ei);
    assert_eq!(color_text(&e), "#00ff00");
}

#[test]
fn applying_an_untouched_color_keeps_the_stored_value() {
    let mut e = led();
    let mut ei = e.get_edit_info(0).unwrap();
    ei.text = Some("#80ff00".into());
    e.set_edit_value(0, &mut ei);
    // 0x80/255 is not a round value; applying the same hex again must keep it exactly
    let before = e.get_edit_info(0).unwrap();
    let mut again = before.clone();
    e.set_edit_value(0, &mut again);
    assert_eq!(e.get_edit_info(0).unwrap().text, before.text);
}

#[test]
fn an_unparseable_color_changes_nothing() {
    let mut e = led();
    let mut ei = e.get_edit_info(0).unwrap();
    ei.text = Some("not a color".into());
    e.set_edit_value(0, &mut ei);
    assert_eq!(color_text(&e), "#ff0000");
}

#[test]
fn max_brightness_current_still_edits_at_index_three() {
    let mut e = led();
    let mut ei = e.get_edit_info(3).unwrap();
    ei.value = 0.02;
    e.set_edit_value(3, &mut ei);
    assert!((e.get_edit_info(3).unwrap().value - 0.02).abs() < 1e-12);
    assert_eq!(color_text(&e), "#ff0000");
}
