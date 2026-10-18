---

description: "Task list for the readable scope labels"
---

# Tasks: Readable scope labels

**Input**: Design documents from `/specs/003-scope-readability/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ui.md, quickstart.md

**Tests**: included — the constitution (II, Test-First) requires them. Write each test, see it fail, then implement.

**Organization**: grouped by user story. Paths are from the repository root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2, US3 from spec.md

## Phase 1: Setup

- [X] T001 Create `tests/scope_labels.rs` (SPDX header, `#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]` with the same comment as `tests/pull_state.rs`) with helpers: one that builds `ScopeView`s from `assets/circuits/lrc.txt` after 40 `Session::update` frames (as `tests/scope_glow.rs` does), and one that draws them with `draw_scopes(&mut SvgPainter, &views, &mut ScopeScales::default(), 900.0, 400.0, light, cursor)` and returns the SVG text; plus a helper that counts the `<path` lines with `opacity="0.75"` and a given `fill="#rrggbb"`

## Phase 2: Foundational (blocks all stories)

- [X] T002 In `src/render/scopes.rs` add a private `draw_label(g: &mut dyn Painter, text: &str, tx: f64, baseline: f64, plate: Color, bounds: (f64, f64))` that: measures `text`, clamps `tx` so the plate stays within `bounds` (the scope's left and right edges), draws the plate with `g.set_alpha(0.75)`, `g.fill_rect(tx - 3.0, baseline - 11.0, width + 6.0, 14.0)` in the colour `plate`, then `g.set_alpha(1.0)`, and finally draws the text with the colour the caller already set (callers set the colour before calling and the helper re-sets it after the plate); document it with `///`. Do not use it yet.

**Checkpoint**: `cargo build` passes; no visible change.

## Phase 3: User Story 1 — plates behind the cursor labels (P1) 🎯 MVP

**Goal**: the value lines and the time next to the cursor are drawn on 75%-opaque plates in the scope's background colour.

**Independent Test**: `cargo test --release --test scope_labels` (US1 tests).

- [X] T003 [US1] Write failing tests in `tests/scope_labels.rs`: with the pointer over a scope (`cursor = Some((x, y))` inside the first scope), the dark theme output has exactly one plate (`fill="#000000"`, `opacity="0.75"`) per plot plus one for the time; with `cursor = None` it has none; with `light = true` the plates are `fill="#ffffff"`; the text colours are unchanged (the value text keeps its trace colour, the time keeps the text colour `#c0c0c0`)
- [X] T004 [US1] Write failing tests in `tests/scope_labels.rs` for the plate position: for a pointer in the left half the plates are to the right of the cursor line, in the right half to the left; the plate rectangles (parse `d="M… L…"` of each plate path) lie inside the scope rectangle for pointers at the four corners and the middle of the first scope
- [X] T005 [US1] In `src/render/scopes.rs` `draw_cursor`, pass the panel colour (`bg`) in (extend the tuple argument `(Color, Color, bool)` to also carry it, and update the call in `draw_scope_inside`), and draw each value line and the time label with `draw_label` instead of `g.draw_string`, keeping the positions (`tx`, vertical stacking) exactly as today
- [X] T006 [US1] Run `cargo test --release --test scope_labels`; T003 and T004 must now pass

**Checkpoint**: the labels have plates in the window (quickstart step 4, first part).

## Phase 4: User Story 2 — no latest-value line (P2)

**Goal**: scopes do not draw the line with the element name and the newest value of each plot.

**Independent Test**: `cargo test --release --test scope_labels` (US2 tests) and `cargo test --release --test scopes`.

- [X] T007 [US2] Write failing tests in `tests/scope_labels.rs`: the SVG of the scopes of `lrc.txt` (docked, no cursor) contains none of the plots' latest-value lines, i.e. no text of the form `"{title} V = …"` / `"{title} I = …"` for the plot titles (take the titles from `views[i].plots[j].title`); the scale texts (the `Max=` style text and `…/div`) and the scope's own text are still present; with the pointer on the scope the cursor labels (`V = …`, `t = …`) and the information-area lines (set `scales.info`) are still present
- [X] T008 [US2] In `src/render/scopes.rs` `draw_scope_inside`, delete the `// readouts` block (the `lines` of `"{title} {quantity} = {value}"`, `widest`, the `(from, to)` computation, `fits` and the drawing loop); keep `left_w` if other code still uses it, and remove variables that become unused so `clippy` is clean
- [X] T009 [US2] Update `tests/scopes.rs::a_scope_in_the_circuit_shows_readouts_once_it_is_big_enough` (around lines 1007-1038): it checks ` V =` in the drawing of a placed scope, which was the removed line. Rewrite it to check what stays: at 128×64 only curves (no scale text), at 420×200 the scale and the measurements are shown, in between they wait for room; use the `…/div` text for the checks, and rename the test `a_scope_in_the_circuit_shows_its_scale_once_it_is_big_enough`. Search `tests/` for any other assertion on the latest-value line (`grep -rn '" V ="\|latest' tests`) and adjust
- [X] T010 [US2] Run `cargo test --release --test scope_labels --test scopes --test worker_draw`; all must pass

**Checkpoint**: quickstart step 4 complete.

## Phase 5: User Story 3 — tidy at the edges, small and placed scopes (P3)

**Goal**: the plates stay inside the scope and the arrangements keep working.

**Independent Test**: `cargo test --release --test scope_labels` (US3 tests) and quickstart step 5.

- [X] T011 [US3] Write tests in `tests/scope_labels.rs` (they may already pass; they pin behaviour): two scopes stacked in one column, pointer on the lower one: every plate rectangle lies inside the lower scope; a scope placed in the circuit (use the undock path from `tests/scopes.rs`, then `s.sim.elements[elm].draw(...)` with the cursor not available there, so use `draw_scope` with a `ScopeSpot { cursor: Some(..), … }`) draws plates inside its rectangle at 420×200; a compact scope (below 160×90) draws no labels and no empty plates; long labels are not cut by their plate (the plate width equals the measured width + 6, clamped to the scope width)
- [X] T012 [US3] If any T011 test fails, fix `draw_label` (T002) or the call sites in `draw_cursor` in `src/render/scopes.rs` until they pass, keeping T003/T004 green
- [X] T013 [US3] Run `cargo test --release --test scope_labels`; all pass

## Phase 6: Polish & cross-cutting

- [X] T014 [P] Update `docs/guia-do-usuario.md` (scopes section): the labels next to the pointer now sit on a translucent plate, and the top line with the element name and value was removed (that information is in the information area at the right)
- [X] T015 [P] Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and the same with `--features gui`; fix warnings
- [X] T016 Run `cargo test --release` (all green) and `cargo test --test scope_labels --test scopes` in debug (overflow checks)
- [X] T019 Plates for the measurement labels (`Δt`, `Δ`) too, asked by the user after T017: test in `tests/scope_labels.rs` (`the_labels_of_a_measurement_are_on_plates_inside_the_scope`), then `draw_measure` in `src/render/scopes.rs` uses `draw_label`; done
- [X] T020 Plates for the scale labels (corner values, `Max=` and the other measurements, time per division), asked by the user after T019: test `every_scale_label_is_on_a_plate`, then `draw_label` takes its height from the font size and `draw_scope_inside` uses it for those labels; done
- [X] T017 Mutation check: replace `draw_label`'s plate with nothing (only the text), and restore the removed readouts block, in turn; the matching tests must fail; restore both
- [ ] T018 (NOT DONE in the window: only an SVG render of both themes was looked at; the release `--features gui` binary is built) Build with `cargo build --release --features gui`, open *Circuito RLC*, follow quickstart steps 4-5 (dark and light background, a placed scope), and compare with the screenshot of the report; adjust the plate margin or opacity constants if needed (record what was and was not seen in the window)

## Dependencies & Execution Order

- Phase 1 → Phase 2 → stories. T002 before T005.
- US1 (T003-T006) is the MVP. US2 is independent of US1 (different block of the same file, so do them in order to avoid conflicts) and changes an existing test (T009). US3 only pins behaviour and needs US1.
- Within a story: tests first (T003/T004, T007, T011) and seen failing, then code.
- Polish after all stories.

## Parallel opportunities

- T003 and T004 (same file, different tests; write together).
- T014 and T015 (different files).
- Little else: the code changes are in one file.

## Implementation strategy

1. MVP: Phases 1-3, then stop and look at it in the window (T018, steps 4 for the plates).
2. Add US2 (removal and the test rewrite), then US3 (pinning tests).
3. Polish and the mutation check last.
