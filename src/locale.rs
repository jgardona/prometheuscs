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
static CURRENT: RwLock<String> = RwLock::new(String::new());

/// The languages with a translation file (`assets/locale/locale_<code>.txt`), and their names
/// in their own language. English is the language of the keys and needs no file.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("pt", "Português"),
    ("csx", "Čeština"),
    ("da", "Dansk"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("fi", "Suomi"),
    ("fr", "Français"),
    ("it", "Italiano"),
    ("ja", "日本語"),
    ("kr", "한국어"),
    ("nb", "Norsk bokmål"),
    ("pl", "Polski"),
    ("ru", "Русский"),
    ("zh-tw", "繁體中文"),
    ("zh", "简体中文"),
];

/// The code of the language in use.
pub fn current_language() -> String {
    CURRENT.read().map(|c| c.clone()).unwrap_or_default()
}

/// Remembers the code of the language in use.
pub fn set_current_language(code: &str) {
    if let Ok(mut c) = CURRENT.write() {
        *c = code.to_string();
    }
}

/// The supported language that matches a locale name such as `pt_BR.UTF-8`; English when none
/// does.
pub fn language_from_locale(name: &str) -> &'static str {
    let name = name.to_ascii_lowercase().replace('_', "-");
    let name = name.split(['.', '@']).next().unwrap_or("");
    if name.starts_with("zh") {
        return if name.contains("tw") || name.contains("hk") || name.contains("hant") {
            "zh-tw"
        } else {
            "zh"
        };
    }
    let lang = name.split('-').next().unwrap_or("");
    let lang = match lang {
        "cs" => "csx",
        "ko" => "kr",
        "no" | "nn" => "nb",
        other => other,
    };
    LANGUAGES.iter().map(|(c, _)| *c).find(|c| *c == lang).unwrap_or("en")
}

/// The language the environment asks for (`LC_ALL`, `LC_MESSAGES`, `LANG`).
pub fn detect_language() -> &'static str {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() && v != "C" && v != "POSIX" {
                return language_from_locale(&v);
            }
        }
    }
    "en"
}

/// Reads the translation file of `code` from `assets/locale`. Missing files and English give an
/// empty table, so every string falls back to its English text.
pub fn load_language(code: &str, assets: &std::path::Path) -> HashMap<String, String> {
    if code == "en" {
        return HashMap::new();
    }
    let path = assets.join("locale").join(format!("locale_{code}.txt"));
    std::fs::read_to_string(path).map(|t| parse_translations(&t)).unwrap_or_default()
}

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
    fn locale_names_map_to_supported_languages() {
        assert_eq!(language_from_locale("pt_BR.UTF-8"), "pt");
        assert_eq!(language_from_locale("zh_TW.UTF-8"), "zh-tw");
        assert_eq!(language_from_locale("zh_CN"), "zh");
        assert_eq!(language_from_locale("ko_KR"), "kr");
        assert_eq!(language_from_locale("cs_CZ"), "csx");
        assert_eq!(language_from_locale("sw_KE"), "en");
    }

    #[test]
    fn every_listed_language_has_a_translation_file() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for (code, _) in LANGUAGES.iter().filter(|(c, _)| *c != "en") {
            assert!(!load_language(code, &assets).is_empty(), "{code}");
        }
    }

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
