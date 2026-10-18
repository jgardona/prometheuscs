# Data Model: Readable scope labels

No stored data and no new state. The labels are computed each frame from what the scope already
has.

| Name | Meaning |
|------|---------|
| Cursor label | one line of text: `"{quantity} = {value}"` for each plot, and `"t = {time}"` |
| Plate | rectangle behind one cursor label: width = measured text width + 6 px, height = 14 px, colour = the scope's background at alpha 0.75 |
| Latest-value line | removed: `"{title} {quantity} = {newest value}"` per plot |

Rules:
- A label whose text does not fit the scope is still drawn as today (clipped); its plate is
  clipped with it.
- The plate is drawn immediately before its text and after the grid and the traces, so it covers
  them; the cursor line is drawn before the plates.
- Alpha is reset to 1.0 after each plate so the text and later drawing are not affected.
