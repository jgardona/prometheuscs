// SPDX-License-Identifier: GPL-2.0-or-later
//! prometheuscs: a Rust port of the CircuitJS1 circuit simulator.
//!
//! The crate is organised in layers. The engine modules ([`format`], [`elements`],
//! [`solver`], [`scope`], [`expr`]) never depend on the GUI, so they can be tested and
//! benchmarked without a display. [`render`] draws circuits through a backend-agnostic
//! painter, and [`app`] holds the relm4 application.

pub mod app;
pub mod elements;
pub mod error;
pub mod expr;
pub mod format;
pub mod locale;
pub mod render;
pub mod scope;
pub mod solver;
