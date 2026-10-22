// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Options → Show Power: the bodies of resistors are coloured by the power they dissipate, which
//! needs Show Voltage off (turning one option on turns the other off, as in the original).

use prometheuscs::app::edit::Toggle;
use prometheuscs::app::session::Session;
use prometheuscs::elements::registry::Registry;
use prometheuscs::render::svg::SvgPainter;
use std::time::Duration;

fn ohms_law() -> Session {
    let text = std::fs::read_to_string("assets/circuits/ohms.txt").unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    for _ in 0..10 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    s
}

/// The picture of resistor `i` as SVG.
fn drawn(s: &mut Session, i: usize) -> String {
    let mut g = SvgPainter::new(400.0, 400.0);
    let ctx = s.draw_ctx.clone();
    s.sim.elements[i].draw(&mut g, &ctx);
    g.finish()
}

fn resistors(s: &Session) -> Vec<usize> {
    (0..s.sim.elements.len()).filter(|&i| s.sim.elements[i].class_name() == "ResistorElm").collect()
}

#[test]
fn with_show_power_on_a_resistor_is_not_drawn_with_the_voltage_gradient() {
    let mut s = ohms_law();
    let r = resistors(&s);
    assert_eq!(r.len(), 2);
    assert!(drawn(&mut s, r[0]).contains("linearGradient"), "voltage gradient by default");
    s.set_toggle(Toggle::ShowPower, true);
    s.update(Duration::from_millis(16)).unwrap();
    assert!(!drawn(&mut s, r[0]).contains("linearGradient"), "power colour instead");
}

#[test]
fn the_two_resistors_of_ohms_law_get_different_power_colours() {
    let mut s = ohms_law();
    s.set_toggle(Toggle::ShowPower, true);
    s.update(Duration::from_millis(16)).unwrap();
    let r = resistors(&s);
    // the body strokes: 100 ohm dissipates 0.25 W, 1 kohm 0.025 W
    assert!(!drawn(&mut s, r[0]).contains("linearGradient"));
    let body = |svg: String| svg.lines().rfind(|l| l.contains("stroke=\"#")).map(String::from);
    let (a, b) = (body(drawn(&mut s, r[0])), body(drawn(&mut s, r[1])));
    assert_ne!(a, b, "the resistors dissipate different powers");
}
