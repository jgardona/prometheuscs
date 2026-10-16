// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (StringTokenizer, GNU Classpath).
//! Whitespace tokenizer with the same semantics as the original's `StringTokenizer`.

use std::str::FromStr;

/// Delimiters used for circuit lines: space, plus, tab and line breaks.
pub const CIRCUIT_DELIMS: &str = " +\t\n\r\x0c";

/// Error reading a field from a [`Tokenizer`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    /// The line ended before the field.
    #[error("missing field")]
    Missing,
    /// The field text is not a valid number.
    #[error("invalid number `{0}`")]
    Invalid(String),
}

/// Splits a line into tokens separated by any of a set of delimiter characters.
#[derive(Debug, Clone)]
pub struct Tokenizer<'a> {
    text: &'a str,
    pos: usize,
    delims: &'a str,
}

impl<'a> Tokenizer<'a> {
    /// Creates a tokenizer using the default delimiters (` \t\n\r\f`).
    pub fn new(text: &'a str) -> Self {
        Self::with_delims(text, " \t\n\r\x0c")
    }

    /// Creates a tokenizer using the given delimiters.
    pub fn with_delims(text: &'a str, delims: &'a str) -> Self {
        Tokenizer { text, pos: 0, delims }
    }

    /// Creates a tokenizer for a circuit line.
    pub fn for_circuit_line(text: &'a str) -> Self {
        Self::with_delims(text, CIRCUIT_DELIMS)
    }

    fn skip_delims(&mut self) {
        let rest = &self.text[self.pos..];
        let skipped = rest.len() - rest.trim_start_matches(|c| self.delims.contains(c)).len();
        self.pos += skipped;
    }

    /// True when another token is available.
    pub fn has_more_tokens(&mut self) -> bool {
        self.skip_delims();
        self.pos < self.text.len()
    }

    /// Returns the next token, or `None` at the end of the line.
    pub fn next_token(&mut self) -> Option<&'a str> {
        self.skip_delims();
        if self.pos >= self.text.len() {
            return None;
        }
        let rest = &self.text[self.pos..];
        let len = rest.find(|c| self.delims.contains(c)).unwrap_or(rest.len());
        self.pos += len;
        Some(&rest[..len])
    }

    /// Returns the next token using a different delimiter set from now on.
    pub fn next_token_with(&mut self, delims: &'a str) -> Option<&'a str> {
        self.delims = delims;
        self.next_token()
    }

    /// The rest of the line (without leading delimiters), for free-form text fields.
    pub fn remainder(&mut self) -> &'a str {
        self.skip_delims();
        let rest = &self.text[self.pos..];
        self.pos = self.text.len();
        rest
    }

    /// Reads the next token and parses it.
    pub fn next_parsed<T: FromStr>(&mut self) -> Result<T, TokenError> {
        let token = self.next_token().ok_or(TokenError::Missing)?;
        token.parse().map_err(|_| TokenError::Invalid(token.to_string()))
    }

    /// Reads the next token as an integer.
    pub fn next_i32(&mut self) -> Result<i32, TokenError> {
        self.next_parsed()
    }

    /// Reads the next token as a floating-point number.
    pub fn next_f64(&mut self) -> Result<f64, TokenError> {
        let token = self.next_token().ok_or(TokenError::Missing)?;
        parse_f64(token)
    }

    /// Number of tokens left.
    pub fn count_tokens(&self) -> usize {
        let mut copy = self.clone();
        let mut count = 0;
        while copy.next_token().is_some() {
            count += 1;
        }
        count
    }
}

/// Parses a number the way Java's `Double` does (accepts `NaN`, `Infinity`, exponents).
pub fn parse_f64(token: &str) -> Result<f64, TokenError> {
    let t = token.trim_end_matches(['d', 'D', 'f', 'F']);
    match t {
        "NaN" => Ok(f64::NAN),
        "Infinity" | "+Infinity" => Ok(f64::INFINITY),
        "-Infinity" => Ok(f64::NEG_INFINITY),
        _ => f64::from_str(t).map_err(|_| TokenError::Invalid(token.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_default_delimiters_and_collapses_runs() {
        let mut t = Tokenizer::new("  r   176\t80 ");
        assert_eq!(t.next_token(), Some("r"));
        assert_eq!(t.next_token(), Some("176"));
        assert_eq!(t.next_token(), Some("80"));
        assert_eq!(t.next_token(), None);
        assert!(!t.has_more_tokens());
    }

    #[test]
    fn circuit_lines_also_split_on_plus() {
        let mut t = Tokenizer::for_circuit_line("w 1+2 3");
        assert_eq!(t.count_tokens(), 4);
        assert_eq!(t.next_token(), Some("w"));
        assert_eq!(t.next_i32(), Ok(1));
        assert_eq!(t.next_i32(), Ok(2));
    }

    #[test]
    fn reports_missing_and_invalid_fields() {
        let mut t = Tokenizer::new("abc");
        assert_eq!(t.next_i32(), Err(TokenError::Invalid("abc".into())));
        assert_eq!(t.next_i32(), Err(TokenError::Missing));
    }

    #[test]
    fn parses_java_style_doubles() {
        assert_eq!(parse_f64("5.0E-6"), Ok(5e-6));
        assert_eq!(parse_f64("-1"), Ok(-1.0));
        assert!(parse_f64("NaN").unwrap().is_nan());
        assert_eq!(parse_f64("Infinity"), Ok(f64::INFINITY));
        assert!(parse_f64("x").is_err());
    }
}

/// Escapes text for one token of a circuit line (the original's `CustomLogicModel.escape`):
/// backslash, newline, space, `+`, `=`, `#`, `&` and carriage return, and `\0` for empty text.
pub fn escape(s: &str) -> String {
    if s.is_empty() {
        return "\\0".to_string();
    }
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            ' ' => out.push_str("\\s"),
            '+' => out.push_str("\\p"),
            '=' => out.push_str("\\q"),
            '#' => out.push_str("\\h"),
            '&' => out.push_str("\\a"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

/// Reverses [`escape`] (the original's `CustomLogicModel.unescape`); an unknown escape drops
/// the backslash.
pub fn unescape(s: &str) -> String {
    if s == "\\0" {
        return String::new();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('s') => out.push(' '),
            Some('p') => out.push('+'),
            Some('q') => out.push('='),
            Some('h') => out.push('#'),
            Some('a') => out.push('&'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod escape_tests {
    use super::*;

    #[test]
    fn escape_round_trips_every_special_character() {
        for t in ["", "a b", "x+y=z#1&2", "back\\slash", "line\nbreak\r", "plain"] {
            assert_eq!(unescape(&escape(t)), t, "{t:?}");
        }
        assert_eq!(escape(""), "\\0");
        assert_eq!(escape("a b"), "a\\sb");
        assert!(!escape("a b+c").contains(' '));
    }
}
