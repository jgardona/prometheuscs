# Quickstart: Validating CircuitJS 5.x Feature Sync

How to check that this feature works end to end. Run from the repository root. Details of formats and commands are in [contracts/](contracts/); state is in [data-model.md](data-model.md).

## Prerequisites

- The usual build environment from spec 001 (Rust edition 2024 toolchain, GTK 4.22, libadwaita 1.9).
- The upstream clone at `~/Workspace/circuitjs1` at `8ac55f22` (CircuitJS 5.0.4), for reading only. Node.js and npm are needed only for the fallback 5.0.4 build.

## 1. Quality gates (every story)

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

All three must pass on a clean checkout.

## 2. Reference data

Nothing to record. The checks use the existing recordings in `tests/compat/golden/` and the hand-calculated values in `tests/compat/circuits-5x/expected.json` (research R-1). Building CircuitJS 5.0.4 is a fallback only, described in `tests/compat/README.md`.

## 3. Story 1 and 2: files and new elements

```sh
cargo test --test compat_golden          # all 373 examples, existing baseline
cargo test --test compat_5x              # 5.x circuits vs expected.json; new cs-* vs old recordings
cargo test --test controlled             # cv/ci, v()/i()/dvdt()/didt(), subcircuit scoping
cargo test --test format_roundtrip       # cv, ci, lb, co, rn, sublib survive save and load
```

By hand: open `cs-resistor.txt` from Circuit Examples. The diamond current source is drawn and the info box shows `I = v(vd)/1000`. Rename the voltmeter label; the info box shows `unknown name: v(vd)`.

## 4. Story 3: editing

By hand, with the properties panel options at their defaults:

1. Press Shift twice: Find Component opens; type `cap`; Enter places a capacitor.
2. Drag the resistor button from the toolbar onto the middle of a wire: the wire splits; Ctrl+Z restores the original wire in one step.
3. Turn on "Show Properties on Click", click two elements in turn: the side panel follows.
4. Roll the wheel one notch over a 1 kΩ resistor: it becomes 1.2 kΩ. Over an NPN transistor: it becomes PNP.

## 5. Story 4: scopes

Open `scope-ac.txt` from `tests/compat/circuits-5x` (a sine on a DC offset with three stacked scopes): turn on AC coupling in auto scale and check the trace centers within one period. Export a scope as SVG with a transparent background and open it in an image viewer.

## 6. Story 5 and 6

- Make a subcircuit persistent, restart, find it under Draw > Subcircuits.
- Move only the speed slider of a freshly opened example and close: no unsaved-changes prompt.

## 7. Documentation (Story 7)

```sh
cargo run --bin exdocs -- check     # no problems reported
cargo test --test example_docs
```

Then confirm each delivered feature is described in `docs/guia-do-usuario.md` (search by its menu or element name) and that "Diferenças em relação ao CircuitJS1" names CircuitJS 5.0.4.

## 8. Performance

```sh
cargo bench --bench solver
cargo run --release --bin bench_circuits
```

Simulation speed on the bundled examples must not drop by more than 5% compared with 1.0.8 (SC-007); record the numbers in the pull request.
