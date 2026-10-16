// SPDX-License-Identifier: GPL-2.0-or-later
//! The relm4 / libadwaita application.

pub mod commands;
#[cfg(feature = "gui")]
pub mod dialogs;
pub mod edit;
pub mod engine;
pub mod examples;
pub mod file;
pub mod icons;
#[cfg(feature = "gui")]
pub mod menus;
pub mod session;
pub mod settings;
#[cfg(feature = "gui")]
pub mod toolbar;
#[cfg(feature = "gui")]
pub mod window;
pub mod worker;

/// Starts the application and runs its main loop.
#[cfg(feature = "gui")]
pub fn run() {
    use relm4::RelmApp;
    // RUST_LOG selects what is logged; warnings and errors by default
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
    RelmApp::new("org.prometheuscs.App").run::<window::App>(());
}
