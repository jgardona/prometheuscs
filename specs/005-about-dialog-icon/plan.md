# Implementation Plan: About Dialog Icon

**Branch**: `feature/new-icon` | **Date**: 2026-10-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/005-about-dialog-icon/spec.md`

## Summary

Replace the generic theme glyph in the About dialog with an original full-colour emblem: a flame (Prometheus's stolen fire) built from circuit traces and nodes, set on a dark rounded badge so it reads the same on light and dark themes. The emblem is one hand-written scalable SVG stored under the standard icon-theme layout (`assets/icons/hicolor/scalable/apps/`) with the application id as its name. The program adds that folder to GTK's icon search path and asks the About dialog for the icon by name. If the file or the theme lookup fails, the dialog falls back to today's glyph, so it always opens. No new crates.

## Technical Context

**Language/Version**: Rust, edition 2024

**Primary Dependencies**: gtk4 0.11.5, relm4 0.11, libadwaita (`adw::AboutDialog::set_application_icon`, `gtk::IconTheme::add_search_path` and `has_icon`). No new crates.

**Storage**: One new asset file, `assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg`. No settings or circuit-format changes.

**Testing**: `cargo test` (unit tests beside `src/app/icons.rs`, integration test under `tests/`), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`. Visual checks with `rsvg-convert` (installed in the dev environment) and by opening the dialog.

**Target Platform**: Linux desktop (GTK4/libadwaita), dark and light styles, 1x and 2x display scale

**Project Type**: desktop-app (single crate; GTK code is behind the `gui` feature)

**Performance Goals**: No measurable change in the time to open the About dialog (SC-004). The SVG is small (target under 8 KB).

**Constraints**: No text in the artwork; no new dependency; the dialog must still open if the icon is missing; other dialog content unchanged; assets found through `PROMETHEUSCS_ASSETS` or the manifest `assets` folder like all other assets.

**Scale/Scope**: One SVG, about 25 lines of Rust, one test file, one attribution row.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Note |
|---|---|---|
| I. Code quality gate | PASS | fmt and clippy `-D warnings` apply; no `unsafe`, no `#[allow]`. |
| II. Test-first | PASS | Tests for the asset and for the name/fallback choice are written first and fail until the SVG and function exist (see quickstart). |
| III. Layered testing | PASS | Integration test under `tests/` for the shipped file; error path (missing file) covered by a unit test. |
| IV. Maintainable design | PASS | One small function in `icons.rs` decides the icon name; no new abstraction. Public items documented. |
| V. Explicit dependencies | PASS | No crate added. Docs (`assets/ATTRIBUTION.md`) updated in the same change. |

Post-design re-check: still PASS. The design adds no dependency and no `unsafe`.

## Project Structure

### Documentation (this feature)

```text
specs/005-about-dialog-icon/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── about-icon.md
└── tasks.md              # created later by /speckit-tasks
```

### Source Code (repository root)

```text
assets/
├── icons/
│   └── hicolor/scalable/apps/
│       └── org.prometheuscs.App.svg   # new: the emblem
└── ATTRIBUTION.md                     # new row under "Imagens do próprio projeto"

src/app/
├── icons.rs        # app icon name, path, fallback choice, search-path registration
└── dialogs.rs      # about(): uses the icon through icons.rs

tests/
└── about_icon.rs   # shipped SVG is valid, text-free, square, small, attributed
```

**Structure Decision**: Single crate. The icon logic lives in `src/app/icons.rs`, which already owns icon lookup. The artwork follows the freedesktop icon-theme layout, so the same file serves a window icon or a `.desktop` launcher later (spec Story 3) with no rename.

## Complexity Tracking

No constitution violations.
