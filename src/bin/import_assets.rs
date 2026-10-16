// SPDX-License-Identifier: GPL-2.0-or-later
//! Developer tool: copies examples, the example catalog, locales and icons from a CircuitJS1
//! checkout into `assets/`.
//!
//! Usage: `cargo run --bin import_assets [path-to-circuitjs1]`
//! (default `/home/jgardona/Workspace/Rust/circuitjs1`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_SOURCE: &str = "/home/jgardona/Workspace/Rust/circuitjs1";
const PUBLIC: &str = "src/com/lushprojects/circuitjs1/public";
const TOOLBAR: &str = "src/com/lushprojects/circuitjs1/client/Toolbar.java";
const FONTELLO: &str = "war/font/fontello.svg";

fn main() -> ExitCode {
    let source = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| DEFAULT_SOURCE.into()));
    match run(&source, Path::new("assets")) {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("import_assets: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(source: &Path, dest: &Path) -> Result<String, String> {
    let public = source.join(PUBLIC);
    let circuits = copy_matching(&public.join("circuits"), &dest.join("circuits"), |_| true)?;
    fs::copy(public.join("setuplist.txt"), dest.join("setuplist.txt"))
        .map_err(|e| format!("copying setuplist.txt: {e}"))?;
    let locales = copy_matching(&public, &dest.join("locale"), |name| {
        name.starts_with("locale_") && name.ends_with(".txt")
    })?;
    let toolbar = fs::read_to_string(source.join(TOOLBAR))
        .map_err(|e| format!("reading Toolbar.java: {e}"))?;
    let component_icons = write_component_icons(&toolbar, &dest.join("icons/components"))?;
    let fontello = fs::read_to_string(source.join(FONTELLO))
        .map_err(|e| format!("reading fontello.svg: {e}"))?;
    let ui_icons = write_ui_icons(&fontello, &dest.join("icons/ui"))?;
    Ok(format!(
        "imported {circuits} circuits, {locales} locale files, {component_icons} component icons, \
         {ui_icons} UI icons"
    ))
}

fn copy_matching(from: &Path, to: &Path, accept: impl Fn(&str) -> bool) -> Result<usize, String> {
    fs::create_dir_all(to).map_err(|e| format!("creating {}: {e}", to.display()))?;
    let mut count = 0;
    let entries = fs::read_dir(from).map_err(|e| format!("reading {}: {e}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_file() && accept(&name) {
            fs::copy(entry.path(), to.join(&name)).map_err(|e| format!("copying {name}: {e}"))?;
            count += 1;
        }
    }
    Ok(count)
}

/// Extracts every `final String <name>Icon = "..." + "...";` definition from `Toolbar.java`.
fn extract_icon_definitions(java: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut lines = java.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("final String ") else {
            continue;
        };
        let Some((name, value)) = rest.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if !name.ends_with("Icon") {
            continue;
        }
        let mut text = value.to_string();
        while !text.trim_end().ends_with(';') {
            match lines.next() {
                Some(next) => {
                    text.push('\n');
                    text.push_str(next);
                }
                None => break,
            }
        }
        result.push((name.trim_end_matches("Icon").to_string(), join_string_literals(&text)));
    }
    result
}

/// Concatenates all Java string literals found in `text`, resolving `\"` and `\\` escapes.
fn join_string_literals(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some(other) => out.push(other),
                    None => {}
                },
                other => out.push(other),
            }
        }
    }
    out
}

/// Wraps the inner part of a `<svg>...</svg>` fragment into a standalone 24x24 SVG document.
fn standalone_svg(fragment: &str) -> Option<String> {
    let inner = fragment.trim().strip_prefix("<svg>")?.strip_suffix("</svg>")?;
    Some(format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'>\
         {inner}</svg>\n"
    ))
}

fn write_component_icons(java: &str, dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let mut count = 0;
    for (name, fragment) in extract_icon_definitions(java) {
        let Some(svg) = standalone_svg(&fragment) else {
            continue;
        };
        fs::write(dir.join(format!("{name}.svg")), svg).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

/// Returns `(glyph-name, path-data, advance-width)` for each glyph of an SVG font.
fn extract_glyphs(svg_font: &str) -> Vec<(String, String, f64)> {
    svg_font
        .lines()
        .filter(|l| l.starts_with("<glyph "))
        .filter_map(|l| {
            Some((
                attribute(l, "glyph-name")?,
                attribute(l, "d")?,
                attribute(l, "horiz-adv-x").and_then(|v| v.parse().ok()).unwrap_or(1000.0),
            ))
        })
        .collect()
}

fn attribute(line: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=\"");
    let start = line.find(&key)? + key.len();
    let end = line[start..].find('"')?;
    Some(line[start..start + end].to_string())
}

fn write_ui_icons(fontello: &str, dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let glyphs = extract_glyphs(fontello);
    for (name, path, width) in &glyphs {
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 {width} \
             1000'><path fill='currentColor' transform='translate(0,850) scale(1,-1)' \
             d='{path}'/></svg>\n"
        );
        fs::write(dir.join(format!("{name}.svg")), svg).map_err(|e| e.to_string())?;
    }
    Ok(glyphs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_concatenated_literals_and_unescapes() {
        let text = "\"<svg>\" +\n \"a\\\"b\" + \"</svg>\";";
        assert_eq!(join_string_literals(text), "<svg>a\"b</svg>");
    }

    #[test]
    fn extracts_icon_definitions_across_lines() {
        let java =
            "    final String wireIcon = \"<svg>\" +\n      \"<g/></svg>\";\n    int x = 1;\n";
        let defs = extract_icon_definitions(java);
        assert_eq!(defs, vec![("wire".to_string(), "<svg><g/></svg>".to_string())]);
    }

    #[test]
    fn wraps_fragment_in_standalone_document() {
        let svg = standalone_svg("<svg><g/></svg>").unwrap();
        assert!(svg.starts_with("<svg xmlns="));
        assert!(svg.contains("<g/>"));
        assert!(standalone_svg("not svg").is_none());
    }

    #[test]
    fn reads_glyph_attributes() {
        let line = "<glyph glyph-name=\"floppy\" unicode=\"&#xe800;\" d=\"M0 0z\" horiz-adv-x=\"857.1\" />";
        assert_eq!(extract_glyphs(line), vec![("floppy".to_string(), "M0 0z".to_string(), 857.1)]);
    }
}
