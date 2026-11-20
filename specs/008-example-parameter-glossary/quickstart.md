# Quickstart: validating the feature

Prerequisites: Rust toolchain, `xelatex`, `rsvg-convert`, Python 3.

1. **Checker and tests** (every wave): `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
   → all pass, including `tests/example_docs.rs` with the new glossary checks.
2. **Document checks**: `cargo run --release --bin exdocs -- check` → no output, exit 0.
3. **Index**: `cargo run --release --bin exdocs -- sync` → no files changed after the wave's commit;
   the index shows `símbolos: documentado` for finished categories and `pendente (X de Y)` for
   the others.
4. **Authoring aid**: `cargo run --release --bin exdocs -- symbols nmosinv` → lists tokens and
   gaps; empty gaps when the document is finished.
5. **Untouched checks (FR-009)**: `git diff` of a wave shows no change in front matter,
   generated regions or numbers; `exdocs check` still runs every `verificacoes` line green.
6. **PDF (FR-012/013)**: `python3 docs/livro/build_book.py` → ends with the PDF path, prints no
   `aviso:` line; open it and confirm for a MOSFET example the symbols table, and at the end the
   glossary appendix with readable tables and working references.
7. **Reader test (SC-003)**: pick three random documents; for each symbol, find name, meaning and
   unit in under a minute from the document alone.
8. **Final gate**: with the final switch on, `exdocs check` reports no document without the
   section; README links the glossary.
