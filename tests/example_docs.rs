// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The documentation of the example circuits (`docs/exemplos`): complete, up to date, and honest.

use prometheuscs::exdocs::{check_all, sync_all};
use std::path::{Path, PathBuf};

#[test]
fn the_documents_are_complete_and_up_to_date() {
    let problems = check_all(Path::new("."));
    assert!(problems.is_empty(), "{} problem(s):\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn the_index_is_the_one_sync_would_write() {
    // `sync` on the real repository would change nothing: the committed index is current
    let copy = scratch("index-current");
    copy_tree(Path::new("docs/exemplos"), &copy.join("docs/exemplos"));
    std::fs::create_dir_all(copy.join("assets")).unwrap();
    std::fs::copy("assets/setuplist.txt", copy.join("assets/setuplist.txt")).unwrap();
    // the circuits are only read for the components tables and the checks; link the folder
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        std::fs::canonicalize("assets/circuits").unwrap(),
        copy.join("assets/circuits"),
    )
    .unwrap();
    let changed = sync_all(&copy).unwrap();
    assert!(changed.is_empty(), "sync would change: {changed:?}");
}

#[test]
fn the_readme_links_the_index() {
    let readme = std::fs::read_to_string("README.md").unwrap();
    assert!(
        readme.contains("docs/exemplos/README.md"),
        "README.md must link docs/exemplos/README.md"
    );
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("exdocs-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    if let Ok(dir) = std::fs::read_dir(from) {
        for e in dir.flatten() {
            let (src, dst) = (e.path(), to.join(e.file_name()));
            if src.is_dir() {
                copy_tree(&src, &dst);
            } else {
                std::fs::copy(&src, &dst).unwrap();
            }
        }
    }
}

// ---- a small repository to tamper with (the examples need passive and sources elements)

#[cfg(all(feature = "passive", feature = "sources"))]
mod tamper {
    use super::*;

    const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 128 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nr 64 0 64 128 0 3000\nw 64 128 0 128 0\n";

    fn doc(example: &str, title: &str, category: &str, expected: &str) -> String {
        format!(
            "---\nexemplo: {example}\ntitulo: {title}\ncategoria: {category}\nverificacoes:\n  - t=0.001 elemento=1:ResistorElm grandeza=v1 esperado={expected}\n---\n\n# {title}\n\n\
<!-- navegacao:inicio -->\n<!-- navegacao:fim -->\n\n## Objetivo\n\nMostrar o divisor.\n\n## O circuito\n\n<!-- componentes:inicio -->\n<!-- componentes:fim -->\n\n\
## Teoria\n\nPelo divisor, $V = 5 \\frac{{3}}{{4}}$.\n\n## Resultados esperados\n\n3,75 V.\n\n## O que observar\n\nA tensão.\n\n## Limites do modelo\n\nIdeal.\n\n## Referências\n\nLivro.\n"
        )
    }

    /// A repository with two examples, `a.txt` and `b.txt`, in the category `Divisores`.
    fn repo(name: &str, docs_for: &[&str]) -> PathBuf {
        let root = scratch(name);
        std::fs::create_dir_all(root.join("assets/circuits")).unwrap();
        std::fs::create_dir_all(root.join("docs/exemplos")).unwrap();
        std::fs::write(
            root.join("assets/setuplist.txt"),
            "# c\n+Divisores\na.txt Divisor A\nb.txt Divisor B\n-\n",
        )
        .unwrap();
        for f in ["a.txt", "b.txt"] {
            std::fs::write(root.join("assets/circuits").join(f), RC).unwrap();
        }
        for stem in docs_for {
            let title = if *stem == "a" { "Divisor A" } else { "Divisor B" };
            std::fs::write(
                root.join(format!("docs/exemplos/{stem}.md")),
                doc(&format!("{stem}.txt"), title, "Divisores", "3.75"),
            )
            .unwrap();
        }
        sync_all(&root).unwrap();
        root
    }

    #[test]
    fn a_synced_repository_with_correct_documents_has_no_problems() {
        let root = repo("ok", &["a", "b"]);
        assert_eq!(check_all(&root), Vec::<String>::new());
        let index = std::fs::read_to_string(root.join("docs/exemplos/README.md")).unwrap();
        assert!(index.contains("## Divisores — documentado"), "{index}");
        // sync is idempotent
        assert!(sync_all(&root).unwrap().is_empty());
        // the navigation line was written
        let a = std::fs::read_to_string(root.join("docs/exemplos/a.md")).unwrap();
        assert!(a.contains("[Próximo: Divisor B](b.md)"), "{a}");
        assert!(a.contains("| 1 | `ResistorElm` |"), "{a}");
    }

    #[test]
    fn a_wrong_expected_value_is_reported_with_the_numbers() {
        let root = repo("wrong", &["a"]);
        let path = root.join("docs/exemplos/a.md");
        let text = std::fs::read_to_string(&path).unwrap().replace("esperado=3.75", "esperado=4.5");
        std::fs::write(&path, text).unwrap();
        let problems = check_all(&root);
        assert!(
            problems.iter().any(|p| p.contains("a.md") && p.contains("3.75") && p.contains("4.5")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_changed_component_makes_the_table_stale_until_sync() {
        let root = repo("stale", &["a"]);
        let circuit = root.join("assets/circuits/a.txt");
        std::fs::write(&circuit, RC.replace("1000", "2000")).unwrap();
        let problems = check_all(&root);
        assert!(problems.iter().any(|p| p.contains("tabela de componentes")), "{problems:?}");
        sync_all(&root).unwrap();
        // the table follows; the check (an element of 1 kohm) is the author's to fix
        assert!(!check_all(&root).iter().any(|p| p.contains("tabela de componentes")));
    }

    #[test]
    fn a_missing_document_in_a_documented_category_and_a_stale_index_are_reported() {
        let root = repo("missing", &["a", "b"]);
        std::fs::remove_file(root.join("docs/exemplos/b.md")).unwrap();
        let problems = check_all(&root);
        // the index still says the category is documented
        assert!(
            problems.iter().any(|p| p.contains("índice") && p.contains("desatualizado")),
            "{problems:?}"
        );
        sync_all(&root).unwrap();
        let index = std::fs::read_to_string(root.join("docs/exemplos/README.md")).unwrap();
        assert!(index.contains("pendente (1 de 2)"), "{index}");
        assert_eq!(check_all(&root), Vec::<String>::new(), "a pending category is not an error");
    }

    #[test]
    fn a_new_entry_in_the_menu_makes_its_category_pending() {
        let root = repo("new-entry", &["a", "b"]);
        let setup = root.join("assets/setuplist.txt");
        std::fs::write(
            &setup,
            "# c\n+Divisores\na.txt Divisor A\nb.txt Divisor B\nc.txt Divisor C\n-\n",
        )
        .unwrap();
        std::fs::write(root.join("assets/circuits/c.txt"), RC).unwrap();
        assert!(
            check_all(&root).iter().any(|p| p.contains("índice")),
            "the index claims the category is documented"
        );
        sync_all(&root).unwrap();
        assert!(
            std::fs::read_to_string(root.join("docs/exemplos/README.md"))
                .unwrap()
                .contains("pendente (2 de 3)")
        );
    }

    #[test]
    fn a_document_without_an_entry_is_an_orphan() {
        let root = repo("orphan", &["a"]);
        std::fs::write(root.join("docs/exemplos/zzz.md"), "solto").unwrap();
        let problems = check_all(&root);
        assert!(
            problems.iter().any(|p| p.contains("zzz.md") && p.contains("menu")),
            "{problems:?}"
        );
    }
}
