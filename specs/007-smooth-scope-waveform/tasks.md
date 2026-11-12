---

description: "Task list for smooth scope waveforms"
---

# Tasks: Smooth scope waveforms

**Input**: Design documents from `/specs/007-smooth-scope-waveform/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/curve-drawing.md, quickstart.md

**Tests**: requested: the constitution (II, Test-First) requires tests that fail before the change. Test tasks come first in each story.

**Organization**: grouped by user story. Paths: `src/scope/mod.rs` (scope data), `src/render/scopes.rs` (scope painter, tests at the bottom in `mod tests` with its `view()` helper and `SvgPainter`), `tests/scopes.rs` (integration).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2, US3 (spec.md)

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: a baseline to compare against

- [X] T001 Record the baseline in `specs/007-smooth-scope-waveform/baseline.md`: run `cargo test --release` and `cargo bench --bench render_bench` (or the `render_bench` test target) and note pass/fail counts and the scope drawing time; note that `tests/example_docs.rs` has 3 known failures unrelated to this feature
- [X] T002 [P] Add a test helper in `src/render/scopes.rs` (`mod tests`) that builds a `ScopeView` from samples given as `(min, max)` pairs, extending the existing `view(values)` helper, and a helper that returns the `d` path of the first curve from `SvgPainter::finish()` output

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: carry the end value of each sample from the recorder to the painter without changing any drawing yet (data-model.md). The old drawing must still give the same output after this phase.

**⚠️ CRITICAL**: no user story starts before this is done

- [X] T003 Write a failing unit test in `src/scope/mod.rs` (`mod tests`, next to `scope_on`): record a known sequence of values into a `ScopePlot` and assert `min[k] <= last[k] <= max[k]`, `last[k]` equals the newest value of sample `k`, and `last` is set when a sample starts and after `clear`
- [X] T004 Add `last: Vec<f64>` (`POINTS` long) to `ScopePlot` in `src/scope/mod.rs`; initialise in `new`, reset in `clear`, set to `v` where `min`/`max` are set for the first sample and for each new sample, and update to `v` on every recorded value (data-model.md rules); extend `ordered()` to return the three buffers oldest first and update its callers
- [X] T005 Add `last` to `Sweep` in `src/scope/mod.rs` (tuple `(min, max, last, filled)`), copy it where the sweep is captured (`record`, around the `Sweep {` construction) and use it in `view()` for a held sweep
- [X] T006 Add `pub last: Vec<f64>` to `PlotView` in `src/scope/mod.rs`, fill it in `view()`, and fix every `PlotView { .. }` construction (the `view()` fixture in `src/render/scopes.rs` tests: use `last` equal to the newest value of each sample, i.e. `max` for the existing fixtures)
- [X] T007 Run `cargo test --release` and compare with `baseline.md`: T003 passes and nothing else changed (the drawing still uses `min`/`max` only)

**Checkpoint**: `last` is recorded, held and handed to the painter; output unchanged.

---

## Phase 3: User Story 2 - Real detail is never smoothed away (Priority: P1)

**Goal**: pin down today's behaviour for noise, glitch, edge, constant and max/min so the new drawing cannot lose them (spec FR-003, FR-004, FR-005). Done first because these tests must pass **before and after** the drawing change.

**Independent Test**: all tests below pass on the current drawing and still pass after Phase 4.

- [X] T008 [P] [US2] In `src/render/scopes.rs` (`mod tests`) add test `noise_keeps_its_full_extent`: samples with `min`/`max` that swing beyond both neighbouring end values; assert the drawn path reaches the y of every `min` and every `max` (contracts/curve-drawing.md, "Noise")
- [X] T009 [P] [US2] Add test `a_one_sample_glitch_keeps_its_height`: a flat signal with one sample whose `max` is much higher; assert the path reaches that `max`
- [X] T010 [P] [US2] Add test `a_square_wave_edge_stays_one_sample_wide`: levels -2 and 2; assert each edge goes between the two levels within one sample and no vertex goes beyond either level (no overshoot or ringing)
- [X] T011 [P] [US2] Add test `a_constant_signal_is_a_straight_line` and test `stored_extremes_are_unchanged`: after recording a sine, `ordered()`/`view()` `min`/`max` and the autoscale value equal those computed independently from the raw values (FR-004)
- [X] T012 [US2] Run T008–T011 on the unchanged drawing; they must pass. If one fails because the expectation does not match today's behaviour, fix the test to describe today's behaviour, not the code

**Checkpoint**: a safety net for detail exists and is green.

---

## Phase 4: User Story 1 - A pure sine looks like a pure sine (Priority: P1) 🎯 MVP

**Goal**: crests and troughs of a noiseless sine are even and the curve has no needles or spikes, at every time scale (FR-001, FR-002).

**Independent Test**: open *Summing Amplifier*: even crests on the 200 Hz scope and a smooth envelope on the sum scope.

### Tests for User Story 1 (write first; they MUST fail on the current drawing)

- [X] T013 [P] [US1] In `src/render/scopes.rs` (`mod tests`) add test `a_sine_has_even_crests`: generate the recorder output for a 5 V sine with 64 steps per sample and 15.6 samples per period (the Summing Amplifier case), build the view, draw, and assert every crest vertex of the path is within 1 pixel of the same y and there is no downward stroke at the troughs
- [X] T014 [P] [US1] Add test `crests_stay_even_at_every_time_scale`: repeat T013 for samples per period 6, 10, 15.6, 26, 40, 100 and a phase offset swept over the pixel grid; assert crest spread ≤ 1 pixel and no spike taller than its neighbours (SC-001)
- [X] T015 [P] [US1] Add test `a_sum_of_two_sines_has_a_smooth_envelope`: 200 Hz + 20 Hz square wave as in `assets/circuits/amp-sum.txt`; assert the crest heights follow the real envelope within 1 pixel and no isolated spike exists
- [X] T016 [US1] Run T013–T015 and record the failures in `specs/007-smooth-scope-waveform/baseline.md` (Red; this also confirms research R1)

### Implementation for User Story 1

- [X] T017 [US1] In `src/render/scopes.rs` add a small documented function (for example `curve_points(min, max, last, first, flat) -> Vec<Vertex>`) that builds the vertices exactly as in contracts/curve-drawing.md: end point at the right edge of each sample, plus a vertex at the sample centre for an excursion above or below both neighbouring end values, plus the vertical stroke when both occur; no other changes
- [X] T018 [US1] Replace the loop under `// curves` in `draw_scope_inside` (`src/render/scopes.rs`, the `for k in first_valid..hi` block) with the vertices from `curve_points`, keeping `begin_path`, colour, line width, `first_valid` handling and the `place` mapping as they are
- [X] T019 [US1] Run T013–T015 (now green) and T008–T011 (still green); fix the function, not the tests
- [X] T020 [P] [US1] Add unit tests for `curve_points` alone in `src/render/scopes.rs`: empty input, one sample, first sample (`prev = end`), `flat` threshold boundary, a monotone ramp (no extra vertices)

**Checkpoint**: the reported defect is fixed and the detail tests still pass; this is the MVP.

---

## Phase 5: User Story 3 - Same behaviour in all scope views (Priority: P2)

**Goal**: docked, undocked, resized, multi-plot, selected and triggered scopes all use the new curve (FR-006, FR-007).

**Independent Test**: the same sine is smooth in docked, undocked and held views.

- [X] T021 [P] [US3] In `tests/scopes.rs` add test `a_held_sweep_is_drawn_smooth`: record a sine through a `Scope` with a trigger, wait for the sweep, take `view()`, draw with `SvgPainter` and assert the held curve has even crests; draw twice and assert identical paths (stable, does not move)
- [X] T022 [P] [US3] In `tests/scopes.rs` add test `docked_and_undocked_scopes_agree`: draw the same `ScopeView` at two widths and as a placed scope (`transparent`/`slot` variants of `ScopeSpot`) and assert crests are even in each
- [X] T023 [P] [US3] In `src/render/scopes.rs` (`mod tests`) add test `several_plots_and_the_selected_plot_use_the_same_curve`: two plots with different units and `selected = Some(1)` (3 px line); assert both paths follow the contract
- [X] T024 [US3] Make any test above pass that does not already (the shared drawing function should make most pass with no further change); fix only the path or data they exposed, such as a missing `last` copy

**Checkpoint**: all views behave the same.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T025 [P] Measure drawing time with `render_bench` and compare with `baseline.md`; it must grow by no more than 10% (SC-004, FR-008); write both numbers in `specs/007-smooth-scope-waveform/baseline.md`
- [X] T026 [P] Search `docs/` and `README.md` for text describing how the scope draws the curve (min/max, peak detection) and update it in the same change (constitution V); if none, note that in `baseline.md`
- [X] T027 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and the same with `--features gui`, then `cargo test --release`; fix anything new (the 3 known `tests/example_docs.rs` failures are not from this feature)
- [ ] T028 Walk through `specs/007-smooth-scope-waveform/quickstart.md` step 5 in the window (Summing Amplifier, several time scales, noise source, short pulse, undocked scope with a trigger) and record the result in `baseline.md`

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → Phase 3 (US2 safety net) → Phase 4 (US1) → Phase 5 (US3) → Phase 6.
- Phase 3 must finish before T017–T018: its tests have to be green on the old drawing first.
- Inside Phase 4: T013–T015 → T016 → T017 → T018 → T019; T020 can run beside T017/T018.
- US3 depends on US1 (same drawing); US2 depends only on Phase 2.
- `src/scope/mod.rs` tasks (T003–T006) are sequential; the test tasks in `src/render/scopes.rs` marked [P] touch the same file but different test functions, so they can be written together and merged.

## Parallel Examples

- Phase 3: T008, T009, T010, T011 together.
- Phase 4 tests: T013, T014, T015 together; T020 beside T017.
- Phase 5: T021, T022, T023 together.
- Phase 6: T025, T026 together.

## Implementation Strategy

1. **MVP**: Phases 1–4 (plumbing, safety net, the new curve). It fixes the reported spikes and proves nothing real was lost.
2. Add Phase 5 for the other views, then Phase 6 gates and the performance number.
3. Stop after any checkpoint if needed: each one leaves the build green.
