# Implementation Plan: Color Chooser Inputs

**Branch**: `feature/chage-text-box-for-color-choosers` | **Date**: 2026-10-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-color-chooser-inputs/spec.md`

## Summary

Replace the LED's three red/green/blue number boxes and the text element's hex "Color" box with one color chooser each in the element properties dialog. The `EditInfo` field already has an unused `is_color` flag and a hex `text` payload, so the plan reuses it: the dialog builder renders a color-chooser row for `is_color` fields and returns the choice as a `#rrggbb` string. The LED converts between its stored 0–1 floats and hex at the edit boundary only, so saved files and rendering are untouched.

## Technical Context

**Language/Version**: Rust, edition 2024

**Primary Dependencies**: gtk4 0.11.5, relm4 0.11 (`gnome_47`, which brings GTK 4.10+ for `ColorDialogButton`), libadwaita. No new crates.

**Storage**: Circuit files (text and XML). Format unchanged; LED keeps `cr/cg/cb` floats, text keeps its optional hex string.

**Testing**: `cargo test` (unit tests in `#[cfg(test)]`, integration tests under `tests/`), `cargo clippy -- -D warnings`, `cargo fmt --check`

**Target Platform**: Linux desktop (GTK4/libadwaita)

**Project Type**: desktop-app (single crate; the `gui` feature gates the GTK code)

**Performance Goals**: N/A (dialog-only change)

**Constraints**: Saved-circuit compatibility. A no-op Apply must not alter stored values. The dialog builder lives behind the `gui` feature, so element logic must stay testable without GTK.

**Scale/Scope**: 2 elements known (LED, text) plus a sweep of all `get_edit_info` implementations. A grep found no other color fields.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Note |
|---|---|---|
| I. Code quality gate | PASS | fmt, clippy `-D warnings` apply; no new `unsafe` or `#[allow]`. |
| II. Test-first | PASS | Plan requires failing tests first for the LED and text conversions (see quickstart). |
| III. Layered testing | PASS | Integration test under `tests/` for edit fields; error path (bad hex) tested. |
| IV. Maintainable design | PASS | Reuses existing `is_color`, no new abstraction; conversion helpers are small and documented. |
| V. Dependencies | PASS | No new crates; user docs (`docs/guia-do-usuario.md`) updated in the same change. |

Post-design re-check: PASS, with no new violations.

## Project Structure

### Documentation (this feature)

```text
specs/004-color-chooser-inputs/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── edit-dialog-color-field.md
├── checklists/requirements.md
└── tasks.md             # created by /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── render/painter.rs        # Color: add 0-1 float <-> Color helpers (parse_hex/hex already exist)
├── elements/
│   ├── element.rs           # EditInfo::color(name, &Color) constructor; is_color docs
│   ├── diodes.rs            # LED: 1 color field instead of 3; shift later indices by 2
│   └── outputs.rs           # text: use EditInfo::color
└── app/dialogs.rs           # Field::Color(ColorDialogButton) row for is_color fields

assets/locale/*.txt          # existing "Color" key reused; no new keys
tests/                       # new integration test for LED/text color edit fields
docs/guia-do-usuario.md      # note the chooser
```

**Structure Decision**: Single crate, existing layout. The dialog change is isolated to `src/app/dialogs.rs`; element changes stay in their own files.

## Complexity Tracking

No constitution violations.
