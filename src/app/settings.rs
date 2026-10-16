// SPDX-License-Identifier: GPL-2.0-or-later
//! Persistent application settings (the Options menu toggles).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Settings that outlive a circuit. Unknown or missing fields fall back to the defaults, so a
/// settings file from another version still loads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Show the toolbar under the menu bar.
    pub show_toolbar: bool,
    /// Draw a crosshair at the mouse position.
    pub crosshair: bool,
    /// Draw resistors as boxes.
    pub euro_resistors: bool,
    /// Add the ohm symbol to resistance values.
    pub show_ohm: bool,
    /// Draw logic gates with IEC rectangles.
    pub iec_gates: bool,
    /// Draw on a white background.
    pub white_background: bool,
    /// Current dots move from plus to minus.
    pub conventional_current: bool,
    /// Ignore every edit.
    pub disable_editing: bool,
    /// The mouse wheel edits the value of the element under the pointer.
    pub mouse_wheel_edit: bool,
    /// Defaults applied to circuits that do not store their own display options.
    pub show_dots: bool,
    /// Colour wires by voltage.
    pub show_volts: bool,
    /// Colour elements by power.
    pub show_power: bool,
    /// Show component values.
    pub show_values: bool,
    /// Use the small grid.
    pub small_grid: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            show_toolbar: true,
            crosshair: false,
            euro_resistors: false,
            show_ohm: false,
            iec_gates: false,
            white_background: false,
            conventional_current: true,
            disable_editing: false,
            mouse_wheel_edit: false,
            show_dots: true,
            show_volts: true,
            show_power: false,
            show_values: true,
            small_grid: false,
        }
    }
}

impl AppSettings {
    /// Reads settings from `path`, using the defaults when the file is missing or damaged.
    pub fn load(path: &Path) -> AppSettings {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Writes the settings to `path`, creating the folder when needed.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
}

/// The per-user configuration folder of the application.
pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("prometheuscs")
}

/// Where the settings file lives.
pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// Where the automatic save of the circuit being edited lives.
pub fn autosave_path() -> PathBuf {
    config_dir().join("autosave.xml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_tolerate_missing_fields() {
        let dir =
            std::env::temp_dir().join(format!("prometheuscs-settings-{}", std::process::id()));
        let path = dir.join("s.json");
        let s = AppSettings { crosshair: true, show_ohm: true, ..AppSettings::default() };
        s.save(&path).unwrap();
        assert_eq!(AppSettings::load(&path), s);
        std::fs::write(&path, r#"{"crosshair": true}"#).unwrap();
        let partial = AppSettings::load(&path);
        assert!(partial.crosshair && partial.show_toolbar);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(AppSettings::load(&path), AppSettings::default());
        let _ = std::fs::remove_dir_all(dir);
    }
}
