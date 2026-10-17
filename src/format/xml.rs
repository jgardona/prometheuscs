// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (XMLDeserializer).
//! A small XML reader for the circuit files that use the XML format, plus typed attribute access.

use crate::error::LoadError;

/// A parsed XML element.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct XmlNode {
    /// Tag name.
    pub tag: String,
    /// Attributes in document order, with entities already decoded.
    pub attrs: Vec<(String, String)>,
    /// Child elements in document order.
    pub children: Vec<XmlNode>,
    /// Concatenated text content directly inside this element.
    pub text: String,
}

impl XmlNode {
    /// Returns an attribute value.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// Integer attribute, or `default` when absent or unparsable.
    pub fn int_attr(&self, name: &str, default: i32) -> i32 {
        self.attr(name).and_then(|v| v.trim().parse().ok()).unwrap_or(default)
    }

    /// Floating-point attribute, or `default` when absent or unparsable.
    pub fn double_attr(&self, name: &str, default: f64) -> f64 {
        self.attr(name)
            .and_then(|v| crate::format::tokenizer::parse_f64(v.trim()).ok())
            .unwrap_or(default)
    }

    /// Boolean attribute, or `default` when absent.
    pub fn bool_attr(&self, name: &str, default: bool) -> bool {
        self.attr(name).map_or(default, |v| v.eq_ignore_ascii_case("true"))
    }

    /// String attribute, or `default` when absent.
    pub fn string_attr(&self, name: &str, default: &str) -> String {
        self.attr(name).map_or_else(|| default.to_string(), str::to_string)
    }

    /// Converts back to a writable element, keeping attribute order.
    pub fn to_out(&self) -> XmlOut {
        XmlOut {
            tag: self.tag.clone(),
            attrs: self.attrs.clone(),
            children: self.children.iter().map(XmlNode::to_out).collect(),
            text: (!self.text.trim().is_empty()).then(|| self.text.clone()),
        }
    }

    /// Comma-separated list of numbers in the text content.
    pub fn double_array(&self) -> Vec<f64> {
        let text = self.text.trim();
        if text.is_empty() {
            return Vec::new();
        }
        text.split(',').filter_map(|p| crate::format::tokenizer::parse_f64(p.trim()).ok()).collect()
    }
}

/// An XML element under construction. Attributes keep insertion order, as in files written by
/// the original.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct XmlOut {
    /// Tag name.
    pub tag: String,
    /// Attributes in insertion order.
    pub attrs: Vec<(String, String)>,
    /// Child elements.
    pub children: Vec<XmlOut>,
    /// Text content, written before the children.
    pub text: Option<String>,
}

impl XmlOut {
    /// Creates an element with the given tag.
    pub fn new(tag: &str) -> Self {
        XmlOut { tag: tag.to_string(), ..XmlOut::default() }
    }

    /// Adds a string attribute (escaped on output).
    pub fn str_attr(&mut self, name: &str, value: &str) -> &mut Self {
        self.attrs.push((name.to_string(), value.to_string()));
        self
    }

    /// Adds an integer attribute.
    pub fn int_attr(&mut self, name: &str, value: i32) -> &mut Self {
        self.str_attr(name, &value.to_string())
    }

    /// Adds a floating-point attribute written like JavaScript's `String(value)`.
    pub fn double_attr(&mut self, name: &str, value: f64) -> &mut Self {
        self.str_attr(name, &crate::format::number::js_number(value))
    }

    /// Adds a boolean attribute.
    pub fn bool_attr(&mut self, name: &str, value: bool) -> &mut Self {
        self.str_attr(name, if value { "true" } else { "false" })
    }

    /// Appends a child element.
    pub fn child(&mut self, c: XmlOut) -> &mut Self {
        self.children.push(c);
        self
    }

    /// Pretty-prints the element with two-space indentation, like the original.
    pub fn to_pretty_string(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out
    }

    fn write_pretty(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        out.push_str(&pad);
        out.push('<');
        out.push_str(&self.tag);
        for (k, v) in &self.attrs {
            out.push_str(&format!(" {k}=\"{}\"", escape_attr(v)));
        }
        let text = self.text.as_deref().map(str::trim).filter(|t| !t.is_empty());
        if text.is_none() && self.children.is_empty() {
            out.push_str("/>\n");
            return;
        }
        out.push('>');
        if let Some(t) = text {
            out.push_str(&escape_text(t));
        }
        if !self.children.is_empty() {
            out.push('\n');
            for c in &self.children {
                c.write_pretty(out, indent + 1);
            }
            out.push_str(&pad);
        }
        out.push_str(&format!("</{}>\n", self.tag));
    }
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('\'', "&apos;").replace('<', "&lt;")
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Decodes the five predefined XML entities.
pub fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Encodes text for use in an attribute value.
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Parses an XML document and returns its root element.
pub fn parse(text: &str) -> Result<XmlNode, LoadError> {
    let mut p = Parser { s: text.as_bytes(), text, pos: 0, depth: 0 };
    p.skip_prolog()?;
    let root = p.element()?;
    Ok(root)
}

/// How deep elements may nest. The parser calls itself for each level (as does dropping the
/// tree), so a damaged file of endless open tags would overflow the stack. The deepest circuit
/// has a few levels: a model inside a model inside a circuit.
const MAX_DEPTH: usize = 256;

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn err(&self, msg: &str) -> LoadError {
        LoadError::Xml(format!("{msg} at byte {}", self.pos))
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    fn starts_with(&self, pat: &str) -> bool {
        self.text[self.pos..].starts_with(pat)
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn skip_until(&mut self, end: &str) -> Result<(), LoadError> {
        match self.text[self.pos..].find(end) {
            Some(i) => {
                self.pos += i + end.len();
                Ok(())
            }
            None => Err(self.err(&format!("unterminated construct, expected {end}"))),
        }
    }

    fn skip_prolog(&mut self) -> Result<(), LoadError> {
        loop {
            self.skip_ws();
            if self.starts_with("<?") {
                self.skip_until("?>")?;
            } else if self.starts_with("<!--") {
                self.skip_until("-->")?;
            } else if self.starts_with("<!") {
                self.skip_until(">")?;
            } else {
                return Ok(());
            }
        }
    }

    fn name(&mut self) -> Result<String, LoadError> {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| !c.is_ascii_whitespace() && !matches!(c, b'=' | b'>' | b'/'))
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.err("expected a name"));
        }
        Ok(self.text[start..self.pos].to_string())
    }

    fn element(&mut self) -> Result<XmlNode, LoadError> {
        if self.depth >= MAX_DEPTH {
            return Err(self.err("elements nested too deeply"));
        }
        self.depth += 1;
        let node = self.element_inner();
        self.depth -= 1;
        node
    }

    fn element_inner(&mut self) -> Result<XmlNode, LoadError> {
        if self.peek() != Some(b'<') {
            return Err(self.err("expected '<'"));
        }
        self.pos += 1;
        let mut node = XmlNode { tag: self.name()?, ..XmlNode::default() };
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'/') => {
                    self.pos += 1;
                    if self.peek() != Some(b'>') {
                        return Err(self.err("expected '>'"));
                    }
                    self.pos += 1;
                    return Ok(node);
                }
                Some(b'>') => {
                    self.pos += 1;
                    break;
                }
                Some(_) => {
                    let key = self.name()?;
                    self.skip_ws();
                    if self.peek() != Some(b'=') {
                        return Err(self.err("expected '='"));
                    }
                    self.pos += 1;
                    self.skip_ws();
                    let quote = self.peek().ok_or_else(|| self.err("unexpected end"))?;
                    if quote != b'"' && quote != b'\'' {
                        return Err(self.err("expected a quoted value"));
                    }
                    self.pos += 1;
                    let start = self.pos;
                    while self.peek().is_some_and(|c| c != quote) {
                        self.pos += 1;
                    }
                    let value = unescape(&self.text[start..self.pos]);
                    self.pos += 1;
                    node.attrs.push((key, value));
                }
                None => return Err(self.err("unexpected end of document")),
            }
        }
        loop {
            if self.starts_with("</") {
                self.skip_until(">")?;
                return Ok(node);
            }
            if self.starts_with("<!--") {
                self.skip_until("-->")?;
            } else if self.starts_with("<![CDATA[") {
                let start = self.pos + 9;
                self.skip_until("]]>")?;
                node.text.push_str(&self.text[start..self.pos - 3]);
            } else if self.peek() == Some(b'<') {
                node.children.push(self.element()?);
            } else if self.peek().is_some() {
                let start = self.pos;
                while self.peek().is_some_and(|c| c != b'<') {
                    self.pos += 1;
                }
                node.text.push_str(&unescape(&self.text[start..self.pos]));
            } else {
                return Err(self.err("unexpected end of document"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_attributes_children_and_text() {
        let doc = "<cir f=\"1\" ts=\"5e-6\">\n  <r x=\"1 2 3 4\" f=\"0\" r=\"10\"/>\n  <o en=\"4\"><p v=\"0\"/></o>\n  <scopedata si=\"0\">1.5,2.5</scopedata>\n</cir>";
        let root = parse(doc).unwrap();
        assert_eq!(root.tag, "cir");
        assert_eq!(root.int_attr("f", 0), 1);
        assert_eq!(root.double_attr("ts", 0.0), 5e-6);
        assert_eq!(root.children.len(), 3);
        assert_eq!(root.children[0].attr("x"), Some("1 2 3 4"));
        assert_eq!(root.children[1].children[0].tag, "p");
        assert_eq!(root.children[2].double_array(), vec![1.5, 2.5]);
    }

    #[test]
    fn decodes_entities_and_skips_prolog() {
        let root = parse("<?xml version=\"1.0\"?><!-- c --><t a=\"x &amp; &lt;y&gt;\"/>").unwrap();
        assert_eq!(root.attr("a"), Some("x & <y>"));
        assert_eq!(unescape(&escape("a<b&\"c\"")), "a<b&\"c\"");
    }

    #[test]
    fn reports_malformed_documents() {
        assert!(parse("<a").is_err());
        assert!(parse("<a b=c/>").is_err());
        assert!(parse("<a>").is_err());
    }

    #[test]
    fn writer_keeps_attribute_order_and_pretty_prints() {
        let mut root = XmlOut::new("cir");
        root.int_attr("f", 1).double_attr("ts", 5e-6);
        let mut r = XmlOut::new("r");
        r.str_attr("x", "1 2 3 4").str_attr("name", "a\"b");
        root.child(r);
        let mut t = XmlOut::new("t");
        t.text = Some("1,2".into());
        root.child(t);
        let text = root.to_pretty_string();
        assert_eq!(
            text,
            "<cir f=\"1\" ts=\"0.000005\">\n  <r x=\"1 2 3 4\" name=\"a&quot;b\"/>\n  <t>1,2</t>\n</cir>\n"
        );
        let back = parse(&text).unwrap();
        assert_eq!(back.children[0].attr("name"), Some("a\"b"));
    }

    #[test]
    fn missing_attributes_use_defaults() {
        let root = parse("<a/>").unwrap();
        assert_eq!(root.int_attr("n", 7), 7);
        assert!(root.bool_attr("b", true));
        assert_eq!(root.string_attr("s", "d"), "d");
    }
}
