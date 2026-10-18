# Implementation Plan: Readable scope labels

**Branch**: `003-scope-readability` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-scope-readability/spec.md`

## Summary

The labels that follow the pointer on a scope (one value line per plot and the time) get a
translucent plate behind the text, in the scope's background colour at 75% opacity. The line at
the top of each scope with the element name and the newest value of each plot is not drawn any
more. Both changes are in the scope painter, so the docked scopes and the scopes placed in the
circuit (same drawing function) follow them together.

## Technical Context

**Language/Version**: Rust, edition 2024

**Primary Dependencies**: none new; the `Painter` trait already has `fill_rect`, `set_alpha`,
`measure_text` (GL, Cairo and SVG backends)

**Storage**: N/A

**Testing**: `cargo test`: unit tests beside `src/render/scopes.rs`; integration tests in `tests/`
with the SVG painter (a translucent fill is a `<path ... fill="#000000" opacity="0.75"/>`)

**Target Platform**: Linux desktop (GTK4), as the rest of the program

**Project Type**: desktop-app (library crate plus `gui` feature)

**Performance Goals**: no measurable cost: at most one rectangle per label line per frame, only
while the pointer is over a scope

**Constraints**: nothing is drawn outside the scope (the scope already clips to its rectangle);
no change to layout, scales, the information area or the measurement labels

**Scale/Scope**: one function changed (`draw_cursor`), one block removed (the latest-value
readouts), existing tests adjusted

## Constitution Check

*GATE: passed before research and again after design.*

| Principle | Result |
|-----------|--------|
| I Code quality | `fmt`, `clippy -D warnings` with and without `gui` |
| II Test-first | Tests first (they fail on the current drawing), then the change |
| III Layered testing | Unit tests for the plate geometry; integration test through `draw_scopes` and the SVG painter, with and without a worker not needed (drawing only) |
| IV Maintainable design | One small helper (`draw_label`) reused for every cursor line; no new abstraction beyond it; public items documented |
| V Dependencies and docs | No crate; `docs/guia-do-usuario.md` mentions the labels, updated in the same change |

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/003-scope-readability/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/ui.md
└── tasks.md             # /speckit-tasks, not created here
```

### Source Code (repository root)

```text
src/render/scopes.rs     # draw_cursor: plates; draw_scope_inside: latest-value readouts removed
tests/scope_labels.rs    # new
tests/scopes.rs          # adjust the tests that expect the removed line
docs/guia-do-usuario.md  # one paragraph
```

**Structure Decision**: single crate; the whole change is in the scope painter.

## Complexity Tracking

None.
