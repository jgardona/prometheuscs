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

    /// The source files of the program, but for this one.
    fn source_files() -> Vec<std::path::PathBuf> {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap() {
                let path = e.unwrap().path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|x| x == "rs")
                    && !path.ends_with("locale.rs")
                {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
        files
    }

    /// The string literals of the program's source (without its tests): `(file, literal)`.
    fn source_literals() -> Vec<(String, String)> {
        let files = source_files();
        let mut out = Vec::new();
        for f in &files {
            let text = std::fs::read_to_string(f).unwrap();
            let text = text.split("#[cfg(test)]").next().unwrap_or("");
            for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
                let mut chars = line.chars();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => {
                            let mut lit = String::new();
                            while let Some(c) = chars.next() {
                                match c {
                                    '\\' => {
                                        lit.push(c);
                                        lit.extend(chars.next());
                                    }
                                    '"' => break,
                                    c => lit.push(c),
                                }
                            }
                            out.push((f.display().to_string(), lit));
                        }
                        _ => {}
                    }
                }
            }
        }
        out
    }

    fn portuguese_translations() -> HashMap<String, String> {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        load_language("pt", &assets)
    }

    #[test]
    fn every_literal_looked_up_with_ls_has_a_portuguese_entry() {
        let pt = portuguese_translations();
        // keys of the original that have no Portuguese text in its locale file either
        let upstream_gaps = ["export", " average"];
        // looked up with a computed argument, so the scan cannot see them
        let mut keys: Vec<String> = [
            "Stop",
            "Run",
            "{count} entry could not be read. The rest of the circuit was loaded.",
            "{count} entries could not be read. The rest of the circuit was loaded.",
        ]
        .iter()
        .map(|k| (*k).to_string())
        .collect();
        let text_of = |lit: &str| lit.replace("\\u{3a9}", "\u{3a9}");
        for f in source_files() {
            let text = std::fs::read_to_string(f).unwrap();
            let text = text.split("#[cfg(test)]").next().unwrap_or("");
            for part in text.split("ls(\"").skip(1) {
                if let Some(end) = part.find('"') {
                    keys.push(text_of(&part[..end]));
                }
            }
        }
        let missing: Vec<&String> = keys
            .iter()
            .filter(|k| !upstream_gaps.contains(&k.as_str()))
            .filter(|k| !pt.contains_key(k.as_str()) && !pt.contains_key(k.trim_end_matches('~')))
            .collect();
        assert!(missing.is_empty(), "no Portuguese entry for: {missing:?}");
        assert!(keys.len() > 40, "the scan found only {} keys", keys.len());
    }

    #[test]
    fn the_portuguese_file_covers_what_the_program_once_added_in_code() {
        let pt = portuguese_translations();
        assert_eq!(pt["Mode~"], "Modo");
        assert_eq!(pt["Language"], "Idioma");
        assert_eq!(pt["Show \u{3a9} Unit"], "Mostrar Unidade \u{3a9}");
        assert_eq!(pt["Reset"], "Reiniciar");
    }

    #[test]
    fn no_portuguese_text_is_hard_coded_in_the_source() {
        const WORDS: [&str; 10] = [
            "Reiniciar",
            "Parar",
            "Executar",
            "Fechar",
            "Aplicar",
            "Copiar",
            "Importar",
            "Salvar",
            "Abrir",
            "Circuito",
        ];
        let found: Vec<(String, String)> = source_literals()
            .into_iter()
            .filter(|(_, lit)| {
                lit.chars().any(|c| "ãõçáéíóúâêôàÃÕÇ".contains(c))
                    || lit.split(|c: char| !c.is_alphabetic()).any(|w| WORDS.contains(&w))
            })
            .collect();
        assert!(found.is_empty(), "Portuguese text must live in locale_pt.txt: {found:?}");
    }

    #[test]
    fn untranslated_text_is_returned_unchanged() {
        assert_eq!(ls("resistor"), "resistor");
        assert_eq!(ls("Mode~"), "Mode");
        assert_eq!(ls(""), "");
    }
}
