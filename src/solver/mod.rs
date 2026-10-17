// SPDX-License-Identifier: GPL-2.0-or-later
//! Modified Nodal Analysis solver: sparse LU, analysis, time stepping.
//!
//! A [`sim::Simulation`] owns the elements. [`sim::Simulation::analyze`] numbers the nodes and
//! checks the circuit; [`sim::Simulation::step`], [`sim::Simulation::run_steps`] and
//! [`sim::Simulation::run_until`] advance time. A circuit that cannot be solved is reported as
//! a [`crate::error::StepError`], never a panic.
//!
//! ```
//! use prometheuscs::elements::registry::Registry;
//! use prometheuscs::format::parser::parse;
//! use prometheuscs::solver::sim::Simulation;
//!
//! // 5 V, 1 kohm, 1 uF: after one time constant the capacitor is at 63 % of the supply
//! let text = "$ 1 0.000005 10 50 5 50 5e-11\n\
//!             v 0 64 0 0 0 0 40 5 0 0 0.5\n\
//!             r 0 0 64 0 0 1000\n\
//!             c 64 0 64 64 4 0.000001 0 0 0\n\
//!             w 64 64 0 64 0\n";
//! let r = parse(text, &Registry::standard());
//! let mut sim = Simulation::new(r.circuit.elements, r.circuit.options.sim_settings());
//! sim.analyze().unwrap();
//! sim.run_until(1e-3).unwrap();
//! let v = sim.elements[2].get_voltage_diff();
//! assert!((v - 5.0 * (1.0 - (-1.0f64).exp())).abs() < 0.05);
//! ```

pub mod dense_lu;
pub mod path;
pub mod sim;
pub mod snapshot;
pub mod sparse_lu;
pub mod stamper;
