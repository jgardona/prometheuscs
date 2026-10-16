# Feature Specification: Circuit Simulator Migration (prometheuscs)

**Feature Branch**: `001-circuit-simulator-migration`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Migration of the JavaScript circuit simulation software (source at /home/jgardona/Workspace/Rust/circuitjs1) to Rust, named prometheuscs. Native desktop UI with the relm4 framework (GTK4 + libadwaita), GPU-accelerated circuit rendering window for fluid animations, UI similar to the provided screenshots, and a strong focus on performance. Additional requirements: bundled circuit examples must be fully compatible with the new version; all circuitjs components must be present; GTK4/libadwaita icons may be used; component and tool icons may be imported from circuitjs."

## Clarifications

### Session 2026-10-12

- Q: Which repository layout should the project use? → A: A single Cargo package at the repository root (the existing `Cargo.toml`), with no multi-crate workspace, to keep the project simple.
- Q: (Added later by the user) Should the program be modular so components are easy to add or remove? → A: Yes. Adding or removing a component type must be a localized change: a self-contained unit plus one registration, with no edits to the engine core, menus, file loading or icon handling.
- Q: Which licence will the project use? → A: GPL-2.0-or-later (the same terms as the original; the user reverted an earlier Apache-2.0 choice because the project is non-commercial and must be publishable without licensing problems).
- Q: Where is the built original simulator available for generating reference results? → A: `/home/jgardona/Bin/circuitjs1` (desktop build of the original; the executable is `/home/jgardona/Bin/circuitjs1/circuitjs1`).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Build and simulate a circuit visually (Priority: P1)

A user opens prometheuscs, sees an empty circuit canvas with a menu bar, toolbar and side panel, places components (wire, resistor, capacitor, inductor, voltage source, switch, ground), connects them, and runs the simulation. Current flow and voltages are animated live on the canvas.

**Why this priority**: This is the core product value. Without a working editor plus simulation loop, nothing else is useful.

**Independent Test**: Build a series RLC circuit from scratch, press run, and verify that animated current flow appears and that node voltages match the reference simulator for the same circuit.

**Acceptance Scenarios**:

1. **Given** an empty canvas in Edit Mode, **When** the user selects a component from the toolbar or Draw menu and drags on the canvas, **Then** the component is placed with editable values and drawn with its label.
2. **Given** a complete circuit, **When** the simulation runs, **Then** current is shown as moving markers along wires and voltage is shown by wire color, updating continuously.
3. **Given** a running simulation, **When** the user changes a value (for example capacitance) via the side-panel slider or mouse wheel, **Then** the simulation reflects the change immediately without restarting.
4. **Given** a running simulation, **When** the user presses Stop or Restart, **Then** the simulation pauses or resets to time zero.

---

### User Story 2 - Open existing circuits and examples (Priority: P1)

A user opens circuit files created with the original simulator, or picks one from the "Circuit Examples" menu (Basics, AC Circuits, Filters, Diodes, Op-Amps, Transistors, MOSFETs, 555 Timer, Logic, and so on), and gets the same behavior as in the original.

**Why this priority**: Compatibility with the existing circuit library validates the migration and gives users immediate value.

**Independent Test**: Load each bundled example and each automated-test circuit from the original project and compare simulation results to the reference.

**Acceptance Scenarios**:

1. **Given** a circuit file saved by the original simulator, **When** the user opens it, **Then** all components, values, positions and settings load correctly.
2. **Given** the Circuit Examples menu, **When** the user chooses an example, **Then** it loads and runs with results matching the reference simulator.
3. **Given** an edited circuit, **When** the user saves, exports as text, or exports as link, **Then** the result can be re-imported with identical content.

---

### User Story 3 - Observe signals with scopes (Priority: P2)

A user attaches scopes to components and sees voltage/current waveforms in the bottom panel, with peak values, component labels, elapsed time, time step and frequency readouts. Scopes can be stacked, combined, or separated from the Time Diagrams menu.

**Why this priority**: Waveform inspection is the main analysis tool, but needs the simulation core first.

**Independent Test**: Run the RLC circuit and verify that waveforms, peak values and timing readouts appear and update smoothly.

**Acceptance Scenarios**:

1. **Given** a running circuit, **When** the user adds a scope to a component, **Then** its waveform scrolls in real time with the correct label and peak value.
2. **Given** several scopes, **When** the user chooses Stack All, Unstack All, Combine All or Separate All, **Then** the scope panel rearranges accordingly.

---

### User Story 4 - Full editing workflow (Priority: P2)

A user performs everyday editing: undo/redo, cut/copy/paste/duplicate, select all, rotate and mirror, find component, center circuit, zoom in/out/100%, and pan.

**Why this priority**: Required for comfortable real use, but a circuit can be built with the basics first.

**Independent Test**: Build a circuit, then exercise each Edit-menu command and verify the expected effect and keyboard shortcut.

**Acceptance Scenarios**:

1. **Given** a series of edits, **When** the user presses undo repeatedly, **Then** each edit is reverted in order, and redo restores them.
2. **Given** a selection, **When** the user rotates, mirrors, duplicates, copies or pastes, **Then** the selection transforms and appears as expected.
3. **Given** the canvas, **When** the user zooms or pans, **Then** the view changes smoothly and the circuit remains interactive.

---

### User Story 5 - Display options, export and subcircuits (Priority: P3)

A user toggles display options (show current, voltage, power, values, small grid, crosshair, European resistor symbols, IEC gates, white background, conventional current direction), exports the circuit as image/SVG/text/link, prints, toggles full screen, and creates and manages subcircuits.

**Why this priority**: Valuable polish and sharing features that build on the core.

**Independent Test**: Toggle each option and confirm the visual change. Export as image and SVG and verify the output opens correctly.

**Acceptance Scenarios**:

1. **Given** the Options menu, **When** the user toggles an option, **Then** the canvas updates immediately and the choice persists across sessions.
2. **Given** a circuit, **When** the user exports as image, SVG, text or link, **Then** a valid file or clipboard content is produced.
3. **Given** a selection, **When** the user creates a subcircuit, **Then** it can be reused and managed from the Tools menu.

---

### Edge Cases

- A circuit with a floating node, short circuit, or singular configuration: the simulator reports a clear error and highlights the offending element rather than crashing or freezing.
- A very large circuit (thousands of elements): the UI stays responsive and the simulation does not stall input.
- A circuit file that is corrupted, truncated, or uses an unknown element type: the user sees an error naming the problem, and the rest of the file loads when possible.
- Simulation that diverges (non-convergence): the user is told the circuit failed to converge and may adjust or restart.
- A machine with no usable GPU acceleration: the app still renders correctly (lower smoothness) and informs the user.
- High-DPI and fractional-scale displays: the circuit and text remain sharp.
- Window resize, minimize, or full-screen toggle while simulating: rendering and simulation continue without glitches.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST be a native desktop application named prometheuscs that reproduces the functionality of the original JavaScript simulator located at /home/jgardona/Workspace/Rust/circuitjs1.
- **FR-002**: The system MUST provide a main window with a menu bar (File, Edit, Draw, Time Diagrams, Options, Tools, Circuit Examples), a toolbar of component and view actions with a mode indicator, a circuit canvas, a scope panel at the bottom, and a right side panel, laid out like the supplied reference screenshots.
- **FR-003**: The right side panel MUST provide Restart and Stop controls, simulation-speed, current-speed and brightness sliders, the current circuit name, and per-circuit adjustable value sliders (for example capacitance, inductance, resistance).
- **FR-004**: The File menu MUST offer: new window, new circuit, open file, import text, save, save as, export as link, export as text, export as image, copy image to clipboard, export as SVG, create subcircuit, find DC operating point, recover autosave, print, toggle full screen, and about.
- **FR-005**: The Edit menu MUST offer undo, redo, cut, copy, paste, duplicate, select all, find component, center circuit, zoom to 100%, zoom in, zoom out, mirror X, mirror Y, rotate counter-clockwise and rotate clockwise, each with its keyboard shortcut.
- **FR-006**: The Draw menu MUST allow inserting every component category supported by the original (wires, passives, inputs and sources, outputs and labels, active components, active building blocks, logic gates, digital ICs, analog and hybrid ICs, subcircuits) and MUST support a drag/pan mode toggle.
- **FR-007**: The system MUST support all component types supported by the original simulator, with identical electrical behavior.
- **FR-008**: The simulation results MUST match the original simulator's results within a documented numerical tolerance for the same circuit, time step and settings. Circuits whose behavior is random or depends on real time (noise sources, audio elements, wall-clock driven elements) cannot match step by step; they MUST be listed explicitly as exceptions and checked by statistical or property-based criteria instead (for example amplitude, mean and spectrum within bounds).
- **FR-009**: The system MUST read and write the original simulator's circuit file format and text export format, and MUST import circuits shared as links.
- **FR-010**: The system MUST include all circuit examples shipped with the original, organized in the same categories in the Circuit Examples menu.
- **FR-011**: The circuit canvas MUST be rendered with hardware (GPU) acceleration so that animation of current flow, scrolling scopes, pan and zoom appear fluid.
- **FR-012**: The system MUST animate current flow and show voltage and power visually on wires and components, controlled by the Options menu toggles.
- **FR-013**: The system MUST provide scopes showing waveforms with peak value, component label, elapsed time, time step and frequency readouts, and MUST support stack, unstack, combine and separate operations.
- **FR-014**: The Options menu MUST provide toggles for show current, show voltage, show power, show values, small grid, toolbar, crosshair cursor, European resistor symbols, Ω unit, IEC logic gates, white background, conventional current direction, disable editing, mouse-wheel value editing, plus dialogs for offsets and other options. These settings MUST persist between sessions.
- **FR-015**: The Tools menu MUST provide converting wires to routed wires and a subcircuit manager.
- **FR-016**: The system MUST report simulation errors (singular matrix, non-convergence, invalid element) to the user with the offending element identified, and MUST NOT crash or hang.
- **FR-017**: The system MUST autosave work in progress so it can be recovered after an unexpected exit.
- **FR-018**: The user interface MUST be presented in Portuguese (matching the reference screenshots) with the ability to add other languages, reusing the original project's language resources where available.
- **FR-019**: The system MUST keep the interface responsive while the simulation runs; user input MUST never be blocked by the simulation.
- **FR-025**: The system MUST be modular with respect to components: each component type (or family of related types) MUST be a self-contained unit that declares its own identity (name, file-format code, menu category, icon, shortcut, editable parameters, electrical behavior, drawing). Adding a component MUST NOT require changes to the simulation core, the application shell, the menus, the file loader or the example catalog; the Draw menu, the placement toolbar entries, file loading and the component inventory MUST discover components from their declarations.
- **FR-026**: A component family MUST be removable at build time without breaking the rest of the system: the application builds and runs with that family absent, and a circuit that uses a missing component loads with a clear error naming it (see FR-016 behavior for unknown elements) while the rest of the circuit loads.
- **FR-027**: The project MUST include a short developer guide, with a worked example, showing how to add a new component and how to remove one.
- **FR-020**: The system MUST include an automated verification suite that runs the original project's test circuits and compares results to the reference simulator.
- **FR-021**: Every bundled example circuit of the original simulator (the examples library, about 370 circuit files including subcircuit examples) MUST load, appear in the Circuit Examples menu, and simulate with behavior fully compatible with the original, with no manual edits to the files.
- **FR-022**: Every component type present in the original simulator MUST be available in prometheuscs, reachable from the Draw menu, with the same editable parameters, default values, and drawing semantics. A component inventory MUST be kept and checked against the original with no omissions.
- **FR-023**: General interface icons (menus, dialogs, window controls, file actions) MUST use the standard icon set of the platform toolkit, so the application looks native.
- **FR-024**: Icons for electrical components and for circuit-editing tools (toolbar and Draw-menu symbols) MUST be reused from the original project's assets, so symbols look familiar to existing users.

### Key Entities

- **Circuit**: The full design: a collection of elements, wires, scopes, display settings and simulation settings. Can be saved, loaded, exported and shared.
- **Element (Component)**: A placeable part (resistor, capacitor, source, gate, IC, and so on) with a type, position, orientation, editable values and terminals.
- **Node**: A connection point between element terminals whose voltage is solved by the simulator.
- **Scope**: A waveform display attached to one or more elements or nodes, with its own scale and display options.
- **Subcircuit**: A reusable circuit definition with named pins, instantiable as an element.
- **Simulation State**: Current time, time step, speed, run/stop status and the solved voltages and currents.
- **Settings/Preferences**: Persistent display and behavior options.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the original project's automated test circuits and bundled examples load and run; simulation results fall within the documented tolerance of the reference results.
- **SC-002**: A user can build and run a simple RLC circuit from an empty canvas in under 3 minutes without documentation.
- **SC-003**: The canvas sustains at least 60 frames per second during animated simulation, pan and zoom on a typical mid-range desktop computer, for circuits of up to 500 elements.
- **SC-004**: Simulation throughput is at least equal to the original JavaScript simulator on every benchmark circuit, and at least 2x faster on large circuits (1000+ elements).
- **SC-005**: Application startup to an interactive window takes under 2 seconds on a typical desktop computer.
- **SC-006**: Input latency (click to visible response) stays under 50 ms even while simulating the largest benchmark circuit.
- **SC-007**: Every menu entry and toolbar action shown in the reference screenshots is present and functional.
- **SC-008**: A circuit saved by prometheuscs reopens with identical content, and a circuit saved by the original opens without loss of components or values (0 data loss on all bundled examples).
- **SC-009**: Zero crashes or hangs across all invalid-circuit and corrupted-file edge cases in the test suite.
- **SC-010**: 100% of the components in the original's component inventory are present and placeable, verified against the inventory (0 missing).
- **SC-011**: 100% of the bundled example circuits pass the compatibility check (load without errors and match reference behavior).
- **SC-012**: A developer who has never touched the project can add a simple new component (for example a new two-terminal element) in under 2 hours following the guide, editing only the component's own unit plus one registration; verified by adding a sample component in an automated test that touches no other source file.
- **SC-013**: With any single component family disabled at build time, the project still builds, all tests not specific to that family pass, and circuits using it report a clear error instead of crashing.

## Assumptions

- Target platform is desktop Linux; other operating systems are out of scope for the first release.
- The technology choices from the request are project constraints, not open questions: the implementation is in Rust, the interface uses relm4 (GTK4 and libadwaita), and the circuit canvas is drawn with GPU acceleration. Detailed design belongs to the planning phase.
- The original project's source, circuit files, examples, language files and tests at /home/jgardona/Workspace/Rust/circuitjs1 are the behavioral reference and are available for reading.
- Features specific to the browser (cloud storage integration such as Dropbox import, browser developer tools, embedded web page mode, and websocket features) are out of scope. Equivalent native features (file dialogs, clipboard, print) replace them.
- Implementation may be delivered incrementally, simulation core first, then rendering and editing, then remaining components and polish, as reflected in the story priorities.
- The project is licensed under GPL-2.0-or-later, matching the original simulator, so its code, example circuits and icons can be reused and published without permission requests; the project records the provenance of every imported asset.
- The project is a single Cargo package at the repository root, organized in internal modules (engine, rendering, application) rather than separate packages.
- A built, runnable copy of the original simulator exists at `/home/jgardona/Bin/circuitjs1` and is the source of reference results for compatibility checks (FR-008, FR-020, FR-021).
- The original examples live in `src/com/lushprojects/circuitjs1/public/circuits` of the reference project (373 circuit files in a single flat folder, with no subfolders; the original keeps subcircuit pin images separately under `img/subcircuits`); component and tool icons are taken from the original's image assets, and their licensing is compatible with the original project's license (see COPYING.txt there). Locating and converting these assets is a planning task.
- Interface text defaults to Portuguese, as shown in the reference screenshots.
- Performance targets assume a typical mid-range desktop (4+ cores, integrated or discrete GPU with current drivers).
