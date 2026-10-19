# Research: Color Chooser Inputs

## 1. Widget for the chooser

- **Decision**: Use `gtk::ColorDialogButton` with a `gtk::ColorDialog` (alpha disabled), added as a suffix of an `adw::ActionRow` titled with the field label.
- **Rationale**: It is the platform-standard picker in GTK4, matching the existing use of `gtk::FileDialog` in `dialogs.rs`. It needs GTK 4.10+, which `relm4` with `gnome_47` already requires (to be confirmed with `cargo check --features gui` as the first implementation step).
- **Alternatives**: the deprecated `ColorChooserDialog` (rejected: deprecated since 4.10); a custom swatch-and-popover picker (rejected: more code, against YAGNI).

## 2. Carrying the value through `EditInfo`

- **Decision**: Keep the `is_color` flag and the hex `text` payload (`#rrggbb`). Add a small constructor `EditInfo::color(name, &Color)`. The dialog reads and writes the hex string for `is_color` fields.
- **Rationale**: `is_color` already exists and is set by the text element but nothing uses it. Reusing it needs no change to the `EditInfo` shape or to other elements, and keeps `set_edit_value` code symmetrical.
- **Alternatives**: a new `color: Option<[f64; 3]>` field (rejected: duplicates state and forces both elements to share a 0–1 representation); a new `Field` type per element (rejected: bypasses the generic dialog).

## 3. LED float ↔ hex without drift

- **Decision**: The LED shows `Color` from `color[i] * 255` rounded and clamped to 0–255. On apply it parses the returned hex and stores the new floats (`channel / 255`) **only if the hex differs from the hex of the current color**.
- **Rationale**: Stored values like 0.5 are not multiples of 1/255. Overwriting unconditionally would change them when the user only presses Apply (spec edge case, FR-005, SC-003). The text element already uses the same "compare to current" pattern for its default color.
- **Alternatives**: always overwrite (rejected: drift); store LED color as bytes (rejected: breaks file compatibility).

## 4. Invalid stored values

- **Decision**: Hex that fails `Color::parse_hex` shows the field default (light gray for text, the LED's own color for LED); LED floats are clamped to 0–1 before conversion.
- **Rationale**: Satisfies FR-010 without error dialogs.

## 5. Field indices and sliders

- **Decision (revised during implementation)**: Keep every LED field index stable. Field 0 is the color chooser; fields 1 and 2 (the former green and blue) stay in the list as `EditInfo::hidden()` placeholders that the dialog does not draw and sliders cannot target; Max Brightness Current stays at 3 and the model fields from 4.
- **Rationale**: Saved sliders (`ei` attribute, shared with CircuitJS files) are keyed by field index (`Adjustable.edit_item`). Shifting indices would need a remap at load, and a re-saved file could not be told apart from an old one. Stable indices need no migration. An old slider on the former red/green/blue fields is accepted at load but no longer changes the color (the value is ignored).
- **Alternatives**: shift by 2 with a load-time remap (rejected: the round-trip ambiguity above).

## 6. Labels and translation

- **Decision**: Use the existing `"Color"` key (already translated in all locale files, `Cor` in Portuguese). No new keys.
- **Rationale**: Satisfies FR-009 without translation work. The old "Red/Green/Blue Value (0-1)" keys become unused; leaving them is harmless.

## 7. Other color inputs (FR-003)

- **Decision**: A scan of every `EditInfo::text` and `EditInfo::new` call found only the LED and text-element color inputs. Implementation includes a repeatable check (`grep` for `Color` and `is_color`) and a test that no field named like a color is a plain number or text.
