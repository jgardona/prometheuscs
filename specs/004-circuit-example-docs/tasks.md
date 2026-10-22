---

description: "Task list for the scientific documentation of the circuit examples"
---

# Tasks: Scientific documentation for every circuit example

**Input**: Design documents from `/specs/004-circuit-example-docs/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/document-format.md, contracts/exdocs-cli.md, quickstart.md

**Tests**: included for the tool (constitution II, test-first). The documents are checked by the
tool and by `tests/example_docs.rs`, not by hand-written tests per document.

**Organization**: grouped by user story. Paths are from the repository root. The tool is built
first (Phase 2) because every document is accepted by it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2, US3 from spec.md

## Phase 1: Setup

- [X] T001 Add the module `pub mod exdocs;` to `src/lib.rs` and create `src/exdocs/mod.rs` (module doc, SPDX header) with the empty submodules `front`, `lint`, `index`, `region`, `checks` as files `src/exdocs/{front,lint,index,region,checks}.rs`
- [X] T002 [P] Create `src/bin/exdocs.rs` with a `main` that parses the command (`sync`, `check`, `facts`, `skeleton`) and prints usage for anything else (exit status 2); no behaviour yet
- [X] T003 [P] Create `tests/example_docs.rs` (SPDX header, `#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]` with the comment used in `tests/pull_state.rs`) with a helper that loads `assets/setuplist.txt` through `app::examples::parse_setup_list` and a helper that lists `docs/exemplos/*.md`

## Phase 2: Foundational (blocks all stories) — the tool, tests first

- [X] T004 [P] Write failing unit tests in `src/exdocs/front.rs` for the front matter: `exemplo`, `titulo`, `categoria` required; exactly one of `verificacoes` (not empty) or `sem-verificacao` (a reason); a missing key, both keys, neither and an unknown example are errors with the message `path: message`
- [X] T005 [P] Write failing unit tests in `src/exdocs/lint.rs` for the sections: the seven headings `Objetivo`, `O circuito`, `Teoria`, `Resultados esperados`, `O que observar`, `Limites do modelo`, `Referências` present, in that order, each with text; for the math: `$$` balanced, `$` balanced per paragraph, no `\usepackage` or `tabular`; for Mermaid: a block starts with `flowchart`, `graph`, `sequenceDiagram`, `stateDiagram-v2`, `classDiagram`, `xychart-beta` or `timeline` and is not empty; for links: a relative link to a file that does not exist is an error, anchors and `http(s)` links are not checked
- [X] T006 [P] Write failing unit tests in `src/exdocs/index.rs`: from a small menu text and a set of existing documents, a category is `documentado` only when every entry under it (sub-groups included) has a document, otherwise `pendente (n de m)`; entries keep the menu order; a link only for an existing document; the output is the same on a second run; a document with no menu entry is reported as an orphan; a file listed under two categories is one document, linked from both and counted as documented in both, and its home is the first occurrence
- [X] T007 [P] Write failing unit tests in `src/exdocs/region.rs`: the components region generated for a small circuit (5 V, 1 kΩ, 3 kΩ) lists each non-wire element with its number, type and values with units; the navigation region gives the index, the previous and the next entry of the same category (none at the ends); for a file listed under two categories, previous and next are those of its first category; replacing a region keeps the text around it and a second run changes nothing; a circuit that does not load without errors gives `None` (skipped), not an error
- [X] T008 [P] Write failing unit tests in `src/exdocs/checks.rs`: parse `t=0.02 elemento=2:CapacitorElm grandeza=vd fn=rms de=0.01 esperado=0.707 tol=0.03` (defaults: `fn=valor`, `tol=0.02`; `abs=` for an absolute tolerance); errors for an unknown quantity, a missing `esperado`, an element whose class differs from the one written; the runner on the 5 V / 1 kΩ / 3 kΩ circuit gives 3.75 V for `v1` and agrees within the tolerance, disagrees (with the numbers in the message) when the expected value is wrong; `max`, `min`, `rms`, `media` and `freq` over a window on a sine source
- [X] T009 Implement `src/exdocs/front.rs` (`FrontMatter`, `parse_front_matter(&str) -> Result<FrontMatter, String>`) with `///` docs, to make T004 pass
- [X] T010 [P] Implement `src/exdocs/lint.rs` (`lint_document(text, dir) -> Vec<String>`: sections, math, Mermaid, links) to make T005 pass
- [X] T011 [P] Implement `src/exdocs/index.rs` (`CategoryStatus`, `status(list, existing_stems)`, `render_index(list, existing_stems) -> String`, `orphans(list, existing_stems)`) with the index format of `contracts/document-format.md` (summary, one block per category with its status, plain title when there is no document), to make T006 pass
- [X] T012 [P] Implement `src/exdocs/region.rs` (`components_region(example_text, registry) -> Option<String>`, `navigation_region(entry, neighbours) -> String`, `replace_region(doc, name, body) -> String` for the markers `<!-- componentes:inicio -->`/`<!-- componentes:fim -->` and `<!-- navegacao:inicio -->`/`<!-- navegacao:fim -->`); element values come from `get_edit_info` with units and the locale for names, to make T007 pass
- [X] T013 [P] Implement `src/exdocs/checks.rs` (`parse_check`, `run_check(example_text, check) -> Result<(), String>`) using `format::parser::parse`, `Simulation::run_until` and, for `fn=…`, the values sampled every step of the window; `freq` is the mean period of the rising zero crossings of the signal minus its mean, to make T008 pass
- [X] T014 Implement in `src/exdocs/mod.rs` the two entry points used by the tool and the tests: `check_all(root) -> Vec<String>` (front matter, lint, regions up to date, checks, orphans, index up to date) and `sync_all(root) -> Result<Vec<String>, String>` (writes the index and the regions); `root` is the repository root
- [X] T015 Implement the commands of `src/bin/exdocs.rs` per `contracts/exdocs-cli.md`: `sync`, `check` (exit 1 on problems), `facts <example.txt>` (title, category, every element with its values, the scopes), `skeleton <example.txt>` (front matter with the example's data, the navigation and component regions, the seven sections empty; refuses to overwrite)
- [X] T016 Implement in `tests/example_docs.rs` the test `the_documents_are_complete_and_up_to_date` that runs `check_all` on the real repository and fails with the list of problems; it passes while there are no documents (every category pending) and the index is up to date; `the_index_is_the_one_sync_would_write`
- [X] T017 Run `cargo run --release --bin exdocs -- sync` once: it writes `docs/exemplos/README.md` with all 21 categories `pendente (0 de n)`; a second `sync` changes nothing; commit-ready
- [X] T018 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (also `--features gui`), `cargo test --release`, and the families builds for the new module (`cargo test --no-default-features --features "<all but one>" --lib --test example_docs` for each of the twelve families: all green)

**Checkpoint**: the tool exists, the index exists and every category is pending.

## Phase 3: User Story 1 — a document for every example (P1) 🎯 MVP = wave 1

**Goal**: every example has a document that passes the checker; the categories turn `documentado` wave by wave.

**Independent Test**: `exdocs check` has no errors; open any documented example's document and follow it in the simulator.

**Per category** (each task below): for every entry of the category in `assets/setuplist.txt`, in
menu order: run `exdocs facts <file>` and read the circuit; run `exdocs skeleton <file>`; write the
seven sections in Portuguese (theory with LaTeX, a Mermaid diagram only where it helps, credit to
Falstad and Sharp, references to textbooks); write `verificacoes` for every result quoted (or
`sem-verificacao: <reason>`), run the example to confirm each value, **never** quote a number
that the simulator does not give; then `exdocs sync` and `exdocs check`. Demonstration examples
(X/Y pictures, puzzles) explain the idea and say so; variants say what differs and link to the
other variants; chaotic circuits are described qualitatively.

### Wave 1 (36 entries) — MVP

- [X] T019 [US1] Basics (10): `docs/exemplos/{ohms,resistors,cap,induct,lrc,voltdivide,pot,potdivide,thevenin,norton}.md`
- [X] T020 [US1] A/C Circuits (9): `docs/exemplos/{capac,inductac,capmultcaps,capmultfreq,indmultind,indmultfreq,impedance,res-series,res-par}.md`
- [X] T021 [US1] Passive Filters (17): the entries of `Passive Filters` in `assets/setuplist.txt`, `docs/exemplos/<stem>.md` each
- [X] T022 [US1] Wave 1 checkpoint: `exdocs sync` and `exdocs check` clean; the index shows Basics, A/C Circuits and Passive Filters `documentado`; gates of T018

### Wave 2 (40 entries)

- [X] T023 [US1] Other Passive Circuits (40, with its sub-groups Series/Parallel, Transformers, Saturable Core, Relays, Coupled LC's): one document per entry
- [X] T024 [US1] Wave 2 checkpoint: `exdocs sync`, `exdocs check`, gates

### Wave 3 (48 entries)

- [X] T025 [US1] Diodes (20, with Zener Diodes and Voltage Multipliers): one document per entry
- [X] T026 [US1] Transistors (28, with Multivibrators, Differential Amplifiers, Push-Pull Follower, Oscillators): one document per entry
- [X] T027 [US1] Wave 3 checkpoint: `exdocs sync`, `exdocs check`, gates (2026-10-21: `sync` and `check` clean; Transistors `documentado`, 124 of 367 entries; `cargo fmt --check` and `cargo clippy --all-targets [--features gui] -- -D warnings` clean; `cargo test --release --no-fail-fast`: 47 suites pass, the only failure is `the_readme_links_the_index` of Phase 4 (T049), which stays red until T050; the twelve families builds were not re-run, T056 covers them)

### Wave 4 (21 entries)

- [X] T028 [US1] MOSFETs (21 entries): one document per entry; `cmosinverter.txt` is listed here and again in *Logic Families > CMOS*: write its document here (its category and navigation are the MOSFETs ones)
- [X] T029 [US1] Wave 4 checkpoint: `exdocs sync`, `exdocs check`, gates (2026-10-21: `sync` and `check` clean in 46 s; MOSFETs `documentado`, 146 of 367 entries; `cargo fmt --check` clean; `cargo test --release --test example_docs`: 8 of 9 pass, the only failure is `the_readme_links_the_index` (T049), red until T050; no Rust code changed in this wave, so clippy and the other suites were not re-run)

### Wave 5 (47 entries)

- [X] T030 [US1] Op-Amps (39, with Amplifiers, Oscillators, Chaotic Circuits): one document per entry (chaotic circuits qualitatively)
- [X] T031 [US1] Active Filters (8): one document per entry
- [X] T032 [US1] Wave 5 checkpoint: `exdocs sync`, `exdocs check`, gates (2026-10-22: `sync` and `check` clean; Op-Amps and Active Filters `documentado`, 193 of 367 entries (192 documents); `cargo test --release --test example_docs`: 8 of 9 pass, the only failure is `the_readme_links_the_index` (T049), red until T050; no Rust code changed in this wave, so fmt, clippy and the other suites were not re-run; the six chaotic circuits carry `sem-verificacao` with the reason, and the three Butterworth noise-driven filters are checked by RMS against the noise-bandwidth theory)

### Wave 6 (34 entries)

- [X] T033 [US1] 555 Timer Chip (10): one document per entry
- [X] T034 [US1] Power Converters (5): one document per entry
- [X] T035 [US1] Phase-Locked Loops (7): one document per entry
- [X] T036 [US1] Controlled Sources (12): one document per entry
- [X] T037 [US1] Wave 6 checkpoint: `exdocs sync`, `exdocs check`, gates (2026-10-22: `sync` and `check` clean; 555 Timer Chip, Power Converters, Phase-Locked Loops and Controlled Sources `documentado`, 227 of 367 entries (226 documents); `cargo test --release --test example_docs`: 8 of 9 pass, the only failure is `the_readme_links_the_index` (T049), red until T050; no Rust code changed in this wave, so fmt, clippy and the other suites were not re-run; the converter, `pll2`, `pll2a` and `freqdouble` documents state that the example's *slider* is bound to the wrong source field on this branch)

### Wave 7 (38 entries)

- [ ] T038 [US1] Logic Families (27 entries, RTL, DTL, TTL, NMOS, CMOS, ECL, Ternary): one document per entry; `cmosinverter.txt` already has its document from wave 4 and is only linked here (26 new documents)
- [ ] T039 [US1] Combinational Logic (11): one document per entry
- [ ] T040 [US1] Wave 7 checkpoint: `exdocs sync`, `exdocs check`, gates

### Wave 8 (35 entries)

- [ ] T041 [US1] Sequential Logic (27, Flip-Flops, Counters, Shift Registers, TD4 CPU): one document per entry
- [ ] T042 [US1] Analog/Digital (8): one document per entry
- [ ] T043 [US1] Wave 8 checkpoint: `exdocs sync`, `exdocs check`, gates

### Wave 9 (17 entries)

- [ ] T044 [US1] Transmission Lines (10): one document per entry
- [ ] T045 [US1] Subcircuits (2) and 2-D Scope (5, demonstrations: explain the idea of the picture and say so): one document per entry
- [ ] T046 [US1] Wave 9 checkpoint: `exdocs sync`, `exdocs check`, gates

### Wave 10 (51 entries)

- [ ] T047 [US1] Misc Devices (51): one document per entry
- [ ] T048 [US1] Wave 10 checkpoint: all 21 categories `documentado`; `exdocs check` has no errors; `docs/exemplos/README.md` links 366 distinct documents under 367 entries; gates

## Phase 4: User Story 2 — find the documents from the README (P2)

**Goal**: the README links the index; from there any document is one more click.

**Independent Test**: from `README.md` reach the document of any example in at most two clicks.

- [ ] T049 [US2] Write a failing test in `tests/example_docs.rs` (`the_readme_links_the_index`): `README.md` contains a link to `docs/exemplos/README.md`; and (`every_document_is_one_click_from_the_index`): every document that exists is linked from the index
- [ ] T050 [US2] Add the row `[docs/exemplos/](docs/exemplos/README.md) | Documentação científica de cada circuito de exemplo (teoria, equações, resultados esperados)` to the documentation table of `README.md` (Portuguese), and one paragraph about the examples' documents to `docs/guia-do-usuario.md` (in the section about the examples menu)
- [ ] T051 [US2] Write a test (`the_index_follows_the_order_and_the_categories_of_the_menu`) that compares the category headings and entries of the index with `parse_setup_list` of the real `assets/setuplist.txt`
- [ ] T052 [US2] Check on the rendered GitHub preview (or a Markdown preview with math and Mermaid) the index and five documents from different waves: links work, nothing shows raw (record in `specs/004-circuit-example-docs/render-check.md` what was and was not seen)

## Phase 5: User Story 3 — trust the numbers (P3)

**Goal**: what a document says agrees with the simulator, and a missing or stale document is caught.

**Independent Test**: change an expected value or an example's component value: `exdocs check` fails.

- [ ] T053 [US3] Write tests in `tests/example_docs.rs` over a temporary copy of the repository's documents (`std::env::temp_dir`): changing one `esperado` makes `check_all` fail with the numbers; changing a component value of a copied example makes the components region stale; deleting a document of a `documentado` category is reported; adding an entry to a copied `setuplist.txt` without a document makes its category `pendente` (and an orphan is reported when a document has no entry)
- [ ] T054 [US3] Audit a sample of at least 20 documents spread over all categories: run each example in the simulator (`cargo run --release --features gui`) and compare by eye what the document says with what the window shows; record the 20 and any difference in `specs/004-circuit-example-docs/audit.md`; fix every difference found
- [ ] T055 [US3] Mutation check: temporarily make `run_check` always succeed, and `components_region` return `None`, in turn; the tests of T008/T053 must fail; restore both
- [ ] T056 [US3] Run the twelve families builds again with all documents present (`cargo test --no-default-features --features "<all but one>" --no-fail-fast`): all green (examples that do not load in a build are skipped, not failed)

## Phase 6: Polish & cross-cutting

- [ ] T057 [P] Document the tool: a section "Documentação dos exemplos" in `docs/arquitetura.md` (the folder, the generated parts, `exdocs`, the checks), and the English guide for authors in `docs/adding-a-component.md` if it lists the checks to run
- [ ] T058 [P] Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and the same with `--features gui`; fix warnings
- [ ] T059 Run `cargo test --release` (all green) and `cargo test --test example_docs` in debug
- [ ] T060 Run `quickstart.md` end to end and record what was and was not done in `specs/004-circuit-example-docs/quickstart-run.md`

## Dependencies & Execution Order

- Phase 1 → Phase 2 (T004–T008 tests first and seen failing, then T009–T015) → T016–T018.
- US1 needs the whole of Phase 2. The category tasks of a wave can be done in any order inside it; a wave checkpoint closes the wave; waves are independent of each other (but the index and the checks are shared, so do them in order to keep the repository green).
- US2 needs the index (T011, T017); T050 can be done as soon as T017 is done, and T052 needs documents.
- US3 needs documents (at least wave 1 for T053, all waves for T054/T056).
- Polish after all stories.

## Parallel opportunities

- T002 and T003 (different files); T004–T008 (different modules); T010–T013 (different modules, after their tests).
- The category tasks inside a wave write different files and can be split among authors, but only one `exdocs sync` at a time (it rewrites the index).
- T057 and T058.

## Implementation strategy

1. **MVP**: Phases 1–2, then wave 1 (T019–T022): a working tool, an index with three categories documented, 36 documents checked by the simulator. Stop and review the writing style and the checks before the other nine waves.
2. Then waves 2–10, each ending green; US2 (README) can be done right after wave 1, so the documents are reachable early.
3. US3's audit and mutation checks at the end, and polish.
