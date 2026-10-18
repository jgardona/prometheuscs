// SPDX-License-Identifier: GPL-2.0-or-later
//! The front matter of an example document: which example it is for and what to check.

/// What the front matter of a document says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontMatter {
    /// The example file (`filt-lopass.txt`).
    pub example: String,
    /// The title in the menu.
    pub title: String,
    /// The category in the menu, groups separated by ` > `.
    pub category: String,
    /// The checks (`verificacoes`), one text line each.
    pub checks: Vec<String>,
    /// The reason there is nothing to check (`sem-verificacao`), when there are no checks.
    pub no_check: Option<String>,
}

/// Reads the front matter at the start of `doc` (between two `---` lines).
pub fn parse_front_matter(doc: &str) -> Result<FrontMatter, String> {
    let mut lines = doc.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err("front matter: o documento deve começar com `---`".into());
    }
    let mut body = Vec::new();
    let mut closed = false;
    for l in lines {
        if l.trim() == "---" {
            closed = true;
            break;
        }
        body.push(l);
    }
    if !closed {
        return Err("front matter: falta o `---` que o fecha".into());
    }
    let (mut example, mut title, mut category) = (None, None, None);
    let (mut checks, mut no_check) = (Vec::new(), None);
    let (mut in_checks, mut has_checks_key) = (false, false);
    for l in body {
        if l.trim().is_empty() {
            continue;
        }
        if in_checks {
            if l.starts_with(' ') && l.trim_start().starts_with("- ") {
                checks.push(l.trim_start()[2..].trim().to_string());
                continue;
            }
            in_checks = false;
        }
        let Some((key, value)) = l.split_once(':') else {
            return Err(format!("front matter: linha sem `chave: valor`: {l}"));
        };
        let value = value.trim();
        match key.trim() {
            "exemplo" => example = Some(value.to_string()),
            "titulo" => title = Some(value.to_string()),
            "categoria" => category = Some(value.to_string()),
            "verificacoes" => {
                has_checks_key = true;
                in_checks = true;
            }
            "sem-verificacao" => no_check = Some(value.to_string()),
            other => return Err(format!("front matter: chave desconhecida `{other}`")),
        }
    }
    let need = |v: Option<String>, name: &str| {
        v.filter(|s| !s.is_empty()).ok_or_else(|| format!("front matter: falta `{name}`"))
    };
    let example = need(example, "exemplo")?;
    let title = need(title, "titulo")?;
    let category = need(category, "categoria")?;
    if has_checks_key && checks.is_empty() {
        return Err("front matter: `verificacoes` está vazio".into());
    }
    match (&no_check, checks.is_empty()) {
        (Some(_), false) => {
            return Err("front matter: use um só de `verificacoes` e `sem-verificacao`".into());
        }
        (None, true) => {
            return Err("front matter: falta `verificacoes` ou `sem-verificacao`".into());
        }
        _ => {}
    }
    if let Some(reason) = &no_check {
        if reason.chars().count() < 8 || reason.eq_ignore_ascii_case("pendente") {
            return Err(
                "front matter: `sem-verificacao` precisa de um motivo (pelo menos 8 caracteres)"
                    .into(),
            );
        }
    }
    Ok(FrontMatter { example, title, category, checks, no_check })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "---\nexemplo: filt-lopass.txt\ntitulo: Low-Pass Filter (RC)\ncategoria: Passive Filters\nverificacoes:\n  - t=0.02 elemento=2:CapacitorElm grandeza=vd esperado=0.5\n  - t=0.03 elemento=1:ResistorElm grandeza=i esperado=0.001\n---\n\n# Título\n";

    #[test]
    fn a_good_front_matter_is_read() {
        let f = parse_front_matter(GOOD).unwrap();
        assert_eq!(f.example, "filt-lopass.txt");
        assert_eq!(f.title, "Low-Pass Filter (RC)");
        assert_eq!(f.category, "Passive Filters");
        assert_eq!(f.checks.len(), 2);
        assert!(f.checks[0].starts_with("t=0.02 elemento=2:CapacitorElm"));
        assert_eq!(f.no_check, None);
    }

    #[test]
    fn a_document_with_no_result_says_why() {
        let doc = "---\nexemplo: a.txt\ntitulo: A\ncategoria: C\nsem-verificacao: demonstração gráfica, sem valores\n---\n";
        let f = parse_front_matter(doc).unwrap();
        assert!(f.checks.is_empty());
        assert_eq!(f.no_check.as_deref(), Some("demonstração gráfica, sem valores"));
    }

    #[test]
    fn missing_keys_and_wrong_combinations_are_errors() {
        for (doc, what) in [
            ("# no front matter\n", "front matter"),
            ("---\ntitulo: A\ncategoria: C\nsem-verificacao: motivo longo\n---\n", "exemplo"),
            ("---\nexemplo: a.txt\ncategoria: C\nsem-verificacao: motivo longo\n---\n", "titulo"),
            ("---\nexemplo: a.txt\ntitulo: A\nsem-verificacao: motivo longo\n---\n", "categoria"),
            ("---\nexemplo: a.txt\ntitulo: A\ncategoria: C\n---\n", "verificacoes"),
            (
                "---\nexemplo: a.txt\ntitulo: A\ncategoria: C\nverificacoes:\nsem-verificacao: motivo longo\n---\n",
                "verificacoes",
            ),
            (
                "---\nexemplo: a.txt\ntitulo: A\ncategoria: C\nverificacoes:\n  - t=1 elemento=0:X grandeza=vd esperado=1\nsem-verificacao: motivo longo\n---\n",
                "um só",
            ),
            (
                "---\nexemplo: a.txt\ntitulo: A\ncategoria: C\nsem-verificacao: PENDENTE\n---\n",
                "motivo",
            ),
        ] {
            let e = parse_front_matter(doc).unwrap_err();
            assert!(e.contains(what), "{what}: {e}");
        }
    }

    #[test]
    fn an_unterminated_front_matter_is_an_error() {
        assert!(parse_front_matter("---\nexemplo: a.txt\n").unwrap_err().contains("front matter"));
    }
}
