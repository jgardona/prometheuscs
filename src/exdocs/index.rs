// SPDX-License-Identifier: GPL-2.0-or-later
//! Which examples have a document, and the index of the documents.

use crate::app::examples::{ExampleItem, ExampleList};
use std::collections::BTreeSet;

/// The document name of an example file: `filt-lopass.txt` gives `filt-lopass`.
pub fn stem(file: &str) -> String {
    file.strip_suffix(".txt").unwrap_or(file).to_string()
}

/// How many entries of `items` (sub-groups included) have a document, and how many entries there
/// are. An example listed under two categories counts in both.
pub fn counts(items: &[ExampleItem], stems: &BTreeSet<String>) -> (usize, usize) {
    let (mut done, mut total) = (0, 0);
    for item in items {
        match item {
            ExampleItem::Circuit { file, .. } => {
                total += 1;
                done += usize::from(stems.contains(&stem(file)));
            }
            ExampleItem::Group { items, .. } => {
                let (d, t) = counts(items, stems);
                done += d;
                total += t;
            }
        }
    }
    (done, total)
}

fn status(items: &[ExampleItem], stems: &BTreeSet<String>) -> String {
    let (d, t) = counts(items, stems);
    if d == t { "documentado".to_string() } else { format!("pendente ({d} de {t})") }
}

fn files(items: &[ExampleItem], out: &mut Vec<String>) {
    for item in items {
        match item {
            ExampleItem::Circuit { file, .. } => out.push(file.clone()),
            ExampleItem::Group { items, .. } => files(items, out),
        }
    }
}

/// One group: its heading, its own examples in menu order, then its sub-groups.
fn render_group(
    out: &mut String,
    title: &str,
    items: &[ExampleItem],
    level: usize,
    stems: &BTreeSet<String>,
) {
    out.push_str(&format!("{} {title} — {}\n\n", "#".repeat(level.min(6)), status(items, stems)));
    let mut any = false;
    for item in items {
        if let ExampleItem::Circuit { file, title } = item {
            any = true;
            if stems.contains(&stem(file)) {
                out.push_str(&format!("- [{title}]({}.md)\n", stem(file)));
            } else {
                out.push_str(&format!("- {title} — *pendente*\n"));
            }
        }
    }
    if any {
        out.push('\n');
    }
    for item in items {
        if let ExampleItem::Group { title, items } = item {
            render_group(out, title, items, level + 1, stems);
        }
    }
}

/// The text of the index (`docs/exemplos/README.md`): the categories of the menu in its order, each
/// documented or pending, with the examples that have a document linked.
pub fn render_index(list: &ExampleList, stems: &BTreeSet<String>) -> String {
    let mut menu_files = Vec::new();
    files(&list.items, &mut menu_files);
    let documents: BTreeSet<String> =
        menu_files.iter().map(|f| stem(f)).filter(|s| stems.contains(s)).collect();
    let (done, total) = counts(&list.items, stems);
    let mut out = String::new();
    out.push_str("# Documentação dos circuitos de exemplo\n\n");
    out.push_str(
        "Documentação científica de cada circuito do menu **Exemplos de Circuitos**: o objetivo, o \
circuito, a teoria com as equações, os resultados esperados e o que observar no simulador. Os \
documentos são escritos por categoria; uma categoria só aparece como *documentado* quando todos os \
seus exemplos têm documento.\n\n",
    );
    out.push_str("> Este índice é gerado por `cargo run --release --bin exdocs -- sync`; não o edite à mão.\n\n");
    out.push_str(&format!(
        "**Situação:** {done} de {total} entradas do menu documentadas ({} documentos).\n\n",
        documents.len()
    ));
    out.push_str("| Categoria | Situação |\n|-----------|----------|\n");
    for item in &list.items {
        if let ExampleItem::Group { title, items } = item {
            out.push_str(&format!("| {title} | {} |\n", status(items, stems)));
        }
    }
    out.push('\n');
    let loose: Vec<&ExampleItem> =
        list.items.iter().filter(|i| matches!(i, ExampleItem::Circuit { .. })).collect();
    if !loose.is_empty() {
        let items: Vec<ExampleItem> = loose.into_iter().cloned().collect();
        render_group(&mut out, "Sem categoria", &items, 2, stems);
    }
    for item in &list.items {
        if let ExampleItem::Group { title, items } = item {
            render_group(&mut out, title, items, 2, stems);
        }
    }
    out
}

/// Documents that no entry of the menu asks for.
pub fn orphans(list: &ExampleList, stems: &BTreeSet<String>) -> Vec<String> {
    let mut menu_files = Vec::new();
    files(&list.items, &mut menu_files);
    let known: BTreeSet<String> = menu_files.iter().map(|f| stem(f)).collect();
    stems.iter().filter(|s| !known.contains(*s)).cloned().collect()
}

/// A neighbour of an example in its group: `(file, title)`.
pub type Neighbour = Option<(String, String)>;

/// The groups the file sits in (first occurrence in the menu order) and its previous and next
/// neighbours of that group that have a document.
pub fn home(
    list: &ExampleList,
    file: &str,
    stems: &BTreeSet<String>,
) -> Option<(Vec<String>, Neighbour, Neighbour)> {
    fn find<'a>(
        items: &'a [ExampleItem],
        file: &str,
        path: &mut Vec<String>,
    ) -> Option<(Vec<String>, &'a [ExampleItem], usize)> {
        for (i, item) in items.iter().enumerate() {
            if matches!(item, ExampleItem::Circuit { file: f, .. } if f == file) {
                return Some((path.clone(), items, i));
            }
            if let ExampleItem::Group { title, items } = item {
                path.push(title.clone());
                if let Some(found) = find(items, file, path) {
                    return Some(found);
                }
                path.pop();
            }
        }
        None
    }
    let (path, siblings, at) = find(&list.items, file, &mut Vec::new())?;
    let with_doc = |item: &ExampleItem| match item {
        ExampleItem::Circuit { file, title } if stems.contains(&stem(file)) => {
            Some((file.clone(), title.clone()))
        }
        _ => None,
    };
    let prev = siblings[..at].iter().rev().find_map(with_doc);
    let next = siblings[at + 1..].iter().find_map(with_doc);
    Some((path, prev, next))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::examples::parse_setup_list;

    const MENU: &str = "# c\n+Basics\na.txt Alpha\nb.txt Beta\nc.txt Gamma\n-\n+Filters\n+Sub\nd.txt Delta\ne.txt Eps\n-\nf.txt Phi\n-\n+Logic\nb.txt Beta again\ng.txt Gee\n-\n";

    fn stems(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_category_is_documented_only_when_every_entry_has_a_document() {
        let l = parse_setup_list(MENU);
        let all = stems(&["a", "b", "c", "d", "e", "f", "g"]);
        let idx = render_index(&l, &all);
        for h in [
            "## Basics — documentado",
            "## Filters — documentado",
            "### Sub — documentado",
            "## Logic — documentado",
        ] {
            assert!(idx.contains(h), "{h}\n{idx}");
        }
        let some = stems(&["a", "b", "d"]);
        let idx = render_index(&l, &some);
        assert!(idx.contains("## Basics — pendente (2 de 3)"), "{idx}");
        assert!(idx.contains("### Sub — pendente (1 de 2)"), "{idx}");
        assert!(idx.contains("## Filters — pendente (1 de 3)"), "{idx}");
        // a file listed under two categories counts in both
        assert!(idx.contains("## Logic — pendente (1 de 2)"), "{idx}");
    }

    #[test]
    fn only_existing_documents_are_linked_and_the_order_is_the_menu_order() {
        let l = parse_setup_list(MENU);
        let idx = render_index(&l, &stems(&["a", "c"]));
        assert!(idx.contains("- [Alpha](a.md)"), "{idx}");
        assert!(idx.contains("- Beta — *pendente*"), "{idx}");
        assert!(!idx.contains("](b.md)"), "{idx}");
        let (a, c) = (idx.find("Alpha").unwrap(), idx.find("Gamma").unwrap());
        assert!(a < c);
        // the same file listed twice is linked from both places once it has a document
        let both = render_index(&l, &stems(&["b"]));
        assert_eq!(both.matches("](b.md)").count(), 2, "{both}");
    }

    #[test]
    fn the_index_is_the_same_on_a_second_run() {
        let l = parse_setup_list(MENU);
        let s = stems(&["a", "d"]);
        assert_eq!(render_index(&l, &s), render_index(&l, &s));
    }

    #[test]
    fn orphans_are_documents_without_an_entry() {
        let l = parse_setup_list(MENU);
        assert_eq!(orphans(&l, &stems(&["a", "zzz", "b"])), vec!["zzz".to_string()]);
        assert!(orphans(&l, &stems(&["a"])).is_empty());
    }

    #[test]
    fn the_home_of_a_file_is_its_first_occurrence() {
        let l = parse_setup_list(MENU);
        let s = stems(&["a", "b", "c", "g"]);
        let (path, prev, next) = home(&l, "b.txt", &s).unwrap();
        assert_eq!(path, vec!["Basics".to_string()]);
        assert_eq!(prev, Some(("a.txt".to_string(), "Alpha".to_string())));
        assert_eq!(next, Some(("c.txt".to_string(), "Gamma".to_string())));
        // neighbours without a document are skipped
        let (_, prev, next) = home(&l, "a.txt", &stems(&["a", "c"])).unwrap();
        assert_eq!(prev, None);
        assert_eq!(next, Some(("c.txt".to_string(), "Gamma".to_string())));
        // the last of its group has no next
        assert_eq!(home(&l, "c.txt", &s).unwrap().2, None);
        // a sub-group is part of the path
        assert_eq!(
            home(&l, "d.txt", &s).unwrap().0,
            vec!["Filters".to_string(), "Sub".to_string()]
        );
        assert!(home(&l, "nope.txt", &s).is_none());
    }
}
