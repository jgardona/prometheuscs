# Data Model: About Dialog Icon

No persistent data. The feature has two things, both read-only.

## Emblem asset

| Attribute | Value / rule |
|---|---|
| File | `assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg` |
| Icon name | The application id, `org.prometheuscs.App` |
| Shape | Square `viewBox` (for example `0 0 128 128`) |
| Content | Rounded dark badge, flame made of traces, node dots, spark |
| Forbidden | Any `<text>`, `currentColor`, external reference, embedded raster, script |
| Size | Under 8 KB |
| Licence | The project's own artwork, GPL-2.0-or-later |

## Icon choice (runtime)

| Input | Meaning |
|---|---|
| `file_present` | The SVG exists under the assets folder |
| `theme_has_icon` | The icon theme resolves the name after the search path is added |

| Output | Rule |
|---|---|
| Icon name for the dialog | `org.prometheuscs.App` when both inputs are true; `applications-engineering-symbolic` otherwise |

No state transitions. The choice is made each time the dialog opens.
