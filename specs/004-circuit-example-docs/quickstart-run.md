# Quickstart run (T060), 2026-11-11

| Step of `quickstart.md` | Result |
|---|---|
| 1. Tests first | Done in Phase 2; the new tests of Phase 4 (`every_document_is_one_click_from_the_index`, `the_index_follows_the_order_and_the_categories_of_the_menu`) were written before the README row and `the_readme_links_the_index` was red until T050. |
| 2. Tool | `exdocs sync` changes nothing on a second run; `exdocs check`: "tudo certo". |
| 3. A wave | Waves 9 and 10 written with `skeleton`/`facts`/`probe`, then `sync` and `check`; all 21 categories `documentado`, 367 of 367 entries, 366 documents. |
| 4. Gates | `cargo fmt --check`, `cargo clippy --all-targets -D warnings` (with and without `--features gui`) clean; `cargo test --release --no-fail-fast`: 568 tests pass, 0 fail; the twelve family builds pass. Three builds (without `diodes`, `analog`, `transistors`) first failed: documents of examples whose elements sit inside a subcircuit or composite (OTA, Norton, `fullrect-sc`, `ujtosc`) loaded without errors but with missing parts, and two parser tests of wave 8 needed `analog`. Fixed by skipping those checks in such builds (`lacks_hidden_family` in `src/exdocs/mod.rs`) and gating the two tests. The debug build of `example_docs` was not run in full (too slow), only the index tests. |
| 5. Rendering | **Not done** (see `render-check.md`). |
| 6. From the README | The documentation table links `docs/exemplos/`; the index links every document (tested). |
| 7. Numbers in the window | **Not done** (see `audit.md`); instead, 31 documents were derived independently and compared with `exdocs`. |
