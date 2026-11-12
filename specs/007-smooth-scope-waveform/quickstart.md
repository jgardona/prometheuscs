# Quickstart: validating the smooth scope waveforms

Prerequisites: Rust toolchain; `cargo build --release --features gui` for the window steps.

1. **Tests first** (Red): `cargo test --release --test scopes smooth` fails on the current
   drawing: the Summing Amplifier crest/trough geometry shows the needle and the uneven crests.
   Record the failure.
2. **Green**: same command passes after the change. Cases: sine at several samples-per-period
   (including 15.6), noise on a sine, one-sample glitch, square wave, constant signal, held
   (triggered) sweep, selected (3 px) plot. Stored min/max compared with the previous values.
3. **Gates**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (also with
   `--features gui`), `cargo test --release` all green.
4. **Performance**: run `cargo bench --bench` / `render_bench` before and after; drawing time grows
   by no more than 10% (SC-004).
5. **In the window**: open *Summing Amplifier*. The 200 Hz scope shows evenly high crests and the
   sum scope a smooth envelope. Change the time per division through several values: no spikes
   appear. Add noise (a noise source) and a short pulse: noise and pulse look as before. Undock the
   scope and enable a trigger: same smooth, held curve.
