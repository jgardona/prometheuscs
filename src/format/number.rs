// SPDX-License-Identifier: GPL-2.0-or-later
//! Number formatting compatible with how the original writes circuit files.
//!
//! The original runs as JavaScript, where `Double.toString` follows `Number.prototype.toString`:
//! shortest round-trip digits, plain notation for exponents in `[-7, 21)`, exponent notation
//! otherwise.

/// Formats `value` like JavaScript's `String(value)`.
pub fn js_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if value == 0.0 {
        return "0".into();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    // `{:e}` yields the shortest round-trip digits: "d.ddde<exp>"
    let sci = format!("{:e}", value.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let exp: i32 = exp.parse().unwrap_or(0);
    let k = digits.len() as i32;
    let n = exp + 1; // position of the decimal point relative to the digits
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let sign_e = if e < 0 { "-" } else { "+" };
        if k == 1 {
            format!("{digits}e{sign_e}{}", e.abs())
        } else {
            format!("{}.{}e{sign_e}{}", &digits[..1], &digits[1..], e.abs())
        }
    };
    format!("{sign}{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_javascript_number_to_string() {
        assert_eq!(js_number(0.0), "0");
        assert_eq!(js_number(5.0), "5");
        assert_eq!(js_number(-2.5), "-2.5");
        assert_eq!(js_number(5e-6), "0.000005");
        assert_eq!(js_number(1.5e-7), "1.5e-7");
        assert_eq!(js_number(1e-7), "1e-7");
        assert_eq!(js_number(123456789.0), "123456789");
        assert_eq!(js_number(1e21), "1e+21");
        assert_eq!(js_number(1.2345e22), "1.2345e+22");
        assert_eq!(js_number(11.251013186076355), "11.251013186076355");
        assert_eq!(js_number(f64::INFINITY), "Infinity");
    }

    #[test]
    fn round_trips_through_parse() {
        for v in [0.1, 1.0 / 3.0, 1e-12, 6.02e23, 4.7e-5, -123.456] {
            assert_eq!(js_number(v).parse::<f64>().unwrap(), v);
        }
    }
}
