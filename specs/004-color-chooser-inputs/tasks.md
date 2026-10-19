# Tasks: Color Chooser Inputs

**Input**: Design documents from `/specs/004-color-chooser-inputs/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/edit-dialog-color-field.md, quickstart.md

**Tests**: Included. The constitution (Principle II) requires test-first: write each test, see it fail, then implement.

**Organization**: Grouped by user story. Tasks run in the single `prometheuscs` crate; the GTK code is behind the `gui` feature.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [X] T001 Confirm `gtk::ColorDialogButton` and `gtk::ColorDialog` compile with the current features by running `cargo check --features gui`. If they are gated off, enable `v4_10` on the `gtk4` dependency in `Cargo.toml` and note it in the PR (Principle V).
- [X] T002 Read how slider entries are stored and loaded (`SliderEntry` in `src/app/edit.rs`, `apply_sliders`, `shared_slider` in `src/format/parser.rs`) and write the finding into research.md item 5: are sliders keyed by field index, and could an old file hold a slider on LED fields 0–2?

---

## Phase 2: Foundational (blocks all user stories)

**Purpose**: Shared color field support used by both the LED and the text element.

- [X] T003 [P] Write failing unit tests in `src/render/painter.rs` (`#[cfg(test)]`) for new `Color` helpers: float channels 0–1 to `Color` rounds and clamps (`-0.2`→0, `1.5`→255, `0.5`→128), and `Color` to floats divides by 255.
- [X] T004 [P] Write a failing unit test in `src/elements/element.rs` that `EditInfo::color("Color", &Color::RED)` gives `is_color == true`, `text == Some("#ff0000")`, `name == "Color"`.
- [X] T005 Implement the `Color` float helpers from T003 in `src/render/painter.rs` with `///` docs.
- [X] T006 Implement `EditInfo::color(name: &str, color: &Color) -> Self` in `src/elements/element.rs` (text = `color.hex()`, `is_color` set, no sliders, per the contract), and update the `is_color` doc to say it selects a color chooser.
- [X] T007 Add `Field::Color(gtk::ColorDialogButton)` to `src/app/dialogs.rs`. In `edit_element_with`, for `ei.is_color` render an `adw::ActionRow` titled `ls(&ei.name)` with the button as suffix. Initialise it from `ei.text` via `Color::parse_hex`, falling back to light gray; build the `ColorDialog` with alpha off. In the Apply handler write the button's color back to `ei.text` as lowercase `#rrggbb`. Handle the new variant in every `match` on `Field`.

**Checkpoint**: `cargo test` and `cargo clippy --features gui -- -D warnings` pass; no element uses the chooser yet except the text element's existing `is_color` field, which now shows it.

---

## Phase 3: User Story 1 - Pick an LED color with one chooser (Priority: P1) 🎯 MVP

**Goal**: One color chooser in the LED dialog replaces the red/green/blue boxes.

**Independent Test**: Open an LED's properties, see one color row, choose a color, apply, and the LED draws in it.

### Tests for User Story 1 (write first, confirm they fail)

- [X] T008 [P] [US1] Add `tests/led_color_edit.rs`: LED `get_edit_info(0)` is a color field (`is_color`, hex of `color` rounded and clamped), `get_edit_info(1)` is "Max Brightness Current (A)", and field 2 onward are the model fields.
- [X] T009 [P] [US1] In `tests/led_color_edit.rs`: `set_edit_value(0, hex)` with the hex of the current color leaves `color` unchanged for a non-byte value such as `0.5`; a different hex stores `channel / 255`; an unparseable text leaves it unchanged.
- [X] T010 [P] [US1] In `tests/led_color_edit.rs`: Max Brightness Current and a model field still read and write correctly at their new indices 1 and 2+, and an LED with out-of-range stored floats (for example `1.7`, `-0.3`) opens with clamped hex.

### Implementation for User Story 1

- [X] T011 [US1] In `src/elements/diodes.rs` `get_edit_info`, for `DiodeKind::Led` return `EditInfo::color("Color", &c)` for `n == 0` (using the T005 helper on `self.color`), `max_brightness_current` for `n == 1`, and `self.model_edit_info(n - 2)` otherwise. Remove the three Red/Green/Blue `EditInfo::new` entries.
- [X] T012 [US1] In `src/elements/diodes.rs` `set_edit_value`, for the LED: `0` parses `ei.text` with `Color::parse_hex`, and only when it differs from the hex of the current color sets `self.color` from the T005 helper; `1` sets `max_brightness_current`; otherwise `self.set_model_edit(n - 2, ei)`.
- [X] T013 [US1] (Resolved by keeping indices stable: LED fields 1 and 2 stay as hidden placeholders, so no remap is needed.) Apply the T002 finding: if saved sliders are keyed by LED field index, drop or remap those on indices 0–2 (and shift 3+ by 2) at load in the relevant file under `src/format/` or `src/app/edit.rs`, with a test using an old-format LED circuit. If sliders cannot reference these fields, record that in research.md and skip.
- [X] T014 [US1] Run the T008–T010 tests and the existing `tests/compat_load.rs`, `tests/compat_golden.rs` and `tests/format_roundtrip.rs`; all must pass with no change to golden files (SC-003).

**Checkpoint**: Quickstart manual steps 1–3 and 6 work for the LED.

---

## Phase 4: User Story 2 - Pick a text label color with a chooser (Priority: P2)

**Goal**: The text element's "Color" field is a chooser (the hex box is gone).

**Independent Test**: Open a text element's properties, see the chooser, pick a color, apply, and the text draws in it.

### Tests for User Story 2 (write first)

- [X] T015 [P] [US2] Add `tests/text_color_edit.rs`: `get_edit_info(3)` of a text element is a color field showing light gray when `color` is `None` and the stored hex otherwise; a malformed stored hex shows light gray.
- [X] T016 [P] [US2] In `tests/text_color_edit.rs`: applying light gray's hex sets `color` back to `None`; applying `#336699` sets `Some("#336699")`; the value survives a save and reload through the circuit format.

### Implementation for User Story 2

- [X] T017 [US2] In `src/elements/outputs.rs` text element `get_edit_info(3)`, build the field with `EditInfo::color("Color", &c)` where `c` is the stored color parsed with `Color::parse_hex` or `Color::LIGHT_GRAY`; keep `set_edit_value` semantics (default clears to `None`) and ignore unparseable input.
- [X] T018 [US2] Run T015–T016 and `tests/format_roundtrip.rs`.

**Checkpoint**: Quickstart manual steps 4–5 work.

---

## Phase 5: User Story 3 - Every other color input follows the same pattern (Priority: P3)

**Goal**: No properties dialog keeps a text or number box for a color.

**Independent Test**: Automated sweep over every element's edit fields.

- [X] T019 [US3] Write `tests/color_fields_sweep.rs`: for every element class that can be built (see `tests/inventory.rs` and `tests/menu_elements.rs` for how), iterate `get_edit_info(n)` and fail if a field whose name contains "color"/"colour" or "Red/Green/Blue" is not `is_color`.
- [X] T020 [US3] If T019 reports offenders, convert each to `EditInfo::color` the same way as T011–T012 (one task per element, each with its own test first). If it reports none, record that in research.md item 7.

---

## Phase 6: Polish & Cross-Cutting

- [X] T021 [P] Update `docs/guia-do-usuario.md` (Portuguese, like the rest of it) where LED and text properties are described, to mention the color chooser (Principle V).
- [X] T022 Remove now-unused English keys only if no code references them; otherwise leave `assets/locale/*.txt` untouched ("Red/Green/Blue Value (0-1)" are harmless).
- [X] T023 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --all-targets --features gui -- -D warnings`, and `cargo test`; then walk the manual steps in quickstart.md.

---

## Dependencies & Execution Order

- Phase 1 first (T001, T002 independent of each other).
- Phase 2 blocks everything: T003 and T004 in parallel, then T005, T006 (need their tests), then T007 (needs T006).
- US1 (Phase 3) and US2 (Phase 4) both depend only on Phase 2 and can proceed in parallel; they touch different files (`diodes.rs` vs `outputs.rs`).
- US3 (Phase 5) runs after US1 and US2 so the sweep sees their results.
- Polish last.

### Within a story
Tests first and failing, then implementation, then the compat and roundtrip tests.

## Parallel Examples

- Phase 2: T003 ‖ T004.
- US1 tests: T008 ‖ T009 ‖ T010 (same new file, so write as one commit if preferred).
- Across stories: T011–T014 ‖ T015–T018 after T007.

## Implementation Strategy

- **MVP**: Phases 1–3 (the LED chooser, the case in the attached image), which also delivers the shared dialog row.
- Then US2, then the US3 sweep, then Polish. Each phase leaves the tree passing `cargo test`.
