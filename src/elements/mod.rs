// SPDX-License-Identifier: GPL-2.0-or-later
//! Circuit elements and the declarative element registry.

#[cfg(feature = "analog")]
pub mod analog;
#[cfg(feature = "digital")]
pub mod bus;
pub mod chip;
#[cfg(feature = "digital")]
pub mod digital;
#[cfg(feature = "diodes")]
pub mod diodes;
pub mod element;
pub mod geometry;
#[cfg(feature = "analog")]
pub mod hybrid;
pub mod junction;
#[cfg(feature = "logic")]
pub mod logic;
#[cfg(feature = "reactive")]
pub mod magnetic;
#[cfg(feature = "digital")]
pub mod memory;
#[cfg(feature = "analog")]
pub mod misc;
pub mod models;
#[cfg(feature = "mosfets")]
pub mod mosfets;
#[cfg(feature = "outputs")]
pub mod outputs;
#[cfg(feature = "passive")]
pub mod passive;
#[cfg(feature = "passive")]
pub mod pot;
#[cfg(feature = "reactive")]
pub mod reactive;
pub mod registry;
#[cfg(feature = "sources")]
pub mod sources;
#[cfg(feature = "sources")]
pub mod sweep;
#[cfg(feature = "switches")]
pub mod switches;
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
    #[cfg(feature = "sources")]
    sources::register(r);
    #[cfg(feature = "sources")]
    sweep::register(r);
    #[cfg(feature = "switches")]
    switches::register(r);
    #[cfg(feature = "outputs")]
    outputs::register(r);
    #[cfg(feature = "logic")]
    logic::register(r);
    #[cfg(feature = "diodes")]
    diodes::register(r);
    #[cfg(feature = "analog")]
    analog::register(r);
    #[cfg(feature = "analog")]
    hybrid::register(r);
    #[cfg(feature = "analog")]
    misc::register(r);
    #[cfg(feature = "transistors")]
    transistors::register(r);
    #[cfg(feature = "mosfets")]
    mosfets::register(r);
    #[cfg(feature = "digital")]
    digital::register(r);
    #[cfg(feature = "digital")]
    memory::register(r);
    #[cfg(feature = "digital")]
    bus::register(r);
    let _ = r;
}
