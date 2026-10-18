# Feature Specification: Readable scope labels

**Feature Branch**: `003-scope-readability`

**Created**: 2026-10-17

**Status**: Draft

**Input**: User description: "We must enhance visibility of the data on oscilloscope. 1. On the labels pointed by red arrows: draw a dark, slightly transparent rectangle (about 75% opacity, matching the panel colour, a white version on the light theme) behind the label text. The text keeps its trace colour, but the grid and trace no longer show through. 2. The label pointed by the blue arrow is redundant. We can remove it."

## Clarifications

### Session 2026-10-17

- Q: Should the element name leave the top of the scope together with the latest-value line, given that it is no longer shown there? → A: Yes, remove it: the information area at the right of the scopes already shows that information (the element's name and values).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read the values under the pointer (Priority: P1)

A user moves the pointer over a scope. A vertical cursor line follows it, and next to it the scope
shows the value of each plotted signal at that moment and the time. Today these labels are drawn
straight over the traces and the grid, in the same colour as the trace, so they are hard to read
where they cross a curve. Each label now sits on a dark, slightly transparent plate, so it can be
read wherever the cursor is.

**Why this priority**: this is the label the user reads most, and the one that is hardest to read.

**Independent Test**: open a circuit with a scope, hover the middle of a trace: every cursor label
(the value of each plot and the time) is drawn on a plate and is readable over the curve and the
grid.

**Acceptance Scenarios**:

1. **Given** a scope with a plotted trace and the pointer over it, **When** the cursor labels
   appear, **Then** each label (a value line per plot, and the time line) has a plate behind its
   text that covers the grid and the trace under the text.
2. **Given** the dark theme, **When** a label is drawn, **Then** its plate is dark and slightly
   transparent (about 75% opaque) and its text keeps the colour of its trace; the time label keeps
   its own text colour.
3. **Given** the light theme, **When** a label is drawn, **Then** its plate is white and slightly
   transparent, and the text is still readable.
4. **Given** the pointer in the left half of a scope, **When** the labels appear, **Then** they are
   on the right of the cursor line, and on the left of it when the pointer is in the right half,
   as today, with the plate following the text.

---

### User Story 2 - A scope without the redundant value label (Priority: P2)

At the top of each scope there is a line of text per plot with the name of the element, the
quantity and its latest value (for example "A/C source V = -494.136 mV"). The same value is
already shown in the information area and, with the pointer on the scope, in the cursor labels, so
the line only adds clutter over the curves. It is removed.

**Why this priority**: it clears the upper part of the scope for the traces; it does not change
what can be read elsewhere.

**Independent Test**: open a circuit with scopes: no scope shows the line with the element name
and the latest value; the scale, the time per division and the information area are unchanged.

**Acceptance Scenarios**:

1. **Given** a scope with one or more plots, **When** it is drawn, **Then** no line with the
   element name and the latest value of a plot appears in it.
2. **Given** the same scope, **When** the pointer is over it, **Then** the cursor labels (story 1)
   and the information area still show the values.
3. **Given** a scope with a text of its own (a name set by the user), **When** it is drawn,
   **Then** that text is still shown.

---

### User Story 3 - Labels stay tidy at the edges and in small scopes (Priority: P3)

The plates never spill outside the scope, never hide the settings wheel or the scale text, and
small scopes keep working.

**Why this priority**: it avoids regressions in the many arrangements of scopes (stacked, side by
side, compact, placed in the circuit).

**Independent Test**: hover each corner and edge of docked, stacked, compact and placed scopes: the
plates are inside the scope and the labels stay readable.

**Acceptance Scenarios**:

1. **Given** the pointer near the right or left edge, **When** the labels appear, **Then** the
   plate stays inside the scope.
2. **Given** two scopes stacked in one column, **When** the pointer is over one of them, **Then**
   the plates do not leave their scope.
3. **Given** a compact scope (a small one placed in the circuit), **When** the pointer is over it,
   **Then** the labels follow today's rules about when they are shown and are readable.

---

### Edge Cases

- Several plots in one scope: one plate per line or one plate for the group; in both cases every
  line is covered and none overlaps another.
- A very long label (large number with units) wider than half the scope: it is clipped by the
  scope as today, and the plate with it.
- The pointer on the first or last sample: the labels stay on the side with room.
- A spectrum (FFT) or X/Y scope: their own readouts are not changed by this feature.
- The measurement labels shown while dragging to measure a stretch of the signal (the time and the
  change in value): they get the same plates as the other labels.
- A scope too small for the labels: they are left out as today, with no empty plate.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every label that follows the pointer on a scope (the value of each plot, the time,
  and the time and value change shown while dragging to measure) MUST be drawn on a plate that
  hides the grid and the traces behind the text.
- **FR-001a**: The scale labels drawn over the curves (the values in the top and bottom left
  corners, the maximum and other measurements under them, and the time per division in the bottom
  right corner) MUST be drawn on plates in the same way.
- **FR-002**: The plate MUST be dark and about 75% opaque on the dark theme, and white and about
  75% opaque on the light theme.
- **FR-003**: The text of each label MUST keep its colour: the colour of its trace for the values,
  the usual text colour for the time.
- **FR-004**: The plate MUST fit the text with a small margin and MUST stay inside the scope.
- **FR-005**: The labels' position rules (side of the cursor line, vertical stacking) MUST NOT
  change.
- **FR-006**: The line of each scope showing the element name, the quantity and the latest value
  of every plot MUST NOT be drawn.
- **FR-007**: Removing that line MUST NOT remove the scale text, the time per division, the
  scope's own text, the information area, the cursor line or the cursor labels.
- **FR-008**: Scopes placed in the circuit and docked scopes MUST both follow FR-001 to FR-007.

### Key Entities

- **Cursor label**: a short text near the pointer on a scope: the value of one plot, the time, or
  one of the two lines of a measurement.
- **Plate**: the translucent rectangle behind a cursor label or a scale label.
- **Latest-value line**: the line at the top of a scope with the element name and the newest value
  of a plot (removed).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of the tested positions of the pointer over a trace, each cursor label has a
  plate and the text is readable (the text colour differs clearly from what is behind it).
- **SC-002**: No scope shows the latest-value line, in all tested arrangements (docked, stacked,
  side by side, placed in the circuit, compact).
- **SC-003**: The plate colour and opacity match the panel colour at about 75% opacity, on both
  themes.
- **SC-004**: The plates never extend outside their scope in any tested pointer position.
- **SC-005**: A user can read the value and the time under the pointer in a scope with two
  overlapping traces without moving the pointer to a quieter spot.

## Assumptions

- The red arrows in the reference screenshot point at the cursor labels ("V = …" and "t = …"),
  and the blue arrow at the line "A/C source V = …" at the top of the first scope; the same kind of
  line in the second scope ("output V = …") is removed too.
- Removing the latest-value line also removes the element name from the top of the scope. The
  user confirmed this: the information area at the right of the scopes shows that information, so
  nothing is lost.
- "About 75% opaque" and "matching the panel colour" are taken from the user's wording: the dark
  plate uses the scope's background colour, the light plate white.
- The measurement labels (the time and value differences shown while dragging) are included: the
  user asked for them after the first version (2026-10-17).
- The scale labels were added after the first version, at the user's request (2026-10-17). The
  scale line of manual-scale mode (with its coloured dots) is not changed.
- No change to the file format, the simulation, the scope's scales or the information area.
- Depends on the existing scope drawing; no new settings.
