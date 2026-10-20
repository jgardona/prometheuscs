# Research: About Dialog Icon

## R1. How to show a custom icon in the About dialog

- **Decision**: Put the SVG in an icon-theme tree and set it with `adw::AboutDialog::set_application_icon(<name>)`. Register the tree with `gtk::IconTheme::for_display(..).add_search_path(<assets>/icons)`.
- **Rationale**: `AboutDialog` takes an icon-theme *name*, not a file or texture. A search path is the supported way to add icons without installing system-wide, and the project already locates assets by folder (`PROMETHEUSCS_ASSETS` or the manifest folder).
- **Alternatives considered**: Compile the SVG into a GResource (adds a build step and a resource compiler dependency for one file); install to `~/.local/share/icons` at start-up (writes outside the project, needs cleanup); replace the dialog's icon widget with an image (not possible: the dialog owns the header).

## R2. Name and location of the asset

- **Decision**: `assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg`, name = `APP_ID` (`org.prometheuscs.App`).
- **Rationale**: Standard freedesktop layout and the id already registered with GTK, so the same file also works as window icon or launcher icon later (Story 3) without renaming. The folder `hicolor` does not clash with the existing `assets/icons/components` and `assets/icons/ui` folders.
- **Alternatives considered**: `assets/icons/ui/prometheus.svg` (the `ui` folder holds imported Fontello glyphs recoloured at run time; this icon must not be recoloured); a name not tied to the app id (loses the launcher reuse).

## R3. Making it look right on light and dark themes

- **Decision**: Draw the emblem on its own dark rounded-square badge with a fixed palette, and give it no `currentColor` use.
- **Rationale**: A badge makes contrast independent of the dialog background (FR-006) and matches how GNOME application icons look. The existing `themed_svg` recolouring is for single-colour glyphs and must not touch this file.
- **Alternatives considered**: Two files, one per theme (double maintenance, GTK has no per-style full-colour lookup for apps); a symbolic one-colour icon (loses the flame colour; the spec allows colour).

## R4. Design concept

- **Decision**: A stylised flame whose body is drawn as circuit traces (45-degree bends, round via nodes) ending in a few node dots; a small spark detached above it. Palette: badge deep navy; flame amber to orange to red; traces and nodes cool cyan. No letters.
- **Rationale**: Prometheus's gift was fire carried to people, so the flame is the legend; traces and nodes are the circuit-simulator reference; fusing them into one silhouette meets FR-004. Shapes (flame outline versus thin trace lines) carry the meaning, so colour is not required to read it (FR-009). It stays simple enough to survive small sizes.
- **Alternatives considered**: A literal Titan or hand holding a torch (detailed, poor at small sizes, too literal per the spec's assumption); a fennel-stalk torch (accurate to the myth but unreadable to most viewers); a flame beside a resistor symbol (two images side by side, violates FR-004).

## R5. Verifying without new dependencies

- **Decision**: Automated: a test reads the SVG with the project's own XML parser and checks it is well-formed, square `viewBox`, has no `<text>`, no `currentColor`, no external references, and is under 8 KB. Visual: `rsvg-convert` renders it at 16, 32, 64 and 128 px on dark and light backgrounds for a manual review, plus the dialog itself.
- **Rationale**: Adding an SVG rasteriser crate only for tests would break Principle V. The contrast and recognisability goals (SC-001 to SC-003) are human judgements and are checked by the review in the quickstart.
- **Alternatives considered**: Pixel-diff golden images (brittle across librsvg versions).

## R6. Failure behaviour (FR-012)

- **Decision**: Use the emblem only when the file exists and `IconTheme::has_icon` finds it; otherwise keep `applications-engineering-symbolic`.
- **Rationale**: `AboutDialog` would otherwise show a missing-image placeholder. The choice is a pure function of two booleans, so it is unit-testable without GTK.
- **Alternatives considered**: Ignore failure (shows a broken-image icon, violates FR-012).
