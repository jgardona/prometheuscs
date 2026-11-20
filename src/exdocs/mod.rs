// SPDX-License-Identifier: GPL-2.0-or-later
//! The documentation of the example circuits: one document for each example of the menu, an index,
//! and the checks that keep them honest (see `specs/004-circuit-example-docs`).
//!
//! The documents are in `docs/exemplos/<example>.md`. The index and the generated parts of each
//! document (the components table and the navigation line) are written by [`sync_all`]; the text,
//! the equations, the diagrams, the links and the results each document states are checked by
//! [`check_all`].

pub mod checks;
pub mod figure;
pub mod front;
pub mod index;
pub mod lint;
pub mod region;
pub mod symbols;

use crate::app::examples::{ExampleItem, ExampleList, parse_setup_list};
use crate::elements::registry::Registry;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Where the documents are, relative to the repository root.
pub const DOCS_DIR: &str = "docs/exemplos";

/// Where the schematics are, under [`DOCS_DIR`].
const FIGURES: &str = "img";

/// What the menu and the folders say.
struct Context {
    list: ExampleList,
    /// The documents that exist (`docs/exemplos/<stem>.md`, the index left out).
    stems: BTreeSet<String>,
    docs: PathBuf,
    circuits: PathBuf,
}

fn load(root: &Path) -> Result<Context, String> {
    let setup = root.join("assets/setuplist.txt");
    let text = std::fs::read_to_string(&setup).map_err(|e| format!("{}: {e}", setup.display()))?;
    let docs = root.join(DOCS_DIR);
    let mut stems = BTreeSet::new();
    if let Ok(dir) = std::fs::read_dir(&docs) {
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(stem) = name.strip_suffix(".md") {
                if stem != "README" {
                    stems.insert(stem.to_string());
                }
            }
        }
    }
    Ok(Context {
        list: parse_setup_list(&text),
        stems,
        docs,
        circuits: root.join("assets/circuits"),
    })
}

/// For every file of the menu: its title and its groups, at its first occurrence.
fn first_occurrences(
    items: &[ExampleItem],
    path: &mut Vec<String>,
    out: &mut BTreeMap<String, (String, Vec<String>)>,
) {
    for item in items {
        match item {
            ExampleItem::Circuit { file, title } => {
                out.entry(file.clone()).or_insert_with(|| (title.clone(), path.clone()));
            }
            ExampleItem::Group { title, items } => {
                path.push(title.clone());
                first_occurrences(items, path, out);
                path.pop();
            }
        }
    }
}

fn write_if_changed(
    path: &Path,
    text: &str,
    changed: &mut Vec<String>,
    root: &Path,
) -> Result<(), String> {
    if std::fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    changed.push(path.strip_prefix(root).unwrap_or(path).display().to_string());
    Ok(())
}

/// The navigation region a document should have.
fn expected_navigation(ctx: &Context, file: &str) -> Option<String> {
    let (_, prev, next) = index::home(&ctx.list, file, &ctx.stems)?;
    let prev = prev.map(|(f, t)| (index::stem(&f), t));
    let next = next.map(|(f, t)| (index::stem(&f), t));
    Some(region::navigation_region(
        prev.as_ref().map(|(s, t)| (s.as_str(), t.as_str())),
        next.as_ref().map(|(s, t)| (s.as_str(), t.as_str())),
    ))
}

/// The documents that already have the section of symbols.
fn documents_with_symbols(ctx: &Context) -> BTreeSet<String> {
    ctx.stems
        .iter()
        .filter(|stem| {
            std::fs::read_to_string(ctx.docs.join(format!("{stem}.md")))
                .is_ok_and(|doc| doc.contains(&format!("\n## {}\n", symbols::SECTION)))
        })
        .cloned()
        .collect()
}

/// Everything wrong with the symbols sections and the glossary under `root`, one
/// `file: message` per problem. `only` limits the documents checked (the glossary is then
/// not checked).
pub fn check_symbols(root: &Path, only: Option<&str>) -> Vec<String> {
    let ctx = match load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let glossary_text = std::fs::read_to_string(root.join(symbols::GLOSSARY)).unwrap_or_default();
    let (glossary, parse_problems) = symbols::parse_glossary(&glossary_text);
    let mut problems = Vec::new();
    let mut infos = BTreeMap::new();
    for stem in &ctx.stems {
        if only.is_some_and(|o| o != stem) {
            continue;
        }
        let Ok(doc) = std::fs::read_to_string(ctx.docs.join(format!("{stem}.md"))) else {
            continue;
        };
        let (found, info) = symbols::check_document(&doc, &glossary, symbols::REQUIRE_SECTION);
        problems.extend(found.into_iter().map(|m| format!("{DOCS_DIR}/{stem}.md: {m}")));
        infos.insert(stem.clone(), info);
    }
    if only.is_none() {
        problems.extend(symbols::check_glossary(&glossary, parse_problems, &infos));
    }
    problems
}

/// What a document uses and what it lacks, for the author of its symbols section: the symbols of
/// its math, the fields of its components table, and the problems of `check_symbols` for it.
///
/// # Errors
/// The document cannot be read.
pub fn symbols_report(root: &Path, stem: &str) -> Result<String, String> {
    let path = root.join(DOCS_DIR).join(format!("{stem}.md"));
    let doc = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let body = lint::strip_front_matter(&doc);
    let (text, _) = lint::split_fences(&symbols::without_section(body));
    let mut out = String::from("símbolos usados em fórmulas:\n");
    for s in symbols::used_symbols(&text) {
        out.push_str(&format!("  {s}\n"));
    }
    out.push_str("campos da tabela de componentes:\n");
    for f in
        region::get_region(&doc, "componentes").map(symbols::component_fields).unwrap_or_default()
    {
        out.push_str(&format!("  {f}\n"));
    }
    out.push_str("problemas:\n");
    for p in check_symbols(root, Some(stem)) {
        out.push_str(&format!("  {p}\n"));
    }
    Ok(out)
}

/// Writes the generated parts under `root` (the repository root): the index and the regions of
/// every document. Returns the files that changed.
pub fn sync_all(root: &Path) -> Result<Vec<String>, String> {
    let ctx = load(root)?;
    let registry = Registry::standard();
    let mut changed = Vec::new();
    let mut menu = BTreeMap::new();
    first_occurrences(&ctx.list.items, &mut Vec::new(), &mut menu);
    for file in menu.keys() {
        let stem = index::stem(file);
        if !ctx.stems.contains(&stem) {
            continue;
        }
        let path = ctx.docs.join(format!("{stem}.md"));
        let mut doc =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(nav) = expected_navigation(&ctx, file) {
            doc = region::replace_region(&doc, "navegacao", &nav).unwrap_or(doc);
        }
        if let Ok(example) = std::fs::read_to_string(ctx.circuits.join(file)) {
            if let Some(svg) = figure::schematic_svg(&example, &registry) {
                write_if_changed(
                    &ctx.docs.join(FIGURES).join(format!("{stem}.svg")),
                    &svg,
                    &mut changed,
                    root,
                )?;
                doc = region::ensure_figure_region(&doc);
                doc = region::replace_region(&doc, "esquema", &region::figure_region(&stem))
                    .unwrap_or(doc);
            }
            if let Some(table) = region::components_region(&example, &registry) {
                doc = region::replace_region(&doc, "componentes", &table).unwrap_or(doc);
            }
        }
        write_if_changed(&path, &doc, &mut changed, root)?;
    }
    let with_symbols = documents_with_symbols(&ctx);
    let text = index::render_index(&ctx.list, &ctx.stems, &with_symbols);
    write_if_changed(&ctx.docs.join("README.md"), &text, &mut changed, root)?;
    Ok(changed)
}

/// The `categoria` a document of an example under `path` must carry: the path of its menu
/// groups, or `Sem categoria` for an entry that is in no group (the blank circuit).
#[must_use]
pub fn category_label(path: &[String]) -> String {
    if path.is_empty() { "Sem categoria".to_string() } else { path.join(" > ") }
}

/// Whether the example uses a family that this build leaves out *inside* a subcircuit or a
/// composite element (the OTA, the Norton amplifier, the subcircuit rectifier, the unijunction
/// transistor): such a file still loads without errors but its elements are missing, so its
/// numbers cannot be checked here and the check is skipped, as for a file that does not load.
fn lacks_hidden_family(file: &str) -> bool {
    let needs: &[bool] = if file.starts_with("ota-") {
        &[cfg!(feature = "transistors")]
    } else if file.starts_with("norton-") || file == "ujtosc.txt" {
        &[cfg!(feature = "diodes"), cfg!(feature = "analog")]
    } else if file == "fullrect-sc.txt" {
        &[cfg!(feature = "diodes")]
    } else {
        &[]
    };
    needs.iter().any(|have| !have)
}

/// Everything wrong with the documents under `root`, one `file: message` per problem.
pub fn check_all(root: &Path) -> Vec<String> {
    let ctx = match load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let registry = Registry::standard();
    let mut problems = Vec::new();
    let mut menu = BTreeMap::new();
    first_occurrences(&ctx.list.items, &mut Vec::new(), &mut menu);
    for stem in index::orphans(&ctx.list, &ctx.stems) {
        problems.push(format!("{DOCS_DIR}/{stem}.md: não há exemplo com esse nome no menu"));
    }
    for (file, (title, path)) in &menu {
        let stem = index::stem(file);
        if !ctx.stems.contains(&stem) {
            continue;
        }
        let name = format!("{DOCS_DIR}/{stem}.md");
        let mut say = |m: String| problems.push(format!("{name}: {m}"));
        let Ok(doc) = std::fs::read_to_string(ctx.docs.join(format!("{stem}.md"))) else {
            say("não foi possível ler o arquivo".into());
            continue;
        };
        match front::parse_front_matter(&doc) {
            Err(e) => say(e),
            Ok(fm) => {
                if fm.example != *file {
                    say(format!("`exemplo` é {}, deveria ser {file}", fm.example));
                }
                if fm.title != *title {
                    say(format!("`titulo` é `{}`, o menu diz `{title}`", fm.title));
                }
                let category = category_label(path);
                if fm.category != category {
                    say(format!("`categoria` é `{}`, o menu diz `{category}`", fm.category));
                }
                let example = std::fs::read_to_string(ctx.circuits.join(file));
                for (n, line) in fm.checks.iter().enumerate() {
                    match checks::parse_check(line) {
                        Err(e) => say(format!("verificação {}: {e}", n + 1)),
                        Ok(check) => match &example {
                            Ok(_) if lacks_hidden_family(file) => {}
                            Ok(text) => {
                                if let Err(e) = checks::run_check(text, &check, &registry) {
                                    say(format!("verificação {} ({line}): {e}", n + 1));
                                }
                            }
                            Err(e) => say(format!("o exemplo {file} não pôde ser lido: {e}")),
                        },
                    }
                }
            }
        }
        for m in lint::lint_document(&doc, &ctx.docs) {
            say(m);
        }
        match (region::get_region(&doc, "navegacao"), expected_navigation(&ctx, file)) {
            (None, _) => say("falta a região de navegação (`<!-- navegacao:inicio -->`)".into()),
            (Some(have), Some(want)) if have != want => {
                say("a linha de navegação está desatualizada (rode `exdocs sync`)".into());
            }
            _ => {}
        }
        let schematic = std::fs::read_to_string(ctx.circuits.join(file))
            .ok()
            .and_then(|text| figure::schematic_svg(&text, &registry));
        if let Some(svg) = schematic {
            match std::fs::read_to_string(ctx.docs.join(FIGURES).join(format!("{stem}.svg"))) {
                Ok(have) if have == svg => {}
                Ok(_) => say("o esquema (img) está desatualizado (rode `exdocs sync`)".into()),
                Err(_) => say("falta o esquema em img (rode `exdocs sync`)".into()),
            }
            if region::get_region(&doc, "esquema") != Some(region::figure_region(&stem).as_str()) {
                say("falta a região do esquema (rode `exdocs sync`)".into());
            }
        }
        match region::get_region(&doc, "componentes") {
            None => say("falta a região de componentes (`<!-- componentes:inicio -->`)".into()),
            Some(have) => {
                let want = std::fs::read_to_string(ctx.circuits.join(file))
                    .ok()
                    .and_then(|text| region::components_region(&text, &registry));
                if want.is_some_and(|w| w != have) {
                    say("a tabela de componentes está desatualizada (rode `exdocs sync`)".into());
                }
            }
        }
    }
    let want = index::render_index(&ctx.list, &ctx.stems, &documents_with_symbols(&ctx));
    match std::fs::read_to_string(ctx.docs.join("README.md")) {
        Ok(have) if have == want => {}
        _ => problems.push(format!(
            "{DOCS_DIR}/README.md: o índice está desatualizado (rode `exdocs sync`)"
        )),
    }
    problems.extend(check_symbols(root, None));
    problems
}

#[cfg(test)]
mod tests {
    use super::category_label;

    #[test]
    fn the_category_of_an_entry_outside_any_group_is_named() {
        assert_eq!(category_label(&[]), "Sem categoria");
        let path = ["Misc Devices".to_string(), "JFETs".to_string()];
        assert_eq!(category_label(&path), "Misc Devices > JFETs");
    }
}
