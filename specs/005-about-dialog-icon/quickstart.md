# Quickstart: About Dialog Icon

Validation guide. Contract: [contracts/about-icon.md](contracts/about-icon.md). Data: [data-model.md](data-model.md).

## Prerequisites

- Rust toolchain from the project, GTK4 and libadwaita, a display session.
- `rsvg-convert` for the visual review (optional but recommended).

## 1. Tests first (Principle II)

1. Add `tests/about_icon.rs` and the unit tests in `src/app/icons.rs`, then run:
   ```bash
   cargo test about_icon
   cargo test --lib icons
   ```
   Expected: they fail, because the SVG and the functions do not exist yet.
2. After the SVG and code exist, the same commands pass.

## 2. Gates

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Expected: all pass, no new warnings.

## 3. Visual review of the artwork

```bash
for s in 16 32 64 128; do
  rsvg-convert -w $s -h $s assets/icons/hicolor/scalable/apps/org.prometheuscs.App.svg -o /tmp/icon-$s.png
done
```

Open the images on a dark and a light background. Expected: the flame and the traces are distinguishable at 32 px and larger, the shape reads without colour (view in grayscale), and nothing is clipped.

## 4. About dialog (stories 1 and 2)

1. Run the program and open Help → About.
2. Expected: the new emblem sits above "PrometheusCS" and the version; "Details" and "Legal" are unchanged.
3. Switch the system style to dark, then light, and reopen the dialog. Expected: the emblem is clear both times.
4. Run once with a display scale of 2 (for example `GDK_SCALE=2`). Expected: sharp edges.

## 5. Fallback (FR-012)

1. Run with `PROMETHEUSCS_ASSETS` pointing to a folder without the icon.
2. Open the About dialog. Expected: it opens with the previous generic glyph, no broken-image placeholder.

## 6. Recognition review (SC-001, SC-003)

Show the icon for 10 seconds to at least 5 people who have not seen the design; ask what they see. Expected: at least 4 mention fire and circuits, and at least 4 call it more distinctive than the old glyph.

## 7. Attribution (Story 3)

Check `assets/ATTRIBUTION.md` lists the SVG as the project's own artwork with its licence.
