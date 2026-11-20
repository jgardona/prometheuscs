# Feature Specification: Explain the parameters and symbols used in the example documents

**Feature Branch**: `feature/enhance-docs`

**Created**: 2026-11-20

**Status**: Draft

**Input**: User description: "Enhance documentation. Our documentation has an issue, that it doesn't explain the parameters of the examples. We need to enhance this. For example, I don't know what is Vt, Id, Vgs, etc. We need to explain what each parameter is."

## Clarifications

### Session 2026-11-20

- Q: How deep is the per-document symbols section? → A: It lists every symbol used, with basic ones (V, I, R) on one line each.
- Q: Are definitions repeated in each document or only linked? → A: Defined in each document, plus a link to the glossary.
- Q: Are units added to the component tables? → A: No; field names and units are explained in the glossary only, and the tables stay untouched.
- Q: In what order are the waves delivered? → A: Worst-first: MOSFETs, transistors, diodes, op-amps, then the remaining categories.
- Q: What is the glossary format and its place in the PDF? → A: One glossary file, placed as a single appendix in the PDF.
- Q: When is the PDF regenerated? → A: At the end of every wave, replacing the committed PDF.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Understand every symbol while reading an example (Priority: P1)

A reader opens the document of a circuit example (for instance a MOSFET inverter) and meets
symbols such as $V_t$, $I_D$ or $V_{GS}$ in the theory, the component table or the expected
results. For each symbol the reader must be able to learn, without leaving the document or
guessing, what it is: its name, what it represents physically, its unit, and (when it is a
component value) how it relates to the field shown in the simulator.

**Why this priority**: this is the reported problem. A document whose symbols are undefined cannot
be followed, even if its equations are correct.

**Independent Test**: pick any example document, list every symbol and abbreviation that appears
in it, and check that each one is defined in that document (or in a glossary the document links to
at the point of first use).

**Acceptance Scenarios**:

1. **Given** an example document that uses $V_t$, $I_D$ and $V_{GS}$, **When** the reader looks
   for their meaning, **Then** each one is defined with name, meaning and unit.
2. **Given** a symbol used in the equations, **When** it also appears in the results table or in
   the component table, **Then** the same symbol, name and unit are used everywhere in the document.
3. **Given** a value in the component table (for example "Threshold Voltage = 3.25"), **When** the
   reader reads it, **Then** the document explains what that parameter means and in what unit it is
   expressed.

---

### User Story 2 - Look up a symbol once in a shared glossary (Priority: P2)

A reader who has seen a symbol in one example wants a single place to look up any symbol or
parameter used across all examples (voltages, currents, device parameters, op-amp parameters,
logic levels, and so on), organised so it can be searched and scanned quickly.

**Why this priority**: it avoids repeating long definitions in 366 documents and keeps meanings
consistent, but each document is still understandable on its own thanks to User Story 1.

**Independent Test**: open the glossary, search for any symbol that appears in any example
document, and find exactly one entry for it.

**Acceptance Scenarios**:

1. **Given** the examples index, **When** the reader looks for help with symbols, **Then** a link
   to the glossary is visible there and in the project README.
2. **Given** any symbol used in any example document, **When** the reader searches the glossary,
   **Then** exactly one entry exists with name, meaning, unit and a typical example of use.
3. **Given** a glossary entry, **When** the reader reads it, **Then** it points to at least one
   example document that uses the symbol.

---

### User Story 3 - Parameters of the simulator components are explained (Priority: P3)

A reader looking at the component table of an example sees field names such as "Max Voltage",
"Gain", "Beta" or "Threshold Voltage". They want to know what each field does to the circuit
behaviour.

**Why this priority**: it completes the picture for readers who want to modify the example, but
the symbol definitions of User Stories 1 and 2 already unblock reading.

**Independent Test**: for each distinct field name that appears in any component table, find its
explanation in the glossary.

**Acceptance Scenarios**:

1. **Given** a field name in a component table, **When** the reader consults the glossary,
   **Then** the field is explained with its unit and its effect on the circuit.

---

---

### User Story 4 - The PDF book reflects the enhanced documentation (Priority: P2)

A reader who prefers the PDF book of the example circuits (`docs/livro-circuitos-de-exemplo.pdf`)
gets the same symbol definitions, per-document symbols sections and glossary as the Markdown
readers. The PDF is part of the documentation and must never lag behind the Markdown sources.

**Why this priority**: the PDF is a published deliverable of the documentation; leaving it stale
would reintroduce the reported problem for its readers.

**Independent Test**: regenerate the PDF, open it, and check that a chosen example shows its
symbols section and that the glossary is present and readable, with no rendering warnings.

**Acceptance Scenarios**:

1. **Given** the enhanced documents, **When** the PDF book is regenerated, **Then** it contains
   every document's symbols section and the glossary.
2. **Given** the regenerated PDF, **When** it is built, **Then** it completes without conversion
   warnings or unrendered math, and glossary links/references work or are replaced by readable
   page/section references.
3. **Given** any delivery wave, **When** it is finished, **Then** the PDF committed in the
   repository was regenerated from the final sources of that wave.

### Edge Cases

- The glossary uses a Markdown construct the PDF converter does not support: the converter is
  extended or the construct is avoided, so nothing disappears silently from the PDF.

- A symbol has different meanings in different contexts (for example $V_t$ as a transistor
  threshold voltage versus $V_T$ as thermal voltage, or $t$ as time): each meaning has its own
  entry and each document uses the one that applies, written unambiguously.
- A symbol appears only in one example: it is still defined in that document and listed in the
  glossary.
- A document uses a plain-text abbreviation (for example "Id", "Vgs") instead of the formal
  notation: the glossary lists both spellings under the same entry.
- Documents already delivered and verified must keep passing their existing verification after the
  text is enhanced; no expected value or check may change.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every example document MUST define each symbol, abbreviation and parameter it uses,
  at or before its first use, with name, meaning and unit.
- **FR-002**: Each example document MUST include a short "symbols and parameters" section listing
  every symbol it uses (name, meaning, unit), basic ones such as V, I and R on one line each, so it
  can be understood without opening other files. Each section MUST link to the glossary.
- **FR-003**: The project MUST provide one glossary file of symbols and parameters covering every symbol
  used across all example documents, with exactly one entry per distinct meaning.
- **FR-004**: Each glossary entry MUST give the symbol (in every spelling used in the documents),
  its name, its meaning in plain language, its unit, and at least one example document that uses it.
- **FR-005**: The glossary MUST be grouped by area (for example voltages and currents, passive
  components, diodes, bipolar transistors, field-effect transistors, op-amps and logic) and be
  linked from the examples index and from the project README.
- **FR-006**: The glossary MUST explain every field name that appears in the component tables of
  the example documents, including its unit and its effect on the circuit.
- **FR-007**: The same symbol MUST have the same name, meaning and unit in every document and in the
  glossary.
- **FR-008**: The component tables MUST NOT be changed; the unit and meaning of each field MUST be
  given in the glossary (FR-006), so verified tables and their checks are untouched.
- **FR-009**: Enhancing the text MUST NOT change any expected value, tolerance, verification line
  or circuit description already validated, and the existing verification of the documents MUST
  still pass.
- **FR-010**: New and changed text MUST be written in Portuguese, in the same style as the
  existing documents, with equations in the same notation already used.
- **FR-012**: The PDF book MUST be regenerated so it includes the symbols sections of all
  documents and the glossary (as a single appendix), and MUST be kept in sync with the sources
  at the end of every delivery wave, replacing the committed PDF.
- **FR-013**: Regenerating the PDF MUST complete without conversion warnings, and the new content
  MUST render correctly (math, tables, cross-references).
- **FR-011**: A reader MUST be able to tell which documents have already been enhanced and which
  are still pending while the work is delivered in waves, as was done for the original documents.
  Waves are delivered worst-first: MOSFETs, transistors, diodes, op-amps, then the remaining
  categories.

### Key Entities *(include if feature involves data)*

- **Symbol**: a letter or abbreviation used in an equation or table (for example $V_t$). Has a
  notation, a name, a meaning, a unit, and possibly several spellings.
- **Parameter (component field)**: a configurable value of a simulator component as shown in the
  component table (for example "Gain"). Has a name, a unit, and an effect on the circuit.
- **Glossary entry**: the single definition of a symbol or parameter, with the example documents
  that use it.
- **Example document**: one of the 366 documents that describe a circuit example; contains its own
  symbols section and links to the glossary.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the symbols and abbreviations appearing in the example documents have a
  definition (name, meaning, unit) in the same document.
- **SC-002**: 100% of the symbols and component-table field names used across the documents have
  exactly one entry in the glossary.
- **SC-003**: A reader unfamiliar with electronics can name the meaning and unit of any symbol in a
  randomly chosen example within 1 minute, without consulting external sources.
- **SC-004**: 100% of the 366 documents link to the glossary, and the glossary is reachable in one
  click from the examples index and the README.
- **SC-006**: The PDF contains 100% of the symbols sections and the glossary, and its
  regeneration reports zero conversion warnings.
- **SC-005**: Zero changes to expected values or verification lines of the existing documents, and
  the full existing verification still passes after the enhancement.

## Assumptions

- The documents stay in Portuguese, the language already used for the user guide, README and
  examples.
- The existing structure of each document (objective, circuit, components, theory, expected
  results) is kept; a "symbols and parameters" section is added rather than restructuring.
- The glossary lives under the examples documentation folder and is linked from its index.
- Delivery follows the same wave-by-category approach as the original example documentation, with
  the index showing progress.
- Symbols are defined in terms a student with basic electronics knowledge understands; deep
  derivations stay in the theory sections already present.
- The existing PDF generation process is reused; the PDF is regenerated, not redesigned.
- The application's user interface and simulation behaviour are out of scope; only documentation
  changes.
