# Contract: Color field in the element edit dialog

The only external interface touched is the element properties dialog and the `Element` edit-field API it is built from.

## Element side (`get_edit_info` / `set_edit_value`)

- `get_edit_info(n)` returns `EditInfo::color(label, &Color)` for a color property: `is_color == true`, `text == Some("#rrggbb")`, `name == label`.
- `set_edit_value(n, ei)` receives `ei.text == Some("#rrggbb")` for that field. An element MUST ignore a value that does not parse and MUST NOT change stored data when the value equals the hex of the current color.

## Dialog side (`edit_element_with`)

- For a field with `is_color`, show one row: title = translated `name`, a color-chooser button initialised from `text`.
- On Apply the field's `text` becomes the chooser's current color as lowercase `#rrggbb`.
- A cancelled chooser leaves the row's color as it was.
- A `text`that does not parse is shown as light gray, and Apply returns that gray only if the user picked it.
- No text box, number box, or slider is offered for the field.

## Field layout for the LED

| Index | Before | After |
|---|---|---|
| 0 | Red (0–1) | Color (chooser) |
| 1 | Green (0–1) | hidden placeholder |
| 2 | Blue (0–1) | hidden placeholder |
| 3 | Max Brightness Current | unchanged |
| 4+ | model fields | unchanged |

A hidden field (`EditInfo::hidden()`) has no row in the dialog and cannot have a slider.

## Compatibility

Circuit files are not affected, and saved slider indices keep their meaning (see research item 5).
