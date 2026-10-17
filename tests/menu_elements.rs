// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Switches with several poles, the multiplexer, the delay relay, the DC motor, the ohmmeter
//! and the wattmeters in small circuits with known answers.

use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

fn header(ts: &str) -> String {
    format!("$ 1 {ts} 10 50 5 50 5e-11\n")
}

fn posts_of(body: &str) -> Vec<(i32, i32)> {
    let r = parse(&format!("{}{body}\n", header("0.000005")), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let e = &r.circuit.elements[0];
    (0..e.post_count()).map(|i| e.get_post(i).map(|p| (p.x, p.y)).unwrap()).collect()
}

fn run(body: &str, ts: &str, steps: usize) -> Simulation {
    let r = parse(&format!("{}{body}", header(ts)), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    for _ in 0..steps {
        sim.step().expect("step");
    }
    sim
}

fn ground(x: i32, y: i32) -> String {
    format!("g {x} {y} {x} {} 0\n", y + 16)
}

/// A source of `volts` whose positive end is at (`x`, `y`), reached from a grounded far end.
fn source_at(x: i32, y: i32, volts: f64) -> String {
    format!("v {x} {} {x} {y} 0 0 40 {volts} 0 0 0.5\n", y - 64) + &ground(x, y - 64)
}

fn load(x: i32, y: i32, ohms: f64) -> String {
    format!("r {x} {y} {x} {} 0 {ohms}\n", y + 48) + &ground(x, y + 48)
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn a_dpdt_switch_moves_both_poles() {
    for (position, hot) in [(0, 1usize), (1, 2usize)] {
        let line = format!("429 160 160 224 160 0 {position} false 2");
        let p = posts_of(&line);
        assert_eq!(p.len(), 6);
        let text = format!(
            "{line}\n{src}{l1}{l2}{g}",
            src = source_at(p[0].0, p[0].1, 10.0),
            l1 = load(p[1].0, p[1].1, 1000.0),
            l2 = load(p[2].0, p[2].1, 1000.0),
            g = ground(p[3].0, p[3].1),
        );
        let sim = run(&text, "0.000005", 20);
        let e = &sim.elements[0];
        for throw in [1usize, 2] {
            let v = e.get_post_voltage(throw);
            let want = if throw == hot { 10.0 } else { 0.0 };
            assert!((v - want).abs() < 0.01, "position {position} throw {throw}: {v}");
        }
    }
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "switches")]
#[test]
fn a_make_before_break_switch_joins_both_throws_in_its_middle_positions() {
    for (position, first, second) in [(0, true, false), (1, true, true), (2, false, true)] {
        let line = format!("416 160 160 224 160 0 {position} false 0");
        let p = posts_of(&line);
        let text = format!(
            "{line}\n{src}{l1}{l2}",
            src = source_at(p[0].0, p[0].1, 10.0),
            l1 = load(p[1].0, p[1].1, 1000.0),
            l2 = load(p[2].0, p[2].1, 1000.0),
        );
        let sim = run(&text, "0.000005", 20);
        let e = &sim.elements[0];
        for (throw, on) in [(1usize, first), (2usize, second)] {
            let v = e.get_post_voltage(throw);
            assert!(
                (v - if on { 10.0 } else { 0.0 }).abs() < 0.01,
                "position {position} throw {throw}: {v}"
            );
        }
    }
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn an_analog_multiplexer_joins_the_selected_input_to_the_output() {
    let line = "432 160 160 224 160 0 2 20 1e10 2.5";
    let p = posts_of(line);
    // I0..I3, S0, S1, Z
    assert_eq!(p.len(), 7);
    let mut text = format!("{line}\n");
    for (i, v) in [0.0, 0.0, 3.0, 0.0].into_iter().enumerate() {
        text += &source_at(p[i].0 - 64, p[i].1, v);
        text += &format!("w {} {} {} {} 0\n", p[i].0 - 64, p[i].1, p[i].0, p[i].1);
    }
    // select input 2: S0 low, S1 high
    text += &source_at(p[4].0, p[4].1, 0.0);
    text += &source_at(p[5].0, p[5].1, 5.0);
    text += &load(p[6].0, p[6].1, 10_000.0);
    let sim = run(&text, "0.000005", 50);
    let z = sim.elements[0].get_post_voltage(6);
    assert!((z - 3.0 * 10_000.0 / 10_020.0).abs() < 0.01, "{z}");
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_time_delay_relay_closes_after_its_delay() {
    let line = "414 160 160 224 160 0 0.002 0 1 10000000";
    let p = posts_of(line);
    // Vin, gnd, in, out
    let text = format!(
        "{line}\n{vin}{gnd}{inp}{out}",
        vin = source_at(p[0].0, p[0].1, 5.0),
        gnd = ground(p[1].0, p[1].1),
        inp = source_at(p[2].0, p[2].1, 5.0),
        out = load(p[3].0, p[3].1, 1000.0),
    );
    let early = run(&text, "0.00001", 50);
    assert!(early.elements[0].get_post_voltage(3) < 0.1, "open before the delay");
    let late = run(&text, "0.00001", 600);
    assert!(late.elements[0].get_post_voltage(3) > 4.9, "closed after it");
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn a_dc_motor_settles_at_the_speed_its_constants_give() {
    let line = "415 160 160 288 160 0 0.5 1 0.15 0.15 0.02 0.05 1 0";
    let p = posts_of(line);
    let text = format!("{line}\n{}{}", source_at(p[0].0, p[0].1, 5.0), ground(p[1].0, p[1].1));
    let sim = run(&text, "0.002", 4000);
    // V = (R b / K + Kb) w
    let want = 5.0 / (1.0 * 0.05 / 0.15 + 0.15);
    let got = sim.elements[0].visual_state()[1].abs();
    assert!((got - want).abs() < want * 0.03, "{got} vs {want}");
}

#[cfg(feature = "outputs")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_wattmeter_reads_the_power_of_a_load() {
    // 10 V across 100 ohms is 1 W; the current coil is in series, the voltage coil across
    let xml = |p: &[(i32, i32)], extra: &str| {
        format!(
            "<cir f=\"1\" ts=\"0.000005\" ic=\"10\" cb=\"50\" pb=\"50\" vr=\"5\" mts=\"5e-11\">\n\
             <WattmeterTrue x=\"160 160 288 160\" f=\"0\" w=\"32\"/>\n{extra}</cir>",
            extra = if p.is_empty() { String::new() } else { extra.to_string() }
        )
    };
    let probe = parse(&xml(&[], ""), &Registry::standard());
    assert!(probe.errors.is_empty(), "{:?}", probe.errors);
    let e = &probe.circuit.elements[0];
    let p: Vec<(i32, i32)> = (0..4).map(|i| e.get_post(i).map(|q| (q.x, q.y)).unwrap()).collect();
    // posts: 0/1 voltage coil, 2 current coil in, 3 current coil out
    let tag = |line: &str| {
        let mut it = line.split_whitespace();
        let kind = it.next().unwrap();
        let n: Vec<&str> = it.collect();
        match kind {
            "v" => format!(
                "<v x=\"{} {} {} {}\" f=\"0\" wf=\"0\" maxv=\"{}\"/>\n",
                n[0], n[1], n[2], n[3], n[7]
            ),
            "r" => {
                format!("<r x=\"{} {} {} {}\" f=\"0\" r=\"{}\"/>\n", n[0], n[1], n[2], n[3], n[5])
            }
            "g" => format!("<g x=\"{} {} {} {}\" f=\"0\"/>\n", n[0], n[1], n[2], n[3]),
            _ => format!("<w x=\"{} {} {} {}\" f=\"0\"/>\n", n[0], n[1], n[2], n[3]),
        }
    };
    let mut body = String::new();
    body += &tag(&format!("v {} {} {} {} 0 0 40 10 0 0 0.5", p[2].0, p[2].1 - 64, p[2].0, p[2].1));
    body += &tag(&ground(p[2].0, p[2].1 - 64));
    body += &tag(&format!("r {} {} {} {} 0 100", p[3].0, p[3].1, p[3].0, p[3].1 + 48));
    body += &tag(&ground(p[3].0, p[3].1 + 48));
    // the voltage coil sits across the load: its marked end on the load, the other on ground
    body += &tag(&format!("w {} {} {} {} 0", p[1].0, p[1].1, p[3].0, p[3].1));
    body += &tag(&ground(p[0].0, p[0].1));
    let r = parse(&xml(&p, &body), &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    for _ in 0..50 {
        sim.step().unwrap();
    }
    let power = sim.elements[0].get_power();
    assert!((power - 1.0).abs() < 0.01, "{power}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn an_ohmmeter_reports_the_resistance_across_it() {
    let line = "216 160 160 224 160 0 0.001";
    let p = posts_of(line);
    let text = format!(
        "{line}\nr {} {} {} {} 0 470\n{}{}",
        p[0].0,
        p[0].1,
        p[0].0,
        p[0].1 - 48,
        ground(p[0].0, p[0].1 - 48),
        ground(p[1].0, p[1].1)
    );
    let sim = run(&text, "0.000005", 20);
    let mut info = Vec::new();
    sim.elements[0].get_info(&mut info);
    assert!(info[1].contains("470"), "{info:?}");
}

#[cfg(feature = "analog")]
#[cfg(feature = "outputs")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[cfg(feature = "subcircuits")]
#[cfg(feature = "switches")]
#[cfg(feature = "transistors")]
#[test]
fn the_new_elements_survive_save_and_load() {
    use prometheuscs::format::circuit::Circuit;
    use prometheuscs::format::writer::write_xml;
    let reg = Registry::standard();
    for id in [
        "DPDTSwitchElm",
        "MBBSwitchElm",
        "CrossSwitchElm",
        "TimeDelayRelayElm",
        "AnalogMuxElm",
        "DCMotorElm",
        "OhmMeterElm",
        "WattmeterElm",
        "WattmeterTrueElm",
        "AmmeterElm",
        "ThreePhaseMotorElm",
        "MotorProtectionSwitchElm",
        "TriodeElm",
        "SCRElm",
        "TriacElm",
        "DiacElm",
        "NDarlingtonElm",
        "OTAElm",
        "CrystalElm",
        "GyratorElm",
    ] {
        let mut e = reg.create(id, 160, 160).unwrap_or_else(|| panic!("{id}"));
        e.base_mut().x2 = 288;
        e.base_mut().y2 = 160;
        e.set_points();
        let mut c = Circuit::new();
        c.elements.push(e);
        let first = write_xml(&c);
        let again = parse(&first, &reg);
        assert!(again.errors.is_empty(), "{id}: {:?}", again.errors);
        assert_eq!(again.circuit.elements.len(), 1, "{id}");
        assert_eq!(write_xml(&again.circuit), first, "{id}");
    }
}
