# Implementation Plan: CircuitJS 5.x Feature Sync

**Branch**: `update/circuitjs-features-to-prometheus` | **Date**: 2026-11-02 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-circuitjs5-feature-sync/spec.md`

## Summary

Bring prometheuscs from parity with CircuitJS 4.1.5.1js (Java) to parity with CircuitJS 5.0.4 (TypeScript). The work covers 20 upstream additions (spec table A) and 8 older port gaps (table B), and it updates the documentation in the same changes.

The core technical piece is name references in expressions (`v()`, `i()`, `dvdt()`, `didt()`). They are ported from upstream's `ExprEngine` design: each reference becomes two extra node slots on the owning element, resolved in a new analysis pass. The existing matrix-closure logic then pulls the referenced node into the right matrix, and current references reuse the spice-style current-sensing path that already exists (research R-2).

Everything else is localized to the app layer:
- the property editor moves into the side panel;
- Find Component is extended into a palette;
- toolbar drag-and-drop and wire splitting on place and move;
- exact-step wheel popups;
- scope image export through the existing cairo and SVG backends;
- subcircuit scopes and library files;
- documentation through the existing `exdocs` tool.

Planning also corrected two spec assumptions:
- The 5.x subcircuit element uses the same file tag and attributes as the 4.x composite, so it needs no new element (R-5).
- prometheuscs's Find Component searches circuit elements, not component types and commands, so it is a partial match, not "present" (R-8).

## Technical Context

**Language/Version**: Rust, edition 2024 (as spec 001).

**Primary Dependencies**: Existing only: relm4 0.11 / gtk4 0.11 / libadwaita (UI, including GTK 4 drag-and-drop and popovers), cairo-rs (PNG/SVG export), femtovg + glow (canvas), serde/serde_json (settings), thiserror, tracing. **No new crates.**

**Storage**: Circuit files (XML and legacy text). Settings in the existing settings file (new keys in [data-model.md](data-model.md)). A new user subcircuit library file next to the settings (R-6).

**Testing**: `cargo test`. Unit tests beside code. Integration tests in `tests/`: `controlled.rs`, `format_roundtrip.rs` (proptest), `compat_golden.rs`, plus a new `compat_5x.rs`, and `example_docs.rs`. Reference data: the existing recordings (including the old `cs-*` versions) and exact hand calculations. No CircuitJS 5.0.4 build is needed (R-1).

**Target Platform**: Desktop Linux, GTK 4.22+ / libadwaita 1.9+ (unchanged).

**Project Type**: Desktop application (single Cargo package: library + binary + developer tools).

**Performance Goals**: No more than 5% slower simulation on the bundled examples than 1.0.8 (SC-007), measured with `benches/solver.rs` and `src/bin/bench_circuits.rs`. The reference-resolve pass runs only on analysis, never per step.

**Constraints**: Bit-for-bit file compatibility with CircuitJS 5.0.4 for the attributes in [contracts/file-format.md](contracts/file-format.md). Results within the spec-001 tolerance of the existing recordings and of the hand-calculated values. The modularity rule of spec 001 (a component is one unit plus one registration) still holds.

**Scale/Scope**: 28 inventory rows, 7 user stories. Touches about 15 existing modules plus 2 new element types. The 12 `cs-*` example pages and 4 documentation files are updated.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | How this plan complies | Status |
|-----------|------------------------|--------|
| I. Code Quality Is a Gate | fmt, clippy `-D warnings` and tests per story; no new `unsafe` (GTK drag-and-drop and cairo export are safe APIs) | Pass |
| II. Test-First (non-negotiable) | Every story starts with failing tests: expression parsing and resolution unit tests, golden comparisons for the `cs-*` and 5.x reference circuits, round-trip tests for each new attribute, session-level tests for wire splitting, wheel steps, the dirty flag and the palette filter. Bug fixes from N20 each get a regression test | Pass |
| III. Layered Testing | Unit tests in `expr`, `elements/controlled.rs` and `solver/sim.rs`; integration tests in `tests/`; error paths (unknown name, not a current meter, bad SRAM file, bad export size) tested explicitly | Pass |
| IV. Maintainable Design | Reference handling lives in one module shared by all expression owners (R-2/R-3) instead of being copied per element; the property editor is one widget hosted in a panel or a dialog (R-7); "quiet on create" is a registry flag (spec 001 modularity) | Pass |
| V. Explicit Dependencies and Reproducibility | No new crates. Documentation updated in the same change as each behavior (Story 7, R-13). Test circuits and their expected values committed under `tests/compat/circuits-5x/` | Pass |
| Engineering: performance measurement | Before/after numbers from `solver` bench and `bench_circuits` attached to the Story 1 change (it touches the solver's analysis path) | Pass |

No violations. Complexity Tracking is empty.

**Post-design re-check (after Phase 1)**: The design adds no crates, no `unsafe`, no new architectural layers. The analysis pass and node slots extend existing solver concepts. The contracts fix every new file attribute and command id, so tests can be written first. **Still passes.**

## Project Structure

### Documentation (this feature)

```text
specs/006-circuitjs5-feature-sync/
├── spec.md
├── plan.md                         # This file
├── research.md                     # Phase 0
├── data-model.md                   # Phase 1
├── quickstart.md                   # Phase 1
├── contracts/
│   ├── file-format.md              # new tags/attributes, sublib files
│   ├── expression-references.md    # v()/i()/dvdt()/didt() grammar and errors
│   └── ui-commands.md              # commands, options, gestures, palette groups
├── checklists/requirements.md
└── tasks.md                        # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── expr/mod.rs                 # parse reference functions; ExprState gains ref values (Story 1-2)
├── expr/refs.rs                # NEW: shared reference engine (resolve, inputs, numeric Jacobian)
├── elements/
│   ├── controlled.rs           # VCVS/VCCS/CCVS/CCCS use the shared engine; NEW cv/ci elements
│   ├── element.rs              # trait: expr_ref_name(), ref_node_count(), resolve_refs()
│   ├── measure.rs              # ammeter label (lb); stop-trigger arming on load/reset
│   ├── outputs.rs              # voltmeter/probe label (lb); text color (co)
│   ├── subcircuit.rs           # rn attribute, model scope, sublib files
│   ├── composite.rs            # expand rn into instance nodes; i() by node pair
│   ├── memory.rs               # SRAM load contents from file
│   ├── digital.rs              # monostable invert trigger
│   └── registry.rs             # quiet_on_create flag
├── solver/sim.rs               # resolve-references pass between node list and closures
├── format/{xml,parser,writer}.rs  # new tags/attributes; text-format save guard
├── app/
│   ├── edit.rs                 # wheel steps/type flip/sliders; split_wires_at_posts; dirty audit
│   ├── wires.rs                # wire split helper
│   ├── dialogs.rs              # property editor widget extracted; palette; scope export dialog;
│   │                           # subcircuit manager (previews, scope, multi-select, library)
│   ├── window.rs               # side-panel host; drop target; double-Shift; scope drag guards
│   ├── toolbar.rs              # drag sources on part buttons
│   ├── settings.rs             # new settings keys; persistent library path
│   └── commands.rs / menus.rs  # new command ids
├── render/{scopes,export}.rs   # scope image export; stacked labels/legend; sweep hold; X/Y roles
├── scope/mod.rs                # AC coupling fast start/reset/auto scale; sweep buffer
└── bin/exdocs.rs               # recognise cv/ci and meter labels

assets/circuits/cs-*.txt        # replaced by 5.0.4 versions
assets/locale/locale_*.txt      # new strings from 5.0.4

tests/
├── compat_5x.rs                # NEW: 5.x circuits vs expected.json; new cs-* vs old recordings
├── compat/circuits-5x/         # NEW: 5.x-format test circuits + expected.json (hand-calculated)
├── controlled.rs               # extended
├── format_roundtrip.rs         # extended
└── example_docs.rs             # unchanged; must keep passing

docs/
├── guia-do-usuario.md          # sections 4, 6, 9, 10, 11, 13, 16
├── arquitetura.md              # 3.1, 4.x (references), 6.x (side panel)
├── adding-a-component.md       # exposing a reference name
└── exemplos/cs-*.md            # regenerated
README.md                       # "De onde vem", "O que é diferente"
```

**Structure Decision**: Keep the single-package layout from spec 001. The only new source module is `src/expr/refs.rs`, so the reference logic is written once and shared by all six expression-owning elements. Everything else extends existing modules.

## Delivery Order

Each phase is shippable on its own and includes its documentation (Story 7).

1. **Foundation**: write the 5.x test circuits and their hand-calculated expected values, and map the old `cs-*` recordings to the new circuits (R-1). Verify the four "still to verify" rows (R-14).
2. **Stories 1 + 2 (P1)**: reference engine, `cv`/`ci`, meter labels, `rn`, updated `cs-*` examples, N9 to N11, C2, N20 fixes. Docs: guide sections 11 and 16, architecture, component guide, `cs-*` pages, README. Performance measurement.
3. **Story 3 (P2)**: palette, property panel and options, toolbar drag, wire splitting, wheel, Swap Terminals. Docs: guide sections 4, 10 and 13.
4. **Story 4 (P2)**: scope export image, AC coupling, stacked labels, sweep hold, drag guards, X/Y roles. Docs: guide section 6.
5. **Story 5 (P3)**: subcircuit scopes, previews, multi-select, library save, Load Subcircuits Only. Docs: guide section 9.
6. **Story 6 (P3)**: dirty audit, persistent warning, labeled-node hover, dialog polish. Docs: guide sections 5 and 14.

## Complexity Tracking

No constitution violations to justify.
