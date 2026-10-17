// SPDX-License-Identifier: GPL-2.0-or-later
//! Circuit elements and the declarative element registry.

#[cfg(feature = "analog")]
pub mod analog;
#[cfg(feature = "outputs")]
pub mod audio;
#[cfg(feature = "sources")]
pub mod battery;
#[cfg(feature = "logic")]
pub mod buffers;
#[cfg(feature = "digital")]
pub mod bus;
pub mod chip;
#[cfg(feature = "subcircuits")]
pub mod composite;
#[cfg(feature = "analog")]
pub mod controlled;
#[cfg(feature = "digital")]
pub mod custom_logic;
#[cfg(feature = "reactive")]
pub mod custom_transformer;
#[cfg(feature = "digital")]
pub mod digital;
#[cfg(feature = "diodes")]
pub mod diodes;
#[cfg(feature = "digital")]
pub mod displays;
pub mod element;
pub mod geometry;
#[cfg(feature = "analog")]
pub mod hybrid;
pub mod junction;
#[cfg(feature = "logic")]
pub mod logic;
#[cfg(feature = "reactive")]
pub mod magnetic;
#[cfg(feature = "outputs")]
pub mod measure;
#[cfg(feature = "digital")]
pub mod memory;
#[cfg(feature = "analog")]
pub mod misc;
pub mod model_edit;
pub mod models;
#[cfg(feature = "mosfets")]
pub mod mosfets;
#[cfg(feature = "outputs")]
pub mod motors;
#[cfg(feature = "subcircuits")]
pub mod opamps;
#[cfg(feature = "outputs")]
pub mod outputs;
#[cfg(feature = "analog")]
pub mod parts;
#[cfg(feature = "passive")]
pub mod passive;
#[cfg(feature = "passive")]
pub mod pot;
#[cfg(feature = "reactive")]
pub mod reactive;
pub mod registry;
// the coil of a relay is an inductor
#[cfg(all(feature = "switches", feature = "reactive"))]
pub mod relays;
#[cfg(feature = "sources")]
pub mod samples;
#[cfg(feature = "outputs")]
pub mod scope_elm;
#[cfg(feature = "digital")]
pub mod shifters;
#[cfg(feature = "sources")]
pub mod sources;
#[cfg(feature = "subcircuits")]
pub mod subcircuit;
#[cfg(feature = "sources")]
pub mod sweep;
#[cfg(feature = "switches")]
pub mod switches;
#[cfg(feature = "transistors")]
pub mod thyristors;
#[cfg(feature = "transistors")]
pub mod transistors;
pub mod units;

/// Registers every element family that is compiled into this build.
pub fn register_all(r: &mut registry::Registry) {
    #[cfg(feature = "passive")]
    passive::register(r);
    #[cfg(feature = "passive")]
    pot::register(r);
    #[cfg(feature = "reactive")]
    reactive::register(r);
    #[cfg(feature = "reactive")]
    magnetic::register(r);
    #[cfg(feature = "reactive")]
    custom_transformer::register(r);
    #[cfg(feature = "sources")]
    sources::register(r);
    #[cfg(feature = "sources")]
    battery::register(r);
    #[cfg(feature = "sources")]
    sweep::register(r);
    #[cfg(feature = "switches")]
    switches::register(r);
    #[cfg(all(feature = "switches", feature = "reactive"))]
    relays::register(r);
    #[cfg(feature = "outputs")]
    outputs::register(r);
    #[cfg(feature = "outputs")]
    scope_elm::register(r);
    #[cfg(feature = "outputs")]
    measure::register(r);
    #[cfg(feature = "outputs")]
    motors::register(r);
    #[cfg(feature = "outputs")]
    audio::register(r);
    #[cfg(feature = "logic")]
    logic::register(r);
    #[cfg(feature = "logic")]
    buffers::register(r);
    #[cfg(feature = "diodes")]
    diodes::register(r);
    #[cfg(feature = "analog")]
    analog::register(r);
    #[cfg(feature = "analog")]
    hybrid::register(r);
    #[cfg(feature = "analog")]
    controlled::register(r);
    #[cfg(feature = "analog")]
    misc::register(r);
    #[cfg(feature = "analog")]
    parts::register(r);
    #[cfg(feature = "transistors")]
    transistors::register(r);
    #[cfg(feature = "transistors")]
    thyristors::register(r);
    #[cfg(feature = "mosfets")]
    mosfets::register(r);
    #[cfg(feature = "digital")]
    digital::register(r);
    #[cfg(feature = "digital")]
    displays::register(r);
    #[cfg(feature = "digital")]
    custom_logic::register(r);
    #[cfg(feature = "digital")]
    memory::register(r);
    #[cfg(feature = "digital")]
    shifters::register(r);
    #[cfg(feature = "digital")]
    bus::register(r);
    #[cfg(feature = "subcircuits")]
    subcircuit::register(r);
    #[cfg(feature = "subcircuits")]
    composite::register(r);
    #[cfg(feature = "subcircuits")]
    opamps::register(r);
    let _ = r;
}
