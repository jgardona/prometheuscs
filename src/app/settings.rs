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
    /// Interface language: a code of `locale::LANGUAGES`, or `auto` to follow the system.
    pub language: String,
    /// Colour of positive voltages, as red, green and blue.
    pub positive_color: [u8; 3],
    /// Colour of negative voltages.
    pub negative_color: [u8; 3],
    /// Colour of zero voltage.
    pub neutral_color: [u8; 3],
    /// Colour of selected elements.
    pub selection_color: [u8; 3],
    /// Colour of the moving current dots.
    pub current_color: [u8; 3],
    /// Decimals shown in short value text.
    pub decimals_short: usize,
    /// Decimals shown in normal value text.
    pub decimals_long: usize,
    /// Developer mode of the original. It is stored only: nothing in the program reads it yet.
    pub developer_mode: bool,
    /// Frame rate below which the simulation stops keeping up with real time.
    pub min_frame_rate: u32,
    /// How much one notch of the mouse wheel changes a value (1 is a tenth).
    pub wheel_sensitivity: f64,
    /// Size of the text that shows component values.
    pub value_font_size: i32,
}

/// The colours of the dialog before any change: green, red, gray, cyan and yellow.
pub const DEFAULT_COLORS: [[u8; 3]; 5] =
    [[0, 255, 0], [255, 0, 0], [128, 128, 128], [0, 255, 255], [255, 255, 0]];

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
            language: "auto".into(),
            positive_color: DEFAULT_COLORS[0],
            negative_color: DEFAULT_COLORS[1],
            neutral_color: DEFAULT_COLORS[2],
            selection_color: DEFAULT_COLORS[3],
            current_color: DEFAULT_COLORS[4],
            decimals_short: 1,
            decimals_long: 3,
            developer_mode: false,
            min_frame_rate: 20,
            wheel_sensitivity: 1.0,
            value_font_size: 12,
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
    base.join(super::consts::CONFIG_DIR_NAME)
}

/// Where the settings file lives.
pub fn settings_path() -> PathBuf {
    config_dir().join(super::consts::SETTINGS_FILE)
}

/// Where the automatic save of the circuit being edited lives.
pub fn autosave_path() -> PathBuf {
    config_dir().join(super::consts::AUTOSAVE_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_tolerate_missing_fields() {
        let dir = std::env::temp_dir().join(crate::app::consts::temp_name("settings"));
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
