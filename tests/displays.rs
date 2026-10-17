// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The display chips in small circuits with known answers.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

const HEAD: &str = "$ 1 0.000005 10 50 5 50 5e-11\n";

/// Where the posts of the chip described by `line` are, from a trial load.
fn posts(line: &str) -> Vec<(i32, i32)> {
    let r = parse(&format!("{HEAD}{line}\n"), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let e = &r.circuit.elements[0];
    (0..e.post_count()).map(|i| e.get_post(i).map(|p| (p.x, p.y)).unwrap()).collect()
}

fn run(text: &str, steps: usize) -> Simulation {
    let r = parse(&format!("{HEAD}{text}"), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    for _ in 0..steps {
        sim.step().expect("step");
    }
    sim
}

#[cfg(feature = "digital")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn an_led_in_the_array_lights_when_its_row_and_column_are_driven() {
    let chip = "405 0 0 32 0 0 2 2";
    let p = posts(chip);
    // posts: two columns on the bottom, then two rows on the left
    let (col0, row0) = (p[0], p[2]);
    let text = format!(
        "{chip}\n\
         v {x} {y2} {x} {y} 0 0 40 5 0 0 0.5\n\
         r {rx} {ry} {x} {ry} 0 300\n\
         w {x} {ry} {x} {y}  0\n\
         w {cx} {cy} {cx} {gy} 0\n\
         g {cx} {gy} {cx} {gy2} 0\n\
         g {x} {y2} {x} {y3} 0\n",
        x = row0.0 - 64,
        y = row0.1 - 64,
        y2 = row0.1 - 32,
        y3 = row0.1 - 16,
        rx = row0.0,
        ry = row0.1,
        cx = col0.0,
        cy = col0.1,
        gy = col0.1 + 32,
        gy2 = col0.1 + 48,
    );
    let sim = run(&text, 4000);
    let i = sim.elements.iter().position(|e| e.class_name() == "LEDArrayElm").unwrap();
    let state = sim.elements[i].visual_state();
    assert_eq!(state.len(), 4);
    assert!(state[0] > 5e-3, "the driven LED conducts: {state:?}");
    assert!(state[1..].iter().all(|c| c.abs() < 1e-6), "the others stay dark: {state:?}");
}

#[cfg(feature = "digital")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_common_cathode_display_lights_the_driven_segment() {
    let chip = "157 0 0 32 0 0 7 0 1";
    let p = posts(chip);
    assert_eq!(p.len(), 8, "seven segments and the common pin");
    let (a, common) = (p[0], p[7]);
    let text = format!(
        "{chip}\n\
         v {sx} {sy2} {sx} {sy} 0 0 40 5 0 0 0.5\n\
         r {sx} {sy} {ax} {sy} 0 330\n\
         w {ax} {sy} {ax} {ay} 0\n\
         g {sx} {sy2} {sx} {sy3} 0\n\
         w {cx} {cy} {cx} {gy} 0\n\
         g {cx} {gy} {cx} {gy2} 0\n",
        sx = a.0 - 96,
        sy = a.1,
        sy2 = a.1 + 32,
        sy3 = a.1 + 48,
        ax = a.0,
        ay = a.1,
        cx = common.0,
        cy = common.1,
        gy = common.1 + 32,
        gy2 = common.1 + 48,
    );
    let sim = run(&text, 4000);
    let i = sim.elements.iter().position(|e| e.class_name() == "SevenSegElm").unwrap();
    let cur = sim.elements[i].visual_state();
    assert!(cur[0].abs() > 5e-3, "segment a conducts: {cur:?}");
    assert!(cur[1..7].iter().all(|c| c.abs() < 1e-6), "the other segments stay dark: {cur:?}");
}
