// SPDX-License-Identifier: GPL-2.0-or-later
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, missing_docs)]
//! Solver throughput on the three benchmark circuits of `tests/bench/` (made by
//! `cargo run --bin bench_circuits`): time steps per second of a small RLC circuit, a ladder of
//! 500 elements and a mixed nonlinear circuit of 1100 elements. Run with `cargo bench`.

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use prometheuscs::elements::registry::Registry;
use prometheuscs::format::parser::parse;
use prometheuscs::solver::sim::Simulation;

fn load(name: &str) -> Simulation {
    let text = std::fs::read_to_string(format!("tests/bench/{name}.txt"))
        .expect("run `cargo run --bin bench_circuits` first");
    let r = parse(&text, &Registry::standard());
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
    sim.analyze().expect("analysis");
    sim.reset();
    // let the start-up transient (and the first factorization) pass
    sim.run_steps(50).expect("warm-up");
    sim
}

fn steps(c: &mut Criterion) {
    let mut group = c.benchmark_group("steps");
    for (name, count) in [("rlc", 20_000usize), ("ladder", 1_000), ("mixed", 200)] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_function(name, |b| {
            b.iter_batched_ref(
                || load(name),
                |sim| sim.run_steps(count).expect("steps"),
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

fn analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("analysis");
    for name in ["ladder", "mixed"] {
        let text = std::fs::read_to_string(format!("tests/bench/{name}.txt"))
            .expect("run `cargo run --bin bench_circuits` first");
        group.bench_function(name, |b| {
            b.iter(|| {
                let r = parse(&text, &Registry::standard());
                let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
                sim.analyze().expect("analysis");
            });
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20).measurement_time(std::time::Duration::from_secs(4));
    targets = steps, analysis
}
criterion_main!(benches);
