# Quickstart: validating the example documentation

Prerequisites: Rust toolchain.

1. **Tests first** (Red): `cargo test --release --test example_docs` fails (no `exdocs` module
   yet); then each failure mode of the checker has a test that fails until it is implemented.
2. **Tool**: `cargo run --release --bin exdocs -- check` reports the pending categories as pending
   and no errors; `exdocs sync` is idempotent (a second run changes nothing).
3. **A wave**: for each example of the wave, `exdocs skeleton <file>`, write the sections, fill
   `verificacoes`, then `exdocs sync` and `exdocs check`: no errors; the categories of the wave
   show "documentado" in `docs/exemplos/README.md`.
4. **Gates**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (with and without
   `--features gui`), `cargo test --release`, and the families builds
   (`cargo test --no-default-features --features "<all but one>"`) stay green.
5. **Rendering**: open the index and five documents of the wave on GitHub (or a Markdown preview
   with math and Mermaid): equations and diagrams render, no raw `$` or ` ```mermaid ` shows.
6. **From the README**: the documentation table links the index; from the index, any example of
   a documented category is one more click.
7. **Numbers**: pick three documents, run the example in the simulator and compare by eye what
   the document says with the simulator.
