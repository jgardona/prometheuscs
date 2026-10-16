# Implementation Plan: Circuit Simulator Migration (prometheuscs)

**Branch**: `001-circuit-simulator-migration` | **Date**: 2026-10-12 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-circuit-simulator-migration/spec.md`

## Summary

Port the CircuitJS1 simulator (about 58k lines of Java/GWT, 153 element classes, 373 example circuits) to a native Rust desktop application, as a **single Cargo package** at the repository root (clarification 2026-10-12). One library crate with clearly separated modules keeps the project simple while preserving the layering:

1. **Engine** (`src/format`, `src/elements`, `src/solver`, `src/scope`, `src/expr`): no GUI dependencies; circuit text format, element models, Modified Nodal Analysis with sparse LU, scope data. A behavioral port, so results match the original within tolerance (FR-007, FR-008, FR-009).
2. **Renderer** (`src/render`): a backend-agnostic 2D painter used by every element's `draw`, with a GPU backend (OpenGL via `GtkGLArea`) for the live canvas and SVG/PNG backends for export (FR-011, FR-012).
3. **Application** (`src/app`): relm4 + libadwaita UI reproducing the reference layout, with the simulation on a worker thread so input is never blocked (FR-002 to FR-006, FR-013 to FR-019).

**Modularity (FR-025 to FR-027)** is a first-class design rule: components are self-contained family modules that declare themselves in a registry; menus, toolbar entries, file loading and the inventory are generated from the registry, and each family can be disabled with a Cargo feature.

Compatibility is proven by a golden-data harness: every bundled example is run in the original (built copy at `/home/jgardona/Bin/circuitjs1`) and in the port, and node voltages and scope traces are compared (FR-020, FR-021, SC-001, SC-011). See [research.md](research.md).

## Technical Context

**Language/Version**: Rust, edition 2024 (toolchain installed: cargo 1.99.0)

**Primary Dependencies**: relm4 (current release compatible with GTK 4.22 / libadwaita 1.9, both installed), gtk4-rs, libadwaita-rs, femtovg (OpenGL 2D vector renderer) on `GtkGLArea`, `serde` + `toml`/`serde_json` for settings, `flate2` or `lz-str` equivalent for share links, `thiserror`, `tracing`. Dev: `criterion`, `proptest`, `insta`. Exact versions pinned in `Cargo.lock` at the first dependency commit.

**Storage**: Files only: circuit text files (original format), a user settings file, and an autosave file in the user data directory.

**Testing**: `cargo test` (unit, integration, doctests); golden-data comparison harness under `tests/compat`; `criterion` benchmarks for the solver and frame rendering.

**Target Platform**: Desktop Linux (GTK 4.22+, libadwaita 1.9+, OpenGL 3.3 or GLES 3 capable driver, software GL fallback).

**Project Type**: Desktop application, single Cargo package (library plus binary, developer tools as extra binaries).

**Performance Goals**: 60 fps canvas for 500 elements, solver at least equal to the original and 2x faster for 1000+ elements, startup under 2 s, input latency under 50 ms (SC-003 to SC-006).

**Constraints**: Result parity with the original within tolerance 1e-6 relative on node voltages at matching time steps; UI thread never runs the solver; no `unsafe` outside the GL glue module (constitution I); adding or removing a component touches only its own module plus one registration (FR-025); licence GPL-2.0-or-later (research.md R-9).

**Scale/Scope**: 153 element classes grouped into about 20 family modules, about 370 example circuits, 15 locales, around 60 menu commands, about 38 UI icon glyphs and about 40 toolbar SVG icons to import.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Gate | Status |
|-----------|------|--------|
| I. Code Quality Is a Gate | fmt, clippy `-D warnings` in CI from the first commit; `unsafe` confined to one GL glue module with `// SAFETY:` notes | Pass (planned in workspace lints and CI) |
| II. Test-First (NON-NEGOTIABLE) | Each element port begins with a failing golden test; each bug fix adds a regression test | Pass (task ordering must put tests before implementation) |
| III. Layered Testing | Unit tests in modules; integration tests in `tests/`; doctests for public core API; compat harness runs under `cargo test` | Pass |
| IV. Maintainable Design | Small modules per element family, registered declaratively (FR-025); typed errors via `thiserror`; no `unwrap` outside tests; `///` docs on public items | Pass |
| V. Explicit Dependencies | Each dependency justified in research.md (R-2 to R-8); `Cargo.lock` committed | Pass |

**Post-design re-check**: Pass. The single-package layout removes the earlier workspace complexity; remaining justified deviations are in Complexity Tracking.

**Licence**: GPL-2.0-or-later, same as the original (research.md R-9). No open compliance item.

## Project Structure

### Documentation (this feature)

```text
specs/001-circuit-simulator-migration/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── circuit-file-format.md
│   ├── component-module.md
│   ├── engine-api.md
│   └── ui-commands.md
└── tasks.md             # Phase 2 output (/speckit-tasks, not created here)
```

### Source Code (repository root)

```text
Cargo.toml                       # single package: lib + bin(s); one feature per element family
src/
├── lib.rs
├── main.rs                      # binary: prometheuscs (relm4 application entry)
├── error.rs                     # LoadError, AnalysisError, StepError
├── format/                      # tokenizer, parser, writer, share-link codec, Circuit container
├── elements/
│   ├── element.rs               # Element trait
│   ├── registry.rs              # declarative registry: descriptors, factory, menu model, inventory source
│   ├── geometry.rs
│   ├── passive.rs  reactive.rs  sources.rs  switches.rs        # each family: one file, one register() fn
│   ├── diodes.rs  transistors.rs  mosfets.rs  analog.rs  logic.rs  digital.rs  ...
│   └── audio.rs                 # feature-gated like every other family
├── solver/                      # sparse LU, analysis, step loop, DC point, snapshots
├── scope/                       # scope data, FFT, triggers
├── expr/                        # expression evaluator
├── render/                      # Painter trait; GL, SVG, PNG backends; scene and scope drawing; view transform
├── app/                         # relm4 components
│   ├── canvas/  panels/  menus/  dialogs/  editing/  export/
│   ├── worker.rs  settings.rs  i18n.rs  catalog.rs  autosave.rs
└── bin/                         # developer tools
    ├── import_assets.rs         # copy examples, locales, icons from the original
    ├── inventory.rs             # component inventory from the original sources
    └── golden.rs                # generate reference results from the built original
assets/
├── circuits/  setuplist.txt  locale/  icons/  inventory.json
└── ATTRIBUTION.md
benches/                         # criterion benchmarks
docs/
└── adding-a-component.md        # developer guide (FR-027)
tests/                           # integration tests; compat/ holds golden data
```

**Structure Decision**: One package, modules instead of crates, as chosen in the 2026-10-12 clarification. Layer boundaries are kept by module visibility (the engine modules never import `app` or GTK types), so the engine remains testable and benchmarkable without a display. Developer tools are binaries in `src/bin/` instead of a separate `xtask` crate. If compile times or GTK linkage in tests become a problem, the modules can be split into crates later without API changes.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Direct port of the original sparse LU instead of an existing crate | Numerical parity with the original and predictable pivoting for result comparison | Third-party solvers pivot differently and would make golden comparison noisy; revisit after parity using benchmarks |
| Cargo feature per element family | Required by FR-026 (build-time removal of a family) | A single always-on element set cannot satisfy removable components |
