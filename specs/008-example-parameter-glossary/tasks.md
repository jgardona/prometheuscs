# Tasks: Explain the parameters and symbols used in the example documents

**Input**: Design documents from `/specs/008-example-parameter-glossary/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: the constitution requires test-first (Principle II), so the checker tasks write the failing test first. The documents are "tested" by `exdocs check` and `cargo test`.

**Organization**: by user story. Waves (worst-first, FR-011): W1 MOSFETs (21 docs), W2 Transistors (28), W3 Diodes (20), W4 Op-Amps (39), W5 all remaining categories in menu order.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 documents' symbols sections, US2 shared glossary, US3 component fields, US4 PDF

## Writing conventions (apply to every documentation task)

- Portuguese, same style and LaTeX subset as the existing documents (FR-010).
- Never touch front matter, generated regions (navigation, schematic, components) or numbers in "Resultados esperados" (FR-009); `git diff` of a wave must show only the new section and prose that defines symbols.
- Symbols table columns exactly: `| Símbolo | Nome | Significado | Unidade |`, with the first line "Os símbolos usados neste documento (definições completas no [glossário](../glossario-simbolos.md)):" (see contracts/document-format-delta.md).
- Name and unit of a symbol MUST be identical in every document and in the glossary (FR-007). Use `exdocs symbols <exemplo>` to list gaps.

---

## Phase 1: Setup

- [X] T001 Create the empty glossary `docs/glossario-simbolos.md` with title `# Glossário de símbolos e parâmetros`, an introduction in Portuguese (how to read it, link to `exemplos/README.md`) and the area headings of contracts/glossary-format.md, each with its table header row and no rows
- [X] T002 [P] Run `cargo test` and `cargo run --release --bin exdocs -- check` on the untouched tree and note that both are green (baseline for FR-009/SC-005) in `specs/008-example-parameter-glossary/baseline.md`

---

## Phase 2: Foundational (blocks all stories)

**Purpose**: the checker that keeps the documents honest. Test first: each test fails before its code exists.

- [X] T003 Write failing unit tests in `src/exdocs/symbols.rs` for the extractor: `$V_{GS}$` → `V_{GS}`; `$5\ \text{V}$` → no token (unit); `$\sin\omega t$` → `\omega`, `t`; ignore list (`e`, `\pi`, `j`, `\mathrm{d}`); `\text{in}` inside a subscript kept (`V_\text{in}`); normalisation (spaces, `\text`/`\mathrm` wrappers, braces) makes `V_{in}` equal `V_\text{in}`
- [X] T004 Implement the extractor and normaliser in `src/exdocs/symbols.rs` (register `mod symbols;` in `src/exdocs/mod.rs`); public items with `///` docs, errors as typed `Result`
- [X] T005 Run the extractor in report-only mode over all of `docs/exemplos/*.md` (a temporary test or the `symbols` subcommand of T012) and record in `specs/008-example-parameter-glossary/research.md` (section R2 addendum) the count of distinct normalised symbols and the false-alarm cases; tune the ignore list and rules in `src/exdocs/symbols.rs` until the remaining tokens are real symbols
- [X] T006 Write failing unit tests in `src/exdocs/symbols.rs` for the symbols-table parser (four columns, rows, empty cell → error, no rows → error, missing link to `glossario-simbolos.md` → error) and for the glossary parser (six-column symbol tables, five-column field table, empty cell error, duplicate key = normalised notation + name error, a spelling in two entries error)
- [X] T007 Implement the symbols-table and glossary parsers in `src/exdocs/symbols.rs` so T006 passes
- [X] T008 Write failing tests in `src/exdocs/symbols.rs` for rules 9–13 of contracts/document-format-delta.md: section missing when required; symbol used in math but absent from the table; dead row (not used in the document); row without glossary entry; row whose name or unit differs from the glossary; field name of the component region without glossary field entry
- [X] T009 Implement rules 9–13 in `src/exdocs/symbols.rs` and call them from `src/exdocs/lint.rs`; add `Símbolos e parâmetros` to the ordered `SECTIONS` list in `src/exdocs/lint.rs` between `O circuito` and `Teoria`, but accept its absence unless the final switch is on (`const REQUIRE_SYMBOLS_SECTION: bool = false;` documented, research R6); the section order and "not empty" checks apply whenever the section is present
- [X] T010 Load `docs/glossario-simbolos.md` once in `src/exdocs/mod.rs` (`check_all`) and pass it to the checks; report glossary problems as `glossario-simbolos.md: <message>`; add glossary rules 4–5 of contracts/glossary-format.md (example exists and uses the symbol/field; every entry used by at least one document; every used symbol/field has an entry) with failing tests first
- [X] T011 Write the failing test, then extend `src/exdocs/index.rs` so `sync` writes per category `símbolos: documentado` or `símbolos: pendente (X de Y)` (X = documents with the section) and a link to `../glossario-simbolos.md` in the index header; update the expectations of the existing index tests in `src/exdocs/index.rs`
- [X] T012 Add the read-only subcommand `exdocs symbols <exemplo>` in `src/bin/exdocs.rs` and `src/exdocs/mod.rs` (prints extracted tokens, component field names, and what is missing from the table and from the glossary) and its line in `USAGE`; document it in `specs/008-example-parameter-glossary/contracts/exdocs-cli-delta.md` if the output format differs
- [X] T013 Add to `tests/example_docs.rs` a test that runs the glossary and symbols checks over the real tree (it already runs `check_all`; add one test for `exdocs symbols` output on a known document) and verify `tests/example_docs.rs::the_index_is_the_one_sync_would_write` still passes with the new index text
- [X] T014 Run `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo clippy --all-targets --features gui -- -D warnings && cargo test` and fix until green; run `exdocs sync` and commit the regenerated `docs/exemplos/README.md`

**Checkpoint**: the checker exists, rules are tested, nothing in the documents has changed yet.

---

## Phase 3: User Story 1 - Understand every symbol while reading an example (Priority: P1) 🎯 MVP

**Goal**: every example document defines each symbol it uses, with name, meaning and unit.

**Independent Test**: for any document of a finished wave, list its symbols; each is in its "Símbolos e parâmetros" table, and `exdocs check` is green.

Each wave task does, for each document of the category: (a) add the section after "O circuito" with every symbol used in the document (basic ones such as V, I, R on one line each); (b) add or reuse the glossary rows for its symbols in the right area of `docs/glossario-simbolos.md`, and the field rows of its component table (see US3 for the format), so rules 11–12 pass; (c) when the prose uses a plain spelling (`Vgs`, `Id`), write it with the formal symbol on first use and list the spelling in the glossary column `Grafias`; (d) run `exdocs symbols <stem>` until no gaps.

- [X] T015 [US1] Wave W1: MOSFETs, all documents listed under `## MOSFETs` in `docs/exemplos/README.md`, in `docs/exemplos/*.md` (start with `3-cgor`, `3-f221` which use $V_t$); glossary areas "Transistores de efeito de campo" and "Tensões e correntes" in `docs/glossario-simbolos.md`
- [X] T016 [US1] Close wave W1: `exdocs sync`, `exdocs check`, `cargo test`, `git diff --stat` shows no change outside the new sections and glossary (FR-009); then run the PDF regeneration T031 and commit
- [X] T017 [US1] Wave W2: Transistors, all documents under `## Transistors` in `docs/exemplos/*.md`; glossary area "Transistores bipolares" (e.g. $\beta$, $I_C$, $I_B$, $V_{BE}$, $V_{CE}$) in `docs/glossario-simbolos.md`
- [X] T018 [US1] Close wave W2 as in T016 (T031 for the PDF) and commit
- [X] T019 [US1] Wave W3: Diodes, all documents under `## Diodes` in `docs/exemplos/*.md`; glossary area "Diodos" (e.g. $I_S$, $V_T$ thermal voltage vs $V_t$ threshold: two entries, spec edge case) in `docs/glossario-simbolos.md`
- [X] T020 [US1] Close wave W3 as in T016 and commit
- [X] T021 [US1] Wave W4: Op-Amps, all documents under `## Op-Amps` in `docs/exemplos/*.md`; glossary area "Amplificadores operacionais" (gain $A$, $V_+$, $V_-$, output limits, slew rate) in `docs/glossario-simbolos.md`
- [X] T022 [US1] Close wave W4 as in T016 and commit
- [X] T023 [US1] Wave W5a: categories Basics, A/C Circuits, Passive Filters, Other Passive Circuits, Active Filters, 555 Timer Chip in `docs/exemplos/*.md`; glossary areas "Componentes passivos", "Fontes e formas de onda", "Filtros", "Temporizadores e osciladores"
- [X] T024 [US1] Wave W5b: Logic Families, Combinational Logic, Sequential Logic, Analog/Digital in `docs/exemplos/*.md`; glossary area "Lógica e digital"
- [X] T025 [US1] Wave W5c: Power Converters, Phase-Locked Loops, Transmission Lines, Controlled Sources, Subcircuits, Misc Devices, 2-D Scope in `docs/exemplos/*.md`; glossary areas "Conversores de potência", "Linhas de transmissão", "Fontes controladas e outros"
- [X] T026 [US1] Close waves W5a–W5c as in T016 (one commit per sub-wave); then `exdocs sync` must show `símbolos: documentado` for all 21 categories
- [X] T027 [US1] Flip the final switch: set `REQUIRE_SYMBOLS_SECTION` to `true` in `src/exdocs/lint.rs` (the test written in T008 for "section missing" now applies to every document), update tests, run the full gate of T014 (SC-001, SC-004)

**Checkpoint**: US1 complete; every document defines its symbols.

---

## Phase 4: User Story 4 - The PDF reflects the enhanced documentation (Priority: P2)

**Goal**: the PDF book contains the symbols sections and the glossary appendix, with zero conversion warnings. (Placed here because the wave closings above depend on it; T031 is run at the end of each wave from T016 on.)

**Independent Test**: run `python3 docs/livro/build_book.py`; no `aviso:` line; the PDF shows a MOSFET document's symbols table and a glossary appendix.

- [X] T028 [US4] In `docs/livro/build_book.py`: read `docs/glossario-simbolos.md`, convert it with the existing Markdown conversion (`converter`-style routine, tables via `tabela`) into `glossario.tex`, and append it after the last chapter as `\appendix\chapter{Glossário de símbolos e parâmetros}` with label `glossario`; keep `--only` working (glossary included only when `--only` is absent or `glossario` is listed)
- [X] T029 [US4] In `docs/livro/build_book.py` map links to `../glossario-simbolos.md` (function `ligacao`) to `\ref{glossario}` with `\pageref{glossario}` so they stay readable on paper, and links from the glossary to `exemplos/<stem>.md` to the existing example labels; ensure no `aviso:` is printed for the new constructs (extend the converter instead of avoiding them, spec edge case)
- [X] T030 [US4] Build and inspect: `python3 docs/livro/build_book.py --only 3-cgor,3-f221` for a quick check, then the full build; open `docs/livro-circuitos-de-exemplo.pdf` and confirm the symbols table of a MOSFET example, the glossary appendix, wide tables not overflowing the page, math rendered, references resolved; record the result in `specs/008-example-parameter-glossary/render-check.md`
- [X] T031 [US4] Regeneration routine for every wave (referenced by the "close wave" tasks): `python3 docs/livro/build_book.py` → zero `aviso:` lines, replaces `docs/livro-circuitos-de-exemplo.pdf`, commit the PDF with the wave (FR-012, FR-013); add this step to `specs/008-example-parameter-glossary/quickstart.md` if it differs
- [X] T032 [US4] Final PDF: after T027, regenerate the PDF once more and confirm SC-006 (100% symbols sections, glossary present, zero warnings)

**Checkpoint**: PDF in sync with the sources.

---

## Phase 5: User Story 2 - Look up a symbol once in a shared glossary (Priority: P2)

**Goal**: one glossary with exactly one entry per distinct meaning, reachable in one click.

**Independent Test**: take any symbol of any document; searching the glossary finds exactly one entry with name, meaning, unit and an example link.

- [X] T033 [P] [US2] Review `docs/glossario-simbolos.md` for consistency across areas: merge duplicate meanings, split symbols with two meanings into two rows, check every `Exemplo` link, sort rows inside each area alphabetically by notation; `exdocs check` must stay green
- [X] T034 [P] [US2] Link the glossary from the project `README.md` (next to the existing link to `docs/exemplos/README.md`) and from `docs/guia-do-usuario.md` where the examples documentation is mentioned; extend `tests/example_docs.rs` with `the_readme_links_the_glossary` (fails first)
- [X] T035 [US2] Confirm `docs/exemplos/README.md` (generated) links the glossary near the top (T011), run `exdocs sync` and commit

---

## Phase 6: User Story 3 - Parameters of the simulator components are explained (Priority: P3)

**Goal**: every field name of every component table has a glossary entry with unit and effect. (Entries for fields of a wave's documents are written during that wave, because rule 12 requires them; this phase completes and normalises them.)

**Independent Test**: for each distinct field name in any component table, the glossary area "Campos dos componentes" has exactly one row.

- [X] T036 [US3] Write the failing test in `src/exdocs/symbols.rs` (or extend T010) that collects the distinct field names of all component regions of the real tree and asserts each has a glossary row, with a readable failure listing the missing names
- [X] T037 [US3] Complete `## Campos dos componentes` in `docs/glossario-simbolos.md` (about 190 distinct names): columns `Campo | Componentes | Unidade | Efeito | Exemplo`; group rows by component class; the unit of bare numbers (e.g. `Gain = 100k`, `Resistance (ohms) = 1k`) is stated in the `Unidade` column; the component tables stay untouched (FR-008)
- [X] T038 [US3] Run `exdocs check` and `cargo test`; fix glossary gaps until T036 is green

---

## Phase 7: Polish & Cross-Cutting

- [X] T039 [P] Update `specs/004-circuit-example-docs/contracts/document-format.md` to list the new section in the order of sections and rules 9–13 (reference to this feature's contract delta), so the base contract is not stale
- [X] T040 [P] Update `docs/adding-a-component.md` or the example-documentation how-to (where the process of writing a new example document is described) with the rule "define every symbol; add glossary entries" if such a how-to exists; otherwise note it in `docs/exemplos/README.md` generator text in `src/exdocs/index.rs`
- [ ] T041 Reader test (SC-003): open three random documents from different waves; for every symbol find name, meaning and unit within one minute from the document alone; record in `specs/008-example-parameter-glossary/render-check.md`
- [X] T042 FR-009 audit: `git diff master -- docs/exemplos` filtered to lines starting with `-` shows only prose that was rewritten to define symbols, never front matter, regions, check lines or numbers; `exdocs check` runs every `verificacoes` green
- [X] T043 Final gate: the commands of quickstart.md steps 1–8, `cargo fmt --check`, both clippy runs with `-D warnings`, `cargo test`, final PDF (T032)

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (blocks everything) → US1 waves in order W1..W5 (the glossary grows wave by wave; each wave also writes the field rows its documents need).
- T028–T030 (PDF tooling) can start right after Phase 2 and must be done before the first "close wave" (T016); T031 runs at the end of every wave; T032 after T027.
- US2 (Phase 5) T033/T034 can run in parallel with waves once W1 exists; T033 is best after W5.
- US3 (Phase 6) completes after W5; T036 may be written earlier but is only green at the end.
- Polish last.

## Parallel opportunities

- T002 with T001; T003 test-writing and T006 test-writing touch the same file, so do them sequentially.
- Documents inside a wave touch different files: a wave can be split across agents/people per sub-category, with one owner for `docs/glossario-simbolos.md` edits to avoid conflicts.
- T028–T030 (Python) in parallel with T015 (documents).
- T033, T034, T039, T040 in parallel.

## Implementation Strategy

- **MVP**: Phase 1, Phase 2, and wave W1 (T015–T016) with the PDF step: the MOSFET documents, where $V_t$, $I_D$, $V_{GS}$ were reported, are fully explained and the tooling is proven end to end.
- Then W2–W4 (worst-first), W5, the final switch (T027), and the polish. Every wave leaves the repository green and the PDF in sync.

## Implementation notes (2026-11-20)

- The waves were authored in one session with an authoring aid kept outside the repository (a
  dictionary of symbols and fields in Portuguese plus per-document hints); its output is the
  documents and the glossary, all checked by `exdocs`. The "close wave" tasks (T016, T018, T020,
  T022, T026) were done once for all waves, not per wave, and **no commits were made** (none was
  asked for); the PDF (T031/T032) was regenerated once, at the end.
- T041 (the human reader test, SC-003) is not done: it needs a person to judge the documents.
- Documents edited beyond the new section (typography only, FR-009 audited with `git diff`):
  `mux3state`, `555square`, `clockedsrff`, `td4`, `td4-add2` (pin and register names set in
  `\text{}`), `tllopass`, `tlmatch1` (`\text{ABCD}`), so that names are not read as products of
  symbols.
- Beyond the plan: the `exdocs` check accepts a section without a table for a document with no
  symbols (only `td4-add2`), and `REQUIRE_SECTION` is already `true`.
