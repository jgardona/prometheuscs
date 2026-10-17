---

description: "Task list for the scope glow"
---

# Tasks: Scope glow for the selected component

**Input**: Design documents from `/specs/002-scope-glow/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ui.md, quickstart.md

**Tests**: included — the constitution (II, Test-First) requires them. Write each test, see it fail, then implement.

**Organization**: grouped by user story. Paths are from the repository root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2, US3 from spec.md

## Phase 1: Setup

- [X] T001 Create the empty test file `tests/scope_glow.rs` (SPDX header, `#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]` with the same comment as `tests/pull_state.rs`) and a helper that builds a `Session` from a small circuit text with one resistor and a scope plotting it, plus a second resistor with its own scope

## Phase 2: Foundational (blocks all stories)

- [X] T002 Add the constants `GLOW_DARK = (14, 28, 60)` and `GLOW_LIGHT = (222, 232, 255)` with `///` docs and a `pub fn glow_color(light: bool) -> Color` in `src/render/scopes.rs`
- [X] T003 Add `pub glow: bool` (documented) to `ScopeSpot` in `src/render/scopes.rs` and fix every constructor: `src/render/scopes.rs` (`draw_scopes`, tests) and `src/elements/scope_elm.rs` (`draw`) pass `glow: false` for now
- [X] T004 In `src/render/scopes.rs` `draw_scope_inside`, when `slot == 0` fill the background with `glow_color(light)` if `glow`, else the plain colour (only the background; grid, traces, text unchanged)

**Checkpoint**: `cargo build` and the existing tests still pass (no visible change yet).

## Phase 3: User Story 1 — selected component lights its scope (P1) 🎯 MVP

**Goal**: selecting a plotted component tints the docked scope that plots it.

**Independent Test**: `cargo test --release --test scope_glow` (US1 tests).

- [X] T005 [US1] Write failing tests in `tests/scope_glow.rs`: with the first resistor selected, `Session::scope_glow()` is `[true, false]`; with nothing selected `[false, false]`; with a component no scope plots, all false; after deselecting, back to all false; a plot whose element index is out of range never matches
- [X] T006 [US1] Write a failing drawing test in `tests/scope_glow.rs` (SVG painter, `draw_scopes` with `ScopeScales.glow = [true, false]`): the glowing scope's background fill is `GLOW_DARK`, the other is black; the light background gives `GLOW_LIGHT`; grid and trace colours are unchanged
- [X] T007 [US1] Add `pub glow: Vec<bool>` (documented) to `ScopeScales` in `src/render/scopes.rs` and make `draw_scopes` pass `glow: scales.glow.get(i).copied().unwrap_or(false)` to the `ScopeSpot`
- [X] T008 [US1] Add `pub fn scope_glow(&self) -> Vec<bool>` to `Session` in `src/app/edit.rs`: the set is `selected_indices()` plus `self.hover`; scope `i` is true when any plot's `elm` is in the set (one entry per `self.sim.scopes`); documented
- [X] T009 [US1] In `src/app/window.rs`, in the scope-area draw function next to `scales.info = s.info_lines();` set `scales.glow = s.scope_glow();`
- [X] T010 [US1] Run `cargo test --release --test scope_glow`; T005 and T006 must now pass

**Checkpoint**: the docked scope glows in the window (quickstart step 4, selection part).

## Phase 4: User Story 2 — hover (P2)

**Goal**: pointing at a plotted component lights its scope as well.

**Independent Test**: `cargo test --release --test scope_glow` (US2 tests).

- [X] T011 [US2] Write failing tests in `tests/scope_glow.rs`: with nothing selected, `hover_at` over the first resistor gives `[true, false]`; moving away gives all false; one resistor selected and the pointer over the other gives `[true, true]`
- [X] T012 [US2] (no code needed: the scope area is redrawn every frame by `scope_tick.queue_draw()`) Make T011 pass (the `hover` part of `scope_glow` from T008; fix only if the tests show a gap) and confirm the window redraws the scope area when the hover changes, in `src/app/window.rs` (queue a redraw of the scope area from the hover handler if it is not already)

**Checkpoint**: quickstart step 4 complete.

## Phase 5: User Story 3 — every scope arrangement (P3)

**Goal**: scopes placed in the circuit (undocked, any size, compact) and scopes with several plots glow too.

**Independent Test**: `cargo test --release --test scope_glow` (US3 tests) and quickstart step 6.

- [X] T013 [US3] Write failing tests in `tests/scope_glow.rs`: a scope with two plots, one on the selected element, glows; a scope placed in the circuit (use the dock/undock path from `tests/scopes.rs`) plotting the selected element is drawn with the tint (SVG of the circuit), and without the selection it is not; compact size (below 160×90) still tinted; **the same placed-scope check with a worker attached** (`Session` with `attach_worker`, frames pumped with `update`, as in `tests/worker_draw.rs`): the placed scope is tinted when its element is selected and not otherwise
- [X] T014 [US3] Add the glow flag to the placed scope in `src/elements/scope_elm.rs` (field, `set_scope_glow(bool)` on the `Element` trait in `src/elements/element.rs` with a default that does nothing, cleared in `reset`); `draw` passes it as `ScopeSpot.glow`
- [X] T015 [US3] Add `Simulation::set_scope_glow(&mut self, glowing: &[usize])` in `src/solver/sim.rs`: for each placed scope (`self.embedded`) call `set_scope_glow(true/false)` on its element according to whether any plot's `elm` is in `glowing`; it only sets the flag and does not touch the views, and `push_embedded_views` keeps its signature. Call it from `Session::pump` in `src/app/engine.rs` on every frame, **with or without a worker** (outside the `if self.worker.is_none()` branch), passing the selected and hovered indices (the same set as `Session::scope_glow`)
- [X] T016 [US3] Run `cargo test --release --test scope_glow`; T013 must pass

## Phase 6: Polish & cross-cutting

- [X] T017 [P] Add a paragraph on the glow to `docs/guia-do-usuario.md` (scopes section) and one line to `README.md` feature list if it lists scope features
- [X] T018 [P] Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and the same with `--features gui`; fix warnings
- [X] T019 Run `cargo test --release` (all green) and `cargo test --test scope_glow` in debug (overflow checks)
- [X] T020 Mutation check: remove the fill in T004 and the `hover` part of T008 in turn; the matching tests must fail; restore both
- [ ] T021 (NOT DONE: needs the real window; the release build is ready) Build with `cargo build --release --features gui`, open *Circuito RLC*, follow quickstart steps 4–6, and compare the tint with the reference image; adjust the constants of T002 if needed (record what was and was not seen in the window)

## Dependencies & Execution Order

- Phase 1 → Phase 2 → stories. T003 before T004, T007.
- US1 (T005–T010) is the MVP and needs Phase 2. US2 needs US1's `scope_glow` (T008). US3 needs T003/T004 only for the drawing, and T015 is called from `pump` on every frame, with or without a worker.
- Within a story: tests first (T005/T006, T011, T013) and seen failing, then code.
- Polish after all stories.

## Parallel opportunities

- T002 and T001 (different files).
- T005 and T006 (same file, different tests; write together).
- T017 and T018.

## Implementation strategy

1. MVP: Phases 1–3, then stop and look at it in the window (T021 steps 4–5 for the docked case).
2. Add US2 (small), then US3.
3. Polish and the mutation check last.
