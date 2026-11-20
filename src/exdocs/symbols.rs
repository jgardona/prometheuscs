// SPDX-License-Identifier: GPL-2.0-or-later
//! Symbols and parameters: every symbol an example document uses is defined in the table of its
//! section "Símbolos e parâmetros", and every symbol and component field has one entry in the
//! shared glossary (`docs/glossario-simbolos.md`). See `specs/008-example-parameter-glossary`.

use super::lint::{split_fences, strip_front_matter, without_code_spans};
use super::region;
use std::collections::{BTreeMap, BTreeSet};

/// The heading of the section of a document that defines its symbols.
pub const SECTION: &str = "Símbolos e parâmetros";

/// The glossary, relative to the repository root.
pub const GLOSSARY: &str = "docs/glossario-simbolos.md";

/// The name of the glossary file, which each symbols section must link.
pub const GLOSSARY_FILE: &str = "glossario-simbolos.md";

/// Whether every document must have the symbols section (switched on when the last wave is
/// delivered, so that every intermediate state of the repository is green).
pub const REQUIRE_SECTION: bool = true;

/// Symbols that need no definition: constants, the imaginary unit and the `d` of a differential.
const IGNORED: [&str; 4] = ["e", "j", "d", "\\pi"];

/// Greek macros that name a quantity (the uppercase ones are operators or units here).
const GREEK: [&str; 19] = [
    "alpha",
    "beta",
    "gamma",
    "delta",
    "epsilon",
    "varepsilon",
    "zeta",
    "eta",
    "theta",
    "vartheta",
    "kappa",
    "lambda",
    "mu",
    "nu",
    "xi",
    "rho",
    "sigma",
    "tau",
    "omega",
];

/// More Greek macros, kept apart only to keep the table above short.
const GREEK_MORE: [&str; 4] = ["varphi", "phi", "psi", "chi"];

/// Macros whose argument is text or a unit, not a quantity.
const TEXT_MACROS: [&str; 8] =
    ["text", "mathrm", "textbf", "mbox", "operatorname", "mathit", "begin", "end"];

/// Reduces a piece of LaTeX to a comparable form: no `$`, spaces, braces or text wrappers, so that
/// `V_{GS}`, `V_\text{GS}` and `V_GS` are the same notation.
#[must_use]
pub fn normalise(tex: &str) -> String {
    let chars: Vec<char> = tex.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '$' | '{' | '}' | ' ' | '\t' => i += 1,
            '\\' => {
                let (name, next) = command(&chars, i);
                i = next;
                let skip = TEXT_MACROS.contains(&name.as_str())
                    || ["thinspace", "thickspace", "quad", "qquad", "left", "right", "mathbf"]
                        .contains(&name.as_str())
                    || name.is_empty()
                    || name == "lvert"
                    || name == "rvert";
                if name == "min" || name == "max" {
                    out.push_str(&name);
                } else if !skip {
                    out.push('\\');
                    out.push_str(&name);
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// The name of the command that starts at `chars[at]` (a backslash) and the index after it. A
/// backslash before a non-letter is a command with an empty name that consumes that one character.
fn command(chars: &[char], at: usize) -> (String, usize) {
    let mut i = at + 1;
    let mut name = String::new();
    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        name.push(chars[i]);
        i += 1;
    }
    if name.is_empty() && i < chars.len() {
        i += 1;
    }
    (name, i)
}

/// The content of the group that starts at `chars[*i]`: a `{...}`, a `\text{...}`-like command or
/// one character. Advances `*i` past it.
fn group(chars: &[char], i: &mut usize) -> String {
    let Some(&c) = chars.get(*i) else { return String::new() };
    if c == '{' {
        let mut depth = 0usize;
        let mut content = String::new();
        while *i < chars.len() {
            let ch = chars[*i];
            *i += 1;
            match ch {
                '{' => {
                    depth += 1;
                    if depth > 1 {
                        content.push(ch);
                    }
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    content.push(ch);
                }
                _ => content.push(ch),
            }
        }
        content
    } else if c == '\\' {
        let (name, next) = command(chars, *i);
        *i = next;
        if TEXT_MACROS.contains(&name.as_str()) && chars.get(*i) == Some(&'{') {
            group(chars, i)
        } else {
            format!("\\{name}")
        }
    } else {
        *i += 1;
        c.to_string()
    }
}

/// The math spans (`$...$` and `$$...$$`) of `text`, without the delimiters.
fn math_spans(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
        } else if chars[i] == '$' {
            let display = chars.get(i + 1) == Some(&'$');
            i += if display { 2 } else { 1 };
            let mut span = String::new();
            while i < chars.len() {
                if chars[i] == '\\' {
                    span.push(chars[i]);
                    if let Some(&n) = chars.get(i + 1) {
                        span.push(n);
                    }
                    i += 2;
                } else if chars[i] == '$' {
                    i += if display && chars.get(i + 1) == Some(&'$') { 2 } else { 1 };
                    break;
                } else {
                    span.push(chars[i]);
                    i += 1;
                }
            }
            spans.push(span);
        } else {
            i += 1;
        }
    }
    spans
}

/// The symbol tokens of one math span, normalised: a letter or a Greek macro, with its subscript.
fn tokens_of(math: &str, out: &mut BTreeSet<String>) {
    let c: Vec<char> = math.chars().collect();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '\\' => {
                let (name, next) = command(&c, i);
                i = next;
                if TEXT_MACROS.contains(&name.as_str()) {
                    if c.get(i) == Some(&'{') {
                        group(&c, &mut i);
                    }
                } else if name == "mu" && starts_unit(&c, i) {
                    // the micro prefix of a unit
                } else if GREEK.contains(&name.as_str()) || GREEK_MORE.contains(&name.as_str()) {
                    base(&c, &mut i, &format!("\\{name}"), out);
                }
            }
            '_' | '^' => {
                i += 1;
                group(&c, &mut i);
            }
            ch if ch.is_ascii_alphabetic() => {
                i += 1;
                if ch == 'e' && c.get(i) == Some(&'^') {
                    // the exponent of the constant holds quantities
                    i += 1;
                    let exponent = group(&c, &mut i);
                    tokens_of(&exponent, out);
                    continue;
                }
                base(&c, &mut i, &ch.to_string(), out);
            }
            _ => i += 1,
        }
    }
}

fn starts_unit(c: &[char], i: usize) -> bool {
    let rest: String = c[i..].iter().take(8).collect();
    rest.starts_with("\\text") || rest.starts_with("\\mathrm")
}

/// A base letter was read; attach its subscript and drop its superscript.
fn base(c: &[char], i: &mut usize, base: &str, out: &mut BTreeSet<String>) {
    let mut token = base.to_string();
    if c.get(*i) == Some(&'_') {
        *i += 1;
        let sub = normalise(&group(c, i));
        if !sub.is_empty() {
            token = format!("{base}_{sub}");
        }
    }
    if c.get(*i) == Some(&'^') {
        *i += 1;
        group(c, i);
    }
    if !IGNORED.contains(&token.as_str()) {
        out.insert(token);
    }
}

/// The symbols a text uses in its math, normalised (code and the symbols section excluded by the
/// caller).
#[must_use]
pub fn used_symbols(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for span in math_spans(&without_code_spans(text)) {
        tokens_of(&span, &mut out);
    }
    out
}

/// Splits a Markdown table row into trimmed cells; a `|` inside `$...$` or backticks is not a
/// separator.
fn split_row(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = line.strip_suffix('|').unwrap_or(line);
    let mut cells = Vec::new();
    let mut cur = String::new();
    let (mut in_math, mut in_code) = (false, false);
    let mut prev = ' ';
    for ch in line.chars() {
        match ch {
            '$' if !in_code && prev != '\\' => in_math = !in_math,
            '`' => in_code = !in_code,
            '|' if !in_math && !in_code => {
                cells.push(cur.trim().to_string());
                cur.clear();
                prev = ch;
                continue;
            }
            _ => {}
        }
        cur.push(ch);
        prev = ch;
    }
    cells.push(cur.trim().to_string());
    cells
}

fn is_separator(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// A row of the symbols table of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRow {
    /// The notation, normalised.
    pub notation: String,
    /// The name of the quantity.
    pub name: String,
    /// What it is, in a sentence.
    pub meaning: String,
    /// Its unit.
    pub unit: String,
}

/// The lines of the section `heading` of `body` (`## heading` up to the next `## `), if present,
/// with the index of the heading among the `## ` headings.
fn section_of<'a>(body: &'a str, heading: &str) -> Option<(Vec<&'a str>, usize)> {
    let lines: Vec<&str> = body.lines().collect();
    let mut rank = 0;
    let mut at = None;
    for (n, l) in lines.iter().enumerate() {
        if let Some(h) = l.strip_prefix("## ") {
            if h.trim() == heading {
                at = Some((n, rank));
                break;
            }
            rank += 1;
        }
    }
    let (start, rank) = at?;
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with("## "))
        .map_or(lines.len(), |p| start + 1 + p);
    Some((lines[start + 1..end].to_vec(), rank))
}

/// The body of a document without its symbols section.
pub(crate) fn without_section(body: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for l in body.lines() {
        if let Some(h) = l.strip_prefix("## ") {
            skipping = h.trim() == SECTION;
        }
        if !skipping {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
}

/// Parses the symbols table of a document's section. `Ok(None)` when the document has no such
/// section.
///
/// # Errors
/// The table or the section is malformed.
pub fn parse_symbols_section(doc: &str) -> Result<Option<Vec<SymbolRow>>, String> {
    let body = strip_front_matter(doc);
    let Some((lines, _)) = section_of(body, SECTION) else { return Ok(None) };
    if !lines.iter().any(|l| l.contains(GLOSSARY_FILE)) {
        return Err(format!(
            "a seção `{SECTION}` não tem o link para o glossário ({GLOSSARY_FILE})"
        ));
    }
    let mut table = lines.iter().filter(|l| l.trim_start().starts_with('|'));
    let Some(first) = table.next() else {
        // a document without symbols has the section but no table
        return Ok(Some(Vec::new()));
    };
    let header = split_row(first);
    if header != ["Símbolo", "Nome", "Significado", "Unidade"] {
        return Err(format!(
            "a tabela da seção `{SECTION}` deve ter as colunas `Símbolo | Nome | Significado | Unidade`"
        ));
    }
    let mut rows = Vec::new();
    for l in table {
        if is_separator(l) {
            continue;
        }
        let cells = split_row(l);
        if cells.len() != 4 || cells.iter().any(String::is_empty) {
            return Err(format!("linha da tabela de símbolos incompleta: {}", l.trim()));
        }
        if !(cells[0].starts_with('$') && cells[0].ends_with('$')) {
            return Err(format!("o símbolo deve estar entre `$`: {}", cells[0]));
        }
        rows.push(SymbolRow {
            notation: normalise(&cells[0]),
            name: cells[1].clone(),
            meaning: cells[2].clone(),
            unit: cells[3].clone(),
        });
    }
    if rows.is_empty() {
        return Err(format!("a tabela da seção `{SECTION}` não tem linhas"));
    }
    let mut seen = BTreeSet::new();
    for r in &rows {
        if !seen.insert((r.notation.clone(), r.name.clone())) {
            return Err(format!("símbolo repetido na tabela: {} ({})", r.notation, r.name));
        }
    }
    Ok(Some(rows))
}

/// A symbol entry of the glossary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossarySymbol {
    /// The notation, normalised.
    pub notation: String,
    /// Plain-text spellings (`Vgs`).
    pub spellings: Vec<String>,
    /// Name of the quantity.
    pub name: String,
    /// Unit.
    pub unit: String,
    /// Stems of the example documents given as examples.
    pub examples: Vec<String>,
}

/// A component-field entry of the glossary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryField {
    /// The field name as the components tables show it.
    pub field: String,
    /// Stems of the example documents given as examples.
    pub examples: Vec<String>,
}

/// The parsed glossary.
#[derive(Debug, Default, Clone)]
pub struct Glossary {
    /// Symbol entries.
    pub symbols: Vec<GlossarySymbol>,
    /// Field entries.
    pub fields: Vec<GlossaryField>,
}

const SYMBOL_HEADER: [&str; 6] =
    ["Símbolo", "Grafias", "Nome", "Significado", "Unidade", "Exemplo"];
const FIELD_HEADER: [&str; 5] = ["Campo", "Componentes", "Unidade", "Efeito", "Exemplo"];

fn example_stems(cell: &str) -> Vec<String> {
    cell.split("](")
        .skip(1)
        .filter_map(|p| p.split(')').next())
        .filter_map(|link| link.strip_prefix("exemplos/"))
        .filter_map(|f| f.strip_suffix(".md"))
        .map(str::to_string)
        .collect()
}

/// Parses the glossary; the problems found while parsing are returned with the glossary.
#[must_use]
pub fn parse_glossary(text: &str) -> (Glossary, Vec<String>) {
    let mut g = Glossary::default();
    let mut problems = Vec::new();
    let mut mode = 0u8; // 0 none, 1 symbols, 2 fields
    for l in text.lines() {
        let t = l.trim_start();
        if !t.starts_with('|') {
            mode = 0;
            continue;
        }
        if is_separator(t) {
            continue;
        }
        let cells = split_row(t);
        if cells == SYMBOL_HEADER {
            mode = 1;
            continue;
        }
        if cells == FIELD_HEADER {
            mode = 2;
            continue;
        }
        if mode == 0 {
            continue;
        }
        let want = if mode == 1 { 6 } else { 5 };
        if cells.len() != want || cells.iter().any(String::is_empty) {
            problems.push(format!("linha do glossário incompleta: {}", t.trim()));
            continue;
        }
        let examples = example_stems(cells.last().map_or("", String::as_str));
        if examples.is_empty() {
            problems.push(format!(
                "linha do glossário sem exemplo (`exemplos/<nome>.md`): {}",
                cells[0]
            ));
        }
        if mode == 1 {
            g.symbols.push(GlossarySymbol {
                notation: normalise(&cells[0]),
                spellings: cells[1]
                    .split(',')
                    .map(|s| s.trim().trim_matches('`').to_string())
                    .filter(|s| !s.is_empty() && s != "—")
                    .collect(),
                name: cells[2].clone(),
                unit: cells[4].clone(),
                examples,
            });
        } else {
            g.fields
                .push(GlossaryField { field: cells[0].trim_matches('`').to_string(), examples });
        }
    }
    let mut keys = BTreeSet::new();
    let mut spellings: BTreeMap<String, String> = BTreeMap::new();
    for s in &g.symbols {
        if !keys.insert((s.notation.clone(), s.name.clone())) {
            problems.push(format!("símbolo repetido no glossário: {} ({})", s.notation, s.name));
        }
        for sp in &s.spellings {
            let owner = format!("{} ({})", s.notation, s.name);
            if let Some(other) = spellings.insert(sp.clone(), owner.clone()) {
                if other != owner {
                    problems
                        .push(format!("a grafia `{sp}` está em duas entradas: {other} e {owner}"));
                }
            }
        }
    }
    let mut fields = BTreeSet::new();
    for f in &g.fields {
        if !fields.insert(f.field.clone()) {
            problems.push(format!("campo repetido no glossário: {}", f.field));
        }
    }
    (g, problems)
}

/// The field names of a components region (`| N | `Class` | kind | Field = value; ... |`).
#[must_use]
pub fn component_fields(region_text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for l in region_text.lines().filter(|l| l.trim_start().starts_with('|')) {
        let cells = split_row(l);
        if cells.len() != 4 || !cells[1].starts_with('`') {
            continue;
        }
        for part in cells[3].split(';') {
            if let Some(name) = field_name(part) {
                out.insert(name);
            }
        }
    }
    out
}

/// The field name at the start of `part` (`Name = value`): what comes before the first ` = ` that
/// is not inside parentheses (`Series Resistance (0 = infinite)` holds one). Fragments of a value
/// that itself holds `;` do not start with a name and give `None`.
fn field_name(part: &str) -> Option<String> {
    let part = part.trim_start();
    let mut depth = 0usize;
    let bytes: Vec<char> = part.chars().collect();
    for (i, ch) in bytes.iter().enumerate() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ' ' if depth == 0
                && bytes.get(i + 1) == Some(&'=')
                && bytes.get(i + 2).is_none_or(|c| *c == ' ') =>
            {
                let name: String = bytes[..i].iter().collect();
                let name = name.trim();
                let ok = name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '#');
                return ok.then(|| name.to_string());
            }
            _ => {}
        }
    }
    None
}

/// What a document offers to the cross-checks with the glossary.
#[derive(Debug, Default, Clone)]
pub struct DocInfo {
    /// Notations of its symbols table; `None` when it has no symbols section.
    pub notations: Option<BTreeSet<String>>,
    /// Field names of its components table.
    pub fields: BTreeSet<String>,
}

/// Everything wrong with the symbols section of one document (messages without the file name).
#[must_use]
pub fn check_document(doc: &str, glossary: &Glossary, require: bool) -> (Vec<String>, DocInfo) {
    let body = strip_front_matter(doc);
    let fields = region::get_region(doc, "componentes").map(component_fields).unwrap_or_default();
    let mut info = DocInfo { notations: None, fields: fields.clone() };
    let mut out = Vec::new();
    let rows = match parse_symbols_section(doc) {
        Err(e) => return (vec![e], info),
        Ok(None) => {
            if require {
                out.push(format!("falta a seção `{SECTION}`"));
            }
            return (out, info);
        }
        Ok(Some(rows)) => rows,
    };
    let headings: Vec<&str> =
        body.lines().filter_map(|l| l.strip_prefix("## ")).map(str::trim).collect();
    let pos = |h: &str| headings.iter().position(|x| *x == h);
    match (pos("O circuito"), pos(SECTION), pos("Teoria")) {
        (Some(a), Some(b), Some(c)) if a < b && b < c => {}
        _ => out.push(format!("a seção `{SECTION}` deve ficar entre `O circuito` e `Teoria`")),
    }
    let (text, _) = split_fences(&without_section(body));
    let used = used_symbols(&text);
    let have: BTreeSet<String> = rows.iter().map(|r| r.notation.clone()).collect();
    for u in used.difference(&have) {
        out.push(format!("o símbolo `{u}` é usado em fórmula e não está na tabela de símbolos"));
    }
    for h in have.difference(&used) {
        out.push(format!(
            "o símbolo `{h}` está na tabela de símbolos e não é usado em nenhuma fórmula"
        ));
    }
    for r in &rows {
        let same: Vec<&GlossarySymbol> =
            glossary.symbols.iter().filter(|g| g.notation == r.notation).collect();
        if same.is_empty() {
            out.push(format!("o símbolo `{}` não tem entrada no glossário", r.notation));
        } else if !same.iter().any(|g| g.name == r.name && g.unit == r.unit) {
            out.push(format!(
                "o símbolo `{}` está na tabela como «{}» ({}), diferente do glossário: {}",
                r.notation,
                r.name,
                r.unit,
                same.iter()
                    .map(|g| format!("«{}» ({})", g.name, g.unit))
                    .collect::<Vec<_>>()
                    .join(" ou ")
            ));
        }
    }
    let known: BTreeSet<&str> = glossary.fields.iter().map(|f| f.field.as_str()).collect();
    for f in &fields {
        if !known.contains(f.as_str()) {
            out.push(format!(
                "o campo `{f}` da tabela de componentes não tem entrada no glossário"
            ));
        }
    }
    info.notations = Some(have);
    (out, info)
}

/// Everything wrong with the glossary given the documents (`glossario-simbolos.md: ...` lines).
#[must_use]
pub fn check_glossary(
    glossary: &Glossary,
    parse_problems: Vec<String>,
    docs: &BTreeMap<String, DocInfo>,
) -> Vec<String> {
    let mut out: Vec<String> = parse_problems;
    for s in &glossary.symbols {
        let uses = s.examples.iter().any(|e| {
            docs.get(e).and_then(|d| d.notations.as_ref()).is_some_and(|n| n.contains(&s.notation))
        });
        if !uses {
            out.push(format!(
                "o símbolo `{}` ({}) não é usado por nenhum dos documentos de exemplo que cita: {}",
                s.notation,
                s.name,
                s.examples.join(", ")
            ));
        }
    }
    for f in &glossary.fields {
        if !f.examples.iter().any(|e| docs.get(e).is_some_and(|d| d.fields.contains(&f.field))) {
            out.push(format!(
                "o campo `{}` não está na tabela de componentes de nenhum documento que cita: {}",
                f.field,
                f.examples.join(", ")
            ));
        }
    }
    out.into_iter().map(|m| format!("{GLOSSARY}: {m}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn used(tex: &str) -> Vec<String> {
        used_symbols(tex).into_iter().collect()
    }

    #[test]
    fn notations_that_mean_the_same_are_normalised_alike() {
        assert_eq!(normalise("$V_{GS}$"), "V_GS");
        assert_eq!(normalise("V_\\text{GS}"), "V_GS");
        assert_eq!(normalise("V_GS"), "V_GS");
        assert_eq!(normalise("\\omega_0"), "\\omega_0");
        assert_eq!(normalise("V_\\min"), "V_min");
    }

    #[test]
    fn symbols_are_extracted_from_math_only() {
        assert_eq!(used("o valor $V_{GS}$ e $I_D$ em Vgs"), ["I_D", "V_GS"]);
        assert_eq!(used("sem fórmulas, I_D"), Vec::<String>::new());
    }

    #[test]
    fn units_numbers_and_text_are_not_symbols() {
        assert_eq!(used("$5\\ \\text{V}$ e $2{,}944\\ \\text{mA}$"), Vec::<String>::new());
        assert_eq!(used("$10\\mu\\text{F}$ e $1\\ \\text{k}\\Omega$"), Vec::<String>::new());
        assert_eq!(used("$f = 40\\ \\text{Hz}$ \\text{ se }"), ["f"]);
    }

    #[test]
    fn subscripts_attach_and_superscripts_are_dropped() {
        assert_eq!(used("$V_\\text{in} = R_f\\thinspace i$"), ["R_f", "V_in", "i"]);
        assert_eq!(used("$\\omega_0^2 = 1/LC$"), ["C", "L", "\\omega_0"]);
        assert_eq!(used("$e^{-t/RC}$"), ["C", "R", "t"]);
    }

    #[test]
    fn constants_and_functions_are_ignored() {
        assert_eq!(used("$j\\omega \\pi \\sin x \\ln y$"), ["\\omega", "x", "y"]);
        assert_eq!(used("$\\frac{dv}{dt}$"), ["t", "v"]);
    }

    #[test]
    fn field_names_ignore_equals_signs_inside_parentheses_and_values() {
        assert_eq!(field_name(" Gain = 100k").as_deref(), Some("Gain"));
        assert_eq!(
            field_name(" Series Resistance (0 = infinite) = 0").as_deref(),
            Some("Series Resistance (0 = infinite)")
        );
        assert_eq!(field_name(" 3=F = -1").as_deref(), None);
        assert_eq!(field_name(" Label = ").as_deref(), Some("Label"));
        assert_eq!(field_name(" Label =").as_deref(), Some("Label"));
    }

    #[test]
    fn a_pipe_inside_math_does_not_split_a_cell() {
        assert_eq!(split_row("| $|V_t|$ | tensão | x | V |").len(), 4);
    }

    const DOC: &str = "---\nexemplo: a.txt\n---\n\n## Objetivo\n\nx\n\n## O circuito\n\n<!-- componentes:inicio -->\n| N.º | Tipo | Elemento | Valores |\n|----:|------|----------|---------|\n| 0 | `MosfetElm` | mosfet | Threshold Voltage = 1.5; Gain = 2 |\n<!-- componentes:fim -->\n\n## Símbolos e parâmetros\n\nVeja o [glossário](../glossario-simbolos.md):\n\n| Símbolo | Nome | Significado | Unidade |\n|---|---|---|---|\n| $V_t$ | tensão de limiar | começa a conduzir | V |\n\n## Teoria\n\nValor $V_t$.\n";

    const GLOSS: &str = "# G\n\n| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |\n|---|---|---|---|---|---|\n| $V_t$ | `Vt` | tensão de limiar | x | V | [a](exemplos/a.md) |\n\n| Campo | Componentes | Unidade | Efeito | Exemplo |\n|---|---|---|---|---|\n| Threshold Voltage | `MosfetElm` | V | x | [a](exemplos/a.md) |\n| Gain | `MosfetElm` | — | x | [a](exemplos/a.md) |\n";

    #[test]
    fn a_complete_document_has_no_problems() {
        let (g, p) = parse_glossary(GLOSS);
        assert!(p.is_empty(), "{p:?}");
        let (problems, info) = check_document(DOC, &g, true);
        assert!(problems.is_empty(), "{problems:?}");
        let docs = BTreeMap::from([("a".to_string(), info)]);
        assert!(check_glossary(&g, p, &docs).is_empty());
    }

    #[test]
    fn a_missing_section_is_an_error_only_when_required() {
        let doc = DOC.replace("## Símbolos e parâmetros", "## Outra");
        let (g, _) = parse_glossary(GLOSS);
        assert!(check_document(&doc, &g, false).0.is_empty());
        assert_eq!(check_document(&doc, &g, true).0.len(), 1);
    }

    #[test]
    fn a_used_symbol_missing_from_the_table_is_reported() {
        let doc = DOC.replace("Valor $V_t$.", "Valor $V_t$ e $I_D$.");
        let (g, _) = parse_glossary(GLOSS);
        let (p, _) = check_document(&doc, &g, true);
        assert!(p.iter().any(|m| m.contains("I_D") && m.contains("não está na tabela")), "{p:?}");
    }

    #[test]
    fn a_dead_row_is_reported() {
        let doc = DOC.replace("Valor $V_t$.", "Valor.");
        let (g, _) = parse_glossary(GLOSS);
        let (p, _) = check_document(&doc, &g, true);
        assert!(p.iter().any(|m| m.contains("não é usado")), "{p:?}");
    }

    #[test]
    fn a_row_without_or_unlike_the_glossary_entry_is_reported() {
        let (empty, _) = parse_glossary("# G\n");
        assert!(check_document(DOC, &empty, true).0.iter().any(|m| m.contains("não tem entrada")));
        let (g, _) = parse_glossary(&GLOSS.replace("| V | [a]", "| mV | [a]"));
        assert!(
            check_document(DOC, &g, true).0.iter().any(|m| m.contains("diferente do glossário"))
        );
    }

    #[test]
    fn a_field_without_a_glossary_entry_is_reported() {
        let (g, _) = parse_glossary(
            &GLOSS.replace("| Gain | `MosfetElm` | — | x | [a](exemplos/a.md) |\n", ""),
        );
        assert!(check_document(DOC, &g, true).0.iter().any(|m| m.contains("`Gain`")));
    }

    #[test]
    fn a_section_without_a_table_is_a_document_without_symbols() {
        let (g, _) = parse_glossary(GLOSS);
        let doc = DOC
            .replace("| Símbolo | Nome | Significado | Unidade |\n|---|---|---|---|\n| $V_t$ | tensão de limiar | começa a conduzir | V |\n", "Este documento não usa símbolos em fórmulas.\n")
            .replace("Valor $V_t$.", "Sem fórmulas.");
        assert!(
            check_document(&doc, &g, true).0.is_empty(),
            "{:?}",
            check_document(&doc, &g, true).0
        );
        // ...but not when its math does use symbols
        let doc = doc.replace("Sem fórmulas.", "Valor $I_D$.");
        assert!(check_document(&doc, &g, true).0.iter().any(|m| m.contains("I_D")));
    }

    #[test]
    fn a_table_without_the_glossary_link_or_rows_is_an_error() {
        let (g, _) = parse_glossary(GLOSS);
        let no_link = DOC.replace("[glossário](../glossario-simbolos.md)", "glossário");
        assert!(check_document(&no_link, &g, true).0[0].contains("link"));
        let no_rows = DOC.replace("| $V_t$ | tensão de limiar | começa a conduzir | V |\n", "");
        assert!(check_document(&no_rows, &g, true).0[0].contains("não tem linhas"));
    }

    #[test]
    fn the_section_must_sit_between_the_circuit_and_the_theory() {
        let doc = DOC.replace("## Teoria", "## Teoria0").replace("## Objetivo", "## Teoria");
        let (g, _) = parse_glossary(GLOSS);
        assert!(check_document(&doc, &g, true).0.iter().any(|m| m.contains("deve ficar entre")));
    }

    #[test]
    fn glossary_duplicates_empty_cells_and_shared_spellings_are_reported() {
        let dup =
            format!("{GLOSS}| $V_t$ | `Vt2` | tensão de limiar | x | V | [a](exemplos/a.md) |\n");
        let (_, p) = parse_glossary(&dup.replace("\n\n| Campo", "\n| Campo"));
        let _ = p;
        let text = "| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |\n|---|---|---|---|---|---|\n| $V_t$ | `Vt` | n | x | V | [a](exemplos/a.md) |\n| $V_t$ | `Vt` | n | x | V | [a](exemplos/a.md) |\n| $V_T$ | `Vt` | m | x | V | [a](exemplos/a.md) |\n| $V_x$ |  | m | x | V | [a](exemplos/a.md) |\n";
        let (_, p) = parse_glossary(text);
        assert!(p.iter().any(|m| m.contains("repetido")), "{p:?}");
        assert!(p.iter().any(|m| m.contains("duas entradas")), "{p:?}");
        assert!(p.iter().any(|m| m.contains("incompleta")), "{p:?}");
    }

    #[test]
    fn a_glossary_entry_nobody_uses_is_reported() {
        let (g, p) = parse_glossary(GLOSS);
        let docs = BTreeMap::from([("a".to_string(), DocInfo::default())]);
        let problems = check_glossary(&g, p, &docs);
        assert!(problems.iter().any(|m| m.contains("V_t")), "{problems:?}");
        assert!(problems.iter().any(|m| m.contains("Gain")), "{problems:?}");
    }
}
