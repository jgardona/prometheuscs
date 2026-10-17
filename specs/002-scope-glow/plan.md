# Implementation Plan: Scope glow for the selected component

**Branch**: `002-scope-glow` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-scope-glow/spec.md`

## Summary

A scope whose plots include a selected (or hovered) element gets a tinted background. The session
works out which scopes qualify; the painter draws the tint instead of the plain background. Same
rule for the docked scopes (bottom area) and for scopes placed in the circuit. The reference image
was measured: the glow panel is (14, 52, 52) over a plain (17, 17, 17); the blue version keeps the
same lift over the plain colour with the hue moved to blue.

## Technical Context

**Language/Version**: Rust, edition 2024

**Primary Dependencies**: none new (relm4/GTK4, femtovg and Cairo painters already in place)

**Storage**: N/A (the tint is not saved in files)

**Testing**: `cargo test` — unit tests beside `src/render/scopes.rs`, integration tests in `tests/`
using the SVG painter and `Session`

**Target Platform**: Linux desktop (GTK4), as the rest of the program

**Project Type**: desktop-app (library crate plus `gui` feature)

**Performance Goals**: no measurable cost: one filled rectangle per tinted scope per frame

**Constraints**: the painters stay the only drawing path; nothing in the simulation thread changes

**Scale/Scope**: a handful of scopes per circuit; about 4 source files touched

## Constitution Check

*GATE: passed before research and again after design.*

| Principle | Result |
|-----------|--------|
| I Code quality (fmt, clippy `-D warnings`) | Will be run, with and without `gui` |
| II Test-first | Tests written before the code (see quickstart); a failing run is recorded |
| III Layered testing | Unit tests for colours and the rule; integration test with `Session` and SVG output |
| IV Maintainable design | One bool in `ScopeSpot`, one method in `Session`; no new abstraction. Public items documented |
| V Dependencies and docs | No crate added. `docs/guia-do-usuario.md` gets one paragraph |

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/002-scope-glow/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/ui.md
└── tasks.md             # /speckit-tasks, not created here
```

### Source Code (repository root)

```text
src/
├── render/scopes.rs     # ScopeSpot.glow, ScopeScales.glow, tint colours, draw_scopes uses them
├── elements/scope_elm.rs# a scope placed in the circuit keeps its glow flag and passes it on
├── elements/element.rs  # Element::set_scope_glow (default: nothing)
├── solver/sim.rs        # Simulation::set_scope_glow: marks the placed scopes
├── app/engine.rs        # pump calls it every frame, with or without a worker
├── app/edit.rs          # Session::scope_glow(): which scopes plot a selected / hovered element
└── app/window.rs        # feeds the flags to the scope area (like `info`)
tests/
└── scope_glow.rs        # new
docs/guia-do-usuario.md  # one paragraph
```

**Structure Decision**: single crate; the rule lives in the session (it knows selection and
hover), the colour and drawing in the scope painter.

## Complexity Tracking

None.
