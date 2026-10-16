// SPDX-License-Identifier: GPL-2.0-or-later
//! Reader for the original's auto-test files: a circuit in XML followed by the recorded run
//! (`<test len>`, `<switchevent>` and `<scopedata>` tags).

use prometheuscs::format::xml::{XmlNode, parse};

/// A switch operated while the recorded run was made.
#[derive(Clone, Debug)]
pub struct SwitchEvent {
    /// Simulated time of the event.
    pub time: f64,
    /// Index of the switch element.
    pub element: usize,
}

/// Samples one scope recorded.
#[derive(Clone, Debug)]
pub struct ScopeData {
    /// Scope index.
    pub scope: usize,
    /// Plot index inside the scope.
    pub plot: usize,
    /// Element index the plot watches.
    pub element: usize,
    /// Value type.
    pub value: i32,
    /// Units code.
    pub units: i32,
    /// Seconds per sample.
    pub sample_time: f64,
    /// The recorded samples.
    pub values: Vec<f64>,
}

/// One recorded run.
#[derive(Clone, Debug)]
pub struct AutoTest {
    /// File name without extension.
    pub name: String,
    /// The whole file, loadable by the circuit parser.
    pub text: String,
    /// Simulated length of the run.
    pub length: f64,
    /// Switch operations during the run.
    pub switch_events: Vec<SwitchEvent>,
    /// Recorded scope data.
    pub scope_data: Vec<ScopeData>,
}

fn int(n: &XmlNode, name: &str) -> usize {
    usize::try_from(n.int_attr(name, 0)).unwrap_or(0)
}

/// Reads one auto-test file.
pub fn read(name: &str, text: &str) -> AutoTest {
    let root = parse(text).expect("auto-test XML");
    let mut t = AutoTest {
        name: name.to_string(),
        text: text.to_string(),
        length: 0.0,
        switch_events: Vec::new(),
        scope_data: Vec::new(),
    };
    for c in &root.children {
        match c.tag.as_str() {
            "test" => t.length = c.double_attr("len", 0.0),
            "switchevent" => t
                .switch_events
                .push(SwitchEvent { time: c.double_attr("t", 0.0), element: int(c, "en") }),
            "scopedata" => t.scope_data.push(ScopeData {
                scope: int(c, "si"),
                plot: int(c, "pi"),
                element: int(c, "en"),
                value: c.int_attr("v", 0),
                units: c.int_attr("u", 0),
                sample_time: c.double_attr("ts", 0.0),
                values: c.double_array(),
            }),
            _ => {}
        }
    }
    t
}

/// File names of the fixtures under `tests/compat/auto/`.
pub const NAMES: &[&str] =
    &["analogmux", "capac", "fullrect-sc", "lrc", "lrcswitch", "ota-vca", "td4"];

/// Loads a fixture by name.
pub fn load(name: &str) -> AutoTest {
    let path = format!("tests/compat/auto/{name}.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    read(name, &text)
}

/// Loads every fixture.
pub fn all() -> Vec<AutoTest> {
    NAMES.iter().map(|n| load(n)).collect()
}
