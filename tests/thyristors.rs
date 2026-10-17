// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The triode and the thyristors in small circuits with known answers.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

const HEAD: &str = "$ 1 0.000005 10 50 5 50 5e-11\n";

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

/// A ground symbol with its post at (`x`, `y`).
fn ground(x: i32, y: i32) -> String {
    format!("g {x} {y} {x} {} 0\n", y + 16)
}

/// A source of `volts` whose positive end is at (`x`, `y`), reached from a grounded far end
/// 64 pixels above.
fn source_at(x: i32, y: i32, volts: f64) -> String {
    format!(
        "v {x} {y0} {x} {y} 0 0 40 {volts} 0 0 0.5\ng {x} {y0} {x} {y1} 0\n",
        y0 = y - 64,
        y1 = y - 48
    )
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "transistors")]
#[test]
fn a_triode_with_the_grid_at_the_cathode_conducts_the_child_langmuir_current() {
    let line = "173 160 160 224 160 0 93 680";
    let p = posts(line);
    let (plate, grid, cath) = (p[0], p[1], p[2]);
    let text = format!(
        "{line}\n{src}g {gx} {gy} {gx} {gy2} 0\ng {cx} {cy} {cx} {cy2} 0\n",
        src = source_at(plate.0, plate.1, 250.0),
        gx = grid.0,
        gy = grid.1,
        gy2 = grid.1 + 16,
        cx = cath.0,
        cy = cath.1,
        cy2 = cath.1 + 16,
    );
    let sim = run(&text, 400);
    // Ip = (Vgk + Vpk/mu)^1.5 / kg1
    let want = (250.0_f64 / 93.0).powf(1.5) / 680.0;
    let got = sim.elements[0].get_current();
    assert!((got - want).abs() < want * 0.02, "{got} vs {want}");
}

/// Builds an SCR circuit: 10 V through 1 k to the anode, a gate driven through `rg` from
/// `vg`, cathode grounded. Returns the anode voltage after the run.
fn scr_anode(vg: f64, rg: f64) -> f64 {
    let line = "177 192 160 192 288 0 0 0 0.01 0.0082 50";
    let p = posts(line);
    let (anode, cath, gate) = (p[0], p[1], p[2]);
    let text = format!(
        "{line}\n{src}r {ax} {ay0} {ax} {ay} 0 1000\ng {cx} {cy} {cx} {cy2} 0\n\
         v {gx0} {gy} {gx1} {gy} 0 0 40 {vg} 0 0 0.5\ng {gx0} {gy} {gx0} {gyg} 0\n\
         r {gx1} {gy} {gx} {gy} 0 {rg}\n",
        src = source_at(anode.0, anode.1 - 64, 10.0),
        ax = anode.0,
        ay0 = anode.1 - 64,
        ay = anode.1,
        cx = cath.0,
        cy = cath.1,
        cy2 = cath.1 + 16,
        gx = gate.0,
        gy = gate.1,
        gx0 = gate.0 + 128,
        gx1 = gate.0 + 64,
        gyg = gate.1 + 16,
    );
    let sim = run(&text, 2000);
    sim.elements[0].get_post_voltage(0)
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "transistors")]
#[test]
fn an_scr_blocks_until_the_gate_current_passes_the_trigger_then_latches() {
    let off = scr_anode(0.0, 100.0);
    assert!(off > 9.0, "no gate drive, the SCR blocks: {off}");
    let on = scr_anode(5.0, 100.0);
    assert!(on < 2.0, "gate drive of about 33 mA fires it: {on}");
}

#[cfg(feature = "analog")]
#[cfg(feature = "diodes")]
#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[cfg(feature = "transistors")]
#[test]
fn the_thyristor_examples_run() {
    for name in ["scr", "scractrig", "triacdimmer", "triode", "triodeamp"] {
        let text = std::fs::read_to_string(format!("assets/circuits/{name}.txt")).unwrap();
        let r = parse(&text, &Registry::standard());
        assert!(r.errors.is_empty(), "{name}: {:?}", r.errors);
        let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
        sim.analyze().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        sim.reset();
        for _ in 0..2000 {
            sim.step().unwrap_or_else(|e| panic!("{name}: {e:?}"));
        }
        assert!(sim.elements.iter().all(|e| e.get_current().is_finite()), "{name}");
    }
}

#[cfg(feature = "analog")]
#[cfg(feature = "diodes")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[test]
fn the_unijunction_oscillator_relaxes() {
    let text = std::fs::read_to_string("assets/circuits/ujtosc.txt").unwrap();
    let r = parse(&text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    let ujt = sim.elements.iter().position(|e| e.class_name() == "UnijunctionElm").unwrap();
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for k in 0..40000 {
        sim.step().expect("step");
        if k > 2000 {
            let v = sim.elements[ujt].get_post_voltage(0);
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    assert!(hi - lo > 4.0, "the emitter swings between firings: {lo} .. {hi}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn a_cross_switch_routes_each_pole_to_the_straight_or_the_crossed_throw() {
    for (position, hot_throw) in [(0, 1usize), (1, 3usize)] {
        let line = format!("430 160 160 224 160 0 {position} false");
        let p = posts(&line);
        let text = format!(
            "{line}\n{src}r {ax} {ay} {ax2} {ay} 0 1000\n{g1}r {bx} {by} {bx2} {by} 0 1000\n{g2}",
            src = source_at(p[0].0, p[0].1, 10.0),
            ax = p[1].0,
            ay = p[1].1,
            ax2 = p[1].0 + 64,
            g1 = ground(p[1].0 + 64, p[1].1),
            bx = p[3].0,
            by = p[3].1,
            bx2 = p[3].0 + 64,
            g2 = ground(p[3].0 + 64, p[3].1) + &ground(p[2].0, p[2].1),
        );
        let sim = run(&text, 20);
        let e = &sim.elements[0];
        for throw in [1usize, 3] {
            let v = e.get_post_voltage(throw);
            if throw == hot_throw {
                assert!((v - 10.0).abs() < 0.01, "position {position}: throw {throw} at {v}");
            } else {
                assert!(v.abs() < 0.01, "position {position}: throw {throw} at {v}");
            }
        }
    }
}
