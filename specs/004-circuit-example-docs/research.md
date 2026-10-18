# Research: Scientific documentation for every circuit example

## R1. Where the documents live and how they are named

- **Decision**: `docs/exemplos/<file stem of the example>.md` (for `filt-lopass.txt`:
  `docs/exemplos/filt-lopass.md`), all in one folder, plus `docs/exemplos/README.md` as the index.
- **Rationale**: the mapping from the menu to the document is one rule that a program can check
  (FR-001, FR-009); the user guide already lives in `docs/`; the README links to a folder index
  (2 clicks, SC-002).
- **Alternatives**: one folder per category (moves a document when the menu changes; the file
  name alone no longer identifies it); the documents next to `assets/circuits` (mixes content
  with data that is imported from the original).

## R2. The index is generated, not written by hand

- **Decision**: `exdocs sync` writes `docs/exemplos/README.md` from `assets/setuplist.txt` (the
  menu tree, in the menu's order) and from the documents that exist. A category is
  "documentado" when every example under it has a document, otherwise "pendente (n de m)";
  entries without a document are listed as plain text. `exdocs check` fails when the committed
  index differs from what `sync` would write.
- **Rationale**: FR-006/FR-011 follow automatically; waves cannot leave a half-linked index; an
  example added to the menu appears as pending without anyone remembering to edit the index.
- **Alternatives**: a hand-written index (drifts); an index per category (more files to keep).

## R3. The component table is generated from the example file

- **Decision**: each document has a region between `<!-- componentes:inicio -->` and
  `<!-- componentes:fim -->` that `exdocs sync` fills with a table built by loading the example:
  the element number, its type and its editable values (name = value with units). `check` fails
  if the region differs from what `sync` produces.
- **Rationale**: FR-005 for component values becomes a mechanical guarantee, not a proofreading
  task, across 366 documents.
- **Alternatives**: copy values by hand (error-prone); quote them only in prose (cannot be
  checked).

## R4. Results are checked by the simulator

- **Decision**: the document's front matter lists `verificacoes`: each item gives a time, an
  element (by number and type), a quantity, an optional function over a window (`max`, `min`,
  `rms`, `media`, `freq`), the expected value and a tolerance. `tests/example_docs.rs` and
  `exdocs check` load the example, run the simulation and compare. A document with no result to
  check says why with `sem-verificacao: <motivo>`.
- **Rationale**: SC-004 for every document, not for a sample; the tolerance is stated where the
  claim is made (FR-005).
- **Alternatives**: compare against the original simulator's recordings (`tests/compat` already
  does that for the program, but a document's claim is about a formula, not a recording); a
  separate spreadsheet of checks (far from the text).

## R5. Mathematics and diagrams

- **Decision**: inline math `$…$`, display math `$$…$$` and ` ```mermaid ` blocks, as GitHub
  renders them. A light lint in `check`: `$$` delimiters balanced, `$` balanced per paragraph, no
  unsupported environments (`tabular`, `\usepackage`), a Mermaid block starts with a known
  diagram keyword (`flowchart`, `graph`, `sequenceDiagram`, `stateDiagram-v2`, `classDiagram`,
  `xychart-beta`, `timeline`) and is not empty.
- **Rationale**: SC-003: nothing shows raw. The lint catches the common mistakes; real rendering
  is checked by looking at the GitHub preview of a sample (quickstart).
- **Alternatives**: render in CI with a headless browser (heavy, flaky, new tooling).

## R6. Structure of a document

- **Decision**: Portuguese; fixed sections: `## Objetivo`, `## O circuito`, `## Teoria`,
  `## Resultados esperados`, `## O que observar`, `## Limites do modelo`, `## Referências`; a
  navigation line (index, previous and next of the category) after the title. `check` fails for a
  missing section; a section that does not apply says why.
- **Rationale**: FR-002 and SC-006 mechanically; readers know where to look.
- **Alternatives**: free form (no check possible).

## R7. Builds with a family left out (the CI `families` jobs)

- **Decision**: structural checks (existence, orphans, links, sections, math, index and component
  tables) do not depend on families when they only read text; the table of components needs the
  elements, so for an example that does not load without errors in the current build (a family
  is off) the generated table and the `verificacoes` are **skipped, not failed**. The default
  build checks everything.
- **Rationale**: the project promises that with any family left out the tests that are not about
  that family pass; skipping by "does it load" is the precise condition.
- **Alternatives**: `#[cfg(feature = …)]` per example (366 cases); failing the build (breaks
  SC-013 of the first feature).

## R8. Writing 366 documents: waves and sources

- **Decision**: ten waves (see the plan); each document follows the same template; the facts
  (values, topology) come from `exdocs facts <example>`; the theory is written from standard
  textbooks (Sedra & Smith, Horowitz & Hill, Boylestad, Millman & Halkias, Pozar for
  transmission lines) and cited in `## Referências`; the examples are credited to the original
  authors (Paul Falstad and Iain Sharp).
- **Rationale**: a template plus mechanical checks keeps the quality uniform; waves keep each
  delivery reviewable.
- **Alternatives**: documents generated entirely by a tool (would not be scientific text).

## R9. Count of examples

- **Decision**: "every example" is every distinct file of the menu: 367 entries (the 21 top-level
  categories total 367) but 366 distinct files, because `cmosinverter.txt` is listed twice (under
  *MOSFETs* and under *Logic Families > CMOS*). 366 documents. 373 files exist; the 7 that are not
  in the menu (`analogrecip`, `avr8js-analog`, `avr8js-logic`, `avr8js-strobe`, `jsinterface`,
  `motorprotect`, `relays`) are out of scope.
- **Duplicated entry**: one document, named after the file; its `categoria` and its previous and
  next links are those of the **first** occurrence in the menu (here *MOSFETs*); the second
  category links to the same document and counts it as documented. The document is written in the
  wave of its first category (wave 4); wave 7 reuses it.
- **Rationale**: the document is about the circuit, not about the menu; two documents with one
  file name would clash, and two copies would drift.
- **Rationale**: spec assumption; the checker derives the number from `setuplist.txt`.
