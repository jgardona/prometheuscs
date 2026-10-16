# Tasks: Circuit Simulator Migration (prometheuscs)

**Input**: Design documents from `/specs/001-circuit-simulator-migration/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Included and written first. The constitution (II. Test-First, NON-NEGOTIABLE) requires a failing test before each behavior change.

**Red-test rule** (keeps the CI gate green, constitution III and Workflow): a failing test is written first locally and committed in the same change as the implementation that makes it pass. A test whose implementation arrives in a later phase (listed below) is committed with `#[ignore = "<reason>, enabled by T0xx"]` and un-ignored by that later task. Tasks marked "(ignored until Txxx)" follow this rule.

**Organization**: Grouped by user story. Foundational work blocks all stories.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 to US5, matching the user stories in spec.md

## Path Conventions

Single Cargo package at the repository root (spec clarification 2026-10-12). One library crate (`src/lib.rs`) holds the engine, renderer and application modules; the binary is `src/main.rs`; developer tools are extra binaries in `src/bin/`. Integration tests live in `tests/`. The original simulator source is read-only at `/home/jgardona/Workspace/Rust/circuitjs1` (called "JAVA/" below, meaning `src/com/lushprojects/circuitjs1/client/`). A runnable build of the original is at `/home/jgardona/Bin/circuitjs1/circuitjs1`.

> **Licence**: GPL-2.0-or-later (decided; research.md R-9). plan.md and research.md describe the single-package layout.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, licence resolution, quality gates

- [X] T001 Apply the GPL-2.0-or-later decision: add the full licence text as `LICENSE`, set `license = "GPL-2.0-or-later"` in `Cargo.toml`, add an SPDX header and the original authors' copyright notices policy to `README.md` (see `research.md` R-9; provenance goes in `assets/ATTRIBUTION.md`, T007)
- [X] T002 Create the directory layout from plan.md (`assets/`, `docs/`, `benches/`, `tests/compat/`, `src/bin/`) with placeholder files
- [X] T003 Configure `Cargo.toml`: edition 2024, `[lints]` table (warnings as errors for clippy), `[[bin]]` entries for `prometheuscs`, `import_assets`, `inventory`, `golden`, and dependency set from research (relm4, gtk4, libadwaita, femtovg, serde, serde_json, thiserror, tracing; dev: criterion, proptest, insta)
- [X] T004 [P] Add `rustfmt.toml` and `clippy.toml` at the repository root and a `README.md` describing build prerequisites (GTK 4, libadwaita, OpenGL)
- [X] T005 [P] Add CI workflow in `.github/workflows/ci.yml` running `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (constitution I and gates)
- [X] T006 Create module skeleton with `//!` docs in `src/lib.rs`, `src/format/mod.rs`, `src/elements/mod.rs`, `src/solver/mod.rs`, `src/scope/mod.rs`, `src/expr/mod.rs`, `src/render/mod.rs`, `src/app/mod.rs`, `src/error.rs`
- [X] T007 [P] Create `assets/ATTRIBUTION.md` listing the origin and licence of every imported asset class (examples, locales, SVG toolbar icons, Fontello glyphs per `war/font/LICENSE.txt`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Format, element framework, solver core, asset import, golden-data tooling

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Asset import and reference tooling

- [X] T008 [P] Implement `src/bin/import_assets.rs` copying the 373 example files, `setuplist.txt` and the 15 `locale_*.txt` files from `src/com/lushprojects/circuitjs1/public/` of the original into `assets/circuits/`, `assets/setuplist.txt`, `assets/locale/`
- [X] T009 [P] Extend `src/bin/import_assets.rs` to extract the inline SVG icon strings (`wireIcon`, `resistorIcon`, and the rest) from `JAVA/Toolbar.java` into `assets/icons/components/*.svg` and the used Fontello glyphs from `war/font/fontello.svg` into `assets/icons/ui/*.svg`
- [X] T010 Implement `src/bin/inventory.rs` that scans `JAVA/*Elm.java` and dump-type registrations and writes `assets/inventory.json` (expected: 153 element classes)
- [X] T011 Write failing test `tests/inventory.rs` asserting every entry of `assets/inventory.json` has a registered Rust element (commit as `#[ignore = "enabled by the inventory-completion task"]`; un-ignored when every family is ported; marks SC-010); the test MUST skip entries whose family feature is disabled — done: `tests/inventory.rs`; the full check is `#[ignore]`d (133 of 149 classes still unported) and skips families whose feature is off
- [ ] T012 Implement `src/bin/golden.rs` that drives `/home/jgardona/Bin/circuitjs1/circuitjs1` (Electron, remote debugging port) to load a circuit, run N steps and write `tests/compat/golden/<name>.json` with node voltages, element currents and scope samples; document the invocation in `tests/compat/README.md` **[DEFERRED by the user: the Electron build is not driven programmatically; reference data comes from the 7 recorded auto-tests (T013) and analytical oracles; revisit later]** — DEFERRED by the user (no Electron); the recorded auto-tests (T013, T035) replace it for the circuits they cover
- [X] T013 Convert the 7 files in `/home/jgardona/Workspace/Rust/circuitjs1/auto-tests/*.txt` to fixtures in `tests/compat/auto/` with an XML reader in `tests/common/auto_test.rs` — done: 7 fixtures in `tests/compat/auto/`, reader in `tests/common/auto_test.rs`, checks in `tests/auto_fixtures.rs`

### Error model and file format (contracts/circuit-file-format.md)

- [X] T014 [P] Define typed errors `LoadError`, `AnalysisError`, `StepError` with `thiserror` in `src/error.rs`
- [X] T015 [P] Write failing round-trip proptest for the header line and generic element line in `tests/format_roundtrip.rs` — done: `tests/format_roundtrip.rs` (header and generic element lines, including extreme floats)
- [X] T016 Implement tokenizer and original escape rules (port of `JAVA/StringTokenizer.java` and `CustomLogicModel.escape`) in `src/format/tokenizer.rs`
- [X] T017 Implement circuit parser with per-line errors and partial load (guarantee G3) in `src/format/parser.rs`
- [X] T018 Implement circuit writer with `f64` round-trip numbers (guarantees G2, G6) in `src/format/writer.rs` (XML writer, as the original saves in XML)
- [X] T019 [P] Implement share-link codec compatible with the original's LZ-String based links (guarantee G5; port `JAVA/ExportAsUrlDialog.java` encoding) in `src/format/link.rs` — done: `src/format/link.rs`, bit-exact against the bundled lz-string.min.js

### Element framework (data-model.md: Element, Node)

- [X] T020 Define the `Element` trait mirroring `JAVA/CircuitElm.java` (parse, dump, post positions, setup, stamp, step, draw, edit info, info text) in `src/elements/element.rs`
- [X] T021 Write failing test `tests/modularity.rs`: a test-only dummy element defined and registered entirely inside the test file must appear in the factory, the Draw menu model and a file round trip without editing any file in `src/` (SC-012) — done: `tests/modularity.rs` — a dummy element defined and registered in the test appears in factory, Draw menu and round trip; no file under `src/` changed
- [X] T022 Implement the declarative element registry in `src/elements/registry.rs`: each family module exposes one `register(&mut Registry)` function that declares every element with its metadata (display name, dump type code, menu category, icon name, shortcut, constructor); factory dispatch, Draw menu model, placement toolbar entries and inventory are all derived from this registry, with no central `match` over element types (FR-025; port of the dispatch in `JAVA/ElementFactory.java` and `CircuitLoader.java`)
- [X] T023 Add one Cargo feature per element family in `Cargo.toml` (all enabled by default) and gate each family module and its `register` call in `src/elements/mod.rs`; unknown element lines from a disabled family produce the "unknown element" `LoadError` (FR-026) (done for the families that exist: `passive`, `reactive`, `sources`; each new family adds its feature and `register` call)
- [X] T024 [P] Implement geometry helpers (grid points, rotation, mirroring, post positions) in `src/elements/geometry.rs`
- [X] T025 [P] Implement `Node`, node links and `Circuit` container with flags and settings per data-model.md in `src/format/circuit.rs`

### Solver core (contracts/engine-api.md)

- [X] T026 Write failing unit tests for sparse LU on small known matrices in `src/solver/sparse_lu.rs` (`#[cfg(test)]`)
- [X] T027 Port `JAVA/matrix/SparseLU.java`, `DMatrixSparseCSC.java`, `DGrowArray.java`, `IGrowArray.java` to `src/solver/sparse_lu.rs` and `src/solver/csc.rs`
- [X] T028 Write failing test for a resistor divider solved through `analyze` and `step` in `tests/solver_basic.rs`
- [X] T029 Port analysis (node assignment, floating-node and short detection, matrix simplification) from `JAVA/CirSim.java` and `JAVA/CircuitMatrix.java` into `src/solver/analyze.rs` (implemented in `src/solver/sim.rs`, with path search in `src/solver/path.rs`)
- [X] T030 Port the time-step and nonlinear convergence loop from `JAVA/SimulationManager.java` into `src/solver/step.rs`, with no allocation in steady state (implemented in `Simulation::step` in `src/solver/sim.rs`)
- [X] T031 Implement `Simulation`, `FrameSnapshot` and `snapshot()` per data-model.md in `src/solver/simulation.rs` — done: `FrameSnapshot` in `src/solver/snapshot.rs` and `Simulation::snapshot` in `src/solver/sim.rs` (the simulation lives in `sim.rs`, not `simulation.rs`)

### Rendering and settings foundations

- [X] T032 [P] Define the `Painter` trait (line, polyline, polygon, ellipse, arc, text, stroke, color, dash, transform stack) in `src/render/painter.rs`
- [X] T033 [P] Implement the SVG backend of `Painter` in `src/render/svg.rs`, tested by snapshot with `insta` in `tests/render_svg.rs` — done: `src/render/svg.rs` and `tests/render_svg.rs` with insta snapshots in `tests/snapshots/`
- [X] T034 [P] Implement `Settings` load/save in the XDG config directory (unknown keys ignored) in `src/app/settings.rs` with tests in the same file — done: `src/app/settings.rs` (JSON in the user config folder)

**Checkpoint**: Format, element framework, solver and tooling ready. User stories can begin.

---

## Phase 3: User Story 1 - Build and simulate a circuit visually (Priority: P1) 🎯 MVP

**Goal**: Place basic components, connect them, run, and see animated current and voltage with live value changes.

**Independent Test**: Build a series RLC circuit from scratch, run it, verify animated current flow and node voltages matching the reference (`lrc.txt` auto-test).

### Tests for User Story 1 ⚠️ (write first, must fail)

- [X] T035 [P] [US1] Integration test: load `assets/circuits/lrc.txt`, run to the recorded time, compare to the golden values in `tests/us1_rlc.rs` — — done: `tests/us1_rlc.rs` compares lrc, lrcswitch (with its switch event) and capac against the original's recorded scope data to 1e-9; the solver reproduces the recording to the printed digits
- [X] T036 [P] [US1] Unit tests for wire, resistor, capacitor, inductor, ground, DC voltage source stamping in each element file under `src/elements/`
- [X] T037 [P] [US1] Test for the worker thread: edits and parameter messages produce snapshots without blocking in `tests/worker.rs` — done: `tests/worker.rs` (8 tests: bit-exact against a headless run, parameter messages, advancing snapshots, no blocking, dump of live state, error reporting, session integration, edit keeps state and clock)
- [X] T038 [P] [US1] Action-layer test `dispatch(Action) -> State` for Restart, Stop, slider change in `tests/actions.rs` (contract C6) — — done: `tests/actions.rs` and `commands::apply` for Restart, Run, sliders and speeds, plus `dispatch` for menu commands

### Implementation for User Story 1

- [X] T039 [P] [US1] Port `WireElm`, `GroundElm`, `ResistorElm` to `src/elements/passive.rs` (parse, dump, stamp, draw, edit info)
- [X] T040 [P] [US1] Port `CapacitorElm`, `PolarCapacitorElm`, `InductorElm` to `src/elements/reactive.rs` (done; XML dump/undump included)
- [X] T041 [P] [US1] Port `DCVoltageElm`, `ACVoltageElm`, `VoltageElm`, `RailElm`, `CurrentElm` to `src/elements/sources.rs` (done: Voltage, DC, AC, Rail, ACRail, SquareRail, Clock, VarRail, Current; XML dump/undump included)
- [X] T042 [P] [US1] Port `SwitchElm`, `Switch2Elm`, `PushSwitchElm` to `src/elements/switches.rs` — — done: `SwitchElm`, `PushSwitchElm` and `Switch2Elm` (SPDT and multi-throw, link groups, flip handling) in `src/elements/switches.rs`
- [X] T043 [US1] Implement the drawing of current dots, voltage colors and labels (port of `CircuitElm` drawing helpers) in `src/render/scene.rs` (done: `draw_scene`, `View::fit`, `post_draw_list`; elements draw dots/colors/labels themselves)
- [X] T044 [US1] Spike: GL canvas widget on `GtkGLArea` with femtovg drawing 500 animated elements at 60 fps; record result in `research.md` R-2 (go or switch backend) in `src/app/canvas/gl.rs` — done, GO: results in research.md R-2 (500 elements 7.2 ms per whole frame on an RX 7600); `src/bin/gl_spike.rs`
- [X] T045 [US1] Implement the femtovg-backed `Painter` in `src/render/gl.rs` (all `unsafe` confined to this module with `// SAFETY:` comments) — done: `GlPainter` and `GlCanvas` in `src/render/gl.rs` (only module with `unsafe`); the window uses it with the Cairo canvas as fallback (`PROMETHEUSCS_RENDERER=cairo` forces the CPU canvas)
- [X] T046 [US1] Implement the worker thread, command channel and latest-snapshot slot in `src/app/worker.rs` — done: `src/app/worker.rs`; the solver runs on its own thread, the window copies voltages and currents from the newest snapshot (`src/app/engine.rs`)
- [X] T047 [US1]Implement the relm4 root component and main window layout with libadwaita (menu bar, toolbar, canvas, side panel, scope area placeholder) in `src/app/mod.rs` and `src/main.rs` — — done: window, menu bar, toolbar, GPU canvas, side panel and the scope area (a placeholder until US3)
- [X] T048 [US1]Implement the side panel (Restart, Stop, simulation speed, current speed, brightness, circuit title, adjustable sliders) in `src/app/panels/side.rs` — done: restart, stop, simulation speed, current speed, brightness (enabled with Show Power), circuit title and adjustable sliders with -/+ arrows in `src/app/window.rs`
- [X] T049 [US1] Implement the icon layer in `src/app/icons.rs`: a table mapping each original Fontello glyph name used in `JAVA/Menus.java` and `JAVA/Toolbar.java` (for example `doc-new`, `floppy`, `export`, `image`, `microchip`, `magic`, `zoom-in`) to a GTK/Adwaita symbolic icon name (FR-023), falling back to the imported SVG from `assets/icons/ui/` when no symbolic equivalent exists, plus loading of the imported component icons from `assets/icons/components/` (FR-024); include a unit test that every glyph used by the menus and toolbar resolves to an icon — done: `src/app/icons.rs` (symbolic icons, imported SVG fallback recoloured for the theme)
- [X] T050 [US1] Implement the toolbar with icons from the icon layer (T049) and mode indicator in `src/app/panels/toolbar.rs` — done: `src/app/toolbar.rs`
- [X] T051 [US1]Implement canvas input: placement by drag, grid snapping, mouse hover, selection, element edit dialog (port `JAVA/MouseManager.java`, `EditDialog.java`) including the edit/drag mode toggle (Space or Shift+drag pans; tool "Arrastar") that updates the toolbar mode indicator text ("Modo: Modo Edição") and has a unit test for both modes in `src/app/canvas/input.rs` and `src/app/dialogs/edit.rs` — done in `src/app/edit.rs` (mouse gestures, selection, placement, drag modes, hover, wheel) and `window.rs`; edit dialog in `dialogs.rs`; tests for both modes in `edit.rs`
- [X] T052 [US1] Implement live parameter changes (sliders, mouse wheel) sent as messages to the worker in `src/app/worker.rs` — — done: sliders and the mouse wheel send `Command::Edit` to the worker
- [X] T053 [US1] Show simulation errors as a banner naming the element and highlight it (contract C5) in `src/app/mod.rs` — — done: the banner names the element and the canvas highlights it; dismissible
- [X] T054 [US1] Add `tracing` logging for run, stop, reset and errors in `src/app/worker.rs` — done: `tracing` events for run, stop, reset and errors in `src/app/session.rs`; `RUST_LOG` selects the level

**Checkpoint**: US1 fully functional and testable on its own (MVP).

---

## Phase 4: User Story 2 - Open existing circuits and examples (Priority: P1)

**Goal**: Every original example and component works with matching results; examples menu, open, save, export text and link.

**Independent Test**: Run `cargo test --test compat` and `--test inventory`; both pass for all 373 examples and all 153 elements.

### Tests for User Story 2 ⚠️

- [ ] T055 [P] [US2] Compat test loading every `assets/circuits/*.txt`, checking load (G1) and round trip (G2) in `tests/compat_load.rs` (ignored until the compat-pass task of this phase)
- [ ] T056 [P] [US2] Compat test comparing simulation output of each example with `tests/compat/golden/*.json` within the documented tolerance, using the exception rule from FR-008 (nondeterministic and real-time circuits are listed in `tests/compat/nondeterministic.json` and checked by statistical or property-based criteria such as amplitude, mean and spectrum within bounds) in `tests/compat_golden.rs` (ignored until the compat-pass task of this phase)
- [ ] T057 [P] [US2] Test for the example catalog parser against `assets/setuplist.txt` in `tests/catalog.rs` (ignored until the catalog parser task of this phase)
- [ ] T058 [US2] Generate golden files for all examples with `src/bin/golden.rs` into `tests/compat/golden/` (requires T012); while generating, classify each example as deterministic or nondeterministic (random sources, audio, wall-clock) and write the latter to `tests/compat/nondeterministic.json`

### Implementation for User Story 2 (element batches, each batch checked against `tests/inventory.rs`)

- [ ] T059 [P] [US2] Port diodes: `DiodeElm`, `ZenerElm`, `LEDElm`, `TunnelDiodeElm`, `VaractorElm`, `SparkGapElm`, `DiacElm`, plus the diode model from `JAVA/Diode.java` and `DiodeModel.java` in `src/elements/diodes.rs` — PARTIAL: Diac pending
- [ ] T060 [P] [US2] Port transistors: `TransistorElm`, `NTransistorElm`, `PTransistorElm`, `DarlingtonElm`, `NDarlingtonElm`, `PDarlingtonElm`, `TriodeElm`, `UnijunctionElm`, `SCRElm`, `TriacElm` in `src/elements/transistors.rs` — PARTIAL: Darlington, Triode, Unijunction, SCR, Triac pending
- [X] T061 [P] [US2] Port MOSFETs and JFETs: `MosfetElm`, `NMosfetElm`, `PMosfetElm`, `JfetElm`, `NJfetElm` and `MosfetModel` in `src/elements/mosfets.rs` — done
- [ ] T062 [P] [US2] Port op-amps and analog blocks: `OpAmpElm`, `OpAmpRealElm`, `OpAmpSwapElm`, `NortonAmpElm`, `ComparatorElm`, `OTAElm`, `CC2Elm`, `GyratorElm`, `TimerElm`, `VCOElm`, `PhaseCompElm` in `src/elements/analog.rs` — PARTIAL: op-amp, CC2, 555, VCO, phase comparator, ADC/DAC done; real op-amp, Norton, comparator, OTA, gyrator pending
- [ ] T063 [P] [US2] Port controlled sources and transformers: `VCVSElm`, `VCCSElm`, `CCVSElm`, `CCCSElm`, `TransformerElm`, `TappedTransformerElm`, `CustomTransformerElm`, `TransLineElm`, `MemristorElm`, `PotElm`, `ThermistorNTCElm`, `LDRElm`, `FuseElm`, `LampElm`, `CrystalElm` in `src/elements/magnetic.rs` and `src/elements/misc_analog.rs` — PARTIAL: transformer, transmission line, memristor, pot, lamp done (`magnetic.rs`, `misc.rs`, `pot.rs`); controlled sources (need T069), tapped/custom transformer, thermistor, LDR, fuse, crystal pending
- [ ] T064 [P] [US2] Port relays and analog switches: `RelayElm`, `RelayCoilElm`, `RelayContactElm`, `TimeDelayRelayElm`, `AnalogSwitchElm`, `AnalogSwitch2Elm`, `AnalogMuxElm`, `DPDTSwitchElm`, `MBBSwitchElm`, `CrossSwitchElm`, `MotorProtectionSwitchElm`, `TriStateElm` in `src/elements/relays.rs` — PARTIAL: analog switches SPST/SPDT done (`misc.rs`); relays, analog mux, DPDT/MBB/cross/motor-protection switches, tri-state pending
- [ ] T065 [P] [US2] Port logic gates and inputs/outputs: `GateElm`, `AndGateElm`, `NandGateElm`, `OrGateElm`, `NorGateElm`, `XorGateElm`, `XnorGateElm`, `InverterElm`, `SchmittElm`, `InvertingSchmittElm`, `LogicInputElm`, `LogicOutputElm`, `BusLogicInputElm`, `ClockElm`, `SquareRailElm`, `VarRailElm`, `SweepElm`, `NoiseElm`, `ACRailElm`, `AMElm`, `FMElm`, `ExtVoltageElm` in `src/elements/logic.rs` and `src/elements/waveforms.rs` — PARTIAL: gates, logic I/O, sweep, noise, bus logic input done; AM/FM and ext voltage pending
- [X] T066 [US2] Port the chip framework `ChipElm` (pins, bus pins, state) in `src/elements/chip.rs` — done: generic `ChipElm<L: ChipLogic>`
- [ ] T067 [P] [US2] Port digital chips: `DFlipFlopElm`, `JKFlipFlopElm`, `TFlipFlopElm`, `LatchElm`, `CounterElm`, `Counter2Elm`, `RingCounterElm`, `MonostableElm`, `SeqGenElm`, `SipoShiftElm`, `PisoShiftElm`, `HalfAdderElm`, `FullAdderElm`, `MultiplexerElm`, `DeMultiplexerElm`, `ADCElm`, `DACElm`, `DelayBufferElm`, `ROMElm`, `SRAMElm`, `SevenSegElm`, `SevenSegDecoderElm`, `LEDArrayElm`, `DecimalDisplayElm`, `InstructionDisplayElm`, `BusSplitterElm`, `BusTransceiverElm`, `OptocouplerElm`, `DataInputElm`, `DataRecorderElm` in `src/elements/digital.rs` and `src/elements/memory.rs` — PARTIAL: flip-flops, latch, counters, adders, mux/demux, monostable, ADC/DAC, SRAM/ROM (`memory.rs`), bus splitter, bus logic input and instruction display (`bus.rs`) done; seven-segment/LED array/decimal displays, shift registers, sequence generator, delay buffer, bus transceiver, optocoupler, data input/recorder pending
- [ ] T068 [P] [US2] Port motors and measurement/utility elements: `DCMotorElm`, `ThreePhaseMotorElm`, `AmmeterElm`, `OhmMeterElm`, `WattmeterElm`, `WattmeterTrueElm`, `ProbeElm`, `OutputElm`, `TestPointElm`, `AntennaElm`, `StopTriggerElm`, `LabeledNodeElm`, `TextElm`, `BoxElm`, `LineElm`, `GraphicElm`, `RoutedWireElm`, `ScopeElm` (embedded scope element) in `src/elements/measure.rs` and `src/elements/graphic.rs` — PARTIAL: routed wire (without auto-router) done; motors, meters, graphics, embedded scope pending
- [ ] T069 [US2] Port the expression evaluator `JAVA/Expr.java` in `src/expr/mod.rs` with tests, and `CustomLogicElm`/`CustomLogicModel` in `src/elements/custom_logic.rs` — PARTIAL: expression evaluator done in `src/expr/mod.rs` with tests; `CustomLogicElm`/`CustomLogicModel` pending
- [ ] T070 [US2] Port composite elements and models (`CompositeElm`, `CustomCompositeElm`, `CustomCompositeChipElm`, `CustomCompositeModel`) in `src/elements/composite.rs`
- [ ] T071 [US2] Port the relay, diode, transistor and MOSFET model dialogs' data (`RelayModel`, `EditDiodeModelDialog` logic) into `src/elements/models.rs`
- [ ] T072 [US2] Port element-specific per-step logic dependencies (chip states, `FindPathInfo` loop and current-source checks) into `src/solver/paths.rs`
- [ ] T073 [US2] Make `tests/inventory.rs` pass (T011) and fix any missing element reported
- [X] T074 [US2] Implement the example catalog parser for `assets/setuplist.txt` in `src/app/catalog.rs` — done in `src/app/examples.rs` (tested against the bundled list)
- [X] T075 [US2] Implement the "Circuit Examples" menu with nested submenus and the default entry in `src/app/menus/examples.rs` — done: examples menu built from setuplist.txt in the menu bar, plus New Blank Circuit
- [X] T076 [US2] Implement File menu: New, New Window, Open, Save, Save As, Import from Text, Export as Text, Export as Link, with native file dialogs, in `src/app/menus/file.rs` — done in `commands.rs`/`window.rs`/`dialogs.rs`; Create Subcircuit stays disabled until T117
- [X] T077 [US2] Implement the Draw menu with every element category generated from the registry (contract C2) and the "Arrastar" submenu plus the checked "Modo Edição" entry (Espaço ou Shift+arrastar) bound to the canvas mode toggle, in `src/app/menus/draw.rs` — done: generated from the registry, Drag submenu and Select mode; routed wire entry pending
- [ ] T078 [US2] Load Portuguese and other locales from `assets/locale/*.txt` with fallback to the English key in `src/app/i18n.rs` — PARTIAL: Portuguese loaded from `assets/locale/locale_pt.txt` at start-up with English fallback; locale selection pending
- [ ] T079 [US2] Handle loading errors: dialog listing line, type and message, with the rest of the circuit loaded in `src/app/dialogs/load_errors.rs`
- [ ] T080 [US2] Make `tests/compat_load.rs` and `tests/compat_golden.rs` pass for all 373 examples; record any per-circuit tolerance exception with justification in `tests/compat/tolerances.json`, and un-ignore the compat tests

**Checkpoint**: US1 and US2 both work; SC-001, SC-008, SC-010, SC-011 verifiable.

---

## Phase 5: User Story 3 - Observe signals with scopes (Priority: P2)

**Goal**: Waveform scopes with readouts, stack/combine/separate.

**Independent Test**: Run the RLC example; waveforms, peak values, time, step and frequency readouts appear and update smoothly.

### Tests for User Story 3 ⚠️

- [ ] T081 [P] [US3] Unit tests for scope sample buffers, min/max decimation and peak values in `src/scope/buffer.rs`
- [ ] T082 [P] [US3] Test scope serialization round trip (`o` lines) against the 7 auto-tests in `tests/scope_format.rs`
- [ ] T083 [P] [US3] Test stack, unstack, combine, separate operations in `tests/scope_layout.rs`
- [ ] T084 [P] [US3] Unit tests for FFT output on a known sine in `src/scope/fft.rs`

### Implementation for User Story 3

- [ ] T085 [P] [US3] Port `Scope`, `ScopePlot`, `ScopeDataIterator`, `ScopeManager` data model to `src/scope/mod.rs`, `src/scope/plot.rs`, `src/scope/manager.rs`
- [ ] T086 [P] [US3] Port `ScopeSerializer` and trigger logic to `src/scope/serialize.rs` and `src/scope/trigger.rs`
- [ ] T087 [P] [US3] Port `FFT` and `ScopeFFT` to `src/scope/fft.rs`
- [ ] T088 [US3] Include scope samples in `FrameSnapshot` and feed them from the solver in `src/solver/simulation.rs`
- [ ] T089 [US3] Render the scope panel on the GL canvas (waveforms, peak labels, component label, time, step and frequency readouts, gear icon) in `src/render/scope_view.rs`
- [ ] T090 [US3] Implement the scope panel widget, popup menu and properties dialog (port `ScopePopupMenu`, `ScopePropertiesDialog`) in `src/app/panels/scope.rs` and `src/app/dialogs/scope_props.rs`
- [ ] T091 [US3] Implement the Time Diagrams menu (Stack All, Unstack All, Combine All, Separate All) in `src/app/menus/scopes.rs` — PARTIAL: menu entries exist but are disabled until the scope panel (T090) exists

**Checkpoint**: US3 works independently; scope data feeds compat tests.

---

## Phase 6: User Story 4 - Full editing workflow (Priority: P2)

**Goal**: Undo/redo, clipboard, selection, rotate/mirror, find, center, zoom, pan.

**Independent Test**: Build a circuit and run every Edit-menu command, checking effect and shortcut.

### Tests for User Story 4 ⚠️

- [ ] T092 [P] [US4] Test undo/redo stack (circuit text snapshots) in `tests/undo.rs`
- [ ] T093 [P] [US4] Test clipboard copy, cut, paste, duplicate preserving element data in `tests/clipboard.rs`
- [ ] T094 [P] [US4] Test rotate and mirror transformations on selections, including posts and labels, in `tests/transform.rs`
- [ ] T095 [P] [US4] Test view transform: zoom to 100%, zoom in/out, center circuit, in `src/render/view.rs`

### Implementation for User Story 4

- [ ] T096 [US4] Implement undo/redo (port `JAVA/CommandManager.java`) in `src/app/editing/undo.rs`
- [ ] T097 [P] [US4] Implement selection, cut, copy, paste, duplicate, select all in `src/app/editing/selection.rs`
- [ ] T098 [P] [US4] Implement rotate CW/CCW and mirror X/Y in `src/app/editing/transform.rs`
- [ ] T099 [P] [US4] Implement view zoom, pan and center in `src/render/view.rs`
- [ ] T100 [US4] Implement Find Component dialog (port `SearchDialog`) in `src/app/dialogs/search.rs`
- [X] T101 [US4] Implement the Edit menu with shortcuts and disabled states (contracts C3, C4) in `src/app/menus/edit.rs` — done: all Edit entries with shortcuts and disabled states
- [ ] T102 [US4] Implement the shortcuts dialog and keyboard handling (port `KeyNames`, `ShortcutsDialog`) in `src/app/dialogs/shortcuts.rs`

**Checkpoint**: US4 works independently.

---

## Phase 7: User Story 5 - Options, export and subcircuits (Priority: P3)

**Goal**: Display options, exports, print, full screen, subcircuits, autosave, DC point.

**Independent Test**: Toggle each option and see the change; export image and SVG and open them.

### Tests for User Story 5 ⚠️

- [ ] T103 [P] [US5] Test that each display option changes rendering output (SVG snapshot tests) in `tests/options_render.rs`
- [ ] T104 [P] [US5] Test settings persistence across restarts in `tests/settings_persist.rs`
- [ ] T105 [P] [US5] Test SVG and PNG export of a known circuit in `tests/export.rs`
- [ ] T106 [P] [US5] Test autosave write and recovery in `tests/autosave.rs`
- [ ] T107 [P] [US5] Test DC operating point result on a known circuit in `tests/dc_point.rs`
- [ ] T108 [P] [US5] Test subcircuit creation from a selection and reuse in `tests/subcircuit.rs`

### Implementation for User Story 5

- [X] T109 [US5] Implement the Options menu toggles bound to settings and canvas (contract C1) in `src/app/menus/options.rs` — done: all toggles persisted; "Desvios" in the reference is the translation of "Shortcuts...", implemented as the shortcuts dialog; Other Options dialog covers time step, voltage range, auto time step, DC on reset
- [ ] T110 [P] [US5] Implement the "Offsets" and "Other options" dialogs (port `EditOptions`) in `src/app/dialogs/options.rs`
- [ ] T111 [P] [US5] Implement European resistor, IEC gate and white background rendering variants in `src/render/scene.rs`
- [ ] T112 [P] [US5] Implement image export (PNG) and clipboard image copy in `src/app/export/image.rs`
- [ ] T113 [P] [US5] Implement SVG export through the SVG painter in `src/app/export/svg.rs`
- [ ] T114 [P] [US5] Implement print support in `src/app/export/print.rs`
- [ ] T115 [P] [US5] Implement autosave and "Recover autosave" in `src/app/autosave.rs`
- [X] T116 [US5] Implement "Find DC operating point" in `src/solver/dc.rs` and `src/app/menus/file.rs` — done in `Session::find_dc` and the File menu
- [ ] T117 [US5] Implement Create Subcircuit and the subcircuit manager (port `SubcircuitDialog`, `EditCompositeModelDialog`) in `src/app/dialogs/subcircuit.rs`
- [ ] T118 [P] [US5] Implement Tools menu: convert wires to routed wires, subcircuit manager entry in `src/app/menus/tools.rs` — PARTIAL: menu entries exist but are disabled until routed wires and subcircuits exist
- [X] T119 [P] [US5] Implement full-screen toggle and About dialog in `src/app/menus/view.rs` — done: full screen and About
- [ ] T120 [P] [US5] Port `AudioInputElm` and `AudioOutputElm` using a native audio library (add dependency with justification in `research.md`) in `src/elements/audio.rs`

**Checkpoint**: All five stories functional.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T121 [P] Add criterion benchmarks (RLC, 500-element ladder, 1000+ element mixed circuit) in `benches/solver.rs` and record the original's baseline from `/home/jgardona/Bin/circuitjs1` in `tests/bench/baseline.json` (SC-004)
- [ ] T122 Optimize the solver step loop (symbolic factorization reuse, allocation removal) guided by T121 in `src/solver/step.rs`
- [ ] T123 [P] Add render benchmark mode (`--bench-render <file>`) reporting fps for SC-003 in `src/main.rs` and `src/app/bench.rs`
- [ ] T124 [P] Measure startup time and input latency; fix regressions (SC-005, SC-006) in `src/app/mod.rs`
- [ ] T125 [P] Edge-case tests: truncated file, unknown element, floating node, short circuit, non-convergence in `tests/edge_cases.rs` (SC-009)
- [ ] T126 [P] Add software-GL fallback notice when no hardware GL is available in `src/app/canvas/gl.rs`
- [ ] T127 [P] Verify HiDPI and fractional scaling rendering in `src/app/canvas/gl.rs`
- [ ] T128 [P] Add `///` docs to all public items and doctest examples for the engine API in `src/lib.rs` and `src/solver/mod.rs`
- [ ] T129 Audit and remove any `unwrap`/`expect`/`panic!` outside tests and `#[allow]` without comment (constitution I and IV) across `src/`
- [ ] T130 [P] Write the developer guide `docs/adding-a-component.md` with a worked example (add a new two-terminal element) and the removal procedure using family features (FR-027)
- [ ] T131 [P] Add a CI job in `.github/workflows/ci.yml` building and testing with each element family feature disabled one at a time (SC-013)
- [ ] T132 Run the full `quickstart.md` validation and update it for the single-package layout in `specs/001-circuit-simulator-migration/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (1)**: T001 first, then T002 and T003; T006 depends on T001 to T003.
- **Foundational (2)**: depends on Setup; blocks all stories. Inside it: format (T014 to T019) and element framework (T020 to T025) before solver (T026 to T031); tooling T008 to T013 can run in parallel with them.
- **US1 (3)**: depends on Phase 2. T044 (GL spike) gates T045 and T047; the icon layer T049 comes before the toolbar T050 and every menu task.
- **US2 (4)**: depends on Phase 2 and on T039 to T042 from US1 (basic elements); batches T059 to T068 are parallel; T070 depends on T066 and T069; T080 depends on T058 and all element batches.
- **US3 (5)**: depends on US1 (canvas, worker). Compat scope data also benefits from US2.
- **US4 (6)**: depends on US1 and uses file text from Phase 2.
- **US5 (7)**: depends on US1 and US2 (subcircuits depend on T070).
- **Polish (8)**: after the desired stories are complete.

### Within Each Story

- Tests first and failing, then elements and models, then solver integration, then UI.

### Parallel Opportunities

- T004, T005, T007 in Setup; T008, T009, T014, T015, T019, T024, T025, T032 to T034 in Foundational.
- All element-batch tasks T059 to T068 can run in parallel (different files).
- US3, US4 can proceed in parallel after US1.

### Parallel Example: User Story 2 element batches

```text
Task: "Port diodes in src/elements/diodes.rs"
Task: "Port transistors in src/elements/transistors.rs"
Task: "Port MOSFETs and JFETs in src/elements/mosfets.rs"
Task: "Port op-amps and analog blocks in src/elements/analog.rs"
Task: "Port logic gates in src/elements/logic.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 (including the licence setup T001) and Phase 2.
2. Phase 3: US1. Run the GL spike T044 early; if it fails, switch the backend behind the `Painter` trait.
3. Stop and validate with the RLC circuit before continuing.

### Incremental Delivery

1. Setup and Foundational, then US1 (MVP).
2. US2 brings full component and example compatibility (the largest piece, split by element family).
3. US3 and US4 in parallel, then US5, then Polish.

---

## Notes

- Port from the Java source file by file; each element's test is a golden comparison against the original first.
- Keep `unsafe` out of everything except `src/render/gl.rs`.
- Commit after each task or logical group.
