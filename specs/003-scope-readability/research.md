# Research: Readable scope labels

## R1. What the arrows in the reference point at

- **Decision**: red arrows = the cursor labels drawn by `draw_cursor` (`V = …`, `t = …`); blue
  arrow = the "readouts" block of `draw_scope_inside` (one line per plot,
  `"{title} {quantity} = {newest value}"`, centred at the top of the scope).
- **Evidence**: `src/render/scopes.rs` `draw_cursor` (cursor lines and the time) and the
  `// readouts` block that formats `p.title`, `value_name` and the last sample.
- **Alternatives**: none; the texts in the screenshot match these two places exactly.

## R2. Plate colour and opacity

- **Decision**: the plate is the scope's background colour (`bg`: black on the dark theme, white
  on the light theme) at alpha 0.75, drawn with `set_alpha(0.75)` then `fill_rect`, with alpha put
  back to 1.0 before the text.
- **Rationale**: this is what the user asked for ("matching the panel colour", 75%); the backends
  already take an alpha (GL, Cairo, SVG).
- **Alternatives**: a fixed dark grey (does not match the light theme); a border around the plate
  (extra visual weight, not asked).

## R3. Plate geometry

- **Decision**: a rectangle from the text's measured width and the 11 px font: 3 px of margin left
  and right, from the top of the text (baseline − 11) to 3 px below the baseline, clamped to the
  scope's rectangle by moving the text, not by cutting the plate.
- **Rationale**: the scope already clips to its rectangle (`g.clip_rect`), so nothing escapes; the
  clamp keeps the plate whole and the text readable near the edges.
- **Alternatives**: one plate for the whole group — wastes space between lines and hides more of
  the trace; chosen: one plate per line.

## R4. Where the labels go

- **Decision**: unchanged (FR-005): to the right of the cursor line when the pointer is in the left
  half, to the left otherwise; value lines stack from the top, the time at the bottom.
- **Rationale**: spec; the plate only adds a background.

## R5. Removing the latest-value line

- **Decision**: delete the `// readouts` block (its formatting, the `fits` computation and the
  loop) in `draw_scope_inside`. `left_w` is still needed by other text; nothing else depends on
  the removed lines.
- **Rationale**: the same values are in the information area and in the cursor labels (user's
  clarification). The element name leaves with it (clarified).
- **Alternatives**: keep only the name — rejected by the user.

## R6. Tests that expect the removed line

- **Decision**: the existing tests that look for ` V =` in a drawing of a placed scope
  (`tests/scopes.rs`, "a scope in the circuit shows readouts once it is big enough") assert the
  removed line; they are rewritten to look for something that stays (the scale text, or the
  cursor labels with the pointer on the scope).
