# UI contract: Readable scope labels

- With the pointer over a scope, each label next to the cursor line (a value line per plot and the
  time) is drawn on a rectangular plate: the scope's background colour at 75% opacity (black on
  the dark theme, white on the light theme). The grid and the traces behind the text are dimmed.
- The text keeps its colour (the colour of the trace for values, the usual text colour for time).
- The plate hugs the text (3 px margin at the sides) and stays inside the scope, also near the
  left, right and bottom edges.
- Label positions are as before.
- No scope shows the top line with the element name and the newest value of a plot. The scale
  text, the time per division, the scope's own text, the information area, the cursor line and the
  cursor labels remain.
- Applies to docked scopes and to scopes placed in the circuit.

Public API: none added (the helper is private).
