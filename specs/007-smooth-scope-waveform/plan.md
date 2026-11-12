# Implementation Plan: Smooth scope waveforms

**Branch**: `007-smooth-scope-waveform` | **Date**: 2026-11-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/007-smooth-scope-waveform/spec.md`

## Summary

The scope keeps, for every sample (one per pixel), only the minimum and maximum of the values
seen in it, and draws a line through the *maxima* plus a vertical stroke from each maximum down
to its minimum. With the Summing Amplifier example (a 200 Hz sine, 64 simulation steps per
sample, about 15 samples per period) each crest becomes a pointed spike and each trough a
needle, whose size depends on where the period falls on the pixel grid (see research R1). The
fix records one more number per sample, the value at the end of the sample, and draws the curve
through those end values; the minimum and maximum only add a vertex (or a vertical stroke) when
the sample goes beyond what the neighbouring end values explain. A smooth sine then shows a
smooth curve with its true crest, while noise, glitches and edges keep their full extent. The
stored minima and maxima, scales, readouts and measurements are untouched.

## Technical Context

**Language/Version**: Rust, edition 2024

**Primary Dependencies**: none new (the `Painter` trait already offers `move_to`, `line_to`,
`stroke`; GL, Cairo and SVG backends share the drawing function)

**Storage**: N/A (scope history lives in memory and is not saved with circuits)

**Testing**: `cargo test`: unit tests beside `src/scope/mod.rs` (recording) and
`src/render/scopes.rs` (path construction); integration tests in `tests/scopes.rs` with the SVG
painter, which exposes the path of each curve for assertions

**Target Platform**: Linux desktop (GTK4), as the rest of the program

**Project Type**: desktop-app (library crate plus `gui` feature)

**Performance Goals**: the curve loop stays linear in the visible samples (at most about 2048 per
plot) with a few extra arithmetic operations per sample; drawing time grows by no more than 10%
(SC-004), measured with the existing `render_bench`

**Constraints**: stored min/max values and everything derived from them (autoscale, cursor values,
measurements, spectrum, XY) unchanged (FR-004); trigger sweep copy keeps working; no new setting

**Scale/Scope**: one new buffer per plot and per held sweep; one drawing loop rewritten

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Status | How |
|-----------|--------|-----|
| I. Code quality | PASS | `fmt`, `clippy -D warnings` (also with `--features gui`); no `unsafe`, no suppressions |
| II. Test-first | PASS | tests that build the Summing Amplifier crest/trough geometry and a noise, glitch and square case are written first and fail on the current drawing (quickstart step 1) |
| III. Layered testing | PASS | unit tests for recording, SVG integration tests for the drawn path, error paths (empty history, one sample) covered |
| IV. Maintainable design | PASS | the path building moves to one small, documented function that is tested alone; no new abstraction beyond one extra `Vec<f64>` |
| V. Dependencies / docs | PASS | no crate added; scope section of the user guide mentions how the curve is drawn only if it described the old behaviour (checked in tasks) |
| Performance claim needs a measurement | PASS | `render_bench` before and after (quickstart step 4) |

Re-check after design: unchanged, no violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/007-smooth-scope-waveform/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── curve-drawing.md
└── tasks.md             # created by /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── scope/mod.rs         # ScopePlot: record the end value of each sample; Sweep and PlotView carry it
└── render/scopes.rs     # draw_scope_inside: build the curve from end values plus excursions

tests/
└── scopes.rs            # integration tests: sine, noise, glitch, square wave, held sweep
```

**Structure Decision**: single project; the change stays inside the scope data
(`src/scope/mod.rs`) and the scope painter (`src/render/scopes.rs`), the same two places the
earlier scope features (002, 003) changed. The X/Y and spectrum views are not touched.

## Complexity Tracking

No constitution violations.
