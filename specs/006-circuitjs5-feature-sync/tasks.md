---

description: "Task list for CircuitJS 5.x Feature Sync"
---

# Tasks: CircuitJS 5.x Feature Sync

**Input**: Design documents from `specs/006-circuitjs5-feature-sync/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Required. The constitution (principle II, non-negotiable) mandates test-first work. Within each story the test tasks come first and must fail before the matching implementation task.

**Red-test rule** (same as spec 001): a failing test is written first locally and committed in the same change as the implementation that makes it pass. A test whose implementation lands in a later task is committed with `#[ignore = "<reason>, enabled by T0xx"]`, and that later task removes the ignore.

**Reference paths**:
- `UP/` is the upstream clone `~/Workspace/circuitjs1` at commit `8ac55f22` (CircuitJS 5.0.4). TypeScript sources are in `UP/ts/`.
- `JAVA@d6b80f46` means the Java baseline. Read it with `git -C ~/Workspace/circuitjs1 show d6b80f46:src/com/lushprojects/circuitjs1/client/<File>.java`.
- Spec inventory ids (N1 to N20, C1 to C8) refer to the tables in [spec.md](spec.md).

**Documentation rule** (constitution V, spec User Story 7): every story phase ends with its documentation tasks. A story is not done until they are. Documentation keeps its language: Portuguese for the user guide, README, architecture and example pages, English for `docs/adding-a-component.md`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 to US7 from spec.md

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Reference data and inventory checks that every story relies on.

- [X] T001 Add a section "5.x reference strategy" to `tests/compat/README.md` (research R-1): no CircuitJS 5.0.4 build; existing recordings stay the reference; rewritten `cs-*` examples are compared with the recordings of their old versions; new elements are compared with hand-calculated values in `tests/compat/circuits-5x/expected.json`; building 5.0.4 (`npm ci && npm run build` in `UP/ts`, then `golden --app`) is a fallback only for a nonlinear disagreement
- [X] T002 Before T003 replaces them, save the 12 old circuits as `tests/compat/circuits-5x/old/cs-*.txt` (from `assets/circuits/`) and confirm that each `tests/compat/golden/cs-*.json` was recorded from that old version (its element list matches). List the mapping old circuit → recording → new circuit, and the nodes compared (the nodes of elements present in both versions, matched by position), in `tests/compat/circuits-5x/README.md`
- [X] T003 Copy the 12 upstream example files `UP/ts/public/circuits/cs-*.txt` over `assets/circuits/cs-*.txt`, and update `src/bin/import_assets.rs` so its source paths point at the 5.x layout (`ts/public/circuits`, `ts/public/locale`) instead of `src/com/lushprojects/circuitjs1/public` (constant `PUBLIC` at line 13, `TOOLBAR` at line 14)
- [X] T004 [P] Create `tests/compat/circuits-5x/` with 5.x-format circuits, written by hand following [contracts/file-format.md](contracts/file-format.md) and the upstream `cs-*.txt` files (or saved from the online version at falstad.com), one per form: `cv-vnode.txt` (`cv` with `2*v(in)`), `ci-didt.txt` (`ci` with `didt(ia)` and labeled ammeter `ia`), `vccs-vref.txt` (VCCS using `v(x)`), `sub-scoped-ref.txt` (two instances of a subcircuit containing label `x` and `v(x)`), `unknown-ref.txt` (`v(nothere)`), `not-current-meter.txt` (`i(x)` where `x` is a labeled node), `mono-invert.txt`, `text-color.txt`, `stop-trigger-load.txt` (condition true at t=0), `scope-ac.txt` (1 V sine on 5 V offset with three stacked scopes); document each file in `tests/compat/circuits-5x/README.md`
- [X] T005 Write `tests/compat/circuits-5x/expected.json` with the hand-calculated values for each circuit from T004 (node voltages or source currents at given times), and the derivation of each in `tests/compat/circuits-5x/README.md`. Keep the circuits linear, or use a known closed form (for example `dvdt` of a ramp), so the answers are exact (depends on T004)
- [X] T006 [P] Resolve research R-14: compare prometheuscs with `UP/ts` for JFET pin names on highlight (`UP/ts/JfetElm.ts`), NPN/PNP arrow highlighting (`UP/ts/TransistorElm.ts`), demultiplexer invertible outputs (`UP/ts/DeMultiplexerElm.ts`), and average-voltage mode on probes and test points (`UP/ts/ProbeElm.ts`, `UP/ts/TestPointElm.ts`); record the result in a "R-14 outcome" paragraph at the end of `specs/006-circuitjs5-feature-sync/research.md`, and add a task to the US2 phase of this file for each one found missing

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared expression-reference engine and the solver pass that US1, US2 and US5 all need (research R-2 to R-4).

**⚠️ CRITICAL**: US1, US2 and US5 cannot start until this phase is complete. US3, US4 and US6 do not depend on it and may start after Phase 1.

### Tests (write first, must fail)

- [X] T007 [P] Unit tests in `src/expr/mod.rs` (`#[cfg(test)]`) for the grammar in [contracts/expression-references.md](contracts/expression-references.md): `v(a)`, `i(a)`, `dvdt(a)`, `didt(a)` each produce one reference with the right `current`/`derivative` flags; `i` not followed by `(` is input 9; bare `didt` keeps its old meaning; bare `dvdt` is a parse error; the same name twice gives two slots; port the matching cases from `UP/ts/Expr.test.ts` (lines around 88 and 155)
- [X] T008 [P] Unit tests in `src/expr/refs.rs` (`#[cfg(test)]`) for the reference engine: input values for voltage and current references; `dvdt`/`didt` read `(x − last x) / timestep`; numeric derivative uses step `max(|change|, 1e-6)` and clamps the derivative magnitude to at least 1e-6; convergence limit 0.001 under 10 sub-iterations, 0.01 under 200, then 0.1, and ten times tighter for current inputs (port from `UP/ts/ExprEngine.ts` `evalAndStamp`, `checkConvergence`)
- [X] T009 [P] (done as integration tests in `tests/controlled.rs`, which need real elements: `a_reference_pulls_the_named_node_into_the_sources_matrix`, `a_labeled_node_wins_over_a_meter_with_the_same_name`, `unresolved_references_read_zero_and_say_why`) Unit tests in `src/solver/sim.rs` (`#[cfg(test)]`): an element with one voltage reference to a labeled node in an otherwise separate circuit ends up in the same matrix as that node; an unresolved reference leaves both slots on ground; resolution prefers a labeled node over a meter with the same name

### Implementation

- [X] T010 Extend the parser in `src/expr/mod.rs`: add a reference node kind, collect references in parse order as `{name, current, derivative}` (data-model "Expression reference"), add `ref_values` and `last_ref_values` to `ExprState`, and roll them in `update_last_values` (makes T007 pass)
- [X] T011 Create `src/expr/refs.rs` with the shared reference engine ported from `UP/ts/ExprEngine.ts`: fields `pin_inputs`, `refs`, `ref_values`, `last_inputs`; methods `ref_node_count() = 2 * refs.len()`, `input_value(i)`, `eval_and_stamp(negate, stamper, target)` with the stamping hooks `stamp_input_derivative(pos, neg, dx)` and `stamp_input_derivative_vs(vs, dx)`, `check_convergence(base_limit)`, `save_inputs()`, `step_finished(output)`, `reset()`, `unresolved_info()` returning `unknown name: v(<name>)` / `unknown name: i(<name>)` / `not a current meter: <name>`; register the module in `src/expr/mod.rs`; `///` docs on all public items (makes T008 pass)
- [X] T012 Extend the `Element` trait in `src/elements/element.rs` with defaulted methods: `expr_ref_name(&self) -> Option<&str>` (None by default), `ref_node_count(&self) -> usize` (0), `ref_node_base(&self) -> usize` (= posts + internal nodes), `resolve_refs(&mut self, targets: &RefTargets)` (no-op), where `RefTargets` gives labeled-node lookup and, for each named element, its two nodes and its voltage source; make `node_count()` include `ref_node_count()`
- [X] T013 Add the resolve pass to `src/solver/sim.rs`: in `analyze`, after `make_node_list` and before `calculate_closures`, build `RefTargets` from the `LabelRegistry` and from elements whose `expr_ref_name()` is `Some`, call `resolve_refs` on every element with `ref_node_count() > 0`, and add each resolved slot to that node's `links` so the closure flood joins the matrices; current references register their meter's `VsId` through the existing spice-current path (`spice_pairs`/`set_spice_source`/`set_spice_current`) (makes T009 pass)
- [X] T014 Measure the effect of T013 on simulation speed: run `cargo bench --bench solver` and `cargo run --release --bin bench_circuits` on master and on this branch; record both in `specs/006-circuitjs5-feature-sync/research.md` under "Performance baseline" (SC-007: at most 5% slower)

**Checkpoint**: references parse, resolve and pull nodes into the right matrix; no element uses them yet.

---

## Phase 3: User Story 1 - Open and run circuits made with CircuitJS 5.x (Priority: P1) 🎯 MVP

**Goal**: 5.0.4 files with `cv`/`ci`, meter labels, `v()`/`i()`/`dvdt()`/`didt()` and scoped subcircuit references load, simulate like upstream and round-trip (N1 to N5 loading side, N20).

**Independent Test**: `cargo test --test compat_5x` passes: every circuit in `tests/compat/circuits-5x/` matches `expected.json`, and every new `cs-*` example matches its old recording. `cargo test --test format_roundtrip` passes with the new attributes.

### Tests for User Story 1 (write first, must fail)

- [X] T015 [P] [US1] Create `tests/compat_5x.rs`, modeled on `tests/compat_golden.rs`, with two checks using the spec-001 tolerance: (a) every circuit in `tests/compat/circuits-5x/` matches `tests/compat/circuits-5x/expected.json`; (b) every new `assets/circuits/cs-*.txt` matches the recording of its old version in `tests/compat/golden/cs-*.json` on the nodes listed by T002. Make `tests/compat_golden.rs` skip the 12 `cs-*` entries, since `compat_5x.rs` now owns them, so the old test doesn't fail on the changed circuits (spec SC-001)
- [X] T016 [P] [US1] Add round-trip cases to `tests/format_roundtrip.rs` for each row of [contracts/file-format.md](contracts/file-format.md): `cv` and `ci` with `ex`; `lb` on voltmeter and ammeter (written only when non-empty); `rn` on a subcircuit child; plus a proptest strategy generating `ex` strings that contain reference functions
- [X] T017 [P] [US1] Add integration tests to `tests/controlled.rs`: `2*v(in)` gives twice the node voltage (spec US1 scenario 1); `didt(ia)` follows the ammeter's current derivative (scenario 2); two instances of a subcircuit each use their own `x` (scenario 3); `v(nothere)` loads, reads 0 and reports `unknown name: v(nothere)` in the info lines (scenario 4); `i(x)` on a labeled node reports `not a current meter: x`
- [X] T018 [P] [US1] Add regression tests to `tests/compat_load.rs` for the N20 file fixes that apply: gyrator resistance and width survive save and load; relay contacts load from XML; a transformer never writes saturation as `undefined`; editing a voltage source resets its phase; a time-delay relay contact conducts in a circuit split into several matrices (compare with `UP` commits `c06aa8e3`, `13d1a0e9`, `6409df16`, `66b3979b`; drop a case and note why in the test if prometheuscs never had the bug)

### Implementation for User Story 1

- [X] T019 [US1] Move VCVS, VCCS, CCVS and CCCS in `src/elements/controlled.rs` onto the shared engine from `src/expr/refs.rs` (`pin_inputs` = their input-pin count; the CCVS/CCCS current inputs keep their spice-style path); existing tests in `tests/controlled.rs` and the golden examples must still pass
- [X] T020 [US1] Add the two-terminal controlled sources to `src/elements/controlled.rs`, ported from `UP/ts/ControlledSourceElm.ts`, `ControlledVoltageElm.ts`, `ControlledCurrentElm.ts`: ids `ControlledVoltageElm` (XML tag `cv`, default `ex="v(in)"`, edit label "Output Voltage", one voltage source) and `ControlledCurrentElm` (tag `ci`, default `ex=".001*v(in)"`, label "Output Current"); 2 posts; nonlinear; no legacy dump code; diamond drawing with `diamondSize = 18`; info lines `<label> = <expr>`, `V =`, `I =` plus `unresolved_info()`; register both under "Active Building Blocks" in `register()`
- [X] T021 [P] [US1] Add an optional `label` to the ammeter in `src/elements/measure.rs`: edit field "Label" (text, empty allowed), XML `lb` written only when non-empty, label drawn next to the meter, `expr_ref_name()` returns it when non-empty (port from `UP/ts/AmmeterElm.ts:84-91,285`)
- [X] T022 [P] [US1] Same for the voltmeter/probe `ProbeElm` in `src/elements/outputs.rs` (port from `UP/ts/ProbeElm.ts:119-128,437`)
- [X] T023 [US1] Subcircuit references in `src/elements/subcircuit.rs` and `src/elements/composite.rs`: when a model is built, write each child's resolved reference slots as model node numbers in `rn` (`UP/ts/GetCircuitAsSubcircuit.ts:167`); when an instance expands, map `rn` to instance nodes, and for `i()` find the sibling whose voltage source spans the two nodes (`UP/ts/ExprEngine.ts` `resolveRefElmsByNode`); names never resolve across subcircuit boundaries (makes T017 scenario 3 pass)
- [X] T024 [US1] Make the N20 fixes found in T018, in the element files the tests point at (`src/elements/hybrid.rs` for the gyrator, `src/elements/relays.rs`, `src/elements/magnetic.rs`, `src/elements/sources.rs`), one regression test per fix
- [X] T025 [US1] Check that Export As Text and Export As Link (`src/format/writer.rs`, `src/format/link.rs`) carry `cv`, `ci`, `lb` and `rn` (prometheuscs saves XML only, so the spec's legacy-format edge case reduces to these two exports); add a test for each to `tests/format_roundtrip.rs`
- [X] T026 [US1] Remove the `#[ignore]`s from T015 to T018 and make `cargo test` pass; update `tests/compat/known_differences.json` only for differences explained in a comment

### Documentation for User Story 1

- [X] T027 [P] [US1] `docs/arquitetura.md`: add a subsection to "3.1 Do arquivo à simulação" on the resolve pass and the reference node slots; add "4.6 Referências por nome em expressões" describing `src/expr/refs.rs`, the two-node slots, current references through the voltage-source row, and `rn` in subcircuits
- [X] T028 [P] [US1] `docs/adding-a-component.md`: add a section "Exposing a reference name" explaining `expr_ref_name()` and the `lb` convention, with the ammeter as the example
- [X] T029 [P] [US1] `docs/guia-do-usuario.md` section "8. Arquivos, exemplos e compartilhamento": state that circuits from CircuitJS 5.0.4 open and save compatibly; section "16. Diferenças em relação ao CircuitJS1": name CircuitJS 5.0.4 as the reference version
- [X] T030 [P] [US1] `README.md` sections "De onde vem" and "O que é diferente": reference CircuitJS 5.0.4 (TypeScript) instead of the Java edition, keeping the attribution lines

**Checkpoint**: every 5.0.4 file in the reference set opens and matches upstream. MVP shippable.

---

## Phase 4: User Story 2 - Build circuits with the new elements and expressions (Priority: P1)

**Goal**: Users can create the new sources and labels from the UI, use the updated `cs-*` examples, and get the small element additions N9 to N11 and C2.

**Independent Test**: build a CCVS with a labeled ammeter from scratch and compare with a hand calculation; every scenario in spec US2 passes; `cargo run --bin exdocs -- check` reports nothing.

### Tests for User Story 2 (write first, must fail)

- [X] T031 [P] [US2] Extend `tests/menu_elements.rs`: "Add Controlled Voltage Source" and "Add Controlled Current Source" appear under "Active Building Blocks", create with the defaults from [contracts/file-format.md](contracts/file-format.md), and expose the edit fields listed in [contracts/ui-commands.md](contracts/ui-commands.md)
- [X] T032 [P] [US2] Add to `tests/controlled.rs`: editing an expression to `2*(` keeps the previous expression and returns `Parse error in expression: 2*(: <detail>` (spec US2 scenario 3); editing an expression to add a reference changes the node count and triggers re-analysis
- [X] T033 [P] [US2] Add to `tests/more_elements.rs`: monostable with flag bit 2 (`FLAG_INVERT_TRIGGER = 2`, `UP/ts/MonostableElm.ts:30`) fires on a falling trigger edge; text element `co` color round-trips; Stop Trigger does not stop on load or Reset when its condition is already true, using `tests/compat/circuits-5x/stop-trigger-load.txt`
- [X] T034 [P] [US2] Add to `tests/more_elements.rs`: SRAM "Load Contents From File" replaces contents from a file of raw bytes; a file of 128,000 bytes or more is refused with "Cannot load: That file is too large!" and leaves contents unchanged (spec edge case; corrected during implementation: the original reads raw bytes, not numbers)

### Implementation for User Story 2

- [X] T035 [P] [US2] (not needed: the Draw menu shows no icons, and upstream 5.x gives these sources no toolbar button) Add the menu icons for `cv` and `ci` (diamond with + / − and diamond with arrow) to `assets/` and wire them through `src/app/icons.rs`; the icon test `every_toolbar_part_exists_and_has_an_icon` in `src/app/toolbar.rs` must pass
- [X] T036 [US2] Expression editing in `src/elements/controlled.rs`: the expression field disallows sliders, and a parse error keeps the old expression and sets the error text from [contracts/expression-references.md](contracts/expression-references.md); after a successful edit the session re-analyzes because `ref_node_count()` may change (`src/app/edit.rs` apply-edit path)
- [X] T037 [P] [US2] Monostable invert trigger in `src/elements/digital.rs`: checkbox "Invert Trigger" bound to flag bit 2, trigger on the falling edge when set (port `UP/ts/MonostableElm.ts`; JAVA@d6b80f46 has no such option)
- [X] T038 [P] [US2] (already present in prometheuscs; covered by the T033 test) Text color in `src/elements/outputs.rs` (`TextElm`): color edit field, XML `co` written only when set, drawing uses it (port `UP/ts/TextElm.ts:85-95`)
- [X] T039 [P] [US2] Stop Trigger arming in `src/elements/measure.rs`: clear the triggered/armed state on load and in `reset()` so it only fires once the condition is met during the run (port the fix from `UP` commit `29cb42cc`)
- [X] T040 [P] [US2] SRAM "Load Contents From File" in `src/elements/memory.rs`: an edit field with a file chooser (`EditInfo::open_file`) at index 4, "Restore Contents on Reset" moved to 5 as in the original; the file is read as raw bytes (one value per byte, big-endian pairs above 8 data bits, upstream `SRAMElm.fetchLoadFileData`), refused at 128,000 bytes or more, contents unchanged on error
- [X] T041 [US2] Localize new strings: add "Add Controlled Voltage Source", "Add Controlled Current Source", "Output Voltage", "Output Current", "Label", "Invert Trigger", "Load Contents From File", "Parse error in expression", "unknown name", "not a current meter" to every `assets/locale/locale_*.txt`, taking translations from `UP/ts/public/locale/` where present; `src/locale.rs` tests must pass
- [X] T042 [US2] Remove the `#[ignore]`s from T031 to T034 and make `cargo test` pass

### Documentation for User Story 2

- [X] T043 [US2] (nothing to change: the tool reads every element through the registry; it lists `cv`, `ci` and the meter labels as they are) Teach `src/bin/exdocs.rs` the new tags `cv` and `ci` and the meter `lb` label, so `exdocs facts` lists them; extend `tests/example_docs.rs` if it checks tag coverage
- [X] T044 [US2] Regenerate the controlled-source example pages: run `cargo run --bin exdocs -- sync`, then rewrite the hand-written text of `docs/exemplos/cs-currentadder.md`, `cs-diff.md`, `cs-fullrectifier.md`, `cs-opamp.md`, `cs-ramp.md`, `cs-resistor.md`, `cs-varicap.md`, `cs-varinduct.md`, `cs-varyresistor.md` to describe the two-terminal sources and the labels they reference; review `cs-integrator.md`, `cs-multiplier.md`, `cs-opamprail.md` for wording; `cargo run --bin exdocs -- check` must report nothing
- [X] T045 [P] [US2] `docs/guia-do-usuario.md` section "11. Componentes" / "Componentes especiais": a subsection on controlled sources with `v()`, `i()`, `dvdt()`, `didt()` (table from [contracts/expression-references.md](contracts/expression-references.md)), meter labels, and the error messages; mention Invert Trigger (monostable), text color, and SRAM "Load Contents From File" in their component entries

**Checkpoint**: US1 + US2 deliver the full P1 scope.

---

## Phase 5: User Story 3 - Faster editing workflow (Priority: P2)

**Goal**: Find Component as a palette, properties side panel, toolbar drag, wire splitting on place and move, exact-step wheel with type flip, Swap Terminals (N6 to N8, C1, C3 to C6).

**Independent Test**: every scenario in spec US3 passes; the reference task (2-stage RC filter + transistor buffer) needs at least 30% fewer clicks than in 1.0.8 (SC-004).

### Tests for User Story 3 (write first, must fail)

- [X] T046 [P] [US3] Unit tests in `src/app/edit.rs` for `split_wires_at_posts`: a new element's post on a wire's interior splits it into two wires joined at the post; a post exactly on a wire end does not create a zero-length wire; the same after moving an element; one undo restores the original wire and position
- [X] T047 [P] [US3] Unit tests in `src/app/edit.rs` for the wheel: one notch on 1 kΩ gives 1.2 kΩ and back gives 1 kΩ (E12); one notch on a non-R/C/L value moves along the 1-2-5 sequence (1 → 2 → 5 → 10); NPN toggles to PNP and back; N-channel MOSFET and JFET toggle channel; a slider element moves one step
- [X] T048 [P] [US3] Unit tests for the palette filter in `src/app/palette.rs`: case-insensitive substring over localized names; group order Recent, Components, Commands, Elements in this circuit ([contracts/ui-commands.md](contracts/ui-commands.md)); recent list keeps at most 10, newest first, without duplicates; no match returns an empty list
- [X] T049 [P] [US3] Add to `tests/actions.rs`: Swap Terminals exchanges the two posts of a two-terminal element and undo restores them; the "quiet on create" flag suppresses the editor for wires, logic inputs, ground and labeled nodes regardless of settings
- [X] T050 [P] [US3] Add to `tests/actions.rs` (session level, no GTK): with `show_properties_on_click`, selecting element A then B makes the edit target follow the selection; with `show_properties_on_create = false`, placing a resistor opens no editor

### Implementation for User Story 3

- [X] T051 [US3] Add `split_wires_at_posts(element)` to `src/app/wires.rs` and call it from placement and from the end of a move in `src/app/edit.rs`, inside the same undo step (research R-9; covers C4 and N7)
- [X] T052 [US3] Rewrite `Session::wheel_at` in `src/app/edit.rs` (line ~2132): keep E12 for R/C/L; use a 1-2-5 step table for other values; exactly one step per notch (stop using `wheel_sensitivity` for value steps; it keeps controlling zoom in `src/app/window.rs:1354`, and the Other Options control in `src/app/dialogs.rs:1074` is relabeled "Mouse Wheel Zoom Sensitivity", spec FR-013); polarity toggle for `TransistorElm`/MOSFET/JFET families (port `UP/ts/TypeScrollPopup.ts`); slider elements move one slider step (port `UP/ts/ScrollValuePopup.ts`)
- [X] T053 [US3] Show the wheel value popup in `src/app/window.rs`: a `gtk::Popover` near the pointer showing the formatted new value, hidden after about one second or on the next pointer move
- [X] T054 [P] [US3] Add `Cmd::SwapTerminals` (id `swap-terminals`) to `src/app/commands.rs`, in the element context menu and the Edit menu (`src/app/menus.rs`), calling `Base::flip_posts` through an undoable session action (port `JAVA@d6b80f46` `CirSim.java` "flip" command)
- [X] T055 [P] [US3] Add `quiet_on_create()` to `ElementDescriptor` in `src/elements/registry.rs` and set it on the elements upstream skips in commit `6737154f` (at least wires, routed wires, logic inputs, ground, labeled nodes, text, box)
- [X] T056 [US3] Add settings `show_properties_on_create` (default true) and `show_properties_on_click` (default false) to `src/app/settings.rs`, and checkboxes "Show Properties on Create" / "Show Properties on Click" to Other Options in `src/app/dialogs.rs` (`other_options`)
- [X] T057 [US3] Extract the body of `dialogs::edit_element_with` in `src/app/dialogs.rs` into a reusable `PropertyEditor` widget; keep the dialog as one host of it (research R-7)
- [X] T058 [US3] Host `PropertyEditor` in the right-hand side panel in `src/app/window.rs` (the box built around line 1399): it follows the selection, applies each committed field through the session edit path with one undo step per field, and is used when "Show Properties on Click" is on or after placement when "Show Properties on Create" is on and the element is not `quiet_on_create`
- [X] T059 [US3] Create `src/app/palette.rs` with the palette model (groups, filter, recent list) from T048, persisted as `palette_recent` in `src/app/settings.rs` (at most 10 ids, newest first); register the module in `src/app/mod.rs`
- [X] T060 [US3] Rebuild `dialogs::find` in `src/app/dialogs.rs` on the palette model: grouped list, Enter runs the highlighted entry (place a component, run a command, or select an element), picks are recorded as recent
- [X] T061 [US3] Open the palette with a double Shift press within 400 ms in `src/app/window.rs` key handling, ignored while a text entry has focus; `/`, the toolbar button and the Edit menu keep working
- [X] T062 [US3] Toolbar drag in `src/app/toolbar.rs`: add a `gtk::DragSource` to each part button carrying the element id; add a `gtk::DropTarget` on the canvas in `src/app/window.rs` that places the element at the drop point through the normal placement path (then T051 splitting and T058 properties apply)
- [X] T063 [US3] Localize "Swap Terminals", "Show Properties on Create", "Show Properties on Click", "Mouse Wheel Zoom Sensitivity" and the palette group titles in `assets/locale/locale_*.txt`
- [X] T064 [US3] Remove the `#[ignore]`s from T046 to T050, make `cargo test` pass, then count the clicks for the reference task in 1.0.8 and on this branch and record both in `specs/006-circuitjs5-feature-sync/research.md` under "SC-004 measurement"

### Documentation for User Story 3

- [X] T065 [US3] `docs/guia-do-usuario.md` section "4. Montar o circuito": properties side panel and its two options ("Editar valores"), dragging parts from the toolbar and automatic wire splitting ("Colocar componentes", "Fios", "Mover e esticar"), wheel steps and polarity flip ("Editar valores"), Swap Terminals ("Menu do botão direito"); section "10. Opções": the two new options; section "13. Atalhos de teclado e mouse": double Shift, palette groups and recent list

**Checkpoint**: US3 works with or without US1/US2.

---

## Phase 6: User Story 4 - Better scopes (Priority: P2)

**Goal**: scope Export Image (PNG/SVG, transparent), AC coupling refinements, stacked labels/legend, sweep hold, drag guards, X/Y role labels (N12 to N15, C8).

**Independent Test**: every scenario in spec US4 passes on `tests/compat/circuits-5x/scope-ac.txt`.

### Tests for User Story 4 (write first, must fail)

- [X] T066 [P] [US4] Add to `tests/render_svg.rs` and `tests/render_cairo.rs`: exporting one scope at 640×240 produces a file of that size; with transparent background the SVG has no background rect and the PNG's corner pixel has alpha 0; width or height of 0 or above 8192 is rejected with an error
- [X] T067 [P] [US4] (the faithful port reaches ±2% of the amplitude in the 6th period, not the 1st; see research.md "FR-016 and upstream's AC-coupling filter") Unit tests in `src/scope/mod.rs`: with AC coupling on and auto scale, a 1 V sine on 5 V settles to a mean of 0 ± 2% within one period; Reset clears the filter state; AC coupling is allowed in auto-scale mode (port behavior from `UP` commit `a5c29008`)
- [X] T068 [P] [US4] Unit tests in `src/render/scopes.rs` for stacked layout: label and legend are drawn when the scope height leaves room; a label equal to the scope above is not repeated; combined plots are named in on-screen order; plot ids are shown when several plots are combined (`UP` commits `7ec858d6`, `31814a1b`, `8ee04b18`, `324b217f`)
- [X] T069 [P] [US4] Unit tests in `src/scope/mod.rs`: a triggered scope keeps the last complete sweep for drawing until the next one completes (`UP` commit `6c479b07`)

### Implementation for User Story 4

- [X] T070 [US4] Scope export in `src/render/export.rs`: render one scope through the painter into the cairo PNG backend and the SVG backend with width, height and transparent options; reject sizes outside 1 to 8192 (research R-11)
- [X] T071 [US4] (the scope menu has no command ids, like Export CSV; the entry is on docked scopes, which `render::export::scope_png`/`scope_svg` index) Export Image dialog in `src/app/dialogs.rs` (format PNG/SVG, width, height, "Transparent Background") and a scope-menu entry `scope-export-image` in `src/app/window.rs` (`show_scope_menu`)
- [X] T072 [P] [US4] AC coupling refinements in `src/scope/mod.rs`: fast start, filter reset on Reset, allowed in auto scale
- [X] T073 [P] [US4] ("Show Scale" part already present: the auto-scale corner lists only the plotted units and the time scale is always written) Stacked-scope labels and legend in `src/render/scopes.rs` per T068; "Show Scale" in auto-scale scopes shows only the plotted quantity (`UP` commit `732031c2`)
- [X] T074 [US4] Sweep hold in `src/scope/mod.rs` and `src/render/scopes.rs`: keep a buffer of the last complete triggered sweep and draw it until the new one completes
- [X] T075 [US4] (holds by construction: the docked scopes are their own drawing area with their own gestures, so GTK keeps a drag that starts there away from the canvas; there is no canvas/scope splitter, the strip's height is automatic) Drag guards in `src/app/window.rs`: a drag that starts inside a scope never moves or selects a circuit element; dragging the scope splitter never selects a plot (`UP` commits `3d620ab7`, `19974216`)
- [X] T076 [US4] (already present: `Session::plot_roles`/`apply_scope_roles` and `Base::scope_role`, tested in `tests/scopes.rs`) X/Y role labels in `src/render/scopes.rs` and `src/app/edit.rs`: while the pointer is over an X/Y scope, draw "X" or "Y" next to the probes that feed it (`UP` cxx commit `d873cd7c` for the logic)
- [X] T077 [US4] Localize "Export Image...", "Transparent Background", "Width", "Height" in `assets/locale/locale_*.txt`; remove the `#[ignore]`s from T066 to T069 and make `cargo test` pass

### Documentation for User Story 4

- [X] T078 [US4] `docs/guia-do-usuario.md` section "6. Osciloscópios": Export Image ("Menu do osciloscópio"), AC coupling in auto scale and Reset ("Propriedades"), stacked labels and legends ("Organização dos osciloscópios"), triggered sweep hold and X/Y role labels ("O que se vê num osciloscópio")

---

## Phase 7: User Story 5 - Manage subcircuits (Priority: P3)

**Goal**: local / global / persistent models, previews, multi-select, refuse deleting used models, save library, Load Subcircuits Only (N5 manager side, C7).

**Independent Test**: create two subcircuits, make one persistent, restart, find it under Draw > Subcircuits; save a library, delete both, load it with "Load Subcircuits Only"; the circuit is unchanged.

**Depends on**: Phase 2 (references inside models are written as `rn`).

### Tests for User Story 5 (write first, must fail)

- [X] T079 [P] [US5] Add to `tests/format_roundtrip.rs`: a library file with root `<cir … sublib="1">` round-trips its models; loading it adds models without changing the current circuit's elements
- [X] T080 [P] [US5] (written as the integration test `tests/subcircuit_library.rs`, which points `XDG_CONFIG_HOME` at a folder of its own like `tests/autosave.rs`, so persistent models never touch the user's real library) Unit tests in `src/app/subcircuits.rs`: scope transitions Local → Global → Persistent and back never change model contents; deleting a model used in the circuit is refused with an error naming it; persistent models are written to and read from the user library file

### Implementation for User Story 5

- [X] T081 [US5] (the scopes already existed as `ModelScope::{Circuit, Session, Saved}`, and persistent models as one file each in `subcircuits/` in the config folder, read at start-up by `load_saved`; added `subcircuit_scope`/`set_subcircuit_scope`) Model scope in `src/app/subcircuits.rs` (`Local`, `Global`, `Persistent`, data-model "Subcircuit model") and the user library path next to the settings file in `src/app/settings.rs`; load persistent models at start-up
- [X] T082 [US5] (in `src/app/subcircuits.rs`: `library_text`, `is_subcircuit_library`, `load_subcircuits_only`; Open on a library only adds its models) `sublib` files in `src/elements/subcircuit.rs` and `src/format/parser.rs`: detect `sublib="1"` on the root, write selected models with it (`UP/ts/SubcircuitModel.ts:310`, `UP/ts/CircuitLoader.ts:101`)
- [X] T083 [US5] File menu "Load Subcircuits Only..." (`subcircuits-load-only`) in `src/app/commands.rs` and `src/app/menus.rs`
- [X] T084 [US5] (scope selector as three buttons, like upstream's Make Local/Global/Persistent; preview = the model's chip drawn through the cairo painter) Rework `dialogs::subcircuit_manager` in `src/app/dialogs.rs` (line ~843): a preview of each model (drawn through the painter), multi-select, scope selector, Delete refusing used models, "Save Library..." (`subcircuits-save-library`)
- [X] T085 [US5] Make sure Draw > Subcircuits in `src/app/menus.rs` lists local, global and persistent models and refreshes when the manager changes them; localize the new strings; remove the `#[ignore]`s from T079 and T080

### Documentation for User Story 5

- [X] T086 [US5] `docs/guia-do-usuario.md` section "9. Subcircuitos": model scopes, previews and multi-select in the manager, libraries (save, Load Subcircuits Only), where persistent models are stored; section "15. Linha de comando e arquivos do programa": the user library file location

---

## Phase 8: User Story 6 - Fewer surprises (Priority: P3)

**Goal**: unsaved prompt only on content changes, persistent error warning, labeled-node hover, dialog polish (N16 to N19).

**Independent Test**: every scenario in spec US6 passes.

### Tests for User Story 6 (write first, must fail)

- [X] T087 [P] [US6] Add to `tests/actions.rs`: after loading an example, moving the speed, current-speed or brightness control, zooming, panning or rearranging scopes leaves `Session::dirty` false; placing, deleting, moving or editing an element sets it
- [X] T088 [P] [US6] (the solver gets past every small degenerate circuit tried, so the stop is recorded the way the step loop records a singular matrix) Add to `tests/info_box.rs`: hovering a wire connected to a labeled node shows the node's label; a singular-matrix stop keeps its message in the session until the circuit is edited or reset

### Implementation for User Story 6

- [X] T089 [US6] (the controls, zoom and pan were already clean; scope arrangement, a scope's time scale and a trace's vertical position no longer mark the circuit unsaved; adding or removing scopes and their properties still do) Audit every `dirty = true` in `src/app/edit.rs`, `src/app/wires.rs`, `src/app/file.rs`, `src/app/subcircuits.rs` and the slider handlers in `src/app/window.rs`; keep it only for circuit-content changes (research R-12)
- [X] T090 [P] [US6] (the session already kept the message until an edit or reset; it was the banner's Dismiss that cleared it: the message is now drawn on the canvas by `render::scene::draw_stop_message`, and the banner only carries notices) Persistent warning in `src/app/window.rs` and `src/app/session.rs`: show `stop_message` as an on-canvas banner until the circuit changes or is reset
- [X] T091 [P] [US6] (done in `Session::info_lines` in `src/app/edit.rs`: `inspect.rs` is about looking inside subcircuits) Labeled-node hover in `src/app/inspect.rs`: when the hovered element is a wire, look up a labeled node on its closure and add its label to the info lines
- [X] T092 [P] [US6] (checkboxes already used `with_label`, and every dialog is a modal, titled `adw::Dialog`; added `set_focus_on_click(false)` to Reset and Run/Stop) Dialog polish in `src/app/dialogs.rs`: every checkbox has a clickable label (`gtk::CheckButton::with_label` or a mnemonic label); the model-editing and Other Options dialogs are modal with proper titles; Reset and Run/Stop buttons in `src/app/window.rs` do not take keyboard focus (`set_focus_on_click(false)`)
- [X] T093 [US6] Remove the `#[ignore]`s from T087 and T088 and make `cargo test` pass

### Documentation for User Story 6

- [X] T094 [US6] `docs/guia-do-usuario.md` section "5. Executar a simulação" / "Interpretar os erros": the persistent warning; section "14. Quando algo dá errado": when the unsaved-changes prompt appears; mention labeled-node hover under "4. Montar o circuito" / "Fios"

---

## Phase 9: User Story 7 - Documentation matches the program (Priority: P1, cross-cutting)

**Goal**: final consistency pass over all documentation after the stories that shipped (FR-028 to FR-031, SC-008). Per-story documentation is done inside each phase above. This phase only closes gaps.

**Independent Test**: for every row of spec tables A and B marked delivered, its menu or element name is found in `docs/guia-do-usuario.md`; `exdocs check` is clean.

- [X] T095 [US7] Walk spec tables A and B; for each delivered row search `docs/guia-do-usuario.md` for its menu or element name and fix any gap; for each deferred row add it to section "16. Diferenças em relação ao CircuitJS1" with the reason
- [X] T096 [P] [US7] (both tables list only the numbered sections and the feature added only subsections, so neither changed; `docs/arquitetura.md` section 6.4 and 7 gained the new app and export pieces) Update the table of contents ("Sumário") of `docs/guia-do-usuario.md` and `docs/arquitetura.md` for every heading added in this feature
- [X] T097 [P] [US7] Run `cargo run --bin exdocs -- check` and `cargo test --test example_docs`; fix any problem in `docs/exemplos/`
- [X] T098 [P] [US7] (no change: the screenshots show circuits, scopes and the panel's controls, none of which changed; the properties appear only after a click and the palette is not pictured) Update the screenshots referenced in `README.md` section "Imagens" if the side panel or palette changed what they show

---

## Phase 10: Polish & Cross-Cutting Concerns

- [X] T099 [P] (a targeted merge instead of `import_assets`, which would overwrite the curated translations and the examples: upstream translates none of the 27 strings this feature added that a locale file lacks; de, es, fr, it and pt have all of them) Refresh all locale files from `UP/ts/public/locale/` for strings added in this feature that upstream already translates, with `src/bin/import_assets.rs` (updated in T003); `src/locale.rs` tests must pass
- [X] T100 Re-run the performance measurement from T014 on the finished branch and append the numbers to `research.md` "Performance baseline" (SC-007)
- [X] T101 (results recorded in `research.md` "Quickstart validation" for the pull request; the by-hand GUI steps were not run) Run the full [quickstart.md](quickstart.md) validation (sections 1 to 8) and record the results in the pull request description
- [X] T102 Update the spec inventory in `specs/006-circuitjs5-feature-sync/spec.md`: mark each row of tables A and B delivered or deferred with a reason (SC-003)
- [X] T103 Bump the version in `Cargo.toml` (minor version: new elements and file-format additions) and update `Cargo.lock`

---

## Dependencies & Execution Order

### Phase dependencies

- **Phase 1 (Setup)**: no dependencies. T002 runs before T003 (it saves the old `cs-*` circuits first). T005 needs T004.
- **Phase 2 (Foundational)**: needs Phase 1 only for reference data in later tests. It blocks US1, US2 and US5.
- **US1 (Phase 3)**: needs Phase 2.
- **US2 (Phase 4)**: needs Phase 2. Shares `src/elements/controlled.rs` with US1, so T036 comes after T020.
- **US3 (Phase 5)** and **US4 (Phase 6)**: need Phase 1 only. They can run in parallel with US1/US2.
- **US5 (Phase 7)**: needs Phase 2, and T023 from US1 (`rn`).
- **US6 (Phase 8)**: no story dependency.
- **US7 (Phase 9)**: after the stories that are being shipped.
- **Polish (Phase 10)**: last.

### Story completion order (graph)

```text
Setup ─┬─ Foundational ─┬─ US1 ── US2
       │                └─ (US1 T023) ── US5
       ├─ US3
       ├─ US4
       └─ US6
all shipped stories ── US7 ── Polish
```

### Within each story

Tests (marked to fail) → element/engine changes → session/app wiring → UI → localization → remove `#[ignore]` → documentation.

---

## Parallel Execution Examples

- **Phase 1**: T001, T002, T004 and T006 together; then T003 (after T002) and T005 (after T004).
- **Phase 2**: tests T007, T008 and T009 together; then T010 → T011 → T012 → T013 in order.
- **US1**: tests T015 to T018 together; T021 and T022 together after T020; docs T027 to T030 together.
- **US2**: tests T031 to T034 together; T035, T037, T038, T039 and T040 together (different files).
- **US3**: tests T046 to T050 together; T054 and T055 together; then T051 to T062 mostly in order (they share `edit.rs` and `window.rs`).
- **US4**: tests T066 to T069 together; T072 and T073 together.
- **US6**: T090, T091 and T092 together.
- **Across stories**: one developer on US1/US2 (solver and elements), another on US3/US4 (app and render) after Phase 1.

---

## Implementation Strategy

### MVP first

1. Phase 1 (reference data) → Phase 2 (reference engine) → Phase 3 (US1).
2. **Stop and validate**: `cargo test --test compat_5x` passes, so prometheuscs opens CircuitJS 5.0.4 files. Ship this (with its docs, T027 to T030).

### Incremental delivery

3. US2 completes the P1 scope (creating the new elements, updated `cs-*` examples and pages).
4. US3 and US4 (P2) in either order, or in parallel.
5. US5 and US6 (P3).
6. US7 final pass and Polish.

Each increment ends with fmt, clippy `-D warnings` and `cargo test` green, plus its documentation tasks done.
