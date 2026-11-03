# Feature Specification: CircuitJS 5.x Feature Sync

**Feature Branch**: `update/circuitjs-features-to-prometheus`

**Created**: 2026-11-02

**Status**: Draft

**Input**: User description: "New features of circuitjs to prometheus. May you see what new features circuitjs got in recent updates looking at the repository? We need to list these features and bring them to prometheuscs." Follow-up: "We need to update the documentation also. I think some things you listed we already had?"

## Clarifications

### Session 2026-11-02

- Q: Some of the listed features already exist in prometheuscs. Is the list correct? → A: No. The first version compared the upstream TypeScript history with itself. Many TypeScript commits only re-implemented features the Java edition already had, and prometheuscs had already ported most of those. The inventory was rebuilt by comparing three sources: Java 4.1.5.1js (the port baseline), TypeScript 5.0.4 and the prometheuscs code. Items prometheuscs already has are now listed separately as "already present" and are out of scope.
- Q: Should the documentation be updated too? → A: Yes. The user guide, the README, the architecture document, the component guide and the affected example pages must describe every feature this spec delivers (User Story 7, FR-028 to FR-031, SC-008).
- Q: Should the plan build CircuitJS 5.0.4 just to record reference results? → A: No. The unchanged examples keep their existing recordings. The rewritten `cs-*` examples must match the recordings of their old versions, because upstream only redrew them with the new sources; any difference is explained in `known_differences.json`. New formulas are checked against exact hand calculations, and file compatibility against save-and-reload tests. Building 5.0.4 is a fallback only if a nonlinear case disagrees with expectations.
- Q: What should Find Component search? → A: Four groups, in this order: recent picks, component types to place, menu commands, and elements already in the circuit (the current prometheuscs list is kept).
- Q: Should the edit dialog open next to the element being edited, as upstream does? → A: No, it is deferred. GTK cannot position windows on Wayland, and the new side panel already keeps the properties beside the circuit. The dialog stays centered.
- Q: What happens when two labeled elements have the same reference name? → A: The first match in circuit order is used without a warning, exactly as upstream.
- Q: What happens to the "Mouse Wheel Sensitivity" option once one wheel notch means one value step? → A: It keeps controlling zoom speed only, and its text says so. It no longer affects value editing.

## Background: what changed upstream

prometheuscs was ported from the Java/GWT edition of CircuitJS1, release **4.1.5.1js** (commit `d6b80f46`, 2026-09-10). Upstream (`~/Workspace/circuitjs1`, `pfalstad/circuitjs1`) has since been rewritten in TypeScript and released as **5.0.0 to 5.0.4** (commit `8ac55f22`, 2026-10-28). Each feature below was checked in all three code bases. Planning must still confirm the rows marked "confirm".

### A. New in CircuitJS 5.x and missing in prometheuscs (in scope)

| # | Feature | Area | Status (SC-003) |
|---|---------|------|-----------------|
| N1 | Two-terminal **controlled voltage source** and **controlled current source** (diamond symbol) whose output is an expression | Elements | Delivered |
| N2 | Name references in expressions: `v(label)`, `i(label)`, `dvdt(label)`, `didt(label)`. Inside a subcircuit, names resolve within that subcircuit | Expressions | Delivered |
| N3 | Optional **labels on voltmeters and ammeters**, usable as reference names | Elements | Delivered |
| N4 | The controlled-source examples (`cs-*.txt`) rewritten upstream to use the two-terminal sources | Examples | Delivered |
| N5 | 5.x **subcircuit element** (same file tag and attributes as the old custom composite, which prometheuscs already reads; new are the `rn` reference attribute and `sublib` library files, see plan research R-5). The Subcircuit Manager gains previews, multi-select, local / global / persistent scope, and saving a library | Subcircuits | Delivered (manager: preview, multi-select, three scopes, Save Library, refusal to delete models in use) |
| N6 | **Property editor as a non-modal side panel** that follows the selection. New options **Show Properties on Create** and **Show Properties on Click**. Elements rarely edited on placement skip the editor. (Opening the dialog next to the element is **deferred**: not possible on Wayland, see Clarifications.) | Editing | Delivered; opening the dialog next to the element deferred (Wayland cannot place windows; guide section 16) |
| N7 | **Wire split when a moved element's terminal lands on a wire or lead** (upstream already split wires for newly placed elements; see C4) | Editing | Delivered |
| N8 | Mouse-wheel value popup: **one wheel notch = one value step**; the wheel also moves slider elements | Editing | Delivered (E12 for R/C/L, 1-2-5 otherwise, slider elements, value popup) |
| N9 | **Monostable invert-trigger option** | Elements | Delivered |
| N10 | **Custom text color** on text elements | Elements | Delivered (already present; covered by a test) |
| N11 | **Stop Trigger** does not fire as a side effect of loading a circuit or of Reset | Elements | Delivered |
| N12 | Scope **Export Image** dialog: PNG or SVG, width, height, **transparent background** | Scopes | Delivered (docked scopes) |
| N13 | Scope **AC coupling** refinements: fast start, filter reset, use in auto-scale mode | Scopes | Delivered as upstream has it; FR-016's "within one period" is not met (±2% from the 6th period; research.md) |
| N14 | Stacked-scope readability: label and legend shown when there is room, identical labels not repeated, combined-plot names in on-screen order, plot IDs when several plots are combined; "Show Scale" in auto-scale shows only the relevant quantity | Scopes | Delivered ("Show Scale" part already present) |
| N15 | Scope interaction: triggered sweep stays on screen until the next sweep is complete; dragging in a scope no longer drags the plotted element or selects a plot; X/Y role labels on probes when hovering an X/Y scope | Scopes | Delivered (sweep hold new; drag guards hold by construction; X/Y roles already present) |
| N16 | **Unsaved-changes prompt only when the circuit's content changed**. Speed, current-speed and brightness sliders do not count | Files | Delivered |
| N17 | Persistent on-canvas warning after a simulation error | Reliability | Delivered (message drawn on the canvas) |
| N18 | Labeled-node name shown when hovering a wire connected to it | Display | Delivered |
| N19 | Dialog polish: clicking a checkbox's label toggles it; model and Other Options dialogs are modal and titled | UI | Delivered (Reset and Run/Stop no longer take focus; the rest was already so) |
| N20 | File fixes: gyrator resistance and width saved; relay contacts load from XML; transformer saturation not saved as "undefined"; voltage-source phase reset on edit; time-delay relay works with several matrices (confirm which affect prometheuscs) | Compatibility | Delivered (fixes that applied, each with a regression test) |

### B. In CircuitJS since 4.1.5.1js but never ported to prometheuscs (in scope, port gaps)

These are not new upstream. The first port (spec 001) missed them.

| # | Feature | Area | Status (SC-003) |
|---|---------|------|-----------------|
| C1 | **Swap Terminals** command in the element menu | Editing | Delivered |
| C2 | **SRAM "Load Contents From File"** | Elements | Delivered |
| C3 | **Drag a component from the toolbar** onto the canvas | Editing | Delivered |
| C4 | **Wire split when a newly placed element's terminal lands on a wire** | Editing | Delivered |
| C5 | Mouse wheel over a transistor, MOSFET or JFET **flips its type** (NPN/PNP, N/P channel) | Editing | Delivered |
| C6 | Find Component as upstream has it: search **component types to place and menu commands** (prometheuscs's version only lists elements already in the circuit, which stays as an extra group), open with a **double Shift** press, list **recent choices** first | Navigation | Delivered |
| C7 | **Load Subcircuits Only** (load a library file without replacing the circuit) | Subcircuits | Delivered (File menu, and Open on a library file) |
| C8 | Scope image export (prometheuscs exports only the circuit image and scope CSV data) | Scopes | Delivered |

### C. Already present in prometheuscs (out of scope)

Rotate CW / CCW; Mirror X / Y; Duplicate; Select All; Shortcuts dialog; Subcircuit Manager (basic); Convert Wires to Routed Wires; P-channel JFET; negative current conveyor (CC2−); true wattmeter plus "Wattmeter (old)"; latching relay coils (Set / Reset Coil); analog multiplexer pull-down; DeMorgan gate shapes; ohm symbol on resistances; Stop Trigger count and duration; mouse-wheel value editing (with E12 stepping since 1.0.8); scope CSV export; circuit export to PNG / SVG; circuit/scope splitter; Show Scale; basic AC coupling; sparse solver; Finnish translation.

Minor 5.x rows still to confirm during planning (assumed present): JFET pin names on highlight, transistor arrow highlighting, demultiplexer invertible outputs, average-voltage mode on probes and test points.

### D. Out of scope

Web-, Electron- and touch-specific changes (service worker, Electron File > Open, iframe focus, GitHub Pages, single JS bundle, Dropbox import, touch gestures); the disabled developer "Create Test" menu; the experimental C++/Qt port (`cxx/`); fixes for bugs introduced by the TypeScript rewrite itself.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Open and run circuits made with CircuitJS 5.x (Priority: P1)

A user receives a circuit saved by the current online CircuitJS (5.x). It uses two-terminal controlled sources, labeled meters, `v(name)`-style expressions or the new subcircuit element. They open it in prometheuscs and it loads, draws and simulates as it does upstream.

**Why this priority**: File compatibility is the core promise of prometheuscs (spec 001). Every 5.x file that cannot be opened breaks it. Covers N1 to N5 (loading side), N20.

**Independent Test**: Load the upstream 5.0.4 `cs-*.txt` examples and compare them with the recordings of their old versions. Load a reference set of 5.x-format circuits, one per new element and expression form, and compare node voltages with exact hand calculations.

**Acceptance Scenarios**:

1. **Given** a 5.0.4 circuit with a two-terminal controlled voltage source whose expression is `2*v(in)`, **When** the user runs it, **Then** the output is twice the voltage at the node labeled `in`, within the spec-001 tolerance.
2. **Given** an ammeter labeled `ia` and a controlled current source with expression `didt(ia)`, **When** it runs, **Then** the source follows the rate of change of the ammeter current.
3. **Given** a subcircuit containing a label `x` and an expression `v(x)`, **When** two copies are placed, **Then** each copy uses its own `x`.
4. **Given** an expression naming a reference that does not exist, **When** the circuit loads, **Then** the user sees an error naming the missing reference and the rest of the circuit loads.
5. **Given** a circuit saved by prometheuscs with these elements, **When** it is opened in CircuitJS 5.0.4, **Then** it loads with identical elements and values.

---

### User Story 2 - Build circuits with the new elements and expressions (Priority: P1)

A user models a textbook dependent source: a diamond with `10*i(ib)`. They pick the controlled source from the Draw menu, label an ammeter, type the expression and run.

**Why this priority**: The biggest new modeling capability upstream, and the upstream examples now rely on it. Covers N1 to N4, N9 to N11, C2.

**Independent Test**: Build a current-controlled voltage source with a labeled ammeter from scratch and compare with a hand calculation; open each updated `cs-*` example.

**Acceptance Scenarios**:

1. **Given** the Draw menu, **When** the user opens the controlled-sources group, **Then** two-terminal controlled voltage and current sources are listed and drawn as diamonds.
2. **Given** a voltmeter or ammeter, **When** the user edits it, **Then** they can set an optional label that is saved and shown on the canvas.
3. **Given** an expression with a syntax error, **When** the user confirms, **Then** a parse-error message with the error position is shown and the previous value is kept.
4. **Given** a Stop Trigger whose condition is already true, **When** the circuit is loaded or reset, **Then** the simulation does not stop until the condition is met during the run.
5. **Given** a monostable with "invert trigger" on, **When** its trigger input falls, **Then** it fires.
6. **Given** a text element, **When** the user picks a color, **Then** the text is drawn in that color and the color is saved.
7. **Given** an SRAM element, **When** the user chooses "Load Contents From File", **Then** its contents are replaced by the file's values.

---

### User Story 3 - Faster editing workflow (Priority: P2)

A user presses Shift twice, types "cap", and their recent choices show first. They drag a capacitor from the toolbar onto a wire, and the wire splits and connects. They click elements and edit values in a side panel that stays open. They roll the wheel over a transistor to flip it to PNP.

**Why this priority**: The most visible usability changes. They do not affect file compatibility. Covers N6 to N8, C1, C3 to C6.

**Independent Test**: Build a reference task (2-stage RC filter with a transistor buffer) with 1.0.8 and with the new workflow, and check each scenario.

**Acceptance Scenarios**:

1. **Given** the canvas has focus, **When** the user presses Shift twice quickly, **Then** Find Component opens with recently used entries listed first.
2. **Given** a component button in the toolbar, **When** the user drags it onto the canvas, **Then** the component is placed where it is dropped.
3. **Given** an existing wire, **When** a new or moved element's terminal is dropped on the middle of it, **Then** the wire is split there and connected, and one Undo reverts the whole action.
4. **Given** "Show Properties on Click" is on, **When** the user clicks an element, **Then** its properties appear in a non-modal side panel that follows later selections and applies changes immediately.
5. **Given** "Show Properties on Create" is off, **When** the user places a resistor, **Then** no editor opens. Wires and logic inputs never open one on create.
6. **Given** the pointer is over a value-bearing element, **When** the user rolls one notch, **Then** a popup shows the value and the value moves exactly one step. Over a transistor the wheel toggles NPN/PNP; over a MOSFET or JFET it toggles the channel.
7. **Given** a selected two-terminal element, **When** the user chooses Swap Terminals, **Then** its terminals are exchanged and the action can be undone.

---

### User Story 4 - Better scopes (Priority: P2)

A user stacks scopes, AC-couples a signal with a DC offset, keeps a triggered sweep stable, and exports a scope as an image for a report.

**Why this priority**: Scopes are the main analysis tool (specs 002 and 003 invested in them). Covers N12 to N15, C8.

**Independent Test**: A circuit with three stacked scopes, one AC-coupled in auto scale and one triggered; check labels, sweep behavior and PNG / SVG export.

**Acceptance Scenarios**:

1. **Given** a scope, **When** the user chooses Export Image, **Then** one dialog offers PNG or SVG, width, height and transparent background, and the file matches the on-screen plot.
2. **Given** a 1 V sine on a 5 V offset, **When** AC coupling is on in auto scale, **Then** the trace centers on zero within one period, and Reset clears the filter.
3. **Given** stacked scopes tall enough for text, **When** drawn, **Then** each shows its label and legend, and a label identical to the one above is not repeated.
4. **Given** a triggered scope, **When** a sweep is in progress, **Then** the last complete sweep stays visible until the new one finishes.
5. **Given** any scope, **When** the user drags inside it, **Then** no circuit element moves or gets selected.
6. **Given** an X/Y scope, **When** the user hovers it, **Then** the probes feeding it show "X" or "Y".

---

### User Story 5 - Manage subcircuits (Priority: P3)

A user opens the Subcircuit Manager, sees previews, selects several subcircuits, makes them persistent, saves a library, and later loads it without touching the current circuit.

**Why this priority**: Advanced users; depends on Story 1. Covers N5 (management side), C7.

**Independent Test**: Create two subcircuits, make one persistent, restart, confirm it is available; save a library, delete both, load it with "Load Subcircuits Only", confirm both return and the circuit is unchanged.

**Acceptance Scenarios**:

1. **Given** subcircuits exist, **When** the manager opens, **Then** each is listed with a preview and its scope (local, global, persistent).
2. **Given** a subcircuit in use, **When** the user deletes it, **Then** deletion is refused with a message naming it.
3. **Given** a library file, **When** loaded with "Load Subcircuits Only", **Then** its subcircuits are added and the circuit is unchanged.

---

### User Story 6 - Fewer surprises (Priority: P3)

A user nudges the speed slider and closes the window without an unsaved-changes prompt. Later a numerical error leaves a visible warning explaining why the simulation stopped.

**Why this priority**: Small, low-risk polish. Covers N16 to N19.

**Independent Test**: Run each scenario by hand.

**Acceptance Scenarios**:

1. **Given** a freshly opened circuit, **When** only the speed, current-speed or brightness slider moves and the window closes, **Then** no prompt appears.
2. **Given** a circuit that becomes singular, **When** the error happens, **Then** a warning stays on the canvas until the circuit is changed or reset.
3. **Given** a wire connected to a labeled node, **When** hovered, **Then** the label is shown.
4. **Given** a dialog checkbox, **When** its text is clicked, **Then** it toggles.

---

### User Story 7 - Documentation matches the program (Priority: P1)

A user reading the user guide, or a contributor reading the architecture and component guides, finds every feature delivered here described where they would look for it, in the language each document is already written in.

**Why this priority**: The constitution (principle V) requires public behavior changes to be documented in the same change. Each story above ships with its documentation, so this story runs alongside all the others.

**Independent Test**: For every row of tables A and B marked delivered, find its description in the user guide (and README feature list where relevant); for every new element, find its entry in the component list and its example page.

**Acceptance Scenarios**:

1. **Given** a delivered feature, **When** a reader searches the user guide, **Then** they find how to use it in the matching section (for example "Editar valores", "Osciloscópios", "Subcircuitos", "Atalhos de teclado e mouse").
2. **Given** an updated `cs-*` example, **When** a reader opens its page under `docs/exemplos/`, **Then** the text and pictures describe the two-terminal sources it now uses.
3. **Given** the "Diferenças em relação ao CircuitJS1" section, **When** read after this feature, **Then** it no longer lists as missing anything this feature delivered, and it names the CircuitJS version prometheuscs now follows (5.0.4).

### Edge Cases

- Two labeled elements share a name: the first match in circuit order is used, without a warning, as upstream.
- An expression names a meter inside another subcircuit: it does not resolve, and a "missing reference" error is shown, as upstream.
- A 4.x circuit using the old custom composite element: it loads as the new subcircuit element.
- Export As Text and Export As Link with the XML-only elements (two-terminal sources, meter labels): the exported text keeps them, so pasting it back restores the circuit unchanged. (prometheuscs always saves XML, so the legacy text format never applies to saving.)
- A terminal dropped exactly on a wire's endpoint: it connects, and no zero-length wire is created.
- An SRAM file of 128,000 bytes or more: the load is refused with "Cannot load: That file is too large!" and the contents are unchanged. (The file is read as raw bytes, one value per byte, or per big-endian pair of bytes when the data is wider than 8 bits, as in the original; there is no malformed content.)
- An image export size of 0 or above the supported maximum: the dialog rejects it.
- A double Shift press while typing in a text field: it does not open Find Component.

## Requirements *(mandatory)*

### Functional Requirements

**Compatibility and elements**

- **FR-001**: The system MUST load, simulate and save two-terminal controlled voltage and current sources whose output is an expression, matching CircuitJS 5.0.4 within the spec-001 tolerance.
- **FR-002**: Expressions MUST support `v(name)`, `i(name)`, `dvdt(name)` and `didt(name)` for labeled nodes and labeled meters, scoped to the enclosing subcircuit. Bare `i` (9th input) and bare `didt` MUST keep their current meaning.
- **FR-003**: Voltmeters and ammeters MUST accept an optional label that is saved, displayed and usable as a reference name.
- **FR-004**: The bundled controlled-source examples MUST match upstream 5.0.4, and all bundled examples MUST still pass the compatibility harness.
- **FR-005**: The system MUST load 5.x subcircuit elements and MUST keep loading 4.x custom composite elements.
- **FR-006**: The monostable MUST offer an invert-trigger option; text elements MUST offer a saved text color; the SRAM MUST offer "Load Contents From File".
- **FR-007**: The Stop Trigger MUST NOT fire as a side effect of loading or resetting a circuit.
- **FR-008**: The file fixes in N20 that apply to prometheuscs MUST be made so circuits round-trip with CircuitJS 5.0.4 without losing values.

**Editing workflow**

- **FR-009**: Find Component MUST search, by localized name, four groups in this order: recent picks (at most 10, newest first, remembered across sessions), component types to place, menu commands, and elements already in the circuit. Running an entry places the component, runs the command or selects the element. It MUST open with `/`, a double Shift press (not while a text field has focus), the toolbar button and the Edit menu.
- **FR-010**: Users MUST be able to edit properties in a non-modal panel that follows the selection, with "Show Properties on Create" and "Show Properties on Click" options saved between sessions. Rarely edited elements MUST skip the editor on create. When the edit dialog is used, it stays centered (opening it next to the element is deferred, see Clarifications).
- **FR-011**: Users MUST be able to place a component by dragging it from the toolbar.
- **FR-012**: When a new or moved element's terminal lands on the interior of a wire or lead, the system MUST split it there and connect it, undoable as one action.
- **FR-013**: One wheel notch over an element MUST change its value by exactly one step with a value popup; over transistors, MOSFETs and JFETs the wheel MUST toggle polarity; over slider elements it MUST move the slider. The "Mouse Wheel Sensitivity" option MUST affect only zoom speed, and its text MUST say so.
- **FR-014**: The element menu MUST offer Swap Terminals, undoable.

**Scopes**

- **FR-015**: Users MUST be able to export a scope as PNG or SVG from one dialog with width, height and transparent-background options.
- **FR-016**: With AC coupling on, a periodic signal's displayed mean MUST be within ±2% of its peak amplitude of zero after one period. Reset MUST clear the coupling filter, and AC coupling MUST work in auto-scale mode.
- **FR-017**: Stacked scopes MUST show labels and legends when there is room, MUST NOT repeat a label identical to the scope above, and MUST list combined plots in on-screen order.
- **FR-018**: A triggered scope MUST keep the last complete sweep on screen until the next is complete.
- **FR-019**: Dragging inside a scope MUST NOT move or select circuit elements.
- **FR-020**: Hovering an X/Y scope MUST show the X or Y role on the probes feeding it.

**Subcircuits**

- **FR-021**: The Subcircuit Manager MUST show previews, support multi-select, change scope (local, global, persistent), refuse to delete subcircuits in use, and save libraries.
- **FR-022**: Users MUST be able to load a subcircuit library with "Load Subcircuits Only" without changing the current circuit.

**Quality of life**

- **FR-023**: The unsaved-changes prompt MUST appear only when the circuit's content changed.
- **FR-024**: A simulation error MUST leave a visible warning on the canvas until the circuit is changed or reset.
- **FR-025**: Hovering a wire connected to a labeled node MUST show that node's label.
- **FR-026**: Clicking a checkbox's label MUST toggle it; model-editing and Other Options dialogs MUST be modal and titled.

**Localization and documentation**

- **FR-027**: Every new user-visible string MUST be localized through the existing locale files, using upstream 5.0.4 translations where they exist.
- **FR-028**: The user guide (`docs/guia-do-usuario.md`) MUST describe every delivered feature in its matching section: controlled sources and name references under "Componentes" and "Componentes especiais"; the properties panel, wire splitting, toolbar drag, wheel popups and Swap Terminals under "Montar o circuito"; scope changes under "Osciloscópios"; manager changes under "Subcircuitos"; new options under "Opções"; double Shift under "Atalhos de teclado e mouse".
- **FR-029**: The "Diferenças em relação ao CircuitJS1" section of the user guide and the README ("De onde vem", "O que é diferente") MUST name CircuitJS 5.0.4 as the reference and MUST be updated to reflect what this feature delivers or defers.
- **FR-030**: The architecture document (`docs/arquitetura.md`) MUST describe the name-reference mechanism in expressions, the new controlled-source elements and the property panel; the component guide (`docs/adding-a-component.md`) MUST explain how an element exposes a label for name references.
- **FR-031**: The example pages under `docs/exemplos/` for every updated `cs-*` circuit MUST be regenerated or rewritten to match the new circuits, and new examples MUST get pages.

### Key Entities

- **Feature inventory**: tables A (N1–N20), B (C1–C8) and C. Each in-scope row has a delivering story and ends as delivered or explicitly deferred.
- **Named reference**: a label on a node, voltmeter or ammeter plus the subcircuit it lives in, used by expressions.
- **Controlled source**: a two-terminal element whose voltage or current is an expression over named references and time.
- **Subcircuit definition**: a named reusable block with a scope (local, global, persistent) and a pin layout.
- **Documentation set**: README, user guide, architecture document, component guide and example pages.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the bundled examples load and match their existing recordings within the spec-001 tolerance. The rewritten `cs-*` examples match the recordings of their old versions, and any difference is explained in `known_differences.json`.
- **SC-002**: 100% of a reference set of 5.x-format circuits (at least one per element and expression form in N1 to N5) open without errors, keep all elements and values after save and reload, and give results equal to exact hand calculations within the spec-001 tolerance.
- **SC-003**: Every row in tables A and B ends as delivered or explicitly deferred with a recorded reason.
- **SC-004**: Building the reference editing task takes at least 30% fewer clicks than with prometheuscs 1.0.8.
- **SC-005**: Exported scope PNG and SVG files open correctly in common viewers in 100% of test cases.
- **SC-006**: 0 unsaved-changes prompts in sessions where only the speed, current-speed or brightness controls were touched.
- **SC-007**: Simulation speed on the bundled examples drops by no more than 5% compared with 1.0.8.
- **SC-008**: 100% of delivered features are findable in the user guide by their menu or element name, and 100% of updated `cs-*` examples have a matching page in `docs/exemplos/`.

## Assumptions

- Reference behavior: upstream release 5.0.4 (commit `8ac55f22`). Port baseline: Java 4.1.5.1js (commit `d6b80f46`). The reference copy named in spec 001 (`~/Workspace/Rust/circuitjs1`) no longer exists; the clone at `~/Workspace/circuitjs1` replaces it.
- No CircuitJS 5.0.4 build is needed. The existing recordings, made with the Java build at `~/Bin/circuitjs1`, remain the numeric reference. New elements are checked against hand calculations. The 5.x-format test circuits are written by hand from the upstream file format, or saved from the online version at falstad.com. Building 5.0.4 locally is a fallback for nonlinear cases only.
- Table C was built from code search and menu/element registration. Rows found missing during planning move to table B.
- Port gaps (table B) are included because the request is to bring CircuitJS features to prometheuscs, whatever release added them.
- Delivery is incremental by priority; documentation (Story 7) ships with each story, not at the end.
- Documentation keeps its current language: the user guide, README, architecture document and example pages in Portuguese, the component guide in English.
- New UI pieces follow the existing GTK4/libadwaita look of prometheuscs, not a pixel copy of the web version.
