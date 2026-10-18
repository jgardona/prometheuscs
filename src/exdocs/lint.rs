// SPDX-License-Identifier: GPL-2.0-or-later
//! Checks of the text of a document: sections, mathematics, Mermaid diagrams and links.

use std::path::Path;

/// The sections every document has, in order.
pub const SECTIONS: [&str; 7] = [
    "Objetivo",
    "O circuito",
    "Teoria",
    "Resultados esperados",
    "O que observar",
    "Limites do modelo",
    "Referências",
];

/// Everything wrong with the text of `doc` (messages without the file name); links are resolved
/// against `dir`.
pub fn lint_document(doc: &str, dir: &Path) -> Vec<String> {
    let body = strip_front_matter(doc);
    let (text, mermaid) = split_fences(body);
    let mut out = Vec::new();
    sections(&text, &mut out);
    math(&text, &mut out);
    diagrams(&mermaid, &mut out);
    links(&text, dir, &mut out);
    out
}

fn strip_front_matter(doc: &str) -> &str {
    let Some(rest) = doc.strip_prefix("---\n") else { return doc };
    match rest.find("\n---\n") {
        Some(i) => &rest[i + 5..],
        None => doc,
    }
}

/// The text outside fenced code (code lines become empty lines) and the Mermaid blocks.
fn split_fences(body: &str) -> (String, Vec<String>) {
    let mut text = String::new();
    let mut blocks = Vec::new();
    let mut fence: Option<(String, Vec<&str>)> = None;
    for line in body.lines() {
        let t = line.trim_start();
        match &mut fence {
            None => {
                if let Some(info) = t.strip_prefix("```").or_else(|| t.strip_prefix("~~~")) {
                    fence = Some((info.trim().to_string(), Vec::new()));
                    text.push('\n');
                } else {
                    text.push_str(line);
                    text.push('\n');
                }
            }
            Some((info, content)) => {
                if t.starts_with("```") || t.starts_with("~~~") {
                    if info == "mermaid" {
                        blocks.push(content.join("\n"));
                    }
                    fence = None;
                } else {
                    content.push(line);
                }
                text.push('\n');
            }
        }
    }
    (text, blocks)
}

fn sections(text: &str, out: &mut Vec<String>) {
    // (heading, index of its line) of every `## ` heading
    let lines: Vec<&str> = text.lines().collect();
    let headings: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| l.strip_prefix("## ").map(|h| (i, h.trim())))
        .collect();
    let mut last = None;
    for name in SECTIONS {
        let Some(at) = headings.iter().position(|(_, h)| *h == name) else {
            out.push(format!("falta a seção `{name}`"));
            continue;
        };
        if let Some(prev) = last {
            if at < prev {
                out.push(format!(
                    "seções fora de ordem: `{name}` vem antes do esperado (ordem: {})",
                    SECTIONS.join(", ")
                ));
            }
        }
        last = Some(at);
        let from = headings[at].0 + 1;
        let to = headings.get(at + 1).map_or(lines.len(), |h| h.0);
        let has_text = lines[from..to].iter().any(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("<!--")
        });
        if !has_text {
            out.push(format!("a seção `{name}` está vazia"));
        }
    }
}

/// `text` without inline code spans.
fn without_code_spans(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code = false;
    for c in text.chars() {
        if c == '`' {
            in_code = !in_code;
        } else if !in_code || c == '\n' {
            out.push(c);
        }
    }
    out
}

fn math(text: &str, out: &mut Vec<String>) {
    for bad in ["\\usepackage", "\\begin{tabular}"] {
        if text.contains(bad) {
            out.push(format!("`{}` não é suportado pelo GitHub", bad.trim_start_matches('\\')));
        }
    }
    let text = without_code_spans(text);
    let mut display = 0usize;
    let mut paragraph = String::new();
    let mut singles = 0usize;
    let finish = |paragraph: &mut String, singles: &mut usize, out: &mut Vec<String>| {
        if *singles % 2 == 1 {
            let start: String = paragraph.trim().chars().take(40).collect();
            out.push(format!("`$` sem par no parágrafo que começa em: {start}"));
        }
        paragraph.clear();
        *singles = 0;
    };
    for line in text.lines() {
        if line.trim().is_empty() {
            finish(&mut paragraph, &mut singles, out);
            continue;
        }
        paragraph.push_str(line);
        paragraph.push(' ');
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '\\' => i += 2,
                '$' if chars.get(i + 1) == Some(&'$') => {
                    display += 1;
                    i += 2;
                }
                '$' => {
                    singles += 1;
                    i += 1;
                }
                _ => i += 1,
            }
        }
    }
    finish(&mut paragraph, &mut singles, out);
    if display % 2 == 1 {
        out.push("delimitadores `$$` sem par".into());
    }
}

const DIAGRAMS: [&str; 8] = [
    "flowchart",
    "graph",
    "sequenceDiagram",
    "stateDiagram",
    "stateDiagram-v2",
    "classDiagram",
    "xychart-beta",
    "timeline",
];

fn diagrams(blocks: &[String], out: &mut Vec<String>) {
    for (n, b) in blocks.iter().enumerate() {
        let first = b.lines().map(str::trim).find(|l| !l.is_empty());
        match first.and_then(|l| l.split_whitespace().next()) {
            None => out.push(format!("bloco Mermaid {} vazio", n + 1)),
            Some(k) if !DIAGRAMS.contains(&k) => {
                out.push(format!(
                    "bloco Mermaid {} com tipo de diagrama desconhecido: `{k}`",
                    n + 1
                ));
            }
            Some(_) => {}
        }
    }
}

fn links(text: &str, dir: &Path, out: &mut Vec<String>) {
    let text = without_code_spans(text);
    let mut rest = text.as_str();
    while let Some(i) = rest.find("](") {
        rest = &rest[i + 2..];
        let end = rest.find(')').unwrap_or(rest.len());
        let target = rest[..end].split_whitespace().next().unwrap_or("");
        rest = &rest[end..];
        if target.is_empty()
            || target.starts_with('#')
            || ["http://", "https://", "mailto:"].iter().any(|p| target.starts_with(p))
        {
            continue;
        }
        let file = target.split('#').next().unwrap_or(target);
        if !file.is_empty() && !dir.join(file).exists() {
            out.push(format!("o link para `{file}` não existe"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(sections: &[(&str, &str)]) -> String {
        let mut s = String::from(
            "---\nexemplo: a.txt\ntitulo: A\ncategoria: C\nsem-verificacao: motivo longo\n---\n\n# A\n\n",
        );
        for (h, body) in sections {
            s.push_str(&format!("## {h}\n\n{body}\n\n"));
        }
        s
    }

    fn full() -> String {
        let body = [
            ("Objetivo", "Texto."),
            ("O circuito", "Texto."),
            ("Teoria", "Vale $v = R i$.\n\n$$\\frac{a}{b}$$"),
            ("Resultados esperados", "Texto."),
            ("O que observar", "Texto."),
            ("Limites do modelo", "Texto."),
            ("Referências", "Texto."),
        ];
        doc(&body)
    }

    fn lint(text: &str) -> Vec<String> {
        lint_document(text, Path::new("docs/exemplos"))
    }

    #[test]
    fn a_complete_document_has_no_problems() {
        assert_eq!(lint(&full()), Vec::<String>::new());
    }

    #[test]
    fn missing_misordered_and_empty_sections_are_reported() {
        let missing = full().replace("## Teoria\n\nVale $v = R i$.\n\n$$\\frac{a}{b}$$\n\n", "");
        assert!(lint(&missing).iter().any(|m| m.contains("Teoria")), "{:?}", lint(&missing));
        let swapped = full()
            .replace("## Objetivo", "## XX")
            .replace("## O circuito", "## Objetivo")
            .replace("## XX", "## O circuito");
        assert!(lint(&swapped).iter().any(|m| m.contains("ordem")), "{:?}", lint(&swapped));
        let empty = full().replace("## Limites do modelo\n\nTexto.", "## Limites do modelo\n");
        assert!(
            lint(&empty).iter().any(|m| m.contains("Limites do modelo") && m.contains("vazia")),
            "{:?}",
            lint(&empty)
        );
    }

    #[test]
    fn unbalanced_mathematics_is_reported() {
        let open = full().replace("$$\\frac{a}{b}$$", "$$\\frac{a}{b}");
        assert!(lint(&open).iter().any(|m| m.contains("$$")), "{:?}", lint(&open));
        let inline = full().replace("Vale $v = R i$.", "Vale $v = R i.");
        assert!(lint(&inline).iter().any(|m| m.contains('$')), "{:?}", lint(&inline));
        let package = full().replace("Texto.", "\\usepackage{amsmath}");
        assert!(lint(&package).iter().any(|m| m.contains("usepackage")));
        // a dollar sign in code or escaped is not mathematics
        let ok = full().replace("Texto.", "Custa \\$5 e `$x` é código.");
        assert_eq!(lint(&ok), Vec::<String>::new());
    }

    #[test]
    fn mermaid_blocks_need_a_known_diagram_and_content() {
        let good = full().replace(
            "Vale $v = R i$.",
            "```mermaid\nflowchart LR\n  A --> B\n```\n\nVale $v = R i$.",
        );
        assert_eq!(lint(&good), Vec::<String>::new());
        let unknown = good.replace("flowchart LR", "pizza LR");
        assert!(lint(&unknown).iter().any(|m| m.contains("Mermaid")), "{:?}", lint(&unknown));
        let empty = good.replace("flowchart LR\n  A --> B\n", "");
        assert!(lint(&empty).iter().any(|m| m.contains("Mermaid")), "{:?}", lint(&empty));
    }

    #[test]
    fn relative_links_must_point_to_files_that_exist() {
        let dir = Path::new("docs");
        let ok = full().replace(
            "Texto.",
            "[guia](guia-do-usuario.md#algo) e [fora](https://example.org) e [aqui](#teoria).",
        );
        assert_eq!(lint_document(&ok, dir), Vec::<String>::new(), "links to existing files");
        let bad = full().replace("Texto.", "[nada](nao-existe.md)");
        assert!(
            lint_document(&bad, dir).iter().any(|m| m.contains("nao-existe.md")),
            "{:?}",
            lint_document(&bad, dir)
        );
    }
}
