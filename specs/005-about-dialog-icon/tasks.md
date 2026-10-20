---

description: "Task list for the About dialog icon"
---

# Tasks: About Dialog Icon

**Input**: Design documents from `/specs/005-about-dialog-icon/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/about-icon.md, quickstart.md

**Tests**: Requested by the constitution (Principle II, test-first), so test tasks come before the code they check.

**Organization**: Tasks are grouped by user story. The emblem is a single file, so story 1 delivers the working feature, story 2 verifies and tunes how it looks, and story 3 covers reuse and attribution.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Paths are relative to the repository root

## Phase 1: Setup

**Purpose**: Make the folder and check the tools used by later tasks

- [X] T001 Create the folder `assets/icons/hicolor/scalable/apps/` (the existing `assets/icons/components` and `assets/icons/ui` stay untouched)
- [X] T002 [P] Confirm `rsvg-convert` is installed for the visual review (`rsvg-convert --version`); if missing, note it in `specs/005-about-dialog-icon/quickstart.md` as the only reason step 3 would be skipped

---

## Phase 2: Foundational

**Purpose**: The names and paths every story relies on

**⚠️ CRITICAL**: Complete before the user stories

- [X] T003 Add to `src/app/icons.rs`: `pub const APP_ICON_NAME: &str = super::consts::APP_ID;` and `pub const FALLBACK_ICON_NAME: &str = "applications-engineering-symbolic";`, each with a `///` doc comment
- [X] T004 Add to `src/app/icons.rs`: `pub fn app_icon_svg(assets: &Path) -> PathBuf` returning `assets/icons/hicolor/scalable/apps/<APP_ICON_NAME>.svg`, documented

**Checkpoint**: Names and the asset path exist; story work can start

---

## Phase 3: User Story 1 - See a distinctive app icon in the About dialog (Priority: P1) 🎯 MVP

**Goal**: The About dialog shows the new flame-and-circuit emblem instead of the generic glyph, and still opens if the emblem is missing.

**Independent Test**: Open Help → About and see the new emblem above "PrometheusCS" and the version; run with `PROMETHEUSCS_ASSETS` pointing to a folder without the icon and see the dialog still open with the old glyph.

### Tests for User Story 1 (write first, see them fail) ⚠️

- [X] T005 [P] [US1] Add unit tests in the `tests` module of `src/app/icons.rs`: `app_icon_choice(true, true)` is `APP_ICON_NAME`; every other combination (`(false, true)`, `(true, false)`, `(false, false)`) is `FALLBACK_ICON_NAME`; `APP_ICON_NAME == consts::APP_ID`; `app_icon_svg` ends with `icons/hicolor/scalable/apps/org.prometheuscs.App.svg`
- [X] T006 [P] [US1] Create `tests/about_icon.rs` (with the SPDX header and the same `#![allow(...)]` test preamble as `tests/model_dialogs.rs`) with a test that the file at `app_icon_svg(&assets_dir())` exists, parses with `prometheuscs::format::xml::parse`, has root tag `svg`, a square `viewBox`, no `<text>` element, no `currentColor`, no `href` to another file or `data:` raster, and is under 8192 bytes (data-model.md "Emblem asset")
- [X] T007 [US1] Run `cargo test --lib icons` and `cargo test about_icon`; confirm they fail for the right reasons (missing function, missing file)

### Implementation for User Story 1

- [X] T008 [US1] Draw `assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg`: square `viewBox="0 0 128 128"`, a dark navy rounded-square badge, a flame silhouette built from circuit traces with 45-degree bends and round via nodes (amber to orange to red-orange flame, cyan traces and nodes), a small detached spark, consistent line weight, no text, no `currentColor`, no raster data, under 8 KB (research.md R4)
- [X] T009 [US1] Implement `pub fn app_icon_choice(file_present: bool, theme_has_icon: bool) -> &'static str` in `src/app/icons.rs`, documented, returning `APP_ICON_NAME` only when both are true (data-model.md "Icon choice")
- [X] T010 [US1] Implement `pub fn install_app_icon(display: &gdk::Display, assets: &Path) -> &'static str` in `src/app/icons.rs` (behind the `gui` feature like the other GTK code there, if the file needs it): add `<assets>/icons` to `gtk::IconTheme::for_display(display)` once per run, then return `app_icon_choice(app_icon_svg(assets).is_file(), theme.has_icon(APP_ICON_NAME))`
- [X] T011 [US1] In `about()` in `src/app/dialogs.rs`, replace the literal `"applications-engineering-symbolic"` with the name returned by `install_app_icon` for the parent widget's display and `consts::assets_dir()`; leave the name, version, comments, licence, copyright and website lines unchanged (FR-013)
- [X] T012 [US1] Run `cargo test --lib icons` and `cargo test about_icon`; confirm they pass
- [X] T013 [US1] Manually open Help → About and check the emblem is shown; then run once with `PROMETHEUSCS_ASSETS` set to an empty folder and check the dialog opens with the previous glyph (quickstart steps 4 and 5)

**Checkpoint**: The emblem shows in the dialog and the fallback works

---

## Phase 4: User Story 2 - The icon looks good in both themes and at any size (Priority: P2)

**Goal**: The emblem is clear and sharp on dark and light styles, at small sizes and on 2x displays.

**Independent Test**: Open the About dialog on dark and light styles and at display scale 2; the emblem is complete, has good contrast and sharp edges. Rendered at 32 px the flame and traces are still distinguishable and the shape reads in grayscale.

- [X] T014 [US2] Render the SVG with `rsvg-convert` at 16, 32, 64 and 128 px into the scratchpad directory, view them on a dark and a light background and in grayscale (`magick <png> -colorspace Gray <out>`), and list any problems (clipping, merged shapes, low contrast) in `specs/005-about-dialog-icon/quickstart.md` notes (quickstart step 3, FR-006, FR-009)
- [X] T015 [US2] Adjust `assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg` to fix what T014 found (stroke widths, spacing, colours) and re-run `cargo test about_icon` to confirm the file still meets its limits
- [ ] T016 [US2] Open the About dialog under the dark style, the light style, and with `GDK_SCALE=2`; confirm the emblem is fully visible and sharp each time (quickstart step 4, FR-007)
- [ ] T017 [US2] Run the recognition review of quickstart step 6 with at least 5 people (SC-001, SC-003) and record the counts in `specs/005-about-dialog-icon/quickstart.md`; if fewer than 4 of 5 name fire and circuits, return to T015

**Checkpoint**: The icon meets the visual goals on both themes and at all sizes

---

## Phase 5: User Story 3 - The icon works as the program's identity elsewhere (Priority: P3)

**Goal**: The emblem is a standalone, attributed asset that can be reused for a window or launcher icon later.

**Independent Test**: Find the SVG in `assets/` as its own file and see it listed in `assets/ATTRIBUTION.md` as the project's own artwork with its licence.

- [X] T018 [P] [US3] Add a test to `tests/about_icon.rs` that reads `assets/ATTRIBUTION.md` and asserts it mentions `org.prometheuscs.App.svg` (write it first and see it fail)
- [X] T019 [US3] Add a row to the "Imagens do próprio projeto" table in `assets/ATTRIBUTION.md` for `assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg`: original artwork of the project, GPL-2.0-or-later (written in Portuguese like the rest of that file)
- [X] T020 [US3] Run `cargo test about_icon` and confirm the attribution test passes

**Checkpoint**: The asset is standalone and attributed

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T021 [P] Mention the icon location in `docs/arquitetura.md` where assets are described (one short sentence in Portuguese, like the rest of the file), as Principle V asks for docs in the same change
- [X] T022 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`; fix anything reported
- [X] T023 Confirm every item in `specs/005-about-dialog-icon/quickstart.md` is done and that nothing else in the About dialog changed (SC-005)

---

## Dependencies & Execution Order

### Phase dependencies

- Setup (Phase 1) has no dependencies.
- Foundational (Phase 2) needs T001 and blocks the stories.
- User Story 1 (Phase 3) needs Phase 2.
- User Story 2 (Phase 4) needs T008 and T011 (the emblem exists and shows).
- User Story 3 (Phase 5) needs T008; T018 and T019 can run any time after it.
- Polish (Phase 6) needs the stories you ship.

### Within User Story 1

- T005 and T006 first (parallel), then T007 to see them fail.
- T008 (artwork) and T009 can run in parallel; T010 needs T009; T011 needs T010.
- T012 and T013 come last.

### Parallel opportunities

- T002 beside T001.
- T005 and T006 together (different files).
- T008 beside T009 (an SVG and a Rust function).
- T018 beside T014 to T017 once the SVG exists.
- T021 beside T022.

## Parallel Example: User Story 1

```text
Task: "Add unit tests in src/app/icons.rs"            (T005)
Task: "Create tests/about_icon.rs"                    (T006)

Task: "Draw org.prometheuscs.App.svg"                (T008)
Task: "Implement app_icon_choice in src/app/icons.rs" (T009)
```

## Implementation Strategy

### MVP first (User Story 1 only)

1. Phases 1 and 2.
2. Phase 3: tests first, then the artwork, the two functions and the dialog change.
3. Stop and check: the emblem shows, the fallback works, `cargo test` passes.

### Incremental delivery

1. MVP (story 1): the new icon is visible.
2. Story 2: tune and review it on both themes and sizes (may send the artwork back for changes).
3. Story 3: attribution and docs, so the asset can be reused.

## Implementation Notes

- **T010 deviation**: `icons.rs` is not behind the `gui` feature and has no GTK code, so the search-path registration lives in `src/app/dialogs.rs` as the private `about_icon(display, assets)`; it uses `icons::app_icon_choice`, `icons::app_icon_svg` and `icons::APP_ICON_NAME`. `icons.rs` stays testable without GTK.
- **T005/T006/T007**: the unit tests passed on the first run because the pure functions (T003, T004, T009) were written together with them; the integration test failed first (missing SVG), as intended.
- **T013**: checked by running the real `about()` in a throwaway program (removed afterwards) and saving the window as an image, in dark and light styles and with an empty `PROMETHEUSCS_ASSETS` folder (the old glyph appears).
- **T014/T015**: rendered at 16, 32, 64 and 128 px on dark and light backgrounds and in grayscale. The flame and the trace tree are distinguishable from 32 px; at 16 px only the flame silhouette reads. One change: a faint light outline on the badge so it stays visible on the dark dialog.
- **T016 not complete**: dark and light styles were checked. A display scale of 2 could not be tested here (`GDK_SCALE=2` had no effect, the window stayed at scale 1). The asset is a vector SVG, so it scales, but this was not seen on screen.
- **T017 not done**: the recognition review needs at least 5 people who have not seen the design; no one has reviewed it yet (SC-001, SC-003).
- **T022**: `cargo fmt` also reformatted three already-committed files (`src/app/edit.rs`, `src/app/settings.rs`, `src/render/scopes.rs`), formatting only, so that `cargo fmt --check` passes.
