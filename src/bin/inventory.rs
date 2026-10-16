// SPDX-License-Identifier: GPL-2.0-or-later
//! Developer tool: builds the component inventory from the original CircuitJS1 sources.
//!
//! Scans every `*Elm.java` in the `client` package and writes `assets/inventory.json` with one
//! entry per class: name, parent class, whether it is abstract, its file-format dump type and
//! whether the original can instantiate it (two-int or six-argument constructor).
//!
//! Usage: `cargo run --bin inventory [path-to-circuitjs1]`.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_SOURCE: &str = "/home/jgardona/Workspace/Rust/circuitjs1";
const CLIENT: &str = "src/com/lushprojects/circuitjs1/client";

/// One element class of the original simulator.
#[derive(Debug, PartialEq, Serialize)]
struct Entry {
    class: String,
    extends: Option<String>,
    is_abstract: bool,
    /// Dump type: a character (for example `r`) or a number, as written in `getDumpType`.
    dump_type: Option<String>,
    /// True when the original factory can create it (placement or file constructor).
    instantiable: bool,
}

fn main() -> ExitCode {
    let source = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| DEFAULT_SOURCE.into()));
    match run(&source) {
        Ok(count) => {
            println!("inventory: {count} element classes written to assets/inventory.json");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("inventory: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(source: &Path) -> Result<usize, String> {
    let dir = source.join(CLIENT);
    let mut names: Vec<String> = fs::read_dir(&dir)
        .map_err(|e| format!("reading {}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with("Elm.java"))
        .collect();
    names.sort();
    let mut entries = Vec::new();
    for name in &names {
        let text = fs::read_to_string(dir.join(name)).map_err(|e| format!("{name}: {e}"))?;
        if let Some(entry) = parse_class(name.trim_end_matches(".java"), &text) {
            entries.push(entry);
        }
    }
    let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    fs::write("assets/inventory.json", json + "\n").map_err(|e| e.to_string())?;
    Ok(entries.len())
}

fn parse_class(class: &str, text: &str) -> Option<Entry> {
    let declaration = text.lines().find(|l| {
        let l = l.trim_start();
        !l.starts_with("//") && l.contains(&format!("class {class}"))
    })?;
    let is_abstract = declaration.contains("abstract ");
    let extends = declaration
        .split_once(" extends ")
        .map(|(_, rest)| {
            rest.split_whitespace().next().unwrap_or("").trim_end_matches('{').to_string()
        })
        .filter(|s| !s.is_empty());
    Some(Entry {
        class: class.to_string(),
        extends,
        is_abstract,
        dump_type: dump_type(text),
        instantiable: !is_abstract && has_instantiating_constructor(class, text),
    })
}

fn dump_type(text: &str) -> Option<String> {
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || !trimmed.contains("getDumpType()") {
            continue;
        }
        let mut window = String::from(trimmed);
        for _ in 0..3 {
            if let Some(next) = lines.peek() {
                if !next.trim_start().starts_with("//") {
                    window.push(' ');
                    window.push_str(next.trim());
                }
                lines.next();
            }
        }
        let after = window.split_once("return ")?.1;
        let value = after.split(';').next()?.trim();
        return Some(value.trim_matches('\'').to_string());
    }
    None
}

fn has_instantiating_constructor(class: &str, text: &str) -> bool {
    let pattern = format!("{class}(");
    text.lines().any(|line| {
        let l = line.trim_start();
        if l.starts_with("//") || l.starts_with("new ") || l.contains(" new ") {
            return false;
        }
        let Some(pos) = l.find(&pattern) else {
            return false;
        };
        if l[..pos].contains('=') || l[..pos].contains("return") {
            return false;
        }
        l[pos + pattern.len()..].trim_start().starts_with("int ")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESISTOR: &str = "class ResistorElm extends CircuitElm {\n    public ResistorElm(int xx, int yy) {}\n    int getDumpType() { return 'r'; }\n}";

    #[test]
    fn parses_character_dump_type_and_constructor() {
        let entry = parse_class("ResistorElm", RESISTOR).unwrap();
        assert_eq!(entry.dump_type.as_deref(), Some("r"));
        assert_eq!(entry.extends.as_deref(), Some("CircuitElm"));
        assert!(entry.instantiable && !entry.is_abstract);
    }

    #[test]
    fn numeric_dump_type_and_abstract() {
        let text = "abstract class ChipElm extends CircuitElm {\n int getDumpType() {\n return 167;\n }\n ChipElm(int x, int y) {}}";
        let entry = parse_class("ChipElm", text).unwrap();
        assert_eq!(entry.dump_type.as_deref(), Some("167"));
        assert!(entry.is_abstract && !entry.instantiable);
    }

    #[test]
    fn ignores_commented_dump_type() {
        let text = "class A extends B {\n//    abstract int getDumpType();\n}";
        assert_eq!(parse_class("A", text).unwrap().dump_type, None);
    }
}
