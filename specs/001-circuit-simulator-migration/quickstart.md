# Quickstart: Validating the Migration

Run these checks from the repository root. They map to the success criteria in [spec.md](spec.md). Some commands exist only once the corresponding tasks are implemented.

## Prerequisites

- Rust 1.99+ (edition 2024), GTK 4 (4.12+; 4.22 installed), libadwaita 1.x development files, an OpenGL-capable session (software GL works).
- The reference project at `/home/jgardona/Workspace/Rust/circuitjs1` (read-only).
- For regenerating golden data only: the built original at `/home/jgardona/Bin/circuitjs1/circuitjs1` (fallback: Java, Node and Firefox to build it from source).

## 1. Quality gates (constitution I)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --features gui -- -D warnings
cargo test
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

Expected: all pass with no warnings. `cargo test` includes the documentation examples of the engine (`src/lib.rs`, `src/solver/mod.rs`).

## 2. Import assets

```bash
# only to regenerate: both read the original's sources at /home/jgardona/Workspace/Rust/circuitjs1
# and rewrite files in assets/ (already committed, so skip them for a normal check)
cargo run --bin import_assets
cargo run --bin inventory
```

Expected: 373 circuit files and the example catalog present; the inventory lists 153 element classes with no unmapped entries.

## 3. Engine parity (FR-008, SC-001)

```bash
cargo test --release --test us1_rlc --test auto_fixtures      # the recorded auto-tests
cargo test --release --test compat_load --test compat_golden  # all 373 examples: load, round trip, golden data
```

Expected: every example loads (G1), round-trips (G2), and matches golden results within tolerance. Differences the original is known to show are listed in `tests/compat/known_differences.json` and the nondeterministic circuits in `tests/compat/nondeterministic.json`. Recordings are regenerated with `cargo run --release --bin golden` (see `tests/compat/README.md`).

## 4. Component completeness (SC-010)

```bash
cargo test --test inventory
```

Expected: zero inventory entries without an implementation.

## 4b. Modularity (SC-012, SC-013)

```bash
cargo test --test modularity
cargo build --no-default-features --features ""            # no element families
cargo build --no-default-features --features passive,sources   # a subset
# every family but one (what the `families` CI job does for each of the twelve):
cargo test --no-default-features --features passive,reactive,sources,switches,logic,diodes,analog,transistors,mosfets,digital,subcircuits
```

Expected: the dummy-component test passes without edits in `src/`; reduced builds compile; a circuit using a missing family reports an error naming it. The `gui` feature needs every family. See [contracts/component-module.md](contracts/component-module.md) and [docs/adding-a-component.md](../../docs/adding-a-component.md).

## 5. Run the application (User Story 1)

```bash
cargo run --release --features gui
```

Manual check:
1. Window shows the layout in the reference screenshots (menu bar, toolbar, canvas, side panel, scope panel).
2. Circuit Examples > Basics > LRC Circuit loads; press run; current dots animate smoothly and scopes scroll.
3. Move the capacitance slider; waveforms change without restart.
4. Toggle Options entries; the canvas updates immediately.
5. File > Save, then Open; the circuit is unchanged.

## 6. Performance (SC-003 to SC-006)

```bash
cargo bench --bench solver
cargo run --release --features gui -- --bench-render tests/bench/ladder.txt
```

Expected: solver at or above the original's throughput (2x on 1000+ element circuits); the render benchmark reports room for well over 60 fps for 500 elements (`ladder.txt`) and prints the time to the first frame (under 2 s) and click-to-frame latency (under 50 ms). Details and recorded numbers: `tests/bench/README.md`.

## 7. Error handling (SC-009)

Open a truncated file and a file with an unknown element type; expect an error message naming the line, the rest of the circuit loaded, and no crash. Build a circuit with a floating node; expect a banner naming the element.
