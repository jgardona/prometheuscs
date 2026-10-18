// SPDX-License-Identifier: GPL-2.0-or-later
//! Developer tool for the documentation of the example circuits (`docs/exemplos`).
//!
//! Usage, from the repository root:
//!
//! - `exdocs sync`: writes the index and the generated parts of every document.
//! - `exdocs check`: runs every check; prints one line per problem; exit status 1 when there are some.
//! - `exdocs facts <example.txt>`: prints what an example contains.
//! - `exdocs skeleton <example.txt>`: starts the document of an example.

use prometheuscs::app::examples::{ExampleItem, parse_setup_list};
use prometheuscs::elements::registry::Registry;
use prometheuscs::exdocs::{self, DOCS_DIR, index, region};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "uso: exdocs sync | check | facts <exemplo.txt> | skeleton <exemplo.txt> | probe <exemplo.txt> <t> [alterna=N[@t],M] | posts <exemplo.txt> | series <exemplo.txt> <linha-de-verificação> | figure <exemplo.txt> <saida.svg>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = Path::new(".");
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["sync"] => sync(root),
        ["check"] => check(root),
        ["facts", file] => facts(root, file),
        ["skeleton", file] => skeleton(root, file),
        ["posts", file] => posts(root, file),
        ["series", file, line] => series(root, file, line),
        ["probe", file, t] => probe(root, file, t, &[]),
        ["probe", file, t, toggles] => toggles_arg(toggles).and_then(|n| probe(root, file, t, &n)),
        ["figure", file, out] => figure(root, file, out),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("exdocs: {e}");
            ExitCode::FAILURE
        }
    }
}

fn sync(root: &Path) -> Result<ExitCode, String> {
    let changed = exdocs::sync_all(root)?;
    if changed.is_empty() {
        println!("nada mudou");
    }
    for f in changed {
        println!("escrito: {f}");
    }
    Ok(ExitCode::SUCCESS)
}

fn check(root: &Path) -> Result<ExitCode, String> {
    let problems = exdocs::check_all(root);
    for p in &problems {
        println!("{p}");
    }
    if problems.is_empty() {
        println!("tudo certo");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("{} problema(s)", problems.len());
        Ok(ExitCode::FAILURE)
    }
}

/// The title and the groups of the first occurrence of `file` in the menu.
fn menu_entry(root: &Path, file: &str) -> Result<(String, Vec<String>), String> {
    let text =
        std::fs::read_to_string(root.join("assets/setuplist.txt")).map_err(|e| e.to_string())?;
    let list = parse_setup_list(&text);
    fn find(
        items: &[ExampleItem],
        file: &str,
        path: &mut Vec<String>,
    ) -> Option<(String, Vec<String>)> {
        for item in items {
            match item {
                ExampleItem::Circuit { file: f, title } if f == file => {
                    return Some((title.clone(), path.clone()));
                }
                ExampleItem::Group { title, items } => {
                    path.push(title.clone());
                    if let Some(found) = find(items, file, path) {
                        return Some(found);
                    }
                    path.pop();
                }
                ExampleItem::Circuit { .. } => {}
            }
        }
        None
    }
    find(&list.items, file, &mut Vec::new())
        .ok_or_else(|| format!("{file} não está no menu de exemplos"))
}

fn facts(root: &Path, file: &str) -> Result<ExitCode, String> {
    let (title, path) = menu_entry(root, file)?;
    let text = std::fs::read_to_string(root.join("assets/circuits").join(file))
        .map_err(|e| format!("{file}: {e}"))?;
    println!("Exemplo: {file}");
    println!("Título: {title}");
    println!("Categoria: {}", path.join(" > "));
    match region::component_rows(&text, &Registry::standard()) {
        None => println!(
            "(o exemplo não carrega nesta compilação: falta alguma família de componentes)"
        ),
        Some(rows) => {
            println!("Elementos (número: tipo, classe, valores):");
            for r in rows {
                println!("  {}: {} ({}) — {}", r.index, r.kind, r.class, r.values);
            }
        }
    }
    let scopes = text.lines().filter(|l| l.starts_with("o ") || l.starts_with("<o ")).count();
    println!("Osciloscópios: {scopes}");
    Ok(ExitCode::SUCCESS)
}

fn figure(root: &Path, file: &str, out: &str) -> Result<ExitCode, String> {
    let text = std::fs::read_to_string(root.join("assets/circuits").join(file))
        .map_err(|e| format!("{file}: {e}"))?;
    let svg = exdocs::figure::schematic_svg(&text, &Registry::standard())
        .ok_or_else(|| format!("{file}: não foi possível desenhar o esquema"))?;
    std::fs::write(out, svg).map_err(|e| format!("{out}: {e}"))?;
    Ok(ExitCode::SUCCESS)
}

fn posts(root: &Path, file: &str) -> Result<ExitCode, String> {
    let text = std::fs::read_to_string(root.join("assets/circuits").join(file))
        .map_err(|e| format!("{file}: {e}"))?;
    match exdocs::checks::posts(&text, &Registry::standard()) {
        None => println!("(o exemplo não carrega nesta compilação)"),
        Some(rows) => {
            for (i, class, pts) in rows {
                let pts: Vec<String> = pts.iter().map(|(x, y)| format!("{x},{y}")).collect();
                println!("{i} {class} {}", pts.join(" "));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn series(root: &Path, file: &str, line: &str) -> Result<ExitCode, String> {
    let text = std::fs::read_to_string(root.join("assets/circuits").join(file))
        .map_err(|e| format!("{file}: {e}"))?;
    let check = exdocs::checks::parse_check(&format!("{line} esperado=0"))?;
    match exdocs::checks::series(&text, &check, &Registry::standard())? {
        None => println!("(o exemplo não carrega nesta compilação)"),
        Some(samples) => {
            for (t, v) in samples {
                println!("{t:.9e} {v:.9e}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn toggles_arg(arg: &str) -> Result<Vec<(usize, f64)>, String> {
    let list =
        arg.strip_prefix("alterna=").ok_or("o último argumento deve ser `alterna=N[@t],M`")?;
    exdocs::checks::parse_toggles(list)
}

fn probe(root: &Path, file: &str, t: &str, toggles: &[(usize, f64)]) -> Result<ExitCode, String> {
    let t: f64 = t.parse().map_err(|_| format!("`{t}` não é um tempo em segundos"))?;
    let text = std::fs::read_to_string(root.join("assets/circuits").join(file))
        .map_err(|e| format!("{file}: {e}"))?;
    match prometheuscs::exdocs::checks::probe(&text, t, toggles, &Registry::standard())? {
        None => println!("(o exemplo não carrega nesta compilação)"),
        Some(rows) => {
            println!("t = {t} s; número: classe, vd, corrente, tensão de cada terminal");
            for r in rows {
                let posts: Vec<String> = r.posts.iter().map(|v| format!("{v:.6}")).collect();
                println!(
                    "  {}: {} vd={:.6} i={:.6e} [{}]",
                    r.index,
                    r.class,
                    r.vd,
                    r.current,
                    posts.join(", ")
                );
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn skeleton(root: &Path, file: &str) -> Result<ExitCode, String> {
    let (title, path) = menu_entry(root, file)?;
    let stem = index::stem(file);
    let out = root.join(DOCS_DIR).join(format!("{stem}.md"));
    if out.exists() {
        return Err(format!("{} já existe; não vou sobrescrever", out.display()));
    }
    let text = std::fs::read_to_string(root.join("assets/circuits").join(file))
        .map_err(|e| format!("{file}: {e}"))?;
    let table = region::components_region(&text, &Registry::standard()).unwrap_or_default();
    let doc = [
        "---".to_string(),
        format!("exemplo: {file}"),
        format!("titulo: {title}"),
        format!("categoria: {}", path.join(" > ")),
        "sem-verificacao: PENDENTE".to_string(),
        "---".to_string(),
        String::new(),
        format!("# {title}"),
        String::new(),
        "<!-- navegacao:inicio -->".to_string(),
        "<!-- navegacao:fim -->".to_string(),
        String::new(),
        "## Objetivo".to_string(),
        String::new(),
        "## O circuito".to_string(),
        String::new(),
        "<!-- componentes:inicio -->".to_string(),
        table,
        "<!-- componentes:fim -->".to_string(),
        String::new(),
        "## Teoria".to_string(),
        String::new(),
        "## Resultados esperados".to_string(),
        String::new(),
        "## O que observar".to_string(),
        String::new(),
        "## Limites do modelo".to_string(),
        String::new(),
        "## Referências".to_string(),
        String::new(),
    ]
    .join("\n");
    std::fs::create_dir_all(root.join(DOCS_DIR)).map_err(|e| e.to_string())?;
    std::fs::write(&out, doc).map_err(|e| e.to_string())?;
    println!("criado: {}", out.display());
    Ok(ExitCode::SUCCESS)
}
