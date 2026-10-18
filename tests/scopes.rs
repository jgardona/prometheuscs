// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Scopes defined by a circuit file fill with data while it runs, with and without a worker.

use prometheuscs::app::edit::ScopeMode;
use prometheuscs::app::session::Session;
use prometheuscs::app::settings::AppSettings;
use prometheuscs::app::worker::Worker;
use prometheuscs::elements::registry::Registry;
use prometheuscs::scope::POINTS;
use std::time::Duration;

fn lrc() -> String {
    std::fs::read_to_string("assets/circuits/lrc.txt").unwrap()
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "switches")]
#[test]
fn scopes_of_the_file_record_the_run() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    assert_eq!(s.scope_count(), 3);
    for _ in 0..100 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let views = s.scope_views();
    assert_eq!(views.len(), 3);
    let v = &views[0];
    assert_eq!(v.plots.len(), 2);
    assert_eq!(v.plots[0].max.len(), POINTS);
    let peak = v.plots[1].max.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    assert!(peak > 1e-3, "the current plot has data: {peak}");
}

#[test]
fn scope_data_comes_through_the_worker() {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(&lrc(), "lrc", None).unwrap();
    let mut peak = 0.0_f64;
    for _ in 0..100 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
        for v in s.scope_views() {
            for p in &v.plots {
                peak = peak.max(p.max.iter().fold(0.0_f64, |m, x| m.max(x.abs())));
            }
        }
    }
    assert!(peak > 1e-3, "the worker's scopes have data: {peak}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "switches")]
#[test]
fn toggling_a_switch_keeps_what_the_scopes_recorded() {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(&lrc(), "lrc", None).unwrap();
    for _ in 0..60 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let before: f64 = s.scope_views()[0].plots[1].max.iter().map(|x| x.abs()).sum();
    assert!(before > 0.0);
    assert!(s.press_at(416.0, 80.0), "the switch was hit");
    for _ in 0..3 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let after: f64 = s.scope_views()[0].plots[1].max.iter().map(|x| x.abs()).sum();
    assert!(after > 0.0, "history survived the restart");
}

fn select(s: &mut Session, class: &str) {
    let i = s.sim.elements.iter().position(|e| e.class_name() == class).unwrap();
    s.deselect_all();
    s.sim.elements[i].base_mut().selected = true;
}

#[cfg(feature = "sources")]
#[test]
fn view_in_scope_adds_a_scope_for_the_selected_element() {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(&lrc(), "lrc", None).unwrap();
    for _ in 0..40 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let before = s.scope_count();
    let first = s.scope_views()[0].plots[1].max.iter().map(|x| x.abs()).sum::<f64>();
    select(&mut s, "VoltageElm");
    assert_eq!(s.view_in_scope(), 1);
    assert_eq!(s.scope_count(), before + 1);
    for _ in 0..40 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let views = s.scope_views();
    assert_eq!(views.len(), before + 1);
    let last = views.last().unwrap();
    assert_eq!(last.plots.len(), 2, "voltage and current of a two-post element");
    assert!(last.plots[0].max.iter().any(|v| v.abs() > 1.0), "the source's voltage is recorded");
    let kept = views[0].plots[1].max.iter().map(|x| x.abs()).sum::<f64>();
    assert!(kept >= first * 0.5, "the older scope kept its history: {kept} vs {first}");
    // the scope is part of what is saved, and undo takes it away again
    assert!(s.circuit_xml().matches("<o ").count() > before);
    assert!(s.undo());
    assert_eq!(s.scope_count(), before);
}

#[test]
fn remove_scopes_clears_them_all() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    assert!(s.scope_count() > 0);
    s.remove_scopes();
    assert_eq!(s.scope_count(), 0);
    assert!(s.scope_views().is_empty());
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "switches")]
#[test]
fn deleting_an_element_renumbers_or_drops_its_scopes() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    let titles = |s: &Session| -> Vec<String> {
        s.scope_views().iter().map(|v| v.plots[0].title.clone()).collect()
    };
    for _ in 0..5 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let before = titles(&s);
    assert_eq!(before.len(), 3);
    select(&mut s, "ResistorElm");
    assert!(s.delete_selected());
    let after = titles(&s);
    assert_eq!(after.len(), 2, "the scope of the deleted resistor is gone: {after:?}");
    assert!(after.iter().all(|t| t.contains("inductor") || t.contains("capacitor")), "{after:?}");
}

fn plot_names(s: &Session) -> Vec<(i32, bool)> {
    s.scope_menu(s.scope_count() - 1).into_iter().map(|(v, _, on)| (v, on)).collect()
}

#[cfg(feature = "reactive")]
#[test]
fn a_scope_can_plot_power_and_the_charge_of_a_capacitor() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    select(&mut s, "CapacitorElm");
    s.view_in_scope();
    let last = s.scope_count() - 1;
    let items = s.scope_menu(last);
    let keys: Vec<&str> = items.iter().map(|i| i.1).collect();
    assert_eq!(keys, ["Show Voltage", "Show Current", "Show Power Consumed", "Show Charge"]);
    assert_eq!(
        plot_names(&s).iter().filter(|(_, on)| *on).count(),
        2,
        "voltage and current to start"
    );
    s.toggle_scope_value(last, prometheuscs::scope::VAL_POWER, true);
    s.toggle_scope_value(last, prometheuscs::scope::VAL_CHARGE, true);
    for _ in 0..30 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let view = s.scope_views().pop().unwrap();
    assert_eq!(view.plots.len(), 4);
    let charge = view.plots.iter().find(|p| p.units == prometheuscs::scope::UNITS_C).unwrap();
    assert!(charge.max.iter().any(|q| q.abs() > 1e-8), "the capacitor's charge is plotted");
    // turning one off again leaves the others
    s.toggle_scope_value(last, prometheuscs::scope::VAL_POWER, false);
    assert_eq!(plot_names(&s).iter().filter(|(_, on)| *on).count(), 3);
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "transistors")]
#[test]
fn a_transistor_scope_starts_with_vce_and_offers_its_currents() {
    let text = std::fs::read_to_string("assets/circuits/colpitts.txt").unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    let i = s.sim.elements.iter().position(|e| e.class_name().ends_with("TransistorElm")).unwrap();
    s.deselect_all();
    s.sim.elements[i].base_mut().selected = true;
    s.view_in_scope();
    let last = s.scope_count() - 1;
    let keys: Vec<&str> = s.scope_menu(last).iter().map(|i| i.1).collect();
    assert!(keys.contains(&"Show Ib") && keys.contains(&"Show Vbe"), "{keys:?}");
    s.toggle_scope_value(last, prometheuscs::scope::VAL_IB, true);
    for _ in 0..30 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let view = s.scope_views().pop().unwrap();
    assert_eq!(view.plots.len(), 2, "Vce and Ib");
    assert!(view.plots[0].max.iter().any(|v| v.abs() > 0.1), "Vce moves");
}

#[test]
fn removing_a_scope_by_index_keeps_the_others() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    let n = s.scope_count();
    s.remove_scope(0);
    assert_eq!(s.scope_count(), n - 1);
    assert_eq!(s.scope_views().len(), n - 1);
}

#[test]
fn an_empty_circuit_runs_without_stopping() {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.new_circuit();
    s.set_running(true);
    for _ in 0..20 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    assert!(s.stop_message.is_none(), "{:?}", s.stop_message);
}

#[cfg(feature = "reactive")]
#[test]
fn scopes_can_be_stacked_combined_and_separated() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    select(&mut s, "CapacitorElm");
    s.view_in_scope();
    select(&mut s, "InductorElm");
    s.view_in_scope();
    let count = s.scope_count();
    assert!(count >= 3);
    let positions = |s: &Session| -> Vec<i32> {
        (0..s.scope_count()).map(|i| s.sim.scopes[i].position).collect()
    };
    s.stack_scopes();
    assert!(positions(&s).iter().all(|p| *p == 0), "all in one place");
    s.unstack_scopes();
    assert_eq!(positions(&s), (0..count as i32).collect::<Vec<_>>());
    let plots: usize = s.sim.scopes.iter().map(|sc| sc.plots.len()).sum();
    s.combine_scopes();
    assert_eq!(s.scope_count(), 1);
    assert_eq!(s.sim.scopes[0].plots.len(), plots);
    s.separate_scopes();
    assert_eq!(s.scope_count(), plots, "one scope per plot");
    // the arrangement survives saving and loading
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    assert_eq!(again.scope_count(), plots);
    s.undo();
    assert_eq!(s.scope_count(), 1, "undo restores the combined scope");
}

#[test]
fn xy_and_spectrum_modes_are_kept_when_saving() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    let modes = |s: &Session| {
        [ScopeMode::Xy, ScopeMode::VoltageCurrent, ScopeMode::Spectrum].map(|m| s.scope_mode(0, m))
    };
    assert_eq!(modes(&s), [false, false, false]);
    s.set_scope_mode(0, ScopeMode::Xy, true);
    s.set_scope_mode(0, ScopeMode::Spectrum, true);
    assert_eq!(modes(&s), [true, false, true]);
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    assert_eq!(modes(&again), [true, false, true]);
    s.undo();
    assert_eq!(modes(&s), [true, false, false]);
    // voltage against current is the same drawing with the axes fixed
    s.set_scope_mode(0, ScopeMode::VoltageCurrent, true);
    assert_eq!(modes(&s), [false, true, false]);
}

#[test]
fn the_modes_are_saved_as_the_original_does() {
    // flags 64 + 128 + 1024 + 65536: X/Y with the spectrum in decibels; the axes are attributes
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    s.set_scope_mode(0, ScopeMode::Xy, true);
    let mut fields = s.scope_properties(0).unwrap();
    let n = fields.len();
    // the last nine: phase angle, log spectrum, X, Y, brightness, red, green, blue, trail
    let at = n - 9;
    fields[at].checkbox = Some(true);
    fields[at + 1].checkbox = Some(true);
    fields[at + 2].choice_index = 1;
    fields[at + 3].choice_index = 0;
    fields[at + 4].choice_index = 2; // "None" is the first choice
    fields[at + 8].value = 30.0;
    s.apply_scope_properties(0, &fields).unwrap();
    let saved = s.circuit_xml();
    let o = saved.lines().find(|l| l.contains("<o ")).unwrap();
    assert!(
        o.contains("xy2x=\"1\"") && o.contains("xy2y=\"0\"") && o.contains("xy2br=\"1\""),
        "{o}"
    );
    assert!(o.contains("tp=\"30\""), "{o}");
    let again = Session::load(&saved, &Registry::standard()).unwrap();
    let f = again.scope_properties(0).unwrap();
    assert_eq!(f[at + 2].choice_index, 1);
    assert_eq!(f[at + 4].choice_index, 2);
    assert_eq!(f[at + 8].value, 30.0);
    assert_eq!(f[at].checkbox, Some(true));
    assert_eq!(f[at + 1].checkbox, Some(true));
    let views = again.scope_views();
    assert!(views[0].xy && views[0].plot_xy);
    assert_eq!(views[0].axes.x, 1);
    // the original's flag bits are in the saved entry
    let flags = o.split("f=\"").nth(1).unwrap().split('"').next().unwrap();
    let bits = i32::from_str_radix(flags.trim_start_matches('x'), 16).unwrap();
    assert_eq!(bits & (64 | 128 | 65536 | (1 << 23)), 64 | 128 | 65536 | (1 << 23));
    assert_eq!(bits & 1024, 0);
}

#[test]
fn the_spectrum_mode_reaches_what_the_worker_publishes() {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(&lrc(), "lrc", None).unwrap();
    for _ in 0..20 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    s.set_scope_mode(0, ScopeMode::Spectrum, true);
    let mut shown = false;
    for _ in 0..60 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
        shown |= s.scope_views()[0].fft;
    }
    assert!(shown, "the published view of scope 0 shows the spectrum");
}

#[test]
fn the_scope_speed_changes_the_time_per_sample_and_is_kept() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    let before = s.scope_views()[0].time_per_sample;
    let speed = s.scope_speed(0);
    s.set_scope_speed(0, speed / 2);
    let after = s.scope_views()[0].time_per_sample;
    assert!((after - before / 2.0).abs() < before * 1e-9, "{before} -> {after}");
    s.set_scope_speed(0, 100_000);
    assert_eq!(s.scope_speed(0), 1024, "the speed stops at 1024");
    s.undo();
    s.undo();
    assert_eq!(s.scope_speed(0), speed);
}

#[cfg(feature = "outputs")]
#[test]
fn a_scope_can_be_undocked_into_the_circuit_and_docked_again() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    let (scopes, elements) = (s.scope_count(), s.sim.elements.len());
    assert!(s.undock_scope(0));
    assert_eq!(s.scope_count(), scopes - 1);
    assert_eq!(s.sim.elements.len(), elements + 1);
    assert_eq!(s.sim.embedded.len(), 1, "the circuit has a scope element now");
    // it survives saving and loading
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    assert_eq!(again.sim.embedded.len(), 1);
    // dock it back
    let elm = s.sim.embedded[0].0;
    s.sim.elements[elm].base_mut().selected = true;
    assert_eq!(s.selected_scope_element(), Some(elm));
    assert!(s.dock_scope(elm));
    assert_eq!(s.scope_count(), scopes);
    assert_eq!(s.sim.elements.len(), elements);
    assert!(s.sim.embedded.is_empty());
    s.undo();
    assert_eq!(s.sim.embedded.len(), 1, "undo brings the scope element back");
}

#[test]
fn plots_can_be_removed_and_the_data_exported() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    for _ in 0..100 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    s.combine_scopes();
    let plots = s.sim.scopes[0].plots.len();
    assert!(plots >= 2);
    let csv = s.scope_csv(0).expect("data");
    let header = csv.lines().next().unwrap();
    assert!(header.starts_with("time,"));
    assert_eq!(header.matches("min").count(), plots);
    let rows = csv.lines().count() - 1;
    assert!(rows > 10, "{rows} rows");
    let cells = csv.lines().nth(5).unwrap().split(',').count();
    assert_eq!(cells, 1 + 2 * plots);
    s.remove_scope_plot(0, 0);
    assert_eq!(s.sim.scopes[0].plots.len(), plots - 1);
    // the last plot takes the scope with it
    for _ in 0..plots {
        s.remove_scope_plot(0, 0);
    }
    assert_eq!(s.scope_count(), 0);
    s.undo();
    assert!(s.scope_count() > 0);
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "switches")]
#[test]
fn resetting_a_scope_empties_its_history() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    for _ in 0..100 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let peak =
        |s: &Session| s.scope_views()[0].plots[1].max.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    assert!(peak(&s) > 0.0);
    s.reset_scope(0);
    assert_eq!(peak(&s), 0.0);
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "sources")]
#[test]
fn after_a_speed_change_only_recorded_samples_are_valid_and_they_are_smooth() {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    let text = std::fs::read_to_string("assets/circuits/res-series.txt").unwrap();
    s.open_text(&text, "res", None).unwrap();
    for _ in 0..40 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    s.set_scope_speed(0, 4);
    for _ in 0..30 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let v = &s.scope_views()[0];
    let p = &v.plots[0];
    assert!(p.valid > 20 && p.valid < p.max.len(), "valid {}", p.valid);
    let start = p.max.len() - p.valid;
    let peak = p.max.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    let jump =
        (start + 1..p.max.len()).map(|k| (p.max[k] - p.max[k - 1]).abs()).fold(0.0, f64::max);
    assert!(
        jump < 0.1 * peak,
        "a jump of {jump} in a signal of {peak}: the first sample is a stray"
    );
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "switches")]
#[test]
fn a_scope_made_late_does_not_fill_its_first_samples_one_step_apart() {
    // the circuit runs for a while, then the speed changes: the new history starts at the
    // current time, with every sample as far apart as the speed says
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    for _ in 0..60 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    s.set_scope_speed(0, 128);
    for _ in 0..60 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let v = &s.scope_views()[0];
    let p = &v.plots[1];
    assert!(p.valid > 10, "{}", p.valid);
    let start = p.max.len() - p.valid;
    let mid: Vec<f64> = (start..p.max.len()).map(|k| 0.5 * (p.max[k] + p.min[k])).collect();
    let steps: Vec<f64> = mid.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    let mut sorted = steps.clone();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    assert!(
        steps[1] > median * 0.1,
        "the first steps {:?} against a median of {median}",
        &steps[..3]
    );
}

const SINE: &str = "$ 1 0.000005 10 50 5 50 5e-11
v 160 96 160 160 0 1 100 5 0 0 0.5
r 160 160 288 160 0 1000
w 288 160 288 96 0
w 288 96 160 96 0
g 160 96 160 112 0
o 1 64 0 4099 5 0.1 0 1 1 3
";

fn run(s: &mut Session, frames: usize) {
    for _ in 0..frames {
        s.update(Duration::from_millis(16)).unwrap();
    }
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn the_trigger_holds_the_display_on_the_crossing() {
    let mut s = Session::load(SINE, &Registry::standard()).unwrap();
    run(&mut s, 200);
    let mut f = s.scope_properties(0).unwrap();
    // Normal mode, rising, level 1 V
    f[4].choice_index = 1;
    f[5].choice_index = 0;
    f[6].value = 1.0;
    s.apply_scope_properties(0, &f).unwrap();
    run(&mut s, 400);
    let v = &s.scope_views()[0];
    let t = v.trigger.as_ref().expect("a scope with a trigger shows its state");
    assert!(t.triggered, "the sine crosses 1 V on the way up every period");
    let p = &v.plots[0];
    let at = p.max.len() - 1 - t.back;
    let mid = |k: usize| 0.5 * (p.max[k] + p.min[k]);
    assert!(mid(at - 1) < 1.0 && mid(at) >= 1.0, "{} -> {}", mid(at - 1), mid(at));
    // it is kept with the circuit
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    let f2 = again.scope_properties(0).unwrap();
    assert_eq!((f2[4].choice_index, f2[5].choice_index, f2[6].value), (1, 0, 1.0));
}

#[cfg(feature = "passive")]
#[test]
fn manual_scales_are_kept_per_plot_and_with_the_circuit() {
    let mut s = Session::load(SINE, &Registry::standard()).unwrap();
    run(&mut s, 50);
    let mut f = s.scope_properties(0).unwrap();
    assert_eq!(f[0].checkbox, Some(false));
    f[0].checkbox = Some(true);
    f[2].value = 10.0;
    f[7].value = 2.0;
    f[8].value = 1.0;
    s.apply_scope_properties(0, &f).unwrap();
    run(&mut s, 5);
    let v = &s.scope_views()[0];
    assert!(v.manual);
    assert_eq!(v.man_divisions, 10);
    assert_eq!(v.plots[0].man_scale, 2.0);
    assert_eq!(v.plots[0].man_pos, 20, "one division of ten is 20 of 200 steps");
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    let f2 = again.scope_properties(0).unwrap();
    assert_eq!(f2[0].checkbox, Some(true));
    assert_eq!((f2[2].value, f2[7].value, f2[8].value), (10.0, 2.0, 1.0));
    // wrong values are refused
    f[2].value = 1.0;
    assert!(s.apply_scope_properties(0, &f).is_err());
    f[2].value = 10.0;
    f[7].value = 0.0;
    assert!(s.apply_scope_properties(0, &f).is_err());
}

const BIASED: &str = "$ 1 0.000005 10 50 5 50 5e-11
v 160 96 160 160 0 1 100 5 3 0 0.5
r 160 160 288 160 0 1000
w 288 160 288 96 0
w 288 96 160 96 0
g 160 96 160 112 0
o 1 64 0 4099 5 0.1 0 1 1 3
";

fn run_until(s: &mut Session, t: f64) {
    for _ in 0..20_000 {
        if s.sim.time() >= t {
            return;
        }
        s.update(Duration::from_millis(16)).unwrap();
    }
}

fn mean_of_newest(s: &Session, count: usize) -> f64 {
    let p = &s.scope_views()[0].plots[0];
    let n = p.max.len();
    (n - count..n).map(|k| 0.5 * (p.max[k] + p.min[k])).sum::<f64>() / count as f64
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn ac_coupling_removes_the_dc_level_of_a_voltage_plot() {
    let mut s = Session::load(BIASED, &Registry::standard()).unwrap();
    let mut f = s.scope_properties(0).unwrap();
    f[3].value = 1.0; // one time step per sample: a short filter time constant
    s.apply_scope_properties(0, &f).unwrap();
    run_until(&mut s, 0.15);
    let dc = mean_of_newest(&s, 2000);
    assert!((dc - 3.0).abs() < 0.5, "the resistor sees 3 V of bias: {dc}");
    let mut f = s.scope_properties(0).unwrap();
    assert_eq!(f[9].checkbox, Some(false), "AC Coupled of the voltage plot");
    f[9].checkbox = Some(true);
    s.apply_scope_properties(0, &f).unwrap();
    run_until(&mut s, 0.4);
    let ac = mean_of_newest(&s, 2000);
    assert!(ac.abs() < 0.6, "the DC level is gone: {ac}");
    // kept with the circuit
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    assert_eq!(again.scope_properties(0).unwrap()[9].checkbox, Some(true));
}

#[cfg(feature = "passive")]
#[test]
fn the_measurements_to_show_are_chosen_in_the_properties_and_kept() {
    let mut s = Session::load(BIASED, &Registry::standard()).unwrap();
    let f = s.scope_properties(0).unwrap();
    // 7 fields, then scale, position and AC coupling of the one voltage plot
    let first_info = 7 + 3;
    // the nine measurements, then the phase angle and the log spectrum; the X/Y fields only
    // come with an X/Y plot
    assert_eq!(f.len(), first_info + 9 + 2);
    assert_eq!(f[first_info].checkbox, Some(true), "the peak value is shown by default");
    assert_eq!(f[first_info + 4].checkbox, Some(false), "RMS is off");
    let mut f = f;
    f[first_info].checkbox = Some(false); // no peak
    f[first_info + 2].checkbox = Some(true); // peak to peak
    f[first_info + 4].checkbox = Some(true); // RMS
    s.apply_scope_properties(0, &f).unwrap();
    let flags = s.scope_views()[0].flags;
    assert_ne!(flags & prometheuscs::scope::FLAG_NO_MAX, 0);
    assert_ne!(flags & prometheuscs::scope::FLAG_P2P, 0);
    assert_ne!(flags & prometheuscs::scope::FLAG_RMS, 0);
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    let g = again.scope_properties(0).unwrap();
    assert_eq!(
        (g[first_info].checkbox, g[first_info + 2].checkbox, g[first_info + 4].checkbox),
        (Some(false), Some(true), Some(true))
    );
}

#[cfg(feature = "passive")]
#[test]
fn a_plot_is_moved_with_the_mouse_only_in_manual_mode() {
    let mut s = Session::load(BIASED, &Registry::standard()).unwrap();
    assert_eq!(s.plot_position(0, 0), Some((false, 0)));
    let mut f = s.scope_properties(0).unwrap();
    f[0].checkbox = Some(true);
    s.apply_scope_properties(0, &f).unwrap();
    s.set_plot_position(0, 0, 50);
    assert_eq!(s.plot_position(0, 0), Some((true, 50)));
    assert_eq!(s.scope_views()[0].plots[0].man_pos, 50);
    s.set_plot_position(0, 0, 5000);
    assert_eq!(s.plot_position(0, 0), Some((true, 200)), "the position stops at the edge");
}

#[cfg(feature = "analog")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn a_lamp_glows_in_the_interface_copy_while_the_worker_heats_it() {
    // 100 W, 120 V lamp across a 120 V source; the filament warms in about 0.4 s
    let text = "$ 1 0.000005 10 50 5 50 5e-11\nv 160 96 160 160 0 0 40 120 0 0 0.5\n181 160 160 224 160 0 300 120 100 0.4 0.4\nw 224 160 224 96 0\nw 224 96 160 96 0\ng 160 96 160 112 0\n";
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(text, "lamp", None).unwrap();
    let lamp = s.sim.elements.iter().position(|e| e.class_name() == "LampElm").unwrap();
    let cold = s.sim.elements[lamp].visual_state()[0];
    for _ in 0..120 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let warm = s.sim.elements[lamp].visual_state()[0];
    assert!(warm > cold + 100.0, "the drawn filament temperature rose: {cold} -> {warm}");
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "transistors")]
#[test]
fn drawing_currents_of_transistors_reach_the_interface_copy() {
    // the current mirror example has bipolar transistors that conduct from the start
    let text = std::fs::read_to_string("assets/circuits/mirror.txt").unwrap();
    let text = text.as_str();
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(text, "q", None).unwrap();
    for _ in 0..40 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let q = s.sim.elements.iter().position(|e| e.class_name().ends_with("TransistorElm")).unwrap();
    let state = s.sim.elements[q].visual_state();
    assert_eq!(state.len(), 3);
    assert!(state.iter().any(|v| v.abs() > 1e-6), "the copy shows terminal currents: {state:?}");
}

#[cfg(feature = "outputs")]
#[test]
fn a_scope_placed_in_the_circuit_has_the_menu_and_the_properties_of_a_diagram() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    assert!(s.undock_scope(0));
    let elm = s.sim.embedded[0].0;
    let plots = s.embedded_scope(elm).unwrap().plots.len();
    assert!(plots >= 2);
    assert_eq!(s.embedded_plot_names(elm).len(), plots);
    assert!(!s.embedded_scope_menu(elm).is_empty());

    // a mode is kept in the element, so it survives saving
    s.set_embedded_scope_mode(elm, ScopeMode::Spectrum, true);
    assert!(s.embedded_scope_mode(elm, ScopeMode::Spectrum));
    let again = Session::load(&s.circuit_xml(), &Registry::standard()).unwrap();
    assert!(again.embedded_scope_mode(again.sim.embedded[0].0, ScopeMode::Spectrum));

    // the properties dialog changes the same settings
    let mut fields = s.embedded_scope_properties(elm).unwrap();
    fields[3].value = 16.0; // time steps per sample
    s.apply_embedded_scope_properties(elm, &fields).unwrap();
    assert_eq!(s.embedded_scope(elm).unwrap().speed, 16);
    fields[3].value = 0.0;
    assert!(s.apply_embedded_scope_properties(elm, &fields).is_err());
    assert_eq!(s.embedded_scope(elm).unwrap().speed, 16, "an invalid field changes nothing");

    // a plot can be dropped, and undo brings it back
    s.remove_embedded_plot(elm, 1);
    assert_eq!(s.embedded_scope(elm).unwrap().plots.len(), plots - 1);
    s.undo();
    assert_eq!(s.embedded_scope(elm).unwrap().plots.len(), plots);

    // and the last plot takes the element with it
    let elements = s.sim.elements.len();
    s.remove_embedded_scope(elm);
    assert_eq!(s.sim.elements.len(), elements - 1);
    assert!(s.sim.embedded.is_empty());
}

#[cfg(feature = "outputs")]
#[test]
fn the_samples_of_a_scope_in_the_circuit_can_be_exported() {
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(&lrc(), "lrc", None).unwrap();
    assert!(s.undock_scope(0));
    for _ in 0..40 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let elm = s.sim.embedded[0].0;
    let csv = s.embedded_scope_csv(elm).expect("the element has a view by now");
    assert!(csv.starts_with("time,"), "{}", &csv[..40.min(csv.len())]);
    assert!(csv.lines().count() > 2);
}

#[cfg(feature = "passive")]
#[cfg(feature = "reactive")]
#[cfg(feature = "switches")]
#[test]
fn samples_keep_their_extremes_and_the_buffer_wraps() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    // one step per sample: the buffer wraps within a few frames
    s.set_scope_speed(0, 1);
    for _ in 0..400 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let v = &s.scope_views()[0];
    for p in &v.plots {
        assert_eq!(p.min.len(), POINTS);
        assert_eq!(p.max.len(), POINTS);
        assert_eq!(p.valid, POINTS, "the buffer is full after the wrap");
        for (lo, hi) in p.min.iter().zip(&p.max) {
            assert!(lo <= hi, "a sample's smallest value is not above its largest");
        }
    }
    // with several steps in a sample the extremes of the sample differ on a changing signal
    s.set_scope_speed(0, 64);
    for _ in 0..200 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let v = &s.scope_views()[0];
    let spread = v.plots[0].min.iter().zip(&v.plots[0].max).filter(|(lo, hi)| hi > lo).count();
    assert!(spread > 0, "a changing signal has samples with a range");
}

#[test]
fn every_example_scope_survives_saving_and_loading() {
    let mut checked = 0;
    for entry in std::fs::read_dir("assets/circuits").unwrap() {
        let path = entry.unwrap().path();
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let Ok(mut first) = Session::load(&text, &Registry::standard()) else { continue };
        let before = first.scope_views();
        if before.is_empty() {
            continue;
        }
        let name = path.display().to_string();
        let again = Session::load(&first.circuit_xml(), &Registry::standard())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let after = again.scope_views();
        assert_eq!(before.len(), after.len(), "{name}: the number of scopes");
        for (a, b) in before.iter().zip(&after) {
            let same_plots = a.plots.len() == b.plots.len()
                && a.plots
                    .iter()
                    .zip(&b.plots)
                    .all(|(p, q)| p.units == q.units && p.value == q.value);
            assert!(same_plots, "{name}: the plots");
            assert_eq!(a.flags & !0x1000, b.flags & !0x1000, "{name}: the flags");
            assert_eq!(
                (a.position, a.xy, a.fft, a.manual),
                (b.position, b.xy, b.fft, b.manual),
                "{name}"
            );
            assert_eq!(a.man_divisions, b.man_divisions, "{name}: the divisions");
            assert!((a.time_per_sample - b.time_per_sample).abs() < 1e-18, "{name}: the speed");
            assert_eq!(a.text, b.text, "{name}: the label");
        }
        checked += 1;
    }
    assert!(checked > 20, "only {checked} examples have scopes");
}

/// Runs `file` and returns the picture its first X/Y scope has painted.
fn xy_picture(file: &str, seconds: f64) -> prometheuscs::scope::trace::Trace {
    let text = std::fs::read_to_string(format!("assets/circuits/{file}")).unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    s.sim.run_until(seconds).unwrap();
    s.sim.scopes.iter().find_map(|sc| sc.trace.clone()).expect("an X/Y scope with a picture")
}

fn lit_cells(t: &prometheuscs::scope::trace::Trace) -> usize {
    t.pixels.chunks(3).filter(|p| p.iter().any(|v| *v != 0)).count()
}

#[cfg(feature = "analog")]
#[cfg(feature = "digital")]
#[cfg(feature = "logic")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "outputs")]
#[test]
fn the_2d_scope_examples_paint_their_pictures_step_by_step() {
    // the smiley and the checkerboards are rasters scanned at 60 Hz and 12 kHz: they need
    // every time step, not the samples the time plots keep
    let smile = xy_picture("plot2d-smile.txt", 0.04);
    let color = xy_picture("plot2d-color.txt", 0.04);
    let checker = xy_picture("plot2d-checker.txt", 0.04);
    for (name, t) in [("smile", &smile), ("color", &color), ("checker", &checker)] {
        let n = lit_cells(t);
        assert!(n > 1000 && n < 512 * 192, "{name}: {n} cells lit");
    }
    // the colour example has colours, the others are white
    let coloured = |t: &prometheuscs::scope::trace::Trace| {
        t.pixels.chunks(3).any(|p| p[0] != p[1] || p[1] != p[2])
    };
    assert!(coloured(&color));
    assert!(!coloured(&checker));
}

#[cfg(feature = "sources")]
#[test]
fn lissajous_figures_are_drawn_from_every_step() {
    let t = xy_picture("lissa.txt", 0.05);
    assert!(lit_cells(&t) > 100, "{}", lit_cells(&t));
}

#[cfg(feature = "analog")]
#[cfg(feature = "digital")]
#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "outputs")]
#[test]
fn the_picture_reaches_the_view_and_is_emptied_by_reset() {
    let text = std::fs::read_to_string("assets/circuits/plot2d-smile.txt").unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    s.sim.run_until(0.03).unwrap();
    let views = prometheuscs::solver::sim::Simulation::snapshot(&s.sim, 1, true).scopes;
    assert!(views.iter().any(|v| v.trace.as_ref().is_some_and(|t| lit_cells(t) > 0)));
    for sc in &mut s.sim.scopes {
        sc.reset(0.0);
    }
    assert!(s.sim.scopes.iter().filter_map(|sc| sc.trace.as_ref()).all(|t| lit_cells(t) == 0));
}

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[cfg(feature = "outputs")]
#[test]
fn the_pointer_over_a_scope_highlights_what_it_plots_and_names_the_axes() {
    use prometheuscs::elements::element::DrawCtx;
    use prometheuscs::render::svg::SvgPainter;
    let text = std::fs::read_to_string("assets/circuits/lissa.txt").unwrap();
    let mut s = Session::load(&text, &Registry::standard()).unwrap();
    let roles = |s: &Session| -> Vec<(usize, String)> {
        s.sim
            .elements
            .iter()
            .enumerate()
            .filter_map(|(i, e)| Some((i, e.base().scope_role.clone()?)))
            .collect()
    };
    assert!(roles(&s).is_empty());
    s.set_scope_hover(Some(0));
    // the first scope of lissa.txt plots element 6 (X) against element 7 (Y)
    let r = roles(&s);
    assert_eq!(r.len(), 2, "{r:?}");
    assert!(r.contains(&(6, "X".to_string())) && r.contains(&(7, "Y".to_string())), "{r:?}");
    assert!(s.sim.elements[6].base().needs_highlight());
    // the terminals and the axis are drawn on the element
    let mut g = SvgPainter::new(900.0, 700.0);
    s.sim.elements[6].draw(&mut g, &DrawCtx::default());
    let svg = g.finish();
    assert!(svg.contains(">X<") && svg.contains(">+<"), "{svg}");
    // another scope, another pair; leaving clears them all
    s.set_scope_hover(Some(1));
    assert!(roles(&s).iter().any(|(i, r)| *i == 20 && r == "X"), "{:?}", roles(&s));
    s.set_scope_hover(None);
    assert!(roles(&s).is_empty());
    assert!(!s.sim.elements[6].base().needs_highlight());
}

#[cfg(feature = "outputs")]
#[test]
fn an_undocked_scope_can_be_dragged_by_any_point_of_it() {
    use prometheuscs::app::edit::Mods;
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    s.fit_pending = false;
    s.view = prometheuscs::render::scene::View { scale: 1.0, x: 0.0, y: 0.0 };
    assert!(s.undock_scope(0));
    let elm = s.sim.embedded[0].0;
    // draw it once so that its box is known
    let mut g = prometheuscs::render::svg::SvgPainter::new(2000.0, 2000.0);
    s.sim.elements[elm].draw(&mut g, &s.draw_ctx.clone());
    let b = s.sim.elements[elm].base().clone();
    let (x0, y0) = (b.x.min(b.x2), b.y.min(b.y2));
    // a point near a corner, far from the centre
    let (px, py) = (f64::from(x0 + 6), f64::from(y0 + 6));
    s.mouse_down(px, py, Mods::default(), 1);
    s.mouse_drag(px + 64.0, py + 48.0);
    s.mouse_up();
    let after = s.sim.elements[elm].base();
    assert_eq!((after.x.min(after.x2), after.y.min(after.y2)), (x0 + 64, y0 + 48));
}

#[cfg(feature = "passive")]
#[test]
fn one_scope_can_be_stacked_unstacked_and_combined_as_in_the_original() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    let position = |s: &Session| s.sim.scopes.iter().map(|c| c.position).collect::<Vec<_>>();
    assert_eq!(position(&s), [0, 1, 2]);
    assert!(s.can_stack_scope(1) && !s.can_unstack_scope(1));
    s.stack_scope(1); // scope 1 joins the column of scope 0; the next one moves up
    assert_eq!(position(&s), [0, 0, 1]);
    assert!(!s.can_stack_scope(1) && s.can_unstack_scope(1));
    s.unstack_scope(1);
    assert_eq!(position(&s), [0, 1, 2]);
    let plots = s.sim.scopes[0].plots.len() + s.sim.scopes[1].plots.len();
    s.combine_scope(1);
    assert_eq!(s.sim.scopes.len(), 2);
    assert_eq!(s.sim.scopes[0].plots.len(), plots);
    // stacking everything puts it in one column and hides the peak values, as the original
    s.stack_scopes();
    assert!(s.sim.scopes.iter().all(|c| c.position == 0));
    assert!(s.sim.scopes.iter().all(|c| c.flags & prometheuscs::scope::FLAG_NO_MAX != 0));
    // Max Scale is a flag of the scope
    assert!(!s.scope_max_scale(0));
    s.set_scope_max_scale(0, true);
    assert!(s.scope_max_scale(0));
}

#[test]
fn the_information_area_tells_the_time_and_what_the_pointer_is_over() {
    use prometheuscs::render::scopes::{info_width, on_wheel};
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    s.sim.run_until(0.002).unwrap();
    // nothing under the pointer: the time and the time step
    let info = s.info_lines();
    assert!(info[0].starts_with("t = ") && info[1].starts_with("time step = "), "{info:?}");
    // over a scope: what its first plot is of, and no longer the time
    s.set_scope_hover(Some(0));
    let over = s.info_lines();
    assert!(!over[0].starts_with("t = ") && over.len() > 1, "{over:?}");
    // the area exists only when the scopes leave room for it
    let views = s.scope_views();
    assert_eq!(info_width(&views, 300.0), 0.0);
    assert!(info_width(&views, 1000.0) > 0.0);
    // the settings wheel: the corner of a scope that is big enough
    assert!(on_wheel((0.0, 0.0, 300.0, 130.0), 10.0, 120.0));
    assert!(!on_wheel((0.0, 0.0, 300.0, 130.0), 100.0, 20.0));
    assert!(!on_wheel((0.0, 0.0, 90.0, 130.0), 10.0, 120.0));
}

#[cfg(feature = "outputs")]
#[test]
fn a_scope_placed_in_the_circuit_has_a_settings_wheel_and_max_scale() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    s.fit_pending = false;
    s.view = prometheuscs::render::scene::View { scale: 1.0, x: 0.0, y: 0.0 };
    assert!(s.undock_scope(0));
    let elm = s.sim.embedded[0].0;
    let b = s.sim.elements[elm].base().clone();
    let (x0, y0) = (b.x.min(b.x2), b.y.min(b.y2));
    let h = (b.y2 - b.y).abs();
    // its box is 128 by 64, too small for a wheel; enlarge it
    s.sim.elements[elm].base_mut().x2 = x0 + 160;
    s.sim.elements[elm].base_mut().y2 = y0 + 160;
    s.sim.elements[elm].base_mut().x = x0;
    s.sim.elements[elm].base_mut().y = y0;
    let _ = h;
    // the corner at the bottom left is the wheel, the middle is not
    assert_eq!(s.embedded_wheel_at(f64::from(x0 + 10), f64::from(y0 + 150)), Some(elm));
    assert_eq!(s.embedded_wheel_at(f64::from(x0 + 80), f64::from(y0 + 80)), None);
    assert!(!s.embedded_max_scale(elm));
    s.set_embedded_max_scale(elm, true);
    assert!(s.embedded_max_scale(elm));
    s.set_embedded_max_scale(elm, false);
    assert!(!s.embedded_max_scale(elm));
}

#[cfg(feature = "outputs")]
#[test]
fn selecting_a_scope_in_the_circuit_highlights_what_it_plots() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    assert!(s.undock_scope(0));
    let elm = s.sim.embedded[0].0;
    let plotted: Vec<usize> = s.embedded_scope(elm).unwrap().plots.iter().map(|p| p.elm).collect();
    let marked = |s: &Session| {
        (0..s.sim.elements.len())
            .filter(|&i| s.sim.elements[i].base().scope_role.is_some())
            .collect::<Vec<_>>()
    };
    s.update(Duration::from_millis(16)).unwrap();
    assert!(marked(&s).is_empty());
    s.deselect_all();
    s.sim.elements[elm].base_mut().selected = true;
    s.update(Duration::from_millis(16)).unwrap();
    let m = marked(&s);
    assert!(!m.is_empty() && m.iter().all(|i| plotted.contains(i)), "{m:?} of {plotted:?}");
    s.deselect_all();
    s.update(Duration::from_millis(16)).unwrap();
    assert!(marked(&s).is_empty());
}

#[cfg(feature = "outputs")]
#[test]
fn a_scope_in_the_circuit_shows_its_scale_once_it_is_big_enough() {
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    for _ in 0..40 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    assert!(s.undock_scope(0));
    for _ in 0..40 {
        s.update(Duration::from_millis(16)).unwrap();
    }
    let elm = s.sim.embedded[0].0;
    s.sim.push_embedded_views();
    let draw = |s: &mut Session| {
        let mut g = prometheuscs::render::svg::SvgPainter::new(900.0, 700.0);
        let ctx = s.draw_ctx.clone();
        s.sim.elements[elm].draw(&mut g, &ctx);
        g.finish()
    };
    // 128 by 64 is the size a scope has when it is made: only the curves fit, as in the original
    let b = s.sim.elements[elm].base();
    assert_eq!(((b.x2 - b.x).abs(), (b.y2 - b.y).abs()), (128, 64));
    assert!(!draw(&mut s).contains("/div"));
    // made bigger, it shows the time per division like a docked scope, and no line with the
    // element's name and its newest value (that is in the information area)
    let b = s.sim.elements[elm].base_mut();
    b.x2 = b.x + 420;
    b.y2 = b.y + 200;
    let big = draw(&mut s);
    assert!(big.contains("/div"));
    assert!(!big.contains(" V ="), "no line with the newest value");
}

#[cfg(feature = "outputs")]
#[test]
fn a_scope_in_the_circuit_is_resized_by_dragging_its_corner() {
    use prometheuscs::app::edit::Mods;
    let mut s = Session::load(&lrc(), &Registry::standard()).unwrap();
    s.fit_pending = false;
    s.view = prometheuscs::render::scene::View { scale: 1.0, x: 0.0, y: 0.0 };
    assert!(s.undock_scope(0));
    let elm = s.sim.embedded[0].0;
    let mut g = prometheuscs::render::svg::SvgPainter::new(2000.0, 2000.0);
    s.sim.elements[elm].draw(&mut g, &s.draw_ctx.clone());
    let size = |s: &Session| {
        let b = s.sim.elements[elm].base();
        ((b.x2 - b.x).abs(), (b.y2 - b.y).abs(), b.x.min(b.x2), b.y.min(b.y2))
    };
    let (w, h, x0, y0) = size(&s);
    assert_eq!((w, h), (128, 64));
    // the far corner is a handle; dragging it makes the scope bigger and leaves the near one
    let (cx, cy) = {
        let b = s.sim.elements[elm].base();
        (f64::from(b.x2), f64::from(b.y2))
    };
    s.mouse_down(cx, cy, Mods::default(), 1);
    s.mouse_drag(cx + 64.0, cy + 48.0);
    s.mouse_up();
    let (w2, h2, x1, y1) = size(&s);
    assert_eq!((w2, h2), (192, 112), "the size follows the corner");
    assert_eq!((x1, y1), (x0, y0), "the other corner stays");
    // it was a resize, not a move, and it can be undone
    assert!(s.undo());
    assert_eq!(size(&s).0, 128);
}
