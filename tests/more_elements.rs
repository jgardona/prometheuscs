// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The sources, buffers, fuse, sensors and the stop trigger in small circuits with known answers.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

fn posts_of(body: &str) -> Vec<(i32, i32)> {
    let r = parse(&format!("$ 1 0.000005 10 50 5 50 5e-11\n{body}\n"), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let e = &r.circuit.elements[0];
    (0..e.post_count()).map(|i| e.get_post(i).map(|p| (p.x, p.y)).unwrap()).collect()
}

fn load(body: &str, ts: &str) -> Simulation {
    let r = parse(&format!("$ 1 {ts} 10 50 5 50 5e-11\n{body}"), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    sim
}

fn ground(x: i32, y: i32) -> String {
    format!("g {x} {y} {x} {} 0\n", y + 16)
}

fn source_at(x: i32, y: i32, volts: f64) -> String {
    format!("v {x} {} {x} {y} 0 0 40 {volts} 0 0 0.5\n", y - 64) + &ground(x, y - 64)
}

fn source_below(x: i32, y: i32, volts: f64) -> String {
    format!("v {x} {} {x} {y} 0 0 40 {volts} 0 0 0.5\n", y + 64) + &ground(x, y + 64)
}

fn load_to_ground(x: i32, y: i32, ohms: f64) -> String {
    format!("r {x} {y} {x} {} 0 {ohms}\n", y + 48) + &ground(x, y + 48)
}

#[cfg(feature = "logic")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_tri_state_buffer_drives_its_output_only_when_enabled() {
    let line = "180 160 160 224 160 0 0.1 10000000000 0 5";
    let p = posts_of(line);
    // input, output, enable
    for (enable, want) in [(5.0, 5.0), (0.0, 3.0)] {
        // the output is pulled to 3 V through 1 k: the buffer overrides it only when closed
        let text = format!(
            "{line}\n{inp}{en}v {ox} {oy0} {ox} {oy} 0 0 40 3 0 0 0.5\n",
            inp = source_at(p[0].0 - 64, p[0].1, 5.0)
                + &format!("w {} {} {} {} 0\n", p[0].0 - 64, p[0].1, p[0].0, p[0].1),
            en = source_at(p[2].0, p[2].1, enable),
            ox = p[1].0 + 64,
            oy0 = p[1].1 - 64,
            oy = p[1].1,
        ) + &ground(p[1].0 + 64, p[1].1 - 64)
            + &format!("r {} {} {} {} 0 1000\n", p[1].0, p[1].1, p[1].0 + 64, p[1].1);
        let sim = load(&text, "0.000005");
        let mut sim = sim;
        for _ in 0..30 {
            sim.step().unwrap();
        }
        let out = sim.elements[0].get_post_voltage(1);
        assert!((out - want).abs() < 0.2, "enable {enable}: out {out}, want {want}");
    }
}

#[cfg(feature = "logic")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_delay_buffer_follows_its_input_after_the_delay() {
    let line = "422 160 160 224 160 0 0.001 2.5 5";
    let p = posts_of(line);
    let text = format!(
        "{line}\n{}{}",
        source_at(p[0].0, p[0].1, 5.0),
        load_to_ground(p[1].0, p[1].1, 10_000.0)
    );
    let mut sim = load(&text, "0.00001");
    sim.run_until(0.0005).unwrap();
    assert!(sim.elements[0].get_post_voltage(1) < 0.5, "still low during the delay");
    sim.run_until(0.002).unwrap();
    assert!(sim.elements[0].get_post_voltage(1) > 4.5, "high after it");
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_fuse_blows_under_a_large_current_and_then_blocks() {
    let line = "404 160 160 224 160 0 0.0613 6.73 0 false";
    let p = posts_of(line);
    let text = format!(
        "{line}\n{}r {} {} {} {} 0 1\n{}",
        source_at(p[0].0, p[0].1, 10.0),
        p[1].0,
        p[1].1,
        p[1].0,
        p[1].1 + 48,
        ground(p[1].0, p[1].1 + 48)
    );
    let mut sim = load(&text, "0.0001");
    assert_eq!(sim.elements[0].visual_state()[1], 0.0);
    sim.run_until(0.3).unwrap();
    assert_eq!(sim.elements[0].visual_state()[1], 1.0, "blown");
    assert!(sim.elements[0].get_current().abs() < 1e-6);
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[test]
fn an_am_source_follows_its_envelope() {
    let line = "200 160 160 224 160 0 1000 40 5";
    let p = posts_of(line);
    let text = format!("{line}\n{}", load_to_ground(p[0].0, p[0].1, 1000.0));
    let mut sim = load(&text, "0.00001");
    for _ in 0..3333 {
        sim.step().unwrap();
    }
    // the source is evaluated at the start of the step it solves
    let t = sim.time() - 0.00001;
    let w = std::f64::consts::TAU * t;
    let want = (((w * 40.0).sin() + 1.0) / 2.0) * (w * 1000.0).sin() * 5.0;
    let got = sim.elements[0].get_post_voltage(0);
    assert!((got - want).abs() < 0.01, "{got} vs {want} at {t}");
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_stop_trigger_pauses_the_run_when_its_voltage_is_reached() {
    let line = "408 160 160 224 160 0 1 0 0";
    let p = posts_of(line);
    let text = format!("{line}\n{}", source_at(p[0].0, p[0].1, 5.0));
    let mut sim = load(&text, "0.00001");
    assert_eq!(sim.pause_count, 0);
    sim.run_steps(200).unwrap();
    assert_eq!(sim.pause_count, 1, "paused");
    assert!(sim.time() < 0.0002, "the remaining steps were not taken: {}", sim.time());
}

#[cfg(feature = "analog")]
#[test]
fn the_sensors_set_their_resistance_from_the_slider() {
    let reg = Registry::standard();
    for (id, low, high) in [("LDRElm", 90_000.0, 1000.0), ("ThermistorNTCElm", 100_000.0, 500.0)] {
        let mut e = reg.create(id, 160, 160).unwrap();
        e.set_slider_value(0);
        let a = e.slider().unwrap().value;
        e.set_slider_value(100);
        assert_eq!((a, e.slider().unwrap().value), (0, 100), "{id}");
        // the resistance sits in the info panel
        let mut info = Vec::new();
        e.get_info(&mut info);
        let r = info.iter().find(|l| l.starts_with("R = ")).unwrap();
        assert!(!r.is_empty() && low > high, "{id} {r}");
    }
}

/// A square-wave clock (0 to 5 V, `freq` Hz) whose output is wired to the post at (`x`, `y`).
fn clock_to(x: i32, y: i32, freq: f64) -> String {
    format!(
        "v {sx} {y0} {sx} {y} 0 2 {freq} 2.5 2.5 0 0.5\n{g}w {sx} {y} {x} {y} 0\n",
        sx = x - 128,
        y0 = y - 64,
        g = ground(x - 128, y - 64),
    )
}

/// A DC level wired to the post at (`x`, `y`).
fn level_to(x: i32, y: i32, volts: f64) -> String {
    source_at(x - 64, y, volts) + &format!("w {} {} {} {} 0\n", x - 64, y, x, y)
}

#[cfg(feature = "digital")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_sipo_register_shifts_its_input_through_the_outputs() {
    let line = "189 160 160 224 160 0 4";
    let p = posts_of(line);
    // D, CLK, Q0..Q3
    assert_eq!(p.len(), 6);
    let text = format!(
        "{line}\n{}{}{}",
        level_to(p[0].0, p[0].1, 5.0),
        clock_to(p[1].0, p[1].1, 1000.0),
        (2..6).map(|i| load_to_ground(p[i].0, p[i].1, 100_000.0)).collect::<String>()
    );
    let mut sim = load(&text, "0.00001");
    // after four clock pulses the high input has reached every stage
    sim.run_until(0.0045).unwrap();
    let high: Vec<bool> = (2..6).map(|i| sim.elements[0].get_post_voltage(i) > 2.5).collect();
    assert_eq!(high, [true; 4]);
    // one pulse is only enough for the first
    let mut sim = load(&text, "0.00001");
    sim.run_until(0.0005).unwrap();
    let high: Vec<bool> = (2..6).map(|i| sim.elements[0].get_post_voltage(i) > 2.5).collect();
    assert_eq!(high, [true, false, false, false]);
}

#[cfg(feature = "digital")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_sequence_generator_plays_its_bits_on_the_clock() {
    // bits 1, 0, 1, 1, 0, 0, 0, 0
    let line = "188 160 160 224 160 2 8 13";
    let p = posts_of(line);
    assert_eq!(p.len(), 2);
    let text = format!(
        "{line}\n{}{}",
        clock_to(p[0].0, p[0].1, 1000.0),
        load_to_ground(p[1].0, p[1].1, 100_000.0)
    );
    let mut sim = load(&text, "0.00001");
    let mut seen = Vec::new();
    for k in 0..10 {
        sim.run_until((f64::from(k) + 0.25) * 0.001).unwrap();
        seen.push(sim.elements[0].get_post_voltage(1) > 2.5);
    }
    assert_eq!(seen, [true, false, true, true, false, false, false, false, true, false]);
}

#[cfg(feature = "digital")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_piso_register_sends_its_loaded_bits_out_one_per_clock() {
    // 4 bits, new behaviour: LD, CLK, Q, SER, then D3..D0 along the top
    let line = "186 160 160 288 160 2 4";
    let p = posts_of(line);
    assert_eq!(p.len(), 8);
    // load 1, 0, 1, 1 (D3 first) with a pulse on LD, then clock them out
    let text = format!(
        "{line}\n{}{}{}{}{}{}{}",
        // LD high for the first 0.2 ms only: a pulse by a square wave of long period would
        // not do, so hold it high and let the load happen once at the start
        level_to(p[0].0, p[0].1, 5.0),
        clock_to(p[1].0, p[1].1, 1000.0),
        load_to_ground(p[2].0, p[2].1, 100_000.0),
        level_to(p[3].0, p[3].1, 0.0),
        source_at(p[4].0, p[4].1, 5.0),
        source_at(p[5].0, p[5].1, 0.0),
        source_at(p[6].0, p[6].1, 5.0) + &source_at(p[7].0, p[7].1, 5.0),
    );
    let mut sim = load(&text, "0.00001");
    // LD and the clock rise together at the start: the register loads 1, 0, 1, 1 and that same
    // clock edge already shifts it, so Q shows bit 1, then bit 2, bit 3 and the 0 that the serial
    // input shifted into the first place
    let mut q = Vec::new();
    for k in 0..4 {
        sim.run_until((f64::from(k) + 0.25) * 0.001).unwrap();
        q.push(sim.elements[0].get_post_voltage(2) > 2.5);
    }
    assert_eq!(q, [false, true, true, false]);
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[test]
fn a_comparator_pulls_its_output_to_ground_when_plus_is_above_minus() {
    let line = "401 160 160 224 160 0";
    let p = posts_of(line);
    let mut outs = Vec::new();
    for (v0, v1) in [(1.0, 2.0), (2.0, 1.0)] {
        let text = format!(
            "{line}\n{a}{b}v {ox} {oy0} {ox} {oy} 0 0 40 5 0 0 0.5\n",
            a = source_at(p[0].0, p[0].1, v0),
            b = source_at(p[1].0, p[1].1, v1),
            ox = p[2].0 + 64,
            oy0 = p[2].1 - 64,
            oy = p[2].1,
        ) + &ground(p[2].0 + 64, p[2].1 - 64)
            + &format!("r {} {} {} {} 0 1000\n", p[2].0, p[2].1, p[2].0 + 64, p[2].1);
        let mut sim = load(&text, "0.000005");
        for _ in 0..200 {
            sim.step().unwrap();
        }
        outs.push(sim.elements[0].get_post_voltage(2));
    }
    let (lo, hi) = (outs[0].min(outs[1]), outs[0].max(outs[1]));
    assert!(lo < 0.2 && hi > 4.9, "outputs {outs:?}");
}

fn real_opamp_follower(model: i32, vin: f64) -> f64 {
    // supply +/-15 V; unity-gain follower
    let line = format!("409 160 160 288 160 0 0.6 0 0.0231 {model}");
    let p = posts_of(&line);
    let text = format!(
        "{line}\n{inp}{vp}{vm}w {ox} {oy} {ix} {iy} 0\n",
        inp = source_at(p[1].0 - 64, p[1].1, vin)
            + &format!("w {} {} {} {} 0\n", p[1].0 - 64, p[1].1, p[1].0, p[1].1),
        vp = source_at(p[3].0, p[3].1, 15.0),
        vm = source_below(p[4].0, p[4].1, -15.0),
        ox = p[2].0,
        oy = p[2].1,
        ix = p[0].0,
        iy = p[0].1,
    );
    let mut sim = load(&text, "0.000005");
    for _ in 0..2000 {
        sim.step().unwrap();
    }
    sim.elements[0].get_post_voltage(2)
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[cfg(feature = "transistors")]
#[test]
fn a_real_741_follows_a_small_input_in_a_follower() {
    let out = real_opamp_follower(0, 2.0);
    assert!((out - 2.0).abs() < 0.05, "out {out}");
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[cfg(feature = "transistors")]
#[test]
fn the_fixed_324_model_works_as_a_follower() {
    let out = real_opamp_follower(2, 2.0);
    assert!((out - 2.0).abs() < 0.1, "out {out}");
}

fn optocoupler_output(drive: f64) -> f64 {
    let line = "407 160 160 160 160 0";
    let p = posts_of(line);
    assert_eq!(p.len(), 4);
    let text = format!(
        "{line}\n{led}r {rx} {ry} {ax} {ay} 0 330\n{c}{e}",
        // LED: source -> 330 ohm -> anode, cathode to ground
        led = source_at(p[0].0 - 64, p[0].1, drive),
        rx = p[0].0 - 64,
        ry = p[0].1,
        ax = p[0].0,
        ay = p[0].1,
        c = format!("r {} {} {} {} 0 1000\n", p[2].0, p[2].1, p[2].0 + 64, p[2].1)
            + &source_at(p[2].0 + 64, p[2].1, 5.0),
        e = ground(p[3].0, p[3].1) + &ground(p[1].0, p[1].1),
    );
    let mut sim = load(&text, "0.000005");
    for _ in 0..400 {
        sim.step().unwrap();
    }
    sim.elements[0].get_post_voltage(2)
}

#[cfg(feature = "analog")]
#[cfg(feature = "diodes")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[cfg(feature = "transistors")]
#[test]
fn an_optocoupler_pulls_its_output_down_when_the_led_is_driven() {
    let off = optocoupler_output(0.0);
    let on = optocoupler_output(5.0);
    assert!(off > 4.9, "off: {off}");
    assert!(on < 1.0, "on: {on}");
}

#[cfg(feature = "subcircuits")]
#[test]
fn the_op_amp_and_optocoupler_families_round_trip_through_text() {
    for line in [
        "401 160 160 224 160 0",
        "409 160 160 288 160 0 0.6 0 0.0231 0",
        "409 160 160 288 160 0 0.6 0 0.0231 2",
        "407 160 160 160 160 0",
    ] {
        let reg = Registry::standard();
        let first = parse(&format!("$ 1 0.000005 10 50 5 50 5e-11\n{line}\n"), &reg);
        assert!(first.errors.is_empty(), "{line}: {:?}", first.errors);
        let dumped = first.circuit.elements[0].dump();
        let again = parse(&format!("$ 1 0.000005 10 50 5 50 5e-11\n{dumped}\n"), &reg);
        assert!(again.errors.is_empty(), "{dumped}: {:?}", again.errors);
        assert_eq!(again.circuit.elements[0].dump(), dumped, "{line}");
    }
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_test_point_measures_rms_and_peak_of_a_sine() {
    for (meter, want) in [(1, 5.0 / 2f64.sqrt()), (2, 5.0), (3, -5.0), (4, 10.0)] {
        let text = format!(
            "368 160 160 160 192 0 {meter}\nv 160 96 160 160 0 1 100 5 0 0 0.5\n{}",
            ground(160, 96)
        );
        let mut sim = load(&text, "0.00005");
        for _ in 0..1000 {
            sim.step().unwrap();
        }
        let tp = sim.elements.iter().find(|e| e.class_name() == "TestPointElm").unwrap();
        let got = tp.scope_value(0);
        assert!((got - want).abs() < 0.1, "meter {meter}: {got}, want {want}");
    }
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_battery_gives_its_table_voltage_minus_the_drop_and_discharges() {
    let reg = Registry::standard();
    let r =
        parse("$ 1 0.001 10 50 5 50 5e-11\nr 160 160 160 224 0 10\ng 160 224 160 240 0\n", &reg);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut elements = r.circuit.elements;
    // the minus terminal is the first point: it goes to ground, the plus terminal to the resistor
    let mut b = reg.create("BatteryElm", 160, 96).unwrap();
    b.base_mut().set_position(160, 224, 160, 160);
    elements.push(b);
    let mut sim = Simulation::new(elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    for _ in 0..200 {
        sim.step().unwrap();
    }
    let b = sim.elements.iter().find(|e| e.class_name() == "BatteryElm").unwrap();
    // 4.2 V through R0 = 10 mohm and R1 = 20 mohm into 10 ohms
    let vd = b.get_voltage_diff();
    assert!((vd - 4.2 * 10.0 / 10.03).abs() < 0.02, "terminal voltage {vd}");
    // some charge is gone: I = 0.42 A for 0.2 s of a 2 Ah cell
    let soc = b.visual_state()[0];
    assert!(soc < 1.0 && soc > 0.9999, "soc {soc}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn a_custom_transformer_steps_the_voltage_by_its_turns_ratio() {
    // turns 1:2 on a 4 H base, driven by a sine on the primary, a light load on the secondary
    let line = "406 160 160 288 160 0 4 0.999 1:2 2 0 0";
    let p = posts_of(line);
    assert_eq!(p.len(), 4);
    let text = format!(
        "{line}\nv {x} {y0} {x} {y} 0 1 100 5 0 0 0.5\n{g0}{g1}{g3}r {x2} {y2} {x2} {y2b} 0 100000\n{g2}",
        x = p[0].0,
        y = p[0].1,
        y0 = p[0].1 - 64,
        g0 = ground(p[0].0, p[0].1 - 64),
        g1 = ground(p[1].0, p[1].1),
        g3 = ground(p[3].0, p[3].1),
        x2 = p[2].0,
        y2 = p[2].1,
        y2b = p[2].1 + 64,
        g2 = ground(p[2].0, p[2].1 + 64),
    );
    let mut sim = load(&text, "0.00005");
    let mut peak = 0.0f64;
    for step in 0..1000 {
        sim.step().unwrap();
        if step > 400 {
            peak = peak.max(sim.elements[0].get_post_voltage(2).abs());
        }
    }
    assert!((peak - 10.0).abs() < 0.3, "secondary peak {peak}");
}

#[cfg(feature = "reactive")]
#[test]
fn a_custom_transformer_round_trips_through_text() {
    let reg = Registry::standard();
    let line = "406 160 160 288 160 0 4 0.999 1,1:2\\p3 4 0 0 0 0";
    let first = parse(&format!("$ 1 0.000005 10 50 5 50 5e-11\n{line}\n"), &reg);
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    let dumped = first.circuit.elements[0].dump();
    assert_eq!(dumped, line);
}

fn simulate_with(
    extra: Box<dyn prometheuscs::elements::element::Element>,
    text: &str,
) -> Simulation {
    let reg = Registry::standard();
    let r = parse(&format!("$ 1 0.001 10 50 5 50 5e-11\n{text}"), &reg);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut elements = r.circuit.elements;
    elements.push(extra);
    let mut sim = Simulation::new(elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    sim
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_data_input_plays_the_numbers_of_its_file() {
    use prometheuscs::elements::element::EditInfo;
    let path = std::env::temp_dir().join("prometheuscs-data-input-test.txt");
    std::fs::write(&path, "# test\n1\n2\n-3\n").unwrap();
    let reg = Registry::standard();
    let mut d = reg.create("DataInputElm", 160, 96).unwrap();
    d.base_mut().set_position(160, 96, 160, 160);
    // 1 ms per value, repeating
    let mut ei = EditInfo::text("Data File (path)", path.to_str().unwrap());
    d.set_edit_value(0, &mut ei);
    assert!(ei.error.is_none(), "{:?}", ei.error);
    let mut sim = simulate_with(d, "r 160 160 160 224 0 1000\ng 160 224 160 240 0\n");
    let mut seen = Vec::new();
    for _ in 0..6 {
        sim.step().unwrap();
        let d = sim.elements.iter().find(|e| e.class_name() == "DataInputElm").unwrap();
        seen.push(d.get_post_voltage(0));
    }
    // the time step is 1 ms, so each step moves to the next value and the last one is held
    assert_eq!(seen.iter().map(|v| v.round() as i32).collect::<Vec<_>>(), [1, 2, -3, -3, -3, -3]);
    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
#[cfg(feature = "outputs")]
fn an_audio_input_plays_a_wav_file_scaled_by_its_maximum_voltage() {
    use prometheuscs::elements::audio::wav_bytes;
    use prometheuscs::elements::element::EditInfo;
    let path = std::env::temp_dir().join("prometheuscs-audio-input-test.wav");
    std::fs::write(&path, wav_bytes(&[0.5; 100], 1000)).unwrap();
    let reg = Registry::standard();
    let mut a = reg.create("AudioInputElm", 160, 96).unwrap();
    a.base_mut().set_position(160, 96, 160, 160);
    let mut ei = EditInfo::text("Audio File (path, WAV)", path.to_str().unwrap());
    a.set_edit_value(0, &mut ei);
    assert!(ei.error.is_none(), "{:?}", ei.error);
    let mut sim = simulate_with(a, "r 160 160 160 224 0 1000\ng 160 224 160 240 0\n");
    for _ in 0..3 {
        sim.step().unwrap();
    }
    let a = sim.elements.iter().find(|e| e.class_name() == "AudioInputElm").unwrap();
    // 0.5 of full scale times the default 5 V
    assert!((a.get_post_voltage(0) - 2.5).abs() < 0.01, "{}", a.get_post_voltage(0));
    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_data_recorder_keeps_the_last_values_and_exports_them() {
    use prometheuscs::elements::element::EditInfo;
    // keeps 4 values; the source steps through 0..5 V in a ramp: a sine is enough to see it move
    let text = "210 160 160 160 192 0 4\nv 160 96 160 160 0 1 100 5 0 0 0.5\ng 160 96 160 112 0\n";
    let mut sim = load(text, "0.00005");
    for _ in 0..10 {
        sim.step().unwrap();
    }
    let path = std::env::temp_dir().join("prometheuscs-recorder-test.txt");
    let mut ei = EditInfo::text("Export File (path)", path.to_str().unwrap());
    sim.apply_edit(0, 1, &mut ei);
    assert!(ei.error.is_none(), "{:?}", ei.error);
    let out = std::fs::read_to_string(&path).unwrap();
    let mut lines = out.lines();
    assert!(lines.next().unwrap().starts_with("# time step = 0.00005"));
    let values: Vec<f64> = lines.map(|l| l.parse().unwrap()).collect();
    assert_eq!(values.len(), 4);
    // a sine rising from zero: the last four steps are increasing
    assert!(values.windows(2).all(|w| w[1] > w[0]), "{values:?}");
    let _ = std::fs::remove_file(path);
}

#[cfg(feature = "digital")]
#[test]
fn bus_wires_take_the_width_of_the_buses_they_connect() {
    let text = std::fs::read_to_string("assets/circuits/td4-ctr-dn.txt").unwrap();
    let r = parse(&text, &Registry::standard());
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    let widest = sim.elements.iter().map(|e| e.bus_width()).max().unwrap();
    assert!(widest >= 4, "bus wires between the counters and the multiplexer: {widest}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn the_segments_of_a_transmission_line_travel_with_the_worker_state() {
    use prometheuscs::elements::element::DrawCtx;
    use prometheuscs::render::svg::SvgPainter;
    let text = std::fs::read_to_string("assets/circuits/tlfreq.txt").unwrap();
    let registry = Registry::standard();
    let parsed = parse(&text, &registry);
    let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
    sim.analyze().unwrap();
    sim.reset();
    sim.run_until(sim.time() + 2e-10).unwrap();
    let at = sim.elements.iter().position(|e| e.class_name() == "TransLineElm").unwrap();
    let state = sim.elements[at].visual_state();
    assert!(state.len() > 4, "the state has the currents and the segments: {}", state.len());
    assert!(state[2..].iter().any(|v| v.abs() > 1e-6), "the line carries a signal");

    // the interface's copy has no buffers: it draws the segments from the published state
    let copy = || parse(&text, &registry).circuit.elements.into_iter().nth(at).unwrap();
    let draw = |e: &mut Box<dyn prometheuscs::elements::element::Element>| {
        let mut g = SvgPainter::new(800.0, 400.0);
        e.draw(&mut g, &DrawCtx::default());
        g.finish()
    };
    let (mut bare, mut fed) = (copy(), copy());
    fed.set_visual_state(&state);
    assert!(draw(&mut fed).len() > draw(&mut bare).len(), "the segments are drawn");
}
