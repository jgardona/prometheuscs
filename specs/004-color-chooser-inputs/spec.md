# Feature Specification: Color Chooser Inputs

**Feature Branch**: `feature/chage-text-box-for-color-choosers`

**Created**: 2026-10-19

**Status**: Draft

**Input**: User description: "In this feature, we will replace all textboxes that are being used to store color information for a color-chooser (attached image). When we have 3 text/input-boxes for color, we can replace with only one color chooser. When we have 1 text/input-box for color (store a hexadecimal color representation), change it for one color chooser also."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pick an LED color with one chooser (Priority: P1)

A user opens the properties dialog of an LED. Today the color is set through three separate numeric boxes (red, green and blue, each 0–1). Instead, the user sees a single color chooser showing the LED's current color. They click it, pick a color visually, and confirm. The LED in the circuit immediately uses the new color.

**Why this priority**: This is the case shown in the attached image and replaces the most cumbersome input (three boxes, values that must be converted by hand from 0–1 ranges).

**Independent Test**: Open an LED's properties, confirm only one color control is shown in place of the three boxes, choose a color, apply, and verify the LED renders in that color.

**Acceptance Scenarios**:

1. **Given** an LED with color (1, 0, 0), **When** the user opens its properties, **Then** a single color chooser is shown displaying red and the three red/green/blue boxes are no longer present.
2. **Given** the LED properties dialog is open, **When** the user picks a new color and applies the dialog, **Then** the LED is drawn with that color.
3. **Given** an LED saved in an existing circuit file, **When** the circuit is loaded and the properties opened, **Then** the chooser shows the same color the LED had before this feature.

---

### User Story 2 - Pick a text label color with a chooser (Priority: P2)

A user opens the properties dialog of a text element. The "Color" field, which currently expects a hexadecimal value typed by hand, is replaced by a color chooser. The user picks a color and the text is drawn in that color.

**Why this priority**: Same improvement for the single-box case; lower priority because typing a hex code already works.

**Independent Test**: Open a text element's properties, confirm the hex text box is replaced by a color chooser, pick a color, apply, and verify the text color changes.

**Acceptance Scenarios**:

1. **Given** a text element with a custom color, **When** the user opens its properties, **Then** the color chooser shows that color.
2. **Given** a text element with no custom color set, **When** the user opens its properties, **Then** the chooser shows the default text color currently used.
3. **Given** the user picks a new color and applies, **Then** the text element is drawn in the new color and the choice persists when the circuit is saved and reopened.

---

### User Story 3 - Every other color input follows the same pattern (Priority: P3)

Any other property dialog in the application that asks for a color via text boxes (three numeric components or one hex string) uses the same color chooser, so color entry is consistent everywhere.

**Why this priority**: Completes the "replace all" intent of the request and guarantees consistency, but depends on the two known cases being done first.

**Independent Test**: Review every element's properties dialog; none contains a text/numeric box whose purpose is to hold a color.

**Acceptance Scenarios**:

1. **Given** any element with a color property, **When** its properties dialog is opened, **Then** the color is edited through a color chooser and not through text boxes.

---

### Edge Cases

- The user opens the chooser and cancels: the previous color is kept unchanged.
- An existing circuit contains a color value that cannot be interpreted (e.g. a malformed hex string): the dialog opens with a sensible default color and does not fail.
- An LED color component stored outside the 0–1 range: it is clamped to the valid range when shown.
- The user applies the dialog without touching the chooser: the color is unchanged (no accidental rounding or shift).
- Chosen colors with transparency: only opaque colors are supported; alpha is not offered.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The LED properties dialog MUST present one color chooser in place of the red, green and blue value boxes.
- **FR-002**: The text element properties dialog MUST present one color chooser in place of the hexadecimal color text box.
- **FR-003**: Every other property dialog that currently uses text or numeric boxes to hold a color MUST use a color chooser instead.
- **FR-004**: Each color chooser MUST display the element's current color when the dialog opens, including the default color when none was set.
- **FR-005**: Applying a color chosen in the chooser MUST update the element so it is drawn in exactly that color.
- **FR-006**: Cancelling or closing the chooser without confirming MUST leave the element's color unchanged.
- **FR-007**: Colors from existing saved circuits MUST load and display unchanged; the circuit file format and previously saved circuits MUST remain compatible.
- **FR-008**: Colors chosen through the chooser MUST be saved with the circuit and restored identically on reload.
- **FR-009**: The color chooser MUST carry the same label as the field it replaces (translated like other dialog labels) so the user knows which color is being edited.
- **FR-010**: Invalid or out-of-range stored colors MUST fall back to a valid color without error when the dialog is opened.

### Key Entities

- **Color property**: A color attribute of an element (e.g. LED color, text color). Has a label, a current value and, for text, an optional "unset/default" state.
- **Properties dialog field**: One row in an element's properties dialog. Color properties are shown as a single color-chooser row rather than one or three text/numeric rows.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 0 properties dialogs contain a text or numeric box used to hold a color after this feature.
- **SC-002**: A user can change an LED's color in 3 actions or fewer (open chooser, pick, confirm) compared with editing 3 separate fields today.
- **SC-003**: 100% of circuits saved before this feature load with every LED and text color displayed identically.
- **SC-004**: Round-trip (choose color, save, reload) preserves the chosen color for 100% of tested colors, with no visible difference.

## Assumptions

- Only the two color inputs identified (LED red/green/blue, text hexadecimal) plus any further color-holding text/numeric inputs found during a full review of the elements' dialogs are in scope.
- The platform's standard color-picking dialog is acceptable as the "color chooser"; a custom-built picker is not required.
- Alpha/transparency is out of scope because current colors are opaque.
- The existing saved-circuit representation of colors is kept; only how users enter the color changes.
- Labels already shown in the dialog (Portuguese UI strings) are reused for the chooser rows.
