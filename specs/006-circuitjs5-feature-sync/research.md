# Research: CircuitJS 5.x Feature Sync

Phase 0 output for [plan.md](plan.md). Each entry records a decision, why, and what else was considered. Upstream paths are relative to `~/Workspace/circuitjs1` (release 5.0.4, `8ac55f22`). prometheuscs paths are relative to the repository root.

## R-1 Reference data for 5.x behavior

- **Decision** (revised by spec clarification 2026-11-02): **No CircuitJS 5.0.4 build.**
  - The 373 existing recordings in `tests/compat/golden/` (made with the Java build) stay the numeric reference.
  - The 12 rewritten `cs-*` examples are compared with the recordings of their **old** versions. Upstream only redrew them with the two-terminal sources, so the node voltages must agree. Any difference is explained in `tests/compat/known_differences.json`.
  - The new elements and reference functions are checked against exact hand calculations, using small linear circuits in `tests/compat/circuits-5x/` whose expected values are listed in `expected.json`.
  - File compatibility is checked with save-and-reload tests.
- **Rationale**: This avoids a Node/npm/Electron toolchain for the tests and needs no new recording run. Each check has an exact or already-recorded answer.
- **Alternatives considered**: Building 5.0.4 and recording with `src/bin/golden.rs` (its `--app` option works because 5.x keeps the `window.CircuitJS1` API, `ts/JSInterface.ts:64`). Kept as a **fallback** only if a nonlinear case, such as `dvdt`/`didt` over many steps, disagrees with expectations.

## R-2 Name references in expressions (`v()`, `i()`, `dvdt()`, `didt()`)

- **Decision**: Port upstream's `ExprEngine` design (`ts/ExprEngine.ts`):
  - The parser collects references as a list `refs[k] = {name, current}`, separate from the pin inputs `a`..`i`. `dvdt(x)`/`didt(x)` reuse the same slot and read `(value − last value) / timestep`. Bare `didt` and the 9th input `i` keep their meaning (`i` is a reference only when followed directly by `(`).
  - Each reference adds **two extra node slots** to the owning element, after its posts and internal nodes. During analysis, after the node list is built and before matrix closures are computed, a new pass resolves each name: a labeled node first (voltage references only), then the first element whose reference name matches (a labeled voltmeter or ammeter). The slots get that node or that meter's two nodes; unresolved slots stay at ground.
  - Because the slots are real nodes of the owner, the existing closure flood (`Simulation::calculate_closures`) pulls the referenced node into the owner's matrix. That makes it legal to stamp the derivative against it, which is exactly why upstream did it this way.
  - A current reference stamps against the meter's voltage-source row. prometheuscs already has this mechanism for spice-style current-controlled sources (`Element::spice_pairs`, `set_spice_source`, `set_spice_current`), so i() reuses it.
- **Rationale**: Same numerical behavior as upstream (numeric Jacobian with `dv` floor 1e-6, same convergence limits). This is needed to match the golden data, and the solver architecture (one matrix per closure, voltage sources assigned to matrices) is the same as upstream's.
- **Alternatives considered**: Read referenced voltages from the previous time step without stamping (simpler, but changes convergence and fails the 5.x comparison). A global "probe table" in the simulator (needs new solver concepts; the node-slot approach reuses existing ones).

## R-3 Which elements get name references

- **Decision**: The new two-terminal sources (`cv`, `ci`) and the existing VCCS family (upstream `VCCSElm.ts` also owns an `ExprEngine`, so 5.x lets VCVS/VCCS/CCVS/CCCS expressions use `v(name)` too). Reference targets: labeled nodes, `ProbeElm` (voltmeter) and `AmmeterElm` with a non-empty label (`lb` attribute; upstream `getExprRefName`).
- **Rationale**: Mirrors upstream exactly, so any 5.x file loads with the same meaning.
- **Alternatives considered**: Only the new sources (would break 5.x files that use `v()` inside a VCCS).

## R-4 References inside subcircuits

- **Decision**: When a subcircuit model is built, resolve each child's references to model node numbers and store them on the child as the `rn` attribute (upstream `GetCircuitAsSubcircuit.ts:167`). When an instance is expanded, an `i()` reference finds its meter as the sibling whose voltage source spans the two recorded nodes (upstream `resolveRefElmsByNode`). Names resolve only inside the subcircuit that contains the expression.
- **Rationale**: Gives each subcircuit copy its own references (spec User Story 1, scenario 3) and keeps the model file format identical to upstream's.
- **Alternatives considered**: Re-resolving by name inside each instance (labels are not unique after flattening; upstream rejected this too).

## R-5 Subcircuit element (5.x) vs custom composite (4.x)

- **Decision**: No new element. Upstream's `SubcircuitElm` uses the same XML tag `cc` and the same attributes (`nm`, `f`, `sx`, `sy`, `bcs`, `hv`, `mo`, pins `nm`/`nd`/`ps`/`sd`/`bw`/`bz`) that `src/elements/subcircuit.rs` already reads and writes. The deltas are: the `rn` child attribute (R-4), library files whose root carries `sublib`, and model scopes (local / global / persistent).
- **Rationale**: Format comparison showed the rename was internal to the TypeScript code. The spec's N5 shrinks to the manager features and `rn`/`sublib`.
- **Alternatives considered**: A parallel 5.x element (duplicate code, no format reason).

## R-6 Persistent subcircuits and library files

- **Decision**: Persistent models are stored as one XML library file in the user data directory next to the existing settings (`app/settings.rs` location), loaded at start-up. "Save library" and "Load Subcircuits Only" read and write the same `sublib` XML format as upstream, so libraries are interchangeable.
- **Rationale**: Upstream keeps persistent models in browser `localStorage`; the desktop equivalent is a file in the user data directory. Reusing the upstream library format keeps files portable.
- **Alternatives considered**: Saving persistent models inside `settings.json` (mixes user data with preferences and grows the settings file without bound).

## R-7 Property editor as a side panel

- **Decision**: Move the body of `dialogs::edit_element_with` into a reusable widget that can be hosted either in the existing right-hand side panel (`app/window.rs`, the box that holds Reset/Run and the sliders) or in a dialog. The panel shows the selected element and rebuilds when the selection changes; edits apply on change, through the same `Session` edit path, with one undo step per committed field. Two new settings: `show_properties_on_create` (default true) and `show_properties_on_click` (default false). A per-element "rarely edited on create" flag lives in the element registry descriptor (wires, logic inputs, ground, labeled nodes and others matching upstream's list in commit `6737154f`).
- **Rationale**: Reuses the existing `EditInfo` field model, so no element code changes. The registry flag keeps the modularity rule from spec 001 (adding a component stays a local change).
- **Alternatives considered**: A floating non-modal window (harder to place on Wayland, where GTK cannot position windows, which also limits "open next to the element"). On Wayland the dialog falls back to centered placement; in the side panel the question does not arise.

## R-8 Find Component becomes a command palette

- **Decision**: Correction to the spec's table C: prometheuscs's current Find Component lists the elements *already in the circuit*, while CircuitJS (Java and 5.x) searches the **component types to place and the menu commands**. The palette will search three groups: components to place, menu commands, and (kept from prometheuscs) elements in the circuit. Matching is case-insensitive substring over the localized name, with recent picks (last 10, stored in settings) listed first. Opened by `/`, a double Shift press within 400 ms (ignored while a text entry has focus), the toolbar button and the Edit menu.
- **Rationale**: Matches upstream behavior without losing the prometheuscs feature users already have.
- **Alternatives considered**: Replacing the existing list (loses a feature); fuzzy ranking (upstream uses substring; not worth a dependency).

## R-9 Toolbar drag and wire splitting

- **Decision**: Toolbar part buttons get a GTK `DragSource` carrying the element id; the circuit canvas gets a `DropTarget` that places the element at the drop point using the same code path as a click-drag placement. Wire splitting is one `Session` operation, `split_wires_at_posts(element)`, called after placement and after a move ends: for each post of the element lying strictly inside a wire or lead segment, split that segment at the post. It runs inside the same undo step as the placement or move.
- **Rationale**: One operation for both C4 (new element) and N7 (moved element). GTK 4 drag-and-drop needs no new crates.
- **Alternatives considered**: Splitting during the drag (makes undo and cancel messy).

## R-10 Mouse wheel: exact steps, type flip, sliders

- **Decision**: Replace the factor nudge in `Session::wheel_at` (`src/app/edit.rs:2132`) with a step table: E12 for R, C, L (already present); for other values, a 1-2-5 sequence per decade, matching upstream `ScrollValuePopup`. One notch moves exactly one step; the existing "wheel sensitivity" setting is retired for value editing. Transistors, MOSFETs and JFETs toggle polarity (upstream `TypeScrollPopup`). Slider elements move their slider by one step. A small popover near the pointer shows the new value for about one second.
- **Rationale**: FR-013. Keeps the 1.0.8 E12 behavior.
- **Alternatives considered**: Keeping the sensitivity multiplier (contradicts one notch = one step).

## R-11 Scope image export

- **Decision**: Render one scope through the existing painter abstraction into the cairo PNG backend and the SVG backend (`render/cairo.rs`, `render/svg.rs`, `render/export.rs` already do this for the whole circuit). The dialog asks for format, width, height (1 to 8192 px) and transparent background (skips the background fill).
- **Rationale**: No new dependencies; the same backends are already tested (`tests/render_cairo.rs`, `tests/render_svg.rs`).
- **Alternatives considered**: Reading back the OpenGL framebuffer (cannot do SVG or transparency).

## R-12 Unsaved-changes tracking

- **Decision**: Keep `Session::dirty` but set it only from circuit-content edits. Audit every `dirty = true` (in `app/edit.rs`, `wires.rs`, `file.rs`, `subcircuits.rs`) and make sure the speed, current-speed and brightness controls, scope layout, zoom and pan do not set it. Add a test per control.
- **Rationale**: N16 is about which actions count, not about the mechanism.
- **Alternatives considered**: Comparing a hash of the saved text (costly on every frame, and the format writer is not guaranteed to be stable across equivalent states).

## R-13 Documentation workflow

- **Decision**: Each user story's tasks end with its documentation update (constitution V). The example pages for `cs-*` are refreshed with the existing `exdocs` tool (`exdocs facts`, `exdocs sync`, `exdocs check`), and `tests/example_docs.rs` keeps them consistent. The tool needs to recognize the new element tags (`cv`, `ci`) and the `lb` meter label so the generated "what the example contains" sections stay correct.
- **Rationale**: The tool and its test already exist; extending them is cheaper and keeps the 228 pages consistent.
- **Alternatives considered**: Editing the pages by hand (drifts from the circuits; `exdocs check` would flag it).

## R-14 Status of the "assumed present" rows

- **Decision**: Checked during planning:
  - Present: Rotate CW/CCW, Mirror X/Y, Duplicate, Select All, Shortcuts dialog, Convert Wires to Routed Wires, P-JFET, CC2−, true and old wattmeter, latching relay coils, analog-mux pull-down, DeMorgan shapes, ohm symbol, Stop Trigger count and duration, scope CSV, circuit PNG/SVG export, circuit/scope splitter, Show Scale, basic AC coupling, sparse solver, Finnish.
  - Partial (moved into scope): **Find Component** (R-8).
  - Still to verify in the first task of the matching story, by comparing with upstream: JFET pin names on highlight, transistor arrow highlighting, demultiplexer invertible outputs, average-voltage mode on probes and test points.
- **Rationale**: Keeps SC-003 honest.

## Performance baseline (T014)

`cargo bench --bench solver` (criterion, median of the reported interval), 2026-11-02, on the
unchanged code (git worktree of `8fa8500`) and on this branch after the reference engine (T013)
and the move of the VCVS family onto it (T019):

| Benchmark | Before | After | Change |
|-----------|--------|-------|--------|
| steps/rlc | 2.745 ms | 2.573 ms | −6.3% (noise) |
| steps/ladder | 9.274 ms | 9.268 ms | −0.1% |
| steps/mixed | 20.269 ms | 20.324 ms | +0.3% |
| analysis/ladder | 2.865 ms | 2.890 ms | +0.8% |
| analysis/mixed | 6.990 ms | 7.093 ms | +1.5% |

All within the SC-007 budget of 5%. The reference pass costs one scan of the elements per
analysis when no element has references, and nothing per step.

### Finished branch (T100)

`cargo bench --bench solver`, 2026-11-02, on the finished branch (all stories), against the
"Before" column above (1.0.8):

| Benchmark | 1.0.8 | Finished branch | Change |
|-----------|-------|-----------------|--------|
| steps/rlc | 2.745 ms | 2.535 ms | −7.7% (noise) |
| steps/ladder | 9.274 ms | 9.272 ms | 0.0% |
| steps/mixed | 20.269 ms | 19.969 ms | −1.5% |
| analysis/ladder | 2.865 ms | 2.902 ms | +1.3% |
| analysis/mixed | 6.990 ms | 7.012 ms | +0.3% |

Within the SC-007 budget of 5%. `bench_circuits` only regenerates the benchmark circuits in
`tests/bench/`; it wrote them unchanged.

## R-14 outcome (T006)

All four rows are **present**; nothing moves to table B.

- Average-voltage mode on probes and test points: present (`TP_AVG` in `src/elements/outputs.rs`).
- JFET pin names on highlight: present (the JFET shares the drawing of `src/elements/mosfets.rs`,
  which writes G, D and S when highlighted).
- Transistor arrow highlighting (upstream `fe9e0bd3`): present; `src/elements/transistors.rs`
  writes B, C and E for horizontal and vertical transistors and draws the PNP arrow along the
  emitter, as 5.x does.
- Demultiplexer invertible outputs: present ("Keep Inactive Outputs High (74139)",
  `FLAG_INVERT_OUTPUTS` in `src/elements/digital.rs`), already in the Java baseline.

## SC-004 measurement (T064)

**Method**: a counted walkthrough of the interaction steps, not an instrumented session. Each
press-and-release is one click (a double click is two). A drag that places a part is one click.
Keys (Enter, Escape) and wheel notches are not clicks. Both versions use the same placement
order, and in both a placing tool stays active after each part.

**Reference task**: a 2-stage RC low-pass (R1, C1, R2, C2) fed by an AC source, buffered by an
NPN emitter follower (collector to a +V rail, emitter resistor Re to ground). Six values are
edited: R1, R2, C1, C2, Re and the source frequency.

| Step | 1.0.8 | This branch (default options) | This branch, "Show Properties on Create" on |
|------|------:|------:|------:|
| AC source (open the group, pick, drag) | 3 | 3 | 3 |
| Resistors (tool, 3 drags) | 4 | 4 | 4 |
| Capacitors (tool, 2 drags) | 3 | 3 | 3 |
| NPN (tool, drag) | 2 | 2 | 2 |
| Rail (tool, drag) | 2 | 2 | 2 |
| Grounds and wires | 9 (ground tool + 4 grounds, wire tool + 3 wires) | 7 (one ground, wire tool + a ground bus the parts are dropped on + 3 wires; the bus splits at each post) | 7 |
| Six value edits | 24 (double click 2, click the field 1, Apply 1) | 12 (click the part 1, click the field 1, Enter) | 6 (the panel opens on placement: click the field 1, Enter) |
| **Total** | **47** | **33** | **27** |
| Reduction | — | **29.8%** | **42.6%** |

**Result**: with the default options (the 5.x defaults: properties on click, not on create)
the reduction is 29.8%, just under the 30% target. With "Show Properties on Create" turned on
it is 42.6%. Wheel editing (when enabled) would lower both further, since notches are not
clicks. To be confirmed by counting clicks by hand in a running build.

## FR-016 and upstream's AC-coupling filter (T067, T072)

The filter is ported as upstream has it (`a5c29008`): the DC level is the running average of
the samples until the factor reaches the normal time constant. For a 1 V sine on 5 V the mean
of the AC-coupled trace per period is −0.39, −0.11, −0.06, −0.04, −0.026, −0.017 V… (periods 1
to 6), so it is within 2% of the amplitude from the sixth period on, not after one period as
FR-016 says. The plain filter it replaces is still about 3 V off after one period. The test
(`scope::tests::ac_coupling_centres_an_offset_sine_quickly_in_auto_scale`) checks what the port
gives; FR-016's wording needs a decision: accept upstream's behavior, or change the filter
beyond upstream (for example, removing the mid-range instead of the running mean during the
first periods), which would depart from 5.0.4.

## Quickstart validation (T101)

Run on the finished branch, 2026-11-02 (to go into the pull request description).

| Section | Result |
|---------|--------|
| 1. Quality gates | `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean with default features and with `gui`; `cargo test`: 690 tests in 57 binaries pass, 703 in 60 with `gui`, none ignored |
| 2. Reference data | Nothing to record (as planned) |
| 3. Stories 1 and 2 | `compat_golden`, `compat_5x`, `controlled`, `format_roundtrip` pass (part of the full run). The by-hand check with `cs-resistor` was not run |
| 4. Story 3 | Covered by tests (`palette` unit tests, `actions`: drop, wire split, properties on create/click, wheel steps and type flip). The by-hand steps in the running app were not run |
| 5. Story 4 | Covered by tests (`scope` and `render::scopes` unit tests, `render_svg`, `render_cairo`). AC coupling centres within 2% of the amplitude from the 6th period, not the 1st (see "FR-016 and upstream's AC-coupling filter"). The by-hand export was not run |
| 6. Stories 5 and 6 | Covered by `subcircuit_library` (persistent model read back at start-up) and `actions` (speed slider leaves the circuit saved). Not run by hand |
| 7. Documentation | `exdocs check`: "tudo certo"; `example_docs` passes; every row of tables A and B is described in the guide, the deferred part of N6 in section 16 |
| 8. Performance | See "Finished branch (T100)" above: within 5% |
