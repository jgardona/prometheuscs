// SPDX-License-Identifier: GPL-2.0-or-later
//! Names and defaults the application shares between its modules.

use std::path::PathBuf;

/// The name shown to the user (window title, About dialog).
pub const APP_NAME: &str = "PrometheusCS";
/// The lower-case program name, from `Cargo.toml`; prefixes temporary files and print jobs.
pub const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");
/// The application id registered with GTK.
pub const APP_ID: &str = "org.prometheuscs.App";

/// Folder under the user's configuration directory. It holds the settings, the autosave, the
/// icon cache and the saved subcircuits: changing it leaves the data of earlier versions behind.
pub const CONFIG_DIR_NAME: &str = "prometheuscs";
/// The settings file, inside the configuration folder.
pub const SETTINGS_FILE: &str = "settings.json";
/// The automatic save of the circuit being edited, inside the configuration folder.
pub const AUTOSAVE_FILE: &str = "autosave.xml";
/// The saved subcircuits, inside the configuration folder.
pub const SUBCIRCUITS_DIR: &str = "subcircuits";
/// The recoloured icons, inside the configuration folder.
pub const ICON_CACHE_DIR: &str = "icon-cache";

/// Environment variable that points at the folder with the bundled assets.
pub const ENV_ASSETS: &str = "PROMETHEUSCS_ASSETS";
/// Environment variable that, set to `cairo`, forces the CPU canvas.
pub const ENV_RENDERER: &str = "PROMETHEUSCS_RENDERER";

/// The list of bundled examples, inside the assets folder.
pub const SETUPLIST_FILE: &str = "setuplist.txt";
/// The circuit opened at start-up when the example list names none.
pub const FALLBACK_STARTUP: &str = "lrc.txt";

/// Initial width of the main window.
pub const DEFAULT_WINDOW_WIDTH: i32 = 1280;
/// Initial height of the main window.
pub const DEFAULT_WINDOW_HEIGHT: i32 = 800;
/// Width of the side panel.
pub const SIDE_PANEL_WIDTH: i32 = 270;

/// Filter pattern for circuit files saved as XML.
pub const GLOB_XML: &str = "*.xml";
/// Filter pattern for the oscilloscope data export.
pub const GLOB_CSV: &str = "*.csv";
/// Filter pattern for the audio export.
pub const GLOB_WAV: &str = "*.wav";
/// Filter pattern for the image export.
pub const GLOB_PNG: &str = "*.png";
/// Filter pattern for the SVG export.
pub const GLOB_SVG: &str = "*.svg";
/// Suggested name of the oscilloscope data export.
pub const SCOPE_DATA_FILE: &str = "scope-data.csv";
/// Suggested name of the audio export.
pub const AUDIO_FILE: &str = "audio.wav";
/// Suggested name of the image export.
pub const IMAGE_FILE: &str = "circuito.png";
/// Suggested name of the SVG export.
pub const SVG_FILE: &str = "circuito.svg";
/// Patterns of the files the open dialog accepts.
pub const CIRCUIT_GLOBS: [&str; 3] = ["*.txt", "*.xml", "*.circuitjs"];

/// A name for a temporary file or folder of this program, for example `prometheuscs-audio-42`.
pub fn temp_name(purpose: &str) -> String {
    format!("{PROGRAM_NAME}-{purpose}-{}", std::process::id())
}

/// Folder with the bundled circuits and `setuplist.txt`.
pub fn assets_dir() -> PathBuf {
    std::env::var_os(ENV_ASSETS)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_folder_keeps_the_name_of_earlier_versions() {
        assert_eq!(CONFIG_DIR_NAME, "prometheuscs");
        assert!(super::super::settings::config_dir().ends_with(CONFIG_DIR_NAME));
    }

    #[test]
    fn the_program_name_comes_from_cargo() {
        assert_eq!(PROGRAM_NAME, "prometheuscs");
        assert!(temp_name("audio").starts_with("prometheuscs-audio-"));
    }

    #[test]
    fn every_glob_matches_its_default_file_name() {
        for (glob, file) in [
            (GLOB_CSV, SCOPE_DATA_FILE),
            (GLOB_WAV, AUDIO_FILE),
            (GLOB_PNG, IMAGE_FILE),
            (GLOB_SVG, SVG_FILE),
        ] {
            assert!(file.ends_with(glob.trim_start_matches('*')), "{file} vs {glob}");
        }
    }
}
