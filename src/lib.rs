// SPDX-License-Identifier: GPL-2.0-or-later
//! prometheuscs: a Rust port of the CircuitJS1 circuit simulator.
//!
//! The crate is organised in layers. The engine modules ([`mod@format`], [`elements`],
//! [`solver`], [`scope`], [`expr`]) never depend on the GUI, so they can be tested and
//! benchmarked without a display. [`render`] draws circuits through a backend-agnostic
//! painter, and [`app`] holds the relm4 application.
//!
//! # Running a circuit without a window
//!
//! ```
//! # #[cfg(all(feature = "passive", feature = "sources"))]
//! # fn main() {
//! use prometheuscs::elements::registry::Registry;
//! use prometheuscs::format::parser::parse;
//! use prometheuscs::solver::sim::Simulation;
//!
//! // 5 V across a 1 kohm and a 3 kohm resistor in series: the middle is at 3.75 V
//! let text = "$ 1 0.000005 10 50 5 50 5e-11\n\
//!             v 0 128 0 0 0 0 40 5 0 0 0.5\n\
//!             r 0 0 64 0 0 1000\n\
//!             r 64 0 64 128 0 3000\n\
//!             w 64 128 0 128 0\n";
//! let parsed = parse(text, &Registry::standard());
//! assert!(parsed.errors.is_empty());
//! let mut sim = Simulation::new(parsed.circuit.elements, parsed.circuit.options.sim_settings());
//! sim.analyze().unwrap();
//! sim.run_steps(3).unwrap();
//! let middle = sim.elements[1].get_post_voltage(1);
//! assert!((middle - 3.75).abs() < 1e-9);
//! # }
//! # #[cfg(not(all(feature = "passive", feature = "sources")))]
//! # fn main() {}
//! ```

pub mod app;
pub mod elements;
pub mod error;
pub mod expr;
pub mod format;
pub mod locale;
pub mod render;
pub mod scope;
pub mod solver;
