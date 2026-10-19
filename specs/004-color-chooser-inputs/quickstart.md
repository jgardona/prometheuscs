# Quickstart: Validating Color Chooser Inputs

## Prerequisites

- GTK4 and libadwaita development packages installed (needed for `--features gui`).
- On branch `feature/chage-text-box-for-color-choosers`.

## Automated checks

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo clippy --all-targets --features gui -- -D warnings   # compiles the dialog change
```

Expected: all pass. New tests (written first, per the constitution) cover:
- LED exposes one color field first and Max Brightness Current second ([contract](contracts/edit-dialog-color-field.md)).
- LED float → hex → float: an untouched Apply leaves `0.5` or any non-byte value unchanged; a changed hex stores `channel/255`.
- Out-of-range LED floats are clamped; a malformed text-element hex falls back to the default.
- The text element's default color clears the option back to `None`; a custom color persists through save and load.
- No element field named like a color is a plain number or text box.

## Manual run

```bash
cargo run --features gui
```

1. Place an LED (Draw → Outputs → LED), double-click it. **Expect**: one "Cor" row with a swatch (red by default), no red/green/blue boxes (US1, FR-001).
2. Click the swatch, pick green, confirm, Apply. **Expect**: the LED lights green when the circuit runs (FR-005).
3. Reopen, open the chooser, cancel, Apply. **Expect**: still green (FR-006).
4. Add a text element, open its properties. **Expect**: "Cor" shows light gray, no hex box (US2). Pick blue and Apply: text is blue.
5. Save, close and reopen the circuit. **Expect**: LED and text colors are identical (FR-008).
6. Open a circuit from before this change (for example an example circuit with an LED). **Expect**: the LED color matches what it was before (SC-003).
7. Click through the dialogs of the other elements. **Expect**: no color-holding text or number box remains (SC-001).
