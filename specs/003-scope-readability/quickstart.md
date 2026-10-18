# Quickstart: validating the readable scope labels

Prerequisites: Rust toolchain; `cargo build --release --features gui` for the window steps.

1. **Tests first** (Red): `cargo test --release --test scope_labels` fails on the current drawing
   (no plates, the latest-value line present); record it.
2. **Green**: same command passes after the change; checks: one plate (alpha 0.75, background
   colour) per cursor label; none when the pointer is away; light theme gives white; plates inside
   the scope near the edges; no latest-value line in any scope; the scale text, the cursor line
   and the information area unchanged.
3. **Gates**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and the same with
   `--features gui`; `cargo test --release` all green (the tests that expected the removed line
   are rewritten, see research R6).
4. **In the window**: open *Circuito RLC* or any example with two scopes, move the pointer over a
   trace: the labels sit on dark plates and are readable over the curve and the grid; the top of
   each scope has no "name V = value" line. Switch to the light background (Options): plates are
   white.
5. **Placed scope**: undock a scope into the circuit, enlarge it and hover: same plates, no
   latest-value line.
