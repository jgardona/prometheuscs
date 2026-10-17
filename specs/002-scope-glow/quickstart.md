# Quickstart: validating the scope glow

Prerequisites: Rust toolchain; `cargo build --release --features gui` for the window steps.

1. **Tests first** (Red): `cargo test --release --test scope_glow` fails before the code exists
   (or does not compile); record it.
2. **Unit and integration** (Green): same command passes; checks: a scope plotting the selected
   element is drawn with the tint colour, others with black; hover works; none selected → none;
   several selected; placed scope; light background; deleted element index.
3. **Gates**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and the same with
   `--features gui`; `cargo test --release` all green.
4. **In the window**: open *Circuito RLC* from the examples, select the 10 Ω resistor: its scope
   turns blue as in the reference image; click empty space: it goes; hover the capacitor: the
   capacitor's scope glows.
5. **Compare with the reference**: the lift over the plain panel should look like the green one in
   the image; adjust `GLOW_DARK` if not.
6. **Placed scope**: place a scope from the scope menu on a component, select that component: the
   scope in the circuit is tinted.
