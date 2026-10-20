# Contract: About dialog icon

## UI contract

- The About dialog shows the emblem in its usual icon position, above the program name and version.
- Nothing else in the dialog changes: name, version, comments, licence, copyright, website, the "Details" and "Legal" rows.
- The emblem is fully visible, uncropped and sharp on the dark and light styles and at 1x and 2x scale.
- If the emblem cannot be found or loaded, the dialog opens normally with the previous generic glyph.

## Asset contract

- Path: `assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg`.
- Scalable SVG, square `viewBox`, no text, no `currentColor`, no external references, no raster data, under 8 KB.
- Listed in `assets/ATTRIBUTION.md` under "Imagens do próprio projeto" as the project's own artwork, GPL-2.0-or-later.

## Code contract (`src/app/icons.rs`)

| Item | Behaviour |
|---|---|
| `APP_ICON_NAME` | Equals `consts::APP_ID` |
| `app_icon_svg(assets)` | Returns the path of the SVG under `assets` |
| `app_icon_choice(file_present, theme_has_icon)` | Returns `APP_ICON_NAME` when both are true, else the fallback glyph name |
| `install_app_icon(display, assets)` | Adds `<assets>/icons` to the icon search path once per run and returns the name to pass to the dialog |
