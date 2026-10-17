// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CircuitElm text helpers).
//! Formatting of electrical quantities with SI prefixes.

use crate::locale;
use std::sync::atomic::{AtomicUsize, Ordering};

static DECIMAL_DIGITS: AtomicUsize = AtomicUsize::new(3);
static SHORT_DECIMAL_DIGITS: AtomicUsize = AtomicUsize::new(1);

/// Sets the number of decimals shown in normal (`short = false`) or short value text.
pub fn set_decimal_digits(digits: usize, short: bool) {
    let target = if short { &SHORT_DECIMAL_DIGITS } else { &DECIMAL_DIGITS };
    target.store(digits, Ordering::Relaxed);
}

/// Number of decimals shown in normal value text.
pub fn decimal_digits() -> usize {
    DECIMAL_DIGITS.load(Ordering::Relaxed)
}

/// Formats `v` with at most the configured decimals and no trailing zeros (pattern
/// `####.###` of the original).
pub fn format(v: f64, short: bool) -> String {
    let digits = if short {
        SHORT_DECIMAL_DIGITS.load(Ordering::Relaxed)
    } else {
        DECIMAL_DIGITS.load(Ordering::Relaxed)
    };
    format_digits(v, digits, false)
}

/// Formats `v` with `digits` decimals; `fixed` keeps trailing zeros.
pub fn format_digits(v: f64, digits: usize, fixed: bool) -> String {
    let mut s = format!("{v:.digits$}");
    if !fixed && s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// Value with an SI prefix and unit, for example `4.7 k\u{3a9}`.
pub fn unit_text(v: f64, unit: &str) -> String {
    unit_text_impl(v, unit, false)
}

/// Like [`unit_text`] but without the space and with fewer decimals.
pub fn short_unit_text(v: f64, unit: &str) -> String {
    unit_text_impl(v, unit, true)
}

fn unit_text_impl(v: f64, unit: &str, short: bool) -> String {
    let sp = if short { "" } else { " " };
    let va = v.abs();
    if va < 1e-14 {
        return format!("0{sp}{unit}");
    }
    let (scaled, prefix) = if va < 1e-9 {
        (v * 1e12, "p")
    } else if va < 1e-6 {
        (v * 1e9, "n")
    } else if va < 1e-3 {
        (v * 1e6, locale::MU)
    } else if va < 1.0 {
        (v * 1e3, "m")
    } else if va < 1e3 {
        (v, "")
    } else if va < 1e6 {
        (v * 1e-3, "k")
    } else if va < 1e9 {
        (v * 1e-6, "M")
    } else if va < 1e12 {
        (v * 1e-9, "G")
    } else {
        return format!("{}{sp}{unit}", exponent_form(v));
    };
    format!("{}{sp}{prefix}{unit}", format(scaled, short))
}

fn exponent_form(v: f64) -> String {
    let s = format!("{v:.2e}");
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let m = m.trim_end_matches('0').trim_end_matches('.');
    let e: i32 = e.parse().unwrap_or(0);
    format!("{m}E{}{:03}", if e < 0 { "-" } else { "" }, e.abs())
}

/// Voltage text, for example `5 V`.
pub fn voltage_text(v: f64) -> String {
    unit_text(v, "V")
}

/// Absolute voltage text.
pub fn voltage_d_text(v: f64) -> String {
    unit_text(v.abs(), "V")
}

/// Current text, for example `2 mA`.
pub fn current_text(i: f64) -> String {
    unit_text(i, "A")
}

/// Absolute current text.
pub fn current_d_text(i: f64) -> String {
    unit_text(i.abs(), "A")
}

/// Time text, switching to minutes and hours for long durations.
pub fn time_text(mut v: f64) -> String {
    if v >= 60.0 {
        let h = (v / 3600.0).floor();
        v -= 3600.0 * h;
        let m = (v / 60.0).floor();
        v -= 60.0 * m;
        let pad = |x: f64| if x >= 10.0 { "" } else { "0" };
        if h == 0.0 {
            return format!("{}:{}{}", js_int(m), pad(v), format(v, false));
        }
        return format!("{}:{}{}:{}{}", js_int(h), pad(m), js_int(m), pad(v), format(v, false));
    }
    unit_text(v, "s")
}

fn js_int(x: f64) -> String {
    format!("{x}")
}

/// Value text for a fixed scale: auto, base unit, milli or micro.
pub fn unit_text_with_scale(val: f64, unit: &str, scale: Scale, fixed: bool) -> String {
    if val.abs() > 1e12 {
        return unit_text(val, unit);
    }
    let digits = decimal_digits();
    let nf = |x: f64| format_digits(x, digits, fixed);
    match scale {
        Scale::One => format!("{} {unit}", nf(val)),
        Scale::Milli => format!("{} m{unit}", nf(1e3 * val)),
        Scale::Micro => format!("{} {}{unit}", nf(1e6 * val), locale::MU),
        Scale::Auto => unit_text(val, unit),
    }
}

/// Display scale of a value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scale {
    /// Choose a prefix automatically.
    #[default]
    Auto,
    /// Base unit.
    One,
    /// Milli.
    Milli,
    /// Micro.
    Micro,
}

impl Scale {
    /// The scale chosen by index in an edit dialog (0 auto, 1 base unit, 2 milli, 3 micro).
    pub fn from_index(i: i32) -> Scale {
        match i {
            1 => Scale::One,
            2 => Scale::Milli,
            3 => Scale::Micro,
            _ => Scale::Auto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_si_prefixes() {
        assert_eq!(unit_text(4700.0, "\u{3a9}"), "4.7 k\u{3a9}");
        assert_eq!(unit_text(0.0005, "A"), "500 \u{3bc}A");
        assert_eq!(unit_text(0.0, "V"), "0 V");
        assert_eq!(unit_text(1.0, "V"), "1 V");
        assert_eq!(unit_text(2.5e-9, "F"), "2.5 nF");
        assert_eq!(unit_text(-3.3, "V"), "-3.3 V");
    }

    #[test]
    fn short_text_has_no_space_and_one_decimal() {
        assert_eq!(short_unit_text(1234.0, "\u{3a9}"), "1.2k\u{3a9}");
        assert_eq!(short_unit_text(100.0, ""), "100");
    }

    #[test]
    fn typed_text_with_multibyte_characters_is_not_a_number_and_not_a_crash() {
        for text in [
            "\u{20ac}a",
            "a\u{20ac}",
            "\u{e9}\u{e9}",
            "1\u{20ac}",
            "\u{1d11e}me",
            "5\u{6bd}eg",
            "1e\u{20ac}",
        ] {
            assert_eq!(parse_edit_value(text), None, "{text}");
        }
        // the plain cases still work
        assert_eq!(parse_edit_value("4k7"), Some(4700.0));
        assert_eq!(parse_edit_value("2 meg"), Some(2e6));
    }

    #[test]
    fn formats_time_and_scaled_values() {
        assert_eq!(time_text(0.0025), "2.5 ms");
        assert_eq!(time_text(75.5), "1:15.5");
        assert_eq!(unit_text_with_scale(0.0123, "A", Scale::Milli, false), "12.3 mA");
    }
}

/// Parses a value typed into an edit field: a plain or scientific number, optionally followed
/// by an SI suffix (`f p n u m k M G`, `meg`), with the `2k2` shorthand and an `rms` suffix.
pub fn parse_edit_value(text: &str) -> Option<f64> {
    let mut s = text.trim().to_string();
    let mut rms = 1.0;
    if let Some(rest) = s.strip_suffix("rms") {
        s = rest.trim().to_string();
        rms = std::f64::consts::SQRT_2;
    }
    // 2k2 -> 2.2k
    let chars: Vec<char> = s.chars().collect();
    if let Some(i) = chars.iter().position(|c| "pPnNuUmMkKgG".contains(*c)) {
        let before = &chars[..i];
        let after = &chars[i + 1..];
        let digits = |v: &[char]| !v.is_empty() && v.iter().all(char::is_ascii_digit);
        if digits(before) && digits(after) {
            s = format!(
                "{}.{}{}",
                before.iter().collect::<String>(),
                after.iter().collect::<String>(),
                chars[i]
            );
        }
    }
    // `get` gives nothing when the last three bytes start inside a character
    if s.len() >= 3 && s.get(s.len() - 3..).is_some_and(|t| t.eq_ignore_ascii_case("meg")) {
        s = format!("{}M", &s[..s.len() - 3]);
    }
    if let Ok(v) = s.parse::<f64>() {
        if s.chars().all(|c| !c.is_alphabetic() || c == 'e' || c == 'E') {
            return Some(v * rms);
        }
    }
    let last = s.chars().last()?;
    let mult = match last {
        'f' | 'F' => 1e-15,
        'p' | 'P' => 1e-12,
        'n' | 'N' => 1e-9,
        'u' | 'U' => 1e-6,
        'm' => 1e-3,
        'k' | 'K' => 1e3,
        'M' => 1e6,
        'G' | 'g' => 1e9,
        _ => 1.0,
    };
    let number = if mult == 1.0 { s.as_str() } else { s[..s.len() - last.len_utf8()].trim() };
    number.parse::<f64>().ok().map(|v| v * mult * rms)
}

/// Text for an edit field: the value with an SI suffix, or a plain number when dimensionless.
pub fn edit_value_text(v: f64, dimensionless: bool) -> String {
    use crate::format::number::js_number;
    let va = v.abs();
    if dimensionless || va.is_infinite() {
        return js_number(v);
    }
    if v == 0.0 {
        return "0".into();
    }
    let (scaled, suffix) = if va < 1e-12 {
        (v * 1e15, "f")
    } else if va < 1e-9 {
        (v * 1e12, "p")
    } else if va < 1e-6 {
        (v * 1e9, "n")
    } else if va < 1e-3 {
        (v * 1e6, "u")
    } else if va < 1.0 {
        (v * 1e3, "m")
    } else if va < 1e3 {
        (v, "")
    } else if va < 1e6 {
        (v * 1e-3, "k")
    } else if va < 1e9 {
        (v * 1e-6, "M")
    } else {
        (v * 1e-9, "G")
    };
    // trim float noise such as 4.700000000000001
    let rounded = (scaled * 1e9).round() / 1e9;
    format!("{}{suffix}", js_number(rounded))
}

#[cfg(test)]
mod edit_value_tests {
    use super::*;

    #[test]
    fn parses_suffixes_shorthand_and_scientific_notation() {
        let near = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() <= b.abs() * 1e-12);
        assert!(near(parse_edit_value("10k"), 1e4));
        assert!(near(parse_edit_value("2k2"), 2200.0));
        assert!(near(parse_edit_value("4.7u"), 4.7e-6));
        assert!(near(parse_edit_value("1meg"), 1e6));
        assert!(near(parse_edit_value("5M"), 5e6));
        assert!(near(parse_edit_value("3m"), 3e-3));
        assert!(near(parse_edit_value("1.2E+3"), 1200.0));
        assert!(near(parse_edit_value(" 100 "), 100.0));
        assert!(near(parse_edit_value("1rms"), std::f64::consts::SQRT_2));
        assert!(parse_edit_value("abc").is_none());
        assert!(parse_edit_value("").is_none());
    }

    #[test]
    fn formats_with_suffixes_and_round_trips() {
        assert_eq!(edit_value_text(1000.0, false), "1k");
        assert_eq!(edit_value_text(4.7e-6, false), "4.7u");
        assert_eq!(edit_value_text(0.0, false), "0");
        assert_eq!(edit_value_text(5.0, true), "5");
        for v in [1e-9, 2.2e-5, 0.47, 10.0, 4700.0, 3.3e6] {
            let t = edit_value_text(v, false);
            let back = parse_edit_value(&t).unwrap();
            assert!((back - v).abs() <= v * 1e-9, "{t}");
        }
    }
}
