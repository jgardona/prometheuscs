# Research: parameter and symbol documentation

## R1. Where do the symbols section and the glossary live?

**Decision**: section `## Símbolos e parâmetros` after `## O circuito` in each document; one
glossary file `docs/glossario-simbolos.md`.
**Rationale**: the circuit prose and the component table (in "O circuito") are where readers first
meet field names; the section must precede "Teoria" where the equations use the symbols. The
glossary is outside `docs/exemplos/` so no existing rule (orphan document, PDF folder scan) needs
an exception.
**Alternatives**: glossary inside `docs/exemplos/` (needs exceptions in `exdocs` and
`build_book.py`); symbols section at the end (readers meet the symbols before it).

## R2. How is "every symbol is defined" made checkable?

**Decision**: a conservative extractor reads the `$…$`/`$$…$$` spans of a document, drops unit
text (`\text{…}`/`\mathrm{…}` after a number or `\ `), numbers, functions (`\sin`, `\ln`,
`\frac`, …) and structure macros, and yields *symbol tokens*: a letter or Greek macro with its
subscript/superscript (`V_t`, `I_D`, `V_{GS}`, `\omega_0`). Every token must appear in the first
column of the document's symbols table (normalised: spaces, `\text`/`\mathrm` wrappers and braces
removed). A fixed ignore list (`e`, `\pi`, `j`, `\mathrm{d}`, `\partial`, `t` as the independent
variable only when written alone) covers constants and operators. The extractor is first run over
the whole corpus in report-only mode to measure noise (about 670 distinct raw tokens before
normalisation); rules that produce false alarms are tuned there, not in the documents.
**Rationale**: 365 documents cannot be verified by eye; a mechanical check is what keeps SC-001
true after later edits.
**Alternatives**: manual review only (decays); a full LaTeX parser (disproportionate).

## R3. Glossary shape

**Decision**: a Markdown file with one table per area (voltages and currents, passive components,
sources and waveforms, diodes, bipolar transistors, field-effect transistors, op-amps and
amplifiers, filters, logic and digital, timers/oscillators, power converters, transmission lines,
control and misc), plus a final area "Campos dos componentes" for the component-table field
names. Columns for symbols: `Símbolo | Grafias | Nome | Significado | Unidade | Exemplo`; for
fields: `Campo | Componentes | Unidade | Efeito | Exemplo`. A symbol with two meanings gets two rows
distinguished by the context in the name (`V_t` threshold vs `V_T` thermal voltage).
**Rationale**: tables render on GitHub and are already converted by the PDF builder; one row per
meaning satisfies FR-003/FR-004 and is checkable by splitting on `|`.
**Alternatives**: definition lists (not supported by the PDF converter); one file per area
(rejected in clarification).

## R4. Documents define symbols themselves (clarification Q2)

**Decision**: the document's table has name, meaning and unit in full (not "see glossary"); the
glossary is the reference that the rules compare against: for each symbol of a table, name and unit
must equal the glossary row's (FR-007).
**Rationale**: the document stays readable alone and in the PDF; equality is cheap to check.

## R5. Component-table field names (FR-006, clarification Q3)

**Decision**: tables untouched; the checker collects the field names from the generated component
regions (`Field = value` pairs, about 190 distinct) and requires one glossary entry each, with
unit and effect.
**Rationale**: the tables are generated from the example files and verified; changing them would
fight the generator.

## R6. Wave progress (FR-011)

**Decision**: `exdocs sync` adds to the index, per category, `símbolos: documentado` or
`símbolos: pendente (X de Y)`, X being the documents that already have the section. The section
is required by the checker only for documents that have it (then it must be well formed and
complete); a final constant, switched on in the last wave, requires it of all documents (SC-004).
**Rationale**: keeps every intermediate commit green while making progress visible and the end
state enforced.
**Alternatives**: a list of enhanced categories (a second source of truth that can drift).

## R7. PDF

**Decision**: the new section is plain Markdown (heading, table, math), already supported by
`build_book.py`. Add: the glossary converted as an appendix after the last chapter
(`\appendix\chapter{Glossário de símbolos e parâmetros}`), and in `ligacao` map links to
`glossario-simbolos.md` (from a document: `../glossario-simbolos.md`) to the appendix label with a
page reference, as is done for example-to-example links. Regenerate at the end of each wave with
`python3 docs/livro/build_book.py`; the build must report zero warnings (FR-013). A quick partial
build (`--only`) is used while writing.
**Rationale**: reuses the existing pipeline; no new tool.
**Risk**: the glossary tables are wide; the builder's table routine already computes visible
widths (`largura_visivel`), verified in render-check (quickstart).

## R8. Wave order (clarification Q4)

**Decision**: MOSFETs, Transistors, Diodes, Op-Amps first (where $V_t$, $I_D$, $V_{GS}$ live; the
corpus has 42 documents using those symbols), then the remaining categories in menu order. The
glossary grows with each wave; a wave is accepted when `exdocs check` is green and the PDF
rebuilt.
