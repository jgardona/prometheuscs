// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Locale).
//! Translation of interface strings. English text is the lookup key.

use std::collections::HashMap;
use std::sync::RwLock;

/// Greek capital omega, the ohm sign used in values.
pub const OHM: &str = "\u{03a9}";
/// Greek small mu, the micro prefix used in values.
pub const MU: &str = "\u{03bc}";

static MAP: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// Replaces the translation table.
pub fn set_translations(map: HashMap<String, String>) {
    if let Ok(mut guard) = MAP.write() {
        *guard = Some(map);
    }
}

/// Parses a translation file: one `"English"="Translation"` pair per line. The sequence `\q`
/// stands for a double quote, and the HTML wrapper the original puts around submenu titles is
/// removed so the text can be used directly.
pub fn parse_translations(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once("\"=") else { continue };
        let key = key.strip_prefix('"').unwrap_or(key);
        let value = value.strip_prefix('"').unwrap_or(value);
        let value = value.strip_suffix('"').unwrap_or(value);
        let clean = |s: &str| {
            let s = s.replace("\\q", "\"");
            let lower = s.to_ascii_lowercase();
            match lower.find("</div>") {
                Some(i) => s[i + "</div>".len()..].to_string(),
                None => s,
            }
        };
        map.insert(key.to_string(), clean(value));
        let stripped = clean(key);
        if stripped != key {
            map.insert(stripped, clean(value));
        }
    }
    map
}

/// Looks up the translation of `s`, falling back to the text itself. A trailing `~` marks
/// strings that need different translations in other contexts and is dropped when untranslated.
pub fn ls(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    if let Ok(guard) = MAP.read() {
        if let Some(found) = guard.as_ref().and_then(|m| m.get(s)) {
            return found.clone();
        }
        if let Some(stripped) = s.strip_suffix('~') {
            if let Some(found) = guard.as_ref().and_then(|m| m.get(stripped)) {
                return found.clone();
            }
            return stripped.to_string();
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pairs_and_strips_submenu_markup() {
        let m = parse_translations(
            "\"Add Resistor\"=\"Inserir Resistor\"\n\"&nbsp;</div>Digital Chips\"=\"&nbsp;</DIV>CIs Digitais\"\n\"a\\qb\"=\"c\\qd\"\nbroken\n",
        );
        assert_eq!(m["Add Resistor"], "Inserir Resistor");
        assert_eq!(m["Digital Chips"], "CIs Digitais");
        assert_eq!(m["a\\qb"], "c\"d");
    }

    #[test]
    fn untranslated_text_is_returned_unchanged() {
        assert_eq!(ls("resistor"), "resistor");
        assert_eq!(ls("Mode~"), "Mode");
        assert_eq!(ls(""), "");
    }
}
