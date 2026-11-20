# Implementation Plan: Explain the parameters and symbols used in the example documents

**Branch**: `feature/enhance-docs` | **Date**: 2026-11-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/008-example-parameter-glossary/spec.md`

## Summary

Every one of the 365 example documents under `docs/exemplos/` gets a new section **"Símbolos e
parâmetros"** (a table: symbol, name, meaning, unit) right after "O circuito", and one shared
glossary, `docs/glossario-simbolos.md`, defines every symbol (with its plain-text spellings such
as `Vgs`) and every component-table field name (unit and effect). The existing `exdocs` tool
(spec 004) is extended so the documentation keeps itself honest: it checks that the section exists
and is well formed, that every symbol used in the document's math appears in its table, that every
symbol of a table has one glossary entry, that every component-table field has a glossary entry,
and that the index reports how many documents of each category already have the section (the wave
progress, FR-011). The PDF builder `docs/livro/build_book.py` is extended to render the new
section (it is ordinary Markdown) and to append the glossary as one appendix; the PDF is
regenerated at the end of every wave (FR-012). No generated region and no check line of an
existing document changes (FR-009). Waves go worst-first: MOSFETs, Transistors, Diodes, Op-Amps,
then the other categories.

## Technical Context

**Language/Version**: Rust, edition 2024 (the `exdocs` module and its tests); Python 3 (the PDF
builder, already in the repository); Markdown with LaTeX for the documents

**Primary Dependencies**: none new; reuses `src/exdocs/` (front matter, regions, lint, index) and
the existing external tools of the PDF build (`xelatex`, `rsvg-convert`)

**Storage**: files: `docs/exemplos/*.md`, `docs/glossario-simbolos.md`,
`docs/livro-circuitos-de-exemplo.pdf`; no database

**Testing**: `cargo test`: unit tests beside the new code in `src/exdocs/`; the integration test
`tests/example_docs.rs` runs the new checks over the real documents; the PDF build is validated by
running it (zero warnings, see quickstart)

**Target Platform**: documents read on GitHub and as PDF; tools run on the developer's machine

**Project Type**: documentation plus a developer tool inside the existing crate

**Performance Goals**: the new checks are text-only; `exdocs check` must not become noticeably
slower (they add under 1 second to the run over 365 documents, excluding the simulations)

**Constraints**: plain LaTeX that both GitHub and the PDF builder already handle; no change to the
simulator, the example files, the generated regions, the check lines or the file format;
documents stay in Portuguese

**Scale/Scope**: 365 documents, about 190 distinct component-table field names and a few hundred
distinct math symbols (to be measured, see research R2); 4 priority waves plus the remaining
categories

## Constitution Check

*GATE: passed before research and again after design.*

| Principle | Result |
|-----------|--------|
| I Code quality | new Rust code passes `fmt` and `clippy -D warnings`, with and without `gui` |
| II Test-first | each new rule of the checker gets its failing test first (missing section, empty table, symbol used but undefined, symbol missing in the glossary, duplicate glossary entry, field without entry, orphan entry); the documents are then written until the checker accepts them |
| III Layered testing | unit tests for the extractor and the rules; the integration test over the real documents; each failure mode has a test |
| IV Maintainable design | one new small module `src/exdocs/symbols.rs` (extraction, glossary parsing, rules); the existing modules only gain a call; public items documented; typed `Result`s |
| V Dependencies and docs | no crate added; README links the glossary in the same change; PDF regenerated |

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/008-example-parameter-glossary/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── document-format-delta.md   # the new section of an example document
│   ├── glossary-format.md         # docs/glossario-simbolos.md
│   └── exdocs-cli-delta.md        # new checks and index columns
└── tasks.md                       # /speckit-tasks, not created here
```

### Source Code (repository root)

```text
src/exdocs/
├── symbols.rs     # NEW: symbol extraction from math, symbols-table parsing, glossary parsing and rules
├── lint.rs        # + the section "Símbolos e parâmetros" in the order of sections; calls symbols rules
├── index.rs       # + per-category "símbolos: X de Y" progress
└── mod.rs         # + loads the glossary once; passes it to the checks

docs/
├── glossario-simbolos.md          # NEW: the glossary (one file, grouped by area)
├── exemplos/*.md                  # + section "Símbolos e parâmetros" (365 documents, in waves)
├── exemplos/README.md             # index (generated): + progress, link to the glossary
├── livro/build_book.py            # + glossary appendix, link mapping to it
└── livro-circuitos-de-exemplo.pdf # regenerated at the end of each wave

tests/example_docs.rs              # + glossary checks over the real tree
README.md                          # + link to the glossary
```

**Structure Decision**: the glossary lives in `docs/`, not in `docs/exemplos/`, because every
`.md` file of `docs/exemplos/` is by rule an example document (an orphan is an error) and the PDF
builder reads the same folder; keeping it outside avoids special cases in both.

## Complexity Tracking

*Empty: no constitution violation.*
