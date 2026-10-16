// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Scope).
//! Scope data: sample buffers, plots, triggers and FFT.

/// Value shown by a scope plot: power.
pub const VAL_POWER: i32 = 7;
/// Old numbering of power in files.
pub const VAL_POWER_OLD: i32 = 1;
/// Value shown by a scope plot: voltage.
pub const VAL_VOLTAGE: i32 = 0;
/// Value shown by a scope plot: current.
pub const VAL_CURRENT: i32 = 3;
/// Transistor base current.
pub const VAL_IB: i32 = 1;
/// Transistor collector current.
pub const VAL_IC: i32 = 2;
/// Transistor emitter current.
pub const VAL_IE: i32 = 3;
/// Transistor base-emitter voltage.
pub const VAL_VBE: i32 = 4;
/// Transistor base-collector voltage.
pub const VAL_VBC: i32 = 5;
/// Transistor collector-emitter voltage.
pub const VAL_VCE: i32 = 6;
/// Resistance.
pub const VAL_R: i32 = 2;
/// Charge.
pub const VAL_CHARGE: i32 = 8;

/// Units of a plotted value: volts.
pub const UNITS_V: i32 = 0;
/// Units: amps.
pub const UNITS_A: i32 = 1;
/// Units: watts.
pub const UNITS_W: i32 = 2;
/// Units: ohms.
pub const UNITS_OHMS: i32 = 3;
/// Units: coulombs.
pub const UNITS_C: i32 = 4;
/// Number of unit kinds.
pub const UNITS_COUNT: i32 = 5;
