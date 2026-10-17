// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "outputs")]
//! The audio output records the voltage at its post.

use prometheuscs::elements::audio::AudioOutput;
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

#[cfg(feature = "passive")]
#[cfg(feature = "sources")]
#[test]
fn it_records_samples_at_the_sampling_rate() {
    // a 0.5 V source straight into the audio output (post 1 at the left end)
    let text = "$ 1 0.000005 10 50 5 50 5e-11\n\
                v 0 0 0 -64 0 0 40 0.5 0 0 0.5\n\
                g 0 0 0 16 0\n\
                w 0 -64 64 -64 0\n\
                211 64 -64 128 -64 0 1 8000 1\n";
    let r = parse(text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    sim.run_until(0.01).unwrap();
    let a = sim
        .elements
        .iter()
        .find_map(|e| e.as_any().downcast_ref::<AudioOutput>())
        .expect("audio element");
    let samples = a.samples();
    assert!((75..=85).contains(&samples.len()), "{} samples in 10 ms at 8 kHz", samples.len());
    assert!(samples.iter().skip(2).all(|s| (s - 0.5).abs() < 1e-3), "{samples:?}");
    assert_eq!(a.sampling_rate(), 8000);
}

#[test]
fn every_audio_example_loads() {
    let mut n = 0;
    for entry in std::fs::read_dir("assets/circuits").unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        if text.lines().any(|l| l.starts_with("211 ") || l.contains("<aout ")) {
            let r = parse(&text, &Registry::standard());
            assert!(
                !r.errors.iter().any(|e| e.to_string().contains("211")),
                "{}: {:?}",
                path.display(),
                r.errors
            );
            n += 1;
        }
    }
    assert!(n >= 10, "{n} audio examples");
}

#[test]
fn a_recording_comes_back_from_the_worker_as_a_wav_file() {
    use prometheuscs::app::session::Session;
    use prometheuscs::app::settings::AppSettings;
    use prometheuscs::app::worker::Worker;
    use std::time::Duration;
    let text = "$ 1 0.000005 10 50 5 50 5e-11\n\
                v 0 0 0 -64 0 0 40 0.5 0 0 0.5\n\
                g 0 0 0 16 0\n\
                w 0 -64 64 -64 0\n\
                211 64 -64 128 -64 0 1 8000 1\n";
    let mut s = Session::empty_with(&Registry::standard(), AppSettings::default());
    s.attach_worker(Worker::spawn(Registry::standard(), AppSettings::default()));
    s.open_text(text, "audio", None).unwrap();
    let outputs = s.audio_outputs();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].1, "Audio Out");
    // nothing to play before the circuit has run
    for _ in 0..30 {
        s.update(Duration::from_millis(16)).unwrap();
        std::thread::sleep(Duration::from_millis(16));
    }
    let wav = s.audio_wav(outputs[0].0).expect("the worker recorded samples");
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert!(wav.len() > 44, "{} bytes", wav.len());
    // an element that is not an audio output has no recording
    assert!(s.audio_wav(9999).is_none());
}
