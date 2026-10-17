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
- [x] T012 Implement `src/bin/golden.rs` that drives `/home/jgardona/Bin/circuitjs1/circuitjs1` (Electron, remote debugging port) to load a circuit, run N steps and write `tests/compat/golden/<name>.json` with node voltages, element currents and scope samples; document the invocation in `tests/compat/README.md` **[DEFERRED by the user: the Electron build is not driven programmatically; reference data comes from the 7 recorded auto-tests (T013) and analytical oracles; revisit later]** — DEFERRED by the user (no Electron); the recorded auto-tests (T013, T035) replace it for the circuits they cover — UPDATE: tool written (`src/bin/golden.rs`, `tests/compat/tools/golden-preload.js`, `tests/compat/README.md`) working on a private copy of the Electron build, because the build rejects remote-debugging switches; NOT yet run: the Electron app does not start inside the assistant's sandbox, so the recordings must be generated on the user's machine. — AUDIT: the tool exists and was run on the user's machine against a private copy of the Electron build; 373 recordings are in `tests/compat/golden/`.
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

- [x] T055 [P] [US2] Compat test loading every `assets/circuits/*.txt`, checking load (G1) and round trip (G2) in `tests/compat_load.rs` (ignored until the compat-pass task of this phase) — AUDIT: `tests/compat_load.rs` checks load and round trip of every example, no longer ignored.
- [x] T056 [P] [US2] Compat test comparing simulation output of each example with `tests/compat/golden/*.json` within the documented tolerance, using the exception rule from FR-008 (nondeterministic and real-time circuits are listed in `tests/compat/nondeterministic.json` and checked by statistical or property-based criteria such as amplitude, mean and spectrum within bounds) in `tests/compat_golden.rs` (ignored until the compat-pass task of this phase) — UPDATE: `tests/compat_golden.rs` written (passes trivially until `tests/compat/golden/` has recordings). — AUDIT: `tests/compat_golden.rs` compares the 373 recordings, with the differences kept in `tests/compat/known_differences.json` and the 7 nondeterministic circuits in `tests/compat/nondeterministic.json`.
- [x] T057 [P] [US2] Test for the example catalog parser against `assets/setuplist.txt` in `tests/catalog.rs` (ignored until the catalog parser task of this phase) — AUDIT: the catalog parser is tested in `src/app/examples.rs` (unit tests, one of them reads `assets/setuplist.txt`), not in `tests/catalog.rs`.
- [x] T058 [US2] Generate golden files for all examples with `src/bin/golden.rs` into `tests/compat/golden/` (requires T012); while generating, classify each example as deterministic or nondeterministic (random sources, audio, wall-clock) and write the latter to `tests/compat/nondeterministic.json` — AUDIT: 373 recordings generated; nondeterministic circuits listed in `tests/compat/nondeterministic.json`.

### Implementation for User Story 2 (element batches, each batch checked against `tests/inventory.rs`)

- [x] T059 [P] [US2] Port diodes: `DiodeElm`, `ZenerElm`, `LEDElm`, `TunnelDiodeElm`, `VaractorElm`, `SparkGapElm`, `DiacElm`, plus the diode model from `JAVA/Diode.java` and `DiodeModel.java` in `src/elements/diodes.rs` — PARTIAL: Diac pending — UPDATE: diac (203) done in `thyristors.rs`. — AUDIT: every class of the family is registered (`tests/inventory.rs` passes).
- [x] T060 [P] [US2] Port transistors: `TransistorElm`, `NTransistorElm`, `PTransistorElm`, `DarlingtonElm`, `NDarlingtonElm`, `PDarlingtonElm`, `TriodeElm`, `UnijunctionElm`, `SCRElm`, `TriacElm` in `src/elements/transistors.rs` — PARTIAL: Darlington, Triode, Unijunction, SCR, Triac pending — UPDATE: Darlington (400), triode (173), unijunction (417), SCR (177) and triac (206) done (`composite.rs`, `thyristors.rs`). — AUDIT: every class of the family is registered (`tests/inventory.rs` passes).
- [X] T061 [P] [US2] Port MOSFETs and JFETs: `MosfetElm`, `NMosfetElm`, `PMosfetElm`, `JfetElm`, `NJfetElm` and `MosfetModel` in `src/elements/mosfets.rs` — done
- [x] T062 [P] [US2] Port op-amps and analog blocks: `OpAmpElm`, `OpAmpRealElm`, `OpAmpSwapElm`, `NortonAmpElm`, `ComparatorElm`, `OTAElm`, `CC2Elm`, `GyratorElm`, `TimerElm`, `VCOElm`, `PhaseCompElm` in `src/elements/analog.rs` — PARTIAL: op-amp, CC2, 555, VCO, phase comparator, ADC/DAC done; real op-amp, Norton, comparator, OTA, gyrator pending — UPDATE: Norton amp (`nor`), OTA (402) and gyrator done; comparator (401) and OpAmpReal still pending. — UPDATE 2: comparator (401, `composite.rs`) and real op-amp (409: LM741, LM324 old/fixed; `opamps.rs`) done, plus the optocoupler (407). Limitations: the old LM324 model does not converge as a follower with a rail-to-rail input (the original marks it as deprecated); the LED model is not selectable in the optocoupler dialog (T071). — AUDIT: every class is registered (`tests/inventory.rs`); known limitation: the old LM324 model does not converge as a follower with a rail-to-rail input (deprecated in the original too).
- [x] T063 [P] [US2] Port controlled sources and transformers: `VCVSElm`, `VCCSElm`, `CCVSElm`, `CCCSElm`, `TransformerElm`, `TappedTransformerElm`, `CustomTransformerElm`, `TransLineElm`, `MemristorElm`, `PotElm`, `ThermistorNTCElm`, `LDRElm`, `FuseElm`, `LampElm`, `CrystalElm` in `src/elements/magnetic.rs` and `src/elements/misc_analog.rs` — PARTIAL: transformer, transmission line, memristor, pot, lamp and the four controlled sources (`controlled.rs`, native style only) done; spice-style CCVS/CCCS, tapped/custom transformer, thermistor, LDR, fuse, crystal pending — UPDATE: tapped transformer (169) done (`magnetic.rs`); spice-style CCVS/CCCS now supported inside composites. — UPDATE 2: custom transformer (406, `custom_transformer.rs`), fuse, LDR and NTC (`parts.rs`) and the crystal done; the battery (XML only, `battery.rs`) added. — AUDIT: every class is registered (`tests/inventory.rs` passes).
- [x] T064 [P] [US2] Port relays and analog switches: `RelayElm`, `RelayCoilElm`, `RelayContactElm`, `TimeDelayRelayElm`, `AnalogSwitchElm`, `AnalogSwitch2Elm`, `AnalogMuxElm`, `DPDTSwitchElm`, `MBBSwitchElm`, `CrossSwitchElm`, `MotorProtectionSwitchElm`, `TriStateElm` in `src/elements/relays.rs` — PARTIAL: analog switches SPST/SPDT (`misc.rs`) and relays (`relays.rs`: all-in-one relay, coil and contact with label linking, relay models in `models.rs`) done; relay model dialogs (T071) reduced to editing the parameters in the element's dialog; time-delay relay, analog mux, DPDT/MBB/cross/motor-protection switches, tri-state pending — UPDATE: cross switch (430) done; DPDT, MBB, time-delay relay and analog mux still pending. — UPDATE 2: DPDT (429), MBB (416) and time-delay relay (414) done (`switches.rs`, `hybrid.rs`); analog mux (432) done in `hybrid.rs`. — AUDIT: every class is registered (`tests/inventory.rs` passes).
- [x] T065 [P] [US2] Port logic gates and inputs/outputs: `GateElm`, `AndGateElm`, `NandGateElm`, `OrGateElm`, `NorGateElm`, `XorGateElm`, `XnorGateElm`, `InverterElm`, `SchmittElm`, `InvertingSchmittElm`, `LogicInputElm`, `LogicOutputElm`, `BusLogicInputElm`, `ClockElm`, `SquareRailElm`, `VarRailElm`, `SweepElm`, `NoiseElm`, `ACRailElm`, `AMElm`, `FMElm`, `ExtVoltageElm` in `src/elements/logic.rs` and `src/elements/waveforms.rs` — PARTIAL: gates, logic I/O, sweep, noise, bus logic input done; AM/FM and ext voltage pending — AUDIT: every class is registered (`tests/inventory.rs` passes), AM, FM and external voltage included.
- [X] T066 [US2] Port the chip framework `ChipElm` (pins, bus pins, state) in `src/elements/chip.rs` — done: generic `ChipElm<L: ChipLogic>`
- [x] T067 [P] [US2] Port digital chips: `DFlipFlopElm`, `JKFlipFlopElm`, `TFlipFlopElm`, `LatchElm`, `CounterElm`, `Counter2Elm`, `RingCounterElm`, `MonostableElm`, `SeqGenElm`, `SipoShiftElm`, `PisoShiftElm`, `HalfAdderElm`, `FullAdderElm`, `MultiplexerElm`, `DeMultiplexerElm`, `ADCElm`, `DACElm`, `DelayBufferElm`, `ROMElm`, `SRAMElm`, `SevenSegElm`, `SevenSegDecoderElm`, `LEDArrayElm`, `DecimalDisplayElm`, `InstructionDisplayElm`, `BusSplitterElm`, `BusTransceiverElm`, `OptocouplerElm`, `DataInputElm`, `DataRecorderElm` in `src/elements/digital.rs` and `src/elements/memory.rs` — PARTIAL: flip-flops, latch, counters, adders, mux/demux, monostable, ADC/DAC, SRAM/ROM (`memory.rs`), bus splitter, bus logic input, instruction display (`bus.rs`) and the displays (seven/fourteen/sixteen segment display and decoder, decimal display, LED array: `displays.rs`) done; shift registers, sequence generator, delay buffer, bus transceiver, optocoupler, data input/recorder pending — AUDIT: every class is registered (`tests/inventory.rs` passes): shift registers, sequence generator, delay buffer, bus transceiver included.
- [x] T068 [P] [US2] Port motors and measurement/utility elements: `DCMotorElm`, `ThreePhaseMotorElm`, `AmmeterElm`, `OhmMeterElm`, `WattmeterElm`, `WattmeterTrueElm`, `ProbeElm`, `OutputElm`, `TestPointElm`, `AntennaElm`, `StopTriggerElm`, `LabeledNodeElm`, `TextElm`, `BoxElm`, `LineElm`, `GraphicElm`, `RoutedWireElm`, `ScopeElm` (embedded scope element) in `src/elements/measure.rs` and `src/elements/graphic.rs` — PARTIAL: routed wire (without auto-router) done; embedded scope (`scope_elm.rs`) and audio output (`audio.rs`; the side panel has a Play Audio button and a save-as-WAV button per output, played through `pw-play`, `paplay` or `aplay`, see `playback.rs`) done; motors, meters, graphics pending — UPDATE: ammeter (370), line (423), antenna (A), external voltage (418), 3-phase motor (427) and motor protection switch (428) done (`measure.rs`, `motors.rs`, `sources.rs`); DC motor, ohmmeter, wattmeters, probe/test point, stop trigger and the rest of the graphics pending. — UPDATE 2: DC motor (415, `motors.rs`), ohmmeter (216, `sources.rs`) and both wattmeters (`measure.rs`) done; still pending: test point, stop trigger, data recorder/input, and the other graphics. — UPDATE 3: test point (368), stop trigger (408) and data recorder (210) done in `measure.rs`; data input (424) and audio input (411) are rail kinds in `sources.rs` (`samples.rs` keeps the loaded files). Files are chosen with a file chooser button next to the path field in the edit dialog (data, audio and the recorder's export; typing the path still works); the recorder exports to that path. Pending: the other graphics. — AUDIT: every class is registered (`tests/inventory.rs` passes); text, box and line are the graphics (`GraphicElm` is only their abstract base).
- [x] T069 [US2] Port the expression evaluator `JAVA/Expr.java` in `src/expr/mod.rs` with tests, and `CustomLogicElm`/`CustomLogicModel` in `src/elements/custom_logic.rs` — PARTIAL: expression evaluator done in `src/expr/mod.rs` with tests; `CustomLogicElm`/`CustomLogicModel` pending — UPDATE: `CustomLogicElm`/`CustomLogicModel` done in `src/elements/custom_logic.rs` (rules, tri-state, text `!` and XML `clm` models; model fields editable in the element dialog, no separate model dialog). — AUDIT: done as described.
- [x] T070 [US2] Port composite elements and models (`CompositeElm`, `CustomCompositeElm`, `CustomCompositeChipElm`, `CustomCompositeModel`) in `src/elements/composite.rs` — PARTIAL: instances and models work (`src/elements/subcircuit.rs`, feature `subcircuits`, not `composite.rs`): `ccm` models (nested, bus pins, embedded in saved files), `cc` instances with node mapping, voltage sources, nonlinear children, state save/restore, flips, model choice and high-logic-voltage editing. Also in `src/elements/composite.rs`: shared `ChildNet`, Darlington pair (400) and OTA (402); embedded scope element (403) in `scope_elm.rs — AUDIT: done. The Edit Pin Layout, View Components and Edit Model buttons are in the edit dialog of an instance. Edit Model sets the circuit aside and opens the model's circuit (`src/app/contexts.rs`); the bar over the canvas has Back, Save and Save Copy; Save turns the canvas back into a model, keeps the pin layout and the place the model was kept in, and brings the circuit back with the instances updated. View Components shows the parts of the instance with the voltages of the running circuit, read-only (`Session::view_components`). Limits: the circuit set aside restarts when it comes back (the state of a running simulation is not kept), and parts inside an instance cannot be opened in turn from the canvas (the session supports the nesting, the dialog is not reachable there). Tests: `tests/model_context.rs`.
- [x] T071 [US2] Port the relay, diode, transistor and MOSFET model dialogs' data (`RelayModel`, `EditDiodeModelDialog` logic) into `src/elements/models.rs` — DONE: `src/elements/model_edit.rs` (`ModelEditor`: fields, applying, naming, storing) and the buttons "Create New (Simple/Advanced) Model" / "Edit Model" in the edit dialogs of diodes, transistors, MOSFETs/JFETs and relays; the optocoupler offers its LED model. Pending: composite and custom-logic model dialogs (T117).
- [x] T072 [US2] Port element-specific per-step logic dependencies (chip states, `FindPathInfo` loop and current-source checks) into `src/solver/paths.rs` — AUDIT: path search and validation live in `src/solver/path.rs` (`Validator::find_path`), used by chips, sources and the loop checks.
- [x] T073 [US2] Make `tests/inventory.rs` pass (T011) and fix any missing element reported — AUDIT: `tests/inventory.rs` passes with no ignored test. A class counts as ported when it is registered or its dump code is read; `GraphicElm` and `CustomCompositeChipElm` are listed there as not file elements, with the reason.
- [X] T074 [US2] Implement the example catalog parser for `assets/setuplist.txt` in `src/app/catalog.rs` — done in `src/app/examples.rs` (tested against the bundled list)
- [X] T075 [US2] Implement the "Circuit Examples" menu with nested submenus and the default entry in `src/app/menus/examples.rs` — done: examples menu built from setuplist.txt in the menu bar, plus New Blank Circuit
- [X] T076 [US2] Implement File menu: New, New Window, Open, Save, Save As, Import from Text, Export as Text, Export as Link, with native file dialogs, in `src/app/menus/file.rs` — done in `commands.rs`/`window.rs`/`dialogs.rs`; Create Subcircuit stays disabled until T117
- [X] T077 [US2] Implement the Draw menu with every element category generated from the registry (contract C2) and the "Arrastar" submenu plus the checked "Modo Edição" entry (Espaço ou Shift+arrastar) bound to the canvas mode toggle, in `src/app/menus/draw.rs` — done: generated from the registry, Drag submenu and Select mode; routed wire entry pending
- [x] T078 [US2] Load Portuguese and other locales from `assets/locale/*.txt` with fallback to the English key in `src/app/i18n.rs` — PARTIAL: Portuguese loaded from `assets/locale/locale_pt.txt` at start-up with English fallback; locale selection pending — UPDATE: language chosen in Options > Language (setting `language`, `auto` follows the system, 16 languages; applied at the next start; English falls back for missing strings). — AUDIT: done as described (16 languages, applied at the next start).
- [x] T079 [US2] Handle loading errors: dialog listing line, type and message, with the rest of the circuit loaded in `src/app/dialogs/load_errors.rs` — UPDATE: dialog listing line, type and message of every entry that could not be read, shown after opening, importing or recovering a circuit. — AUDIT: done as described.
- [x] T080 [US2] Make `tests/compat_load.rs` and `tests/compat_golden.rs` pass for all 373 examples; record any per-circuit tolerance exception with justification in `tests/compat/tolerances.json`, and un-ignore the compat tests — UPDATE: all 373 examples load and round-trip through save/load; the two strict compat_load tests are enabled. Golden comparison (tests/compat_golden.rs) still pending (T012). — AUDIT: all 373 examples load, round-trip and agree with the recordings; per-circuit exceptions are in `tests/compat/known_differences.json` (not `tolerances.json`).

**Checkpoint**: US1 and US2 both work; SC-001, SC-008, SC-010, SC-011 verifiable.

---

## Phase 5: User Story 3 - Observe signals with scopes (Priority: P2)

**Goal**: Waveform scopes with readouts, stack/combine/separate.

**Independent Test**: Run the RLC example; waveforms, peak values, time, step and frequency readouts appear and update smoothly.

### Tests for User Story 3 ⚠️

- [x] T081 [P] [US3] Unit tests for scope sample buffers, min/max decimation and peak values in `src/scope/buffer.rs` — AUDIT: tested in `tests/scopes.rs` (extremes of each sample, buffer wrap, valid count) rather than a `buffer.rs`.
- [x] T082 [P] [US3] Test scope serialization round trip (`o` lines) against the 7 auto-tests in `tests/scope_format.rs` — AUDIT: `tests/scopes.rs` round-trips the scopes of every example that has any (plots, flags, modes, speed, label), through the XML writer.
- [x] T083 [P] [US3] Test stack, unstack, combine, separate operations in `tests/scope_layout.rs` — UPDATE: stack/unstack/combine/separate implemented in the session (`edit.rs`) with a test in `tests/scopes.rs`; 1-2-5 full-scale series, pointer readout, X/Y and spectrum (`src/scope/fft.rs`) done. Trigger (free run/normal/auto, rising/falling, level, indicator), manual scale (per-plot scale and position, divisions) and a properties dialog now done too (`src/scope/mod.rs`, `render/scopes.rs`, `app/edit.rs`); context menu has Remove Plot, Reset, Export CSV, Undock/Dock. Measurements (peak, negative peak, peak to peak, RMS, average, duty cycle, frequency, scale, element info), AC coupling per voltage plot, plot selection under the pointer and vertical dragging in manual scale are done too. Later: cursor measurement by dragging in time (Δt and Δ), X/Y settings (axes, brightness and colour plots, trail persistence) and V-I mode, log spectrum, phase angle and spectrum pointer readout, automatic-mode zero placement (zero at the bottom until the signal goes negative), and the scope menu, properties dialog and CSV export for scopes placed in the circuit (editing one rewrites its settings and restarts its history, so Reset is not offered there). The modes now use the original's flag bits (64, 128, 1024, 65536, 1<<23) and `xy2*`/`tp` attributes, so files from the original keep them. Not verified by hand: the GTK menus and the mouse drag (only the drawing and the session logic have tests); X/Y trail is drawn from the recorded samples, not from a persistent image as the original does. — AUDIT: stack, unstack, combine and separate are tested in `tests/scopes.rs`, along with the later scope features.
- [x] T084 [P] [US3] Unit tests for FFT output on a known sine in `src/scope/fft.rs` — AUDIT: tests in `src/scope/fft.rs` (a known sine, decibels, phase angle).

### Implementation for User Story 3

- [x] T085 [P] [US3] Port `Scope`, `ScopePlot`, `ScopeDataIterator`, `ScopeManager` data model to `src/scope/mod.rs`, `src/scope/plot.rs`, `src/scope/manager.rs` — AUDIT: the data model is in `src/scope/mod.rs` (plots, scopes, triggers); the split into `plot.rs` and `manager.rs` was not made.
- [x] T086 [P] [US3] Port `ScopeSerializer` and trigger logic to `src/scope/serialize.rs` and `src/scope/trigger.rs` — AUDIT: file reading and writing and the trigger are in `src/scope/mod.rs`.
- [x] T087 [P] [US3] Port `FFT` and `ScopeFFT` to `src/scope/fft.rs` — AUDIT: `src/scope/fft.rs`, with the spectrum drawing in `src/render/scopes.rs`.
- [x] T088 [US3] Include scope samples in `FrameSnapshot` and feed them from the solver in `src/solver/simulation.rs` — AUDIT: scopes are recorded on the worker and their views come in every snapshot (`FrameSnapshot.scopes`, `.embedded`).
- [x] T089 [US3] Render the scope panel on the GL canvas (waveforms, peak labels, component label, time, step and frequency readouts, gear icon) in `src/render/scope_view.rs` — AUDIT: drawn by `src/render/scopes.rs`, on the Cairo area under the canvas (not on the GL canvas); the gear icon is not drawn, the properties open from the context menu and a double click on a scope element.
- [x] T090 [US3] Implement the scope panel widget, popup menu and properties dialog (port `ScopePopupMenu`, `ScopePropertiesDialog`) in `src/app/panels/scope.rs` and `src/app/dialogs/scope_props.rs` — AUDIT: popup menu and properties dialog in `src/app/window.rs` and `src/app/edit.rs`.
- [x] T091 [US3] Implement the Time Diagrams menu (Stack All, Unstack All, Combine All, Separate All) in `src/app/menus/scopes.rs` — PARTIAL: menu entries exist but are disabled until the scope panel (T090) exists — AUDIT: the Time Diagrams menu entries are enabled by `src/app/commands.rs`.

**Checkpoint**: US3 works independently; scope data feeds compat tests.

---

## Phase 6: User Story 4 - Full editing workflow (Priority: P2)

**Goal**: Undo/redo, clipboard, selection, rotate/mirror, find, center, zoom, pan.

**Independent Test**: Build a circuit and run every Edit-menu command, checking effect and shortcut.

### Tests for User Story 4 ⚠️

- [x] T092 [P] [US4] Test undo/redo stack (circuit text snapshots) in `tests/undo.rs` — AUDIT: tested in `src/app/edit.rs`.
- [x] T093 [P] [US4] Test clipboard copy, cut, paste, duplicate preserving element data in `tests/clipboard.rs` — AUDIT: tested in `src/app/edit.rs`.
- [x] T094 [P] [US4] Test rotate and mirror transformations on selections, including posts and labels, in `tests/transform.rs` — AUDIT: `tests/transform.rs`.
- [x] T095 [P] [US4] Test view transform: zoom to 100%, zoom in/out, center circuit, in `src/render/view.rs` — AUDIT: tested in `src/app/edit.rs` (zoom keeps the centre fixed and is clamped).

### Implementation for User Story 4

- [x] T096 [US4] Implement undo/redo (port `JAVA/CommandManager.java`) in `src/app/editing/undo.rs` — AUDIT: `Session::push_undo`, `undo`, `redo` in `src/app/edit.rs`.
- [x] T097 [P] [US4] Implement selection, cut, copy, paste, duplicate, select all in `src/app/editing/selection.rs` — AUDIT: in `src/app/edit.rs`.
- [x] T098 [P] [US4] Implement rotate CW/CCW and mirror X/Y in `src/app/editing/transform.rs` — AUDIT: `Session::transform` in `src/app/edit.rs`.
- [x] T099 [P] [US4] Implement view zoom, pan and center in `src/render/view.rs` — AUDIT: zoom, pan and center in `src/app/edit.rs` and `src/render/scene.rs`.
- [x] T100 [US4] Implement Find Component dialog (port `SearchDialog`) in `src/app/dialogs/search.rs` — AUDIT: `dialogs::find` in `src/app/dialogs.rs`.
- [X] T101 [US4] Implement the Edit menu with shortcuts and disabled states (contracts C3, C4) in `src/app/menus/edit.rs` — done: all Edit entries with shortcuts and disabled states
- [x] T102 [US4] Implement the shortcuts dialog and keyboard handling (port `KeyNames`, `ShortcutsDialog`) in `src/app/dialogs/shortcuts.rs` — AUDIT: `dialogs::shortcuts` and the window shortcut controller.

**Checkpoint**: US4 works independently.

---

## Phase 7: User Story 5 - Options, export and subcircuits (Priority: P3)

**Goal**: Display options, exports, print, full screen, subcircuits, autosave, DC point.

**Independent Test**: Toggle each option and see the change; export image and SVG and open them.

### Tests for User Story 5 ⚠️

- [x] T103 [P] [US5] Test that each display option changes rendering output (SVG snapshot tests) in `tests/options_render.rs` — AUDIT: `tests/render_svg.rs` checks that each option that shows in a static drawing changes it (show dots needs a running circuit and is not covered).
- [x] T104 [P] [US5] Test settings persistence across restarts in `tests/settings_persist.rs` — AUDIT: round trip of the settings, tolerating missing fields, in `src/app/settings.rs`.
- [x] T105 [P] [US5] Test SVG and PNG export of a known circuit in `tests/export.rs` — AUDIT: SVG and PNG exports are tested in `src/render/export.rs`.
- [x] T106 [P] [US5] Test autosave write and recovery in `tests/autosave.rs` — AUDIT: `tests/autosave.rs`.
- [x] T107 [P] [US5] Test DC operating point result on a known circuit in `tests/dc_point.rs` — AUDIT: `tests/dc_point.rs`; as in the original, only capacitors, batteries and alternating sources are treated specially in the operating point.
- [x] T108 [P] [US5] Test subcircuit creation from a selection and reuse in `tests/subcircuit.rs` — AUDIT: `tests/subcircuit_maker.rs`.

### Implementation for User Story 5

- [X] T109 [US5] Implement the Options menu toggles bound to settings and canvas (contract C1) in `src/app/menus/options.rs` — done: all toggles persisted; "Desvios" in the reference is the translation of "Shortcuts...", implemented as the shortcuts dialog; Other Options dialog covers time step, voltage range, auto time step, DC on reset
- [x] T110 [P] [US5] Implement the "Offsets" and "Other options" dialogs (port `EditOptions`) in `src/app/dialogs/options.rs` — AUDIT: the Other Options dialog is done; there is no separate Offsets dialog because the reference's "Desvios" is the shortcuts dialog (see T109).
- [x] T111 [P] [US5] Implement European resistor, IEC gate and white background rendering variants in `src/render/scene.rs` — AUDIT: European resistors, IEC gates and the white background are options of the drawing context.
- [x] T112 [P] [US5] Implement image export (PNG) and clipboard image copy in `src/app/export/image.rs` — AUDIT: PNG export and copy of the image to the clipboard (`src/render/export.rs`, `dialogs::copy_png`).
- [x] T113 [P] [US5] Implement SVG export through the SVG painter in `src/app/export/svg.rs` — AUDIT: SVG export through the SVG painter (`src/render/export.rs`).
- [x] T114 [P] [US5] Implement print support in `src/app/export/print.rs` — AUDIT: `dialogs::print`.
- [x] T115 [P] [US5] Implement autosave and "Recover autosave" in `src/app/autosave.rs` — AUDIT: `Session::autosave` and `recover_autosave`, tested in `tests/autosave.rs`.
- [X] T116 [US5] Implement "Find DC operating point" in `src/solver/dc.rs` and `src/app/menus/file.rs` — done in `Session::find_dc` and the File menu
- [x] T117 [US5] Implement Create Subcircuit and the subcircuit manager (port `SubcircuitDialog`, `EditCompositeModelDialog`) in `src/app/dialogs/subcircuit.rs` — DONE in `src/app/subcircuits.rs` and `dialogs.rs`: model made from the selection (labeled nodes are the pins), pin layout dialog as a form (side and place per pin, size, name, label, scope this circuit / session / saved), the manager (place, edit pin layout, delete). No drag-the-pins canvas and no editing of the circuit inside an existing model.
- [ ] T118 [P] [US5] Implement Tools menu: convert wires to routed wires, subcircuit manager entry in `src/app/menus/tools.rs` — PARTIAL: the subcircuit manager entry works; "convert wires" stays disabled (needs the wire router).
- [X] T119 [P] [US5] Implement full-screen toggle and About dialog in `src/app/menus/view.rs` — done: full screen and About
- [ ] T120 [P] [US5] Port `AudioInputElm` and `AudioOutputElm` using a native audio library (add dependency with justification in `research.md`) in `src/elements/audio.rs` — UPDATE: `AudioInputElm` (411) done as a rail that plays a WAV file (PCM/float, first channel); other formats are still pending. AUDIT: playback of the output is done through the system's command line players (`pw-play`, `paplay`, `aplay`; `src/app/playback.rs`) and not a native audio library, so this stays open only for that choice and for WAV formats other than PCM and float.

**Checkpoint**: All five stories functional.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [x] T121 [P] Add criterion benchmarks (RLC, 500-element ladder, 1000+ element mixed circuit) in `benches/solver.rs` and record the original's baseline from `/home/jgardona/Bin/circuitjs1` in `tests/bench/baseline.json` (SC-004) — PARTIAL: benchmarks, circuits (`bin/bench_circuits`) and the tool that measures the original (`golden --bench`) done; this program measures 7.5M / 109k / 10k steps per second on the three circuits; `baseline.json` recorded: the original runs 43.7k / 18.5k / 657 steps per second on the three circuits, so this program is 171x / 5.9x / 15x faster (SC-004 met)
- [ ] T122 Optimize the solver step loop (symbolic factorization reuse, allocation removal) guided by T121 in `src/solver/step.rs` — AUDIT: not needed for now: the benchmarks (T121) show room to spare against SC-003 and SC-004
- [x] T123 [P] Add render benchmark mode (`--bench-render <file>`) reporting fps for SC-003 in `src/main.rs` and `src/app/bench.rs`
- [x] T124 [P] Measure startup time and input latency; fix regressions (SC-005, SC-006) in `src/app/mod.rs`
- [x] T125 [P] Edge-case tests: truncated file, unknown element, floating node, short circuit, non-convergence in `tests/edge_cases.rs` (SC-009)
- [ ] T126 [P] Add software-GL fallback notice when no hardware GL is available in `src/app/canvas/gl.rs`
- [ ] T127 [P] Verify HiDPI and fractional scaling rendering in `src/app/canvas/gl.rs`
- [ ] T128 [P] Add `///` docs to all public items and doctest examples for the engine API in `src/lib.rs` and `src/solver/mod.rs`
- [x] T129 Audit and remove any `unwrap`/`expect`/`panic!` outside tests and `#[allow]` without comment (constitution I and IV) across `src/` — AUDIT: production code has no `unwrap`, `expect` or `panic!` (the `unwrap_used`, `expect_used` and `panic` lints are `warn` in `Cargo.toml`, `clippy.toml` exempts tests only, and `cargo clippy --lib --bins` is clean with and without `gui`; a probe unwrap made the lint fire). `unreachable!` and `panic!` occur only in tests. The seven `#[allow(clippy::too_many_arguments)]` were redundant with the project-wide allow (explained in `Cargo.toml`) and were removed; `#[allow(unreachable_code)]` was replaced by two `cfg` functions; the one `#![allow(unsafe_code)]` (in `render/gl.rs`, the only `unsafe` of the program) now carries a comment. Beyond the lints, fuzzing found and fixed a panic in `parse_edit_value` (a typed `€a` cut a character in two) and three stack overflows that aborted the process: deeply nested XML, deeply nested expressions and a subcircuit model that holds itself (now refused after 16 levels or 50 000 parts). Tests: `tests/edge_cases.rs` (millions of generated inputs; `FUZZ_SEED` and `FUZZ_ROUNDS` repeat or lengthen them). Not covered by the lints: slice indexing in the numeric code (`indexing_slicing` is not enabled: it would flag thousands of lines of the faithful port) and arithmetic overflow, which the debug build checks and the fuzz tests exercised.
- [ ] T130 [P] Write the developer guide `docs/adding-a-component.md` with a worked example (add a new two-terminal element) and the removal procedure using family features (FR-027)
- [x] T131 [P] Add a CI job in `.github/workflows/ci.yml` building and testing with each element family feature disabled one at a time (SC-013) — DONE: the `families` job in `.github/workflows/ci.yml` leaves out each of the twelve families in turn and runs `cargo clippy --lib --bins -D warnings` and `cargo test` (the interface, `gui`, is not part of it). To make that pass: the app code that depends on one family is now behind its feature (`app::subcircuits` and `app::contexts` with `subcircuits`, the audio buttons with `outputs`, the relays only with `switches` and `reactive`, the logic input only with `switches`, the crystal's values only with `reactive` and `passive`); the look-inside-an-instance code moved to `app/inspect.rs`, which does not depend on any family. About 450 tests that use a family's elements now carry `#[cfg(feature = "family")]` for each family they need (added by listing the tests that failed with each family out, then run again: every one of the twelve configurations passes, 301 to 439 tests each, against 448 with everything on). A circuit that uses a missing family loads with an `UnknownType` error naming the element and the rest of it loads (`tests/modularity.rs`, built without `passive`). Limits: the interface build with a family left out is not covered (`window.rs` and `dialogs.rs` still refer to the subcircuit code and the audio buttons unconditionally), so `gui` needs every family; the tests that load every example (`compat_load`, `compat_golden`, `robustness` and the like) run only when all families are on, instead of skipping just the circuits that use the missing family.
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
