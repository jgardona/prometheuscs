# Data Model: Color Chooser Inputs

No stored data changes. This describes the in-memory edit contract.

## Color field (`EditInfo` with `is_color = true`)

| Attribute | Type | Meaning |
|---|---|---|
| `name` | string | Label shown beside the chooser; translated (`"Color"`). |
| `text` | `#rrggbb` string | Current color going into the dialog and chosen color coming back. |
| `is_color` | bool | Selects the color chooser row instead of a text box. |

**Validation**
- Outgoing: always a valid `#rrggbb`. Invalid stored data falls back to the field default.
- Incoming: parsed with `Color::parse_hex`; an unparseable value leaves the element unchanged.
- Alpha is never offered, so there is no alpha channel in the string.

## LED color (existing, unchanged on disk)

| Attribute | Type | Notes |
|---|---|---|
| `color` | `[f64; 3]`, each 0–1 | Written as `cr/cg/cb` (XML) or three numbers (text format). |

**Edit mapping**
- to dialog: `hex = Color::new(round(clamp(c)*255)…)`
- from dialog: if `hex == hex(current)` keep floats; otherwise `c = channel / 255`

## Text element color (existing)

| Attribute | Type | Notes |
|---|---|---|
| `color` | `Option<String>` | `None` means the default light gray; `Some(hex)` otherwise. |

**Edit mapping**: unchanged. A returned hex equal to the default clears the option to `None`.

## State transitions (dialog)

`closed → open (chooser shows current)` → `chooser confirm → pending hex` or `chooser cancel → current kept` → `Apply → element updated` / `dialog closed without Apply → no change`.
