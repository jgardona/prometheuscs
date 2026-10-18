# Feature Specification: Scientific documentation for every circuit example

**Feature Branch**: `004-circuit-example-docs`

**Created**: 2026-10-18

**Status**: Draft

**Input**: User description: "I need to implement scientific documentation for every circuit example in the application. We can use LaTeX, because Markdown supports it, and Mermaid for diagrams when needed. We can link these documents in README.md."

## Clarifications

### Session 2026-10-18

- Q: In which language should the example documents be written? → A: Portuguese, as the user guide and the README.
- Q: What is the rule when the menu lists the same file under two categories? → A: The rule is per distinct file (366 documents for 367 entries): one document, which both categories link to; its previous and next links follow its first category in the menu.
- Q: How should the documents be delivered? → A: In waves by category; each wave is complete for its categories, verified and linked, and the index shows which categories are documented and which are still pending.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Understand an example from its document (Priority: P1)

A student or engineer opens an example circuit in the simulator (for instance, an RC low-pass
filter) and wants to know what it demonstrates and what to expect. They open the document of that
example and find: what the circuit is for, the circuit described with its component values, the
theory behind it with the equations written out, the results the simulator should show, and what
to watch for when running it.

**Why this priority**: this is the purpose of the feature; without one good document per example
nothing else has value.

**Independent Test**: pick any example from the Examples menu, open its document, follow it in the
simulator, and check that every value and waveform it describes is what the simulator shows.

**Acceptance Scenarios**:

1. **Given** an example in the Examples menu, **When** the reader looks for its document, **Then**
   one document exists for it, named after the example.
2. **Given** a document, **When** it is read, **Then** it contains the purpose, the circuit
   description with values, the theory with equations, the expected results, and what to observe.
3. **Given** a document with equations, **When** it is shown in a Markdown viewer (such as the
   repository page on GitHub), **Then** the equations are rendered as mathematics, not as raw
   source text.
4. **Given** a circuit where a diagram helps (a block diagram, a state machine, a signal path),
   **When** its document is shown, **Then** the diagram is rendered; circuits that do not need one
   have none.

---

### User Story 2 - Find the documents from the README (Priority: P2)

A visitor to the project reads the README and wants to browse what the examples teach. The README
links to an index of the documents, grouped by the same categories as the Examples menu, and each
document links back to the index.

**Why this priority**: documents nobody can find do not help; but the documents themselves matter
more than the entry point.

**Independent Test**: from the README, reach the document of any example in at most two clicks.

**Acceptance Scenarios**:

1. **Given** the README, **When** the reader follows the link to the examples documentation,
   **Then** an index lists every example, grouped by menu category, each with a link.
2. **Given** a document, **When** the reader wants to go elsewhere, **Then** a link back to the index
   and to the neighbouring examples of the same category is present.

---

### User Story 3 - Trust the numbers (Priority: P3)

A reader compares what the document says with the simulator. Values, frequencies, time constants
and waveform descriptions in the document agree with the example file and with what the program
computes, so the documentation can be used for teaching.

**Why this priority**: scientific documentation with wrong numbers is worse than none, but a first
version without this check is still useful, so it comes after the documents exist.

**Independent Test**: for a sample of examples, compute each quoted figure from the example file and
by running it; they agree within the stated tolerance.

**Acceptance Scenarios**:

1. **Given** a document that quotes a component value, **When** compared with the example file,
   **Then** they are the same.
2. **Given** a document that quotes a result (a voltage, a frequency, a delay), **When** the example
   is run, **Then** the simulator gives that result within the tolerance the document states.
3. **Given** a new or changed example file, **When** the project is checked, **Then** a missing
   document, or a link to a document that does not exist, is reported.

---

### Edge Cases

- Examples that are demonstrations rather than textbook circuits (pictures drawn on an X/Y scope,
  logic puzzles, a CPU): the document explains the idea and the circuit as far as it applies, and
  says so, instead of forcing a derivation.
- Variants of the same circuit (several coupled-coil examples, several flip-flop styles): each has
  its own document that states what differs and links to the others.
- Chaotic or noisy circuits, whose output differs from run to run: the document describes the
  behaviour statistically or qualitatively and does not quote exact values.
- Examples using a simplified component model: the document states the simplification.
- Examples present in the files but not in the Examples menu: not covered unless they are added to
  the menu.
- A file listed under two categories (at this writing `cmosinverter.txt`, under *MOSFETs* and under
  *Logic Families > CMOS*): one document; its category is the first one; both categories link to it
  and count it as documented.
- Examples added or renamed later: the check reports a missing document or a broken link.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: There MUST be one document for each distinct example file in the Examples menu,
  identified by the file name. A file that the menu lists under two categories has one document,
  which both categories link to.
- **FR-002**: Each document MUST contain: a title; the menu category; the purpose (what the
  example demonstrates); a description of the circuit with its component values taken from the
  example; the theory, with the governing equations; the expected results (values, waveforms or
  behaviour); what to observe and what to try in the simulator; limits and model simplifications;
  and references.
- **FR-003**: Equations MUST be written in LaTeX so that they render as mathematics in a Markdown
  viewer.
- **FR-004**: Diagrams, where they help, MUST be written as Mermaid so that they render in a
  Markdown viewer; a document is not required to have one.
- **FR-004a**: Every document MUST show the schematic of its own circuit, drawn from the example
  file by the program's own SVG painter (`docs/exemplos/img/<example>.svg`, kept current by
  `exdocs sync`, checked by `exdocs check`); a hand-made Mermaid sketch does not replace it.
  Added 2026-10-18 after the Mermaid sketches proved too poor to show a circuit.
- **FR-005**: Every number a document states (component values, time constants, frequencies,
  gains, voltages) MUST agree with the example file, and every stated result MUST agree with what
  the simulator computes, within a tolerance the document states.
- **FR-006**: There MUST be an index of the documents grouped by the categories of the Examples
  menu, in the menu's order, with a link to each document.
- **FR-007**: The README MUST link to the index.
- **FR-008**: Each document MUST link back to the index and to the neighbouring examples (previous
  and next) of its category; for a file listed under two categories, the previous and next links
  follow its first category in the menu.
- **FR-009**: A check MUST report any menu example without a document in a category marked
  documented, any document without a menu example, and any link that does not resolve.
- **FR-010**: The documents MUST be written in Portuguese (the language of the user guide and the
  README); equations and diagram labels use the usual symbols and may keep standard English
  technical terms where Portuguese has no common term.
- **FR-011**: The documents MUST be delivered in waves by category of the Examples menu: a category
  is either fully documented (every example of it) or not started, never half done. The index MUST
  show, for each category, whether it is documented or pending, and MUST list only documents that
  exist.

### Key Entities

- **Example circuit**: one entry of the Examples menu: a title, a category, and a circuit file.
- **Circuit document**: the explanation of one example (sections of FR-002).
- **Index**: the page that lists all documents by category and is linked from the README.
- **Diagram**: an optional picture of the circuit's structure or behaviour inside a document.
- **Reference**: a book, paper or standard cited by a document.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: When all waves are delivered, 100% of the examples in the Examples menu have a
  document (the check of FR-009 reports none missing); at the end of each wave, 100% of the
  examples of the categories marked documented have one.
- **SC-002**: A reader reaches the document of any example from the README in at most 2 clicks.
- **SC-003**: In a rendered preview, 0 documents show raw LaTeX or raw Mermaid source.
- **SC-004**: In a sample of at least 20 documents chosen across all categories, every quoted
  component value matches the example file, and every quoted result matches the simulator within
  the tolerance stated in the document.
- **SC-005**: 0 broken links among the documents, the index and the README.
- **SC-006**: Every document has all the sections of FR-002 (a section that does not apply says
  why, instead of being left out).

## Assumptions

- "Every circuit example" means every distinct example file of the Examples menu: the menu has 367
  entries but 366 distinct files (one file is listed twice), so there are 366 documents. The
  menu comes from the original simulator's example list; the 7 circuit files that are not in the
  menu (`analogrecip`, `avr8js-analog`, `avr8js-logic`, `avr8js-strobe`, `jsinterface`,
  `motorprotect`, `relays`) are out of scope.
- The documents live in the repository as Markdown files next to the other documents, in a folder
  of their own, with the example's file name as the document name.
- LaTeX and Mermaid are used as the user asked; the viewer is the repository page on GitHub and
  common Markdown previewers.
- The examples were created by the original simulator's authors; the documents credit them and add
  the theory, they do not claim the circuits.
- References are textbooks and standard sources (for example, Sedra & Smith, Horowitz & Hill);
  none is quoted at length.
- The documentation is not shown inside the program in this feature (no in-app link); that can be
  a later feature.
- No change to the simulator, the example files or the file format.
