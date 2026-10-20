# Feature Specification: About Dialog Icon

**Feature Branch**: `feature/about-dialog-icon`

**Created**: 2026-10-20

**Status**: Draft

**Input**: User description: "New Icon for About Dialog. We need to create a new icon for about dialog (current in image) that has todo with Prometheus legend and a circuit simulator, and must be stylished."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See a distinctive app icon in the About dialog (Priority: P1)

A user opens the About dialog. Where a generic wrench-and-arch glyph used to sit above the program name, they now see a purpose-made emblem. It combines the Prometheus legend (the Titan who stole fire from the gods and gave it to people) with the idea of an electronic circuit, so that at a glance the icon says "fire + circuits" and belongs to this program, not to a generic toolbox.

**Why this priority**: This is the whole request. The icon is the first thing in the dialog and the program's visual identity; without it there is nothing to deliver.

**Independent Test**: Open the About dialog and compare it with the current one: the generic glyph is gone, the new emblem is shown above the program name and version, and a person who knows the legend can name the reference (fire given to people) while a person who knows electronics can see circuit elements in it.

**Acceptance Scenarios**:

1. **Given** the program is running, **When** the user opens the About dialog, **Then** the new emblem is shown in the icon position above the program name instead of the current glyph.
2. **Given** the new emblem is shown, **When** a viewer looks at it for a few seconds, **Then** they can identify both a fire/flame-bearing motif tied to the Prometheus legend and a circuit motif (for example traces, nodes or component shapes).
3. **Given** the About dialog is open, **When** the user reads the rest of the dialog (name, version, "Details", "Legal"), **Then** nothing else in the dialog has changed.

---

### User Story 2 - The icon looks good in both themes and at any size (Priority: P2)

A user who runs the program with a dark theme, a light theme, or a high-resolution display opens the About dialog and sees an equally clear, sharp icon. The emblem does not blur, get cut off, or disappear into the dialog background.

**Why this priority**: A stylish icon that is unreadable on one theme, or fuzzy on a high-resolution screen, would not meet the "stylised" goal for a large share of users.

**Independent Test**: Open the About dialog under the dark theme, the light theme and at a larger display scale, and confirm the icon is fully visible, has clear edges and keeps its meaning each time.

**Acceptance Scenarios**:

1. **Given** the dark theme, **When** the About dialog opens, **Then** the full emblem is visible with enough contrast against the dialog background.
2. **Given** the light theme, **When** the About dialog opens, **Then** the full emblem is visible with enough contrast against the dialog background.
3. **Given** a display with a larger scale factor, **When** the About dialog opens, **Then** the icon edges stay sharp.

---

### User Story 3 - The icon works as the program's identity elsewhere (Priority: P3)

A maintainer wants to reuse the same emblem later, for example in the window title bar, the repository page or the application launcher. The icon is delivered as a source file in the project that can be reused without redrawing it.

**Why this priority**: Reuse is a convenience for later work, not part of the request, so it comes last.

**Independent Test**: Locate the icon source in the project assets and confirm it can be opened on its own and is listed with its origin and licence in the project's asset attribution file.

**Acceptance Scenarios**:

1. **Given** the project is checked out, **When** a maintainer looks in the assets folder, **Then** the emblem is there as a standalone file.
2. **Given** the asset attribution file, **When** a maintainer reads it, **Then** the new icon is listed as the project's own artwork with its licence.

---

### Edge Cases

- The interface language changes: the icon contains no text, so it needs no translation.
- The icon file is missing or cannot be loaded: the About dialog still opens and shows its name, version and buttons, without a broken-image artefact.
- The dialog is shown in a small window: the icon keeps its proportions and is not stretched or cropped.
- A user with a colour-vision deficiency: the emblem remains recognisable by shape, not only by colour.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The About dialog MUST show the new emblem in place of the current icon, at the same position above the program name.
- **FR-002**: The emblem MUST evoke the Prometheus legend, through a fire or flame motif being carried, held or given.
- **FR-003**: The emblem MUST also evoke an electronic circuit simulator, through circuit elements such as traces, nodes, or component symbols.
- **FR-004**: The two themes MUST be combined into one coherent mark, not two images placed side by side.
- **FR-005**: The emblem MUST have a deliberately designed, polished look (consistent line weight and shapes, a limited colour palette and an intentional composition), not a stock or clip-art look.
- **FR-006**: The emblem MUST be clearly visible and have good contrast on both the dark and the light theme.
- **FR-007**: The emblem MUST stay sharp at the sizes the dialog uses, including on high-resolution displays.
- **FR-008**: The emblem MUST NOT contain text or letters, so that it needs no translation.
- **FR-009**: The emblem MUST be recognisable by shape alone, without relying on colour to tell its parts apart.
- **FR-010**: The emblem MUST be stored in the project as a standalone reusable asset file.
- **FR-011**: The new asset MUST be recorded in the project's asset attribution file as the project's own artwork, with its licence.
- **FR-012**: If the emblem cannot be loaded, the About dialog MUST still open and work.
- **FR-013**: All other content of the About dialog (name, version, details, legal, buttons) MUST stay as it is.

### Key Entities

- **Emblem**: The new graphic mark of the program, combining the Prometheus fire motif with circuit motifs; it appears in the About dialog and can be reused elsewhere.
- **About dialog**: The existing dialog that shows the program name, version and legal information, and where the emblem is displayed.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a quick review with at least 5 people, at least 4 out of 5 name both references (fire/Prometheus and circuits/electronics) after viewing the icon for 10 seconds.
- **SC-002**: The icon is fully visible with no clipped or missing parts in 100% of the checked combinations of dark theme, light theme and normal and high display scales.
- **SC-003**: At least 4 out of 5 reviewers rate the icon as "more distinctive" than the current glyph.
- **SC-004**: Opening the About dialog takes no noticeably longer than before the change.
- **SC-005**: Every piece of text and every control in the About dialog other than the icon is identical before and after the change.

## Assumptions

- The change is limited to the About dialog's icon; the window icon, launcher icon and repository image are not changed here, but the asset is made reusable for them later (Story 3).
- The current glyph is a single-colour symbolic icon, but the new one may use a small number of colours, as long as it works on both themes.
- The artwork is original to this project, so it carries no third-party licence and uses the same licence as the rest of the project.
- The Prometheus reference is deliberately simple and symbolic (fire given to people), not a literal depiction of the myth.
- There is no text in the icon, so no translations are needed.
- Reviewers for the recognition test are people who have not seen the design while it was being made.
