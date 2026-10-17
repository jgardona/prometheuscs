# Feature Specification: Scope glow for the selected component

**Feature Branch**: `002-scope-glow`

**Created**: 2026-10-17

**Status**: Draft

**Input**: User description: "Implementação de osciloscópio glow. Quando selecionamos um componente que está sendo monitorado por um osciloscópio, esse osciloscópio tem um brilho ameno esverdeado (ver imagem anexa). Implementar no prometheuscs a mesma coisa, porém com brilho azulado, nas mesmas proporções da imagem."

## Clarifications

### Session 2026-10-17

- Q: Must the glow also show for scopes placed in the circuit when the simulation runs on its own thread, as in the running program? → A: Yes; the glow is set on the window's copy every frame, independently of where the scope's data comes from, with or without the simulation thread.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See which scope belongs to the selected component (Priority: P1)

A user has a circuit with several scopes open. When they select a component that one of the scopes
is plotting, that scope's whole panel takes on a soft bluish glow, so the matching scope can be
found at a glance. When the component is deselected, the glow goes away.

**Why this priority**: this is the whole feature; it links a part on the circuit with its trace
without reading the scope labels.

**Independent Test**: open a circuit with at least two scopes on different components, click one of
those components and look at the scopes: only the scope that plots it is tinted blue; click empty
space and no scope is tinted.

**Acceptance Scenarios**:

1. **Given** a circuit with a scope plotting a resistor, **When** the user selects that resistor,
   **Then** the scope's panel is drawn with a soft blue tint behind its grid and traces.
2. **Given** the same circuit, **When** the user selects a component no scope plots, **Then** no
   scope is tinted.
3. **Given** a tinted scope, **When** the user clicks empty space, **Then** the tint disappears.
4. **Given** two scopes, only one of them plotting the selected component, **When** the component
   is selected, **Then** only that scope is tinted and the other keeps its normal background.

---

### User Story 2 - Same glow while pointing at a component (Priority: P2)

When the pointer rests over a component that a scope plots (without selecting it), the scope shows
the same glow, as the information box already follows the pointer.

**Why this priority**: it extends the first story to the hover case; useful, but the feature works
without it.

**Independent Test**: move the pointer over a plotted component with nothing selected; its scope is
tinted; move away and the tint goes.

**Acceptance Scenarios**:

1. **Given** nothing selected, **When** the pointer is over a plotted component, **Then** its scope
   is tinted.
2. **Given** a component selected and the pointer over another plotted component, **When** both are
   plotted by different scopes, **Then** both scopes are tinted.

---

### User Story 3 - Glow in every scope arrangement (Priority: P3)

The tint is also shown for scopes that are undocked, resized, in compact mode, or share a panel
with other scopes.

**Why this priority**: consistency; users arrange scopes in many ways.

**Independent Test**: select a plotted component with the scope docked, undocked and compact; the
panel is tinted each time.

**Acceptance Scenarios**:

1. **Given** an undocked scope plotting the selected component, **When** it is drawn, **Then** its
   whole panel is tinted.
2. **Given** several plots in one scope panel, **When** any of them belongs to the selected
   component, **Then** the panel is tinted.

---

### Edge Cases

- Several components selected: no single component is "the" selected one; a scope is tinted only
  if it plots one of the selected components [assumption: any selected component counts].
- A scope plotting two components, one selected: the panel is tinted.
- Light background setting: the tint must stay visible and soft, not hide the traces or text.
- A scope whose component was deleted: never tinted.
- The scope's own selection of a plot (thicker trace) and the cursor are unaffected by the tint.
- A circuit with no scopes: nothing changes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST tint the whole panel of a scope with a soft blue when the scope plots
  a component that is selected.
- **FR-002**: The tint MUST cover the panel background only; grid lines, traces, text and the
  settings icon MUST keep their usual colours and remain readable.
- **FR-003**: The tint MUST have the same strength relative to the normal panel background as the
  green tint in the reference image, with its hue changed to blue.
- **FR-004**: The tint MUST disappear when the component is deselected or the selection changes to a
  component the scope does not plot.
- **FR-005**: The tint MUST also appear while the pointer is over a component that a scope plots.
- **FR-006**: The tint MUST work for docked, undocked, resized and compact scopes, and for scopes
  with more than one plot, in the running program (simulation on its own thread) as well as
  without it.
- **FR-007**: The tint MUST be visible with both the dark and the light background settings.
- **FR-008**: The feature MUST NOT change how scopes plot, scale or lay out, nor the selection and
  hover behaviour of the circuit.

### Key Entities

- **Scope panel**: the rectangle where one scope draws its grid and traces.
- **Plotted component**: a circuit component whose voltage or current a scope shows.
- **Selected component**: a component the user has selected, or is pointing at.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With a plotted component selected, the matching scope is visibly tinted in 100% of
  the cases tested (docked, undocked, compact, multi-plot, dark and light background).
- **SC-002**: With nothing selected and the pointer away, no scope is tinted.
- **SC-003**: A user can identify the scope of a selected component without reading any label, in
  one look.
- **SC-004**: The tint colour differs from the normal panel background by about the same amount as
  in the reference image (a soft, not saturated, glow), and traces stay legible.
- **SC-005**: Selecting and deselecting components does not make the scope area visibly slower.

## Assumptions

- The reference image shows the original program with a green tint over the whole panel of the
  scope that plots the selected resistor; this feature reproduces it in blue.
- "Selected" includes the component under the pointer, as the information box already does.
- With several components selected, a scope is tinted if it plots any of them.
- The exact blue and its strength are chosen to match the proportions of the image (a dark panel
  shifted toward the hue at about one fifth of full intensity); a light-background variant is
  chosen to stay soft.
- Scope behaviour, layout and the information box are unchanged.
- Depends on the existing scope panels and selection/hover state; no change to the file format.
