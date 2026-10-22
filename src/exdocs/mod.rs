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
    let text = index::render_index(&ctx.list, &ctx.stems);
    write_if_changed(&ctx.docs.join("README.md"), &text, &mut changed, root)?;
    Ok(changed)
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
                if fm.category != path.join(" > ") {
                    say(format!(
                        "`categoria` é `{}`, o menu diz `{}`",
                        fm.category,
                        path.join(" > ")
                    ));
                }
                let example = std::fs::read_to_string(ctx.circuits.join(file));
                for (n, line) in fm.checks.iter().enumerate() {
                    match checks::parse_check(line) {
                        Err(e) => say(format!("verificação {}: {e}", n + 1)),
                        Ok(check) => match &example {
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
    let want = index::render_index(&ctx.list, &ctx.stems);
    match std::fs::read_to_string(ctx.docs.join("README.md")) {
        Ok(have) if have == want => {}
        _ => problems.push(format!(
            "{DOCS_DIR}/README.md: o índice está desatualizado (rode `exdocs sync`)"
        )),
    }
    problems
}
