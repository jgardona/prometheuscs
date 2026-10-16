# Contract: Component Module (adding or removing a component)

Defines what a component author writes and what the system guarantees (FR-025 to FR-027, SC-012, SC-013).

## What a component author provides

1. **Element behavior**: a type implementing the `Element` trait (parse, dump, post positions, setup, stamp, step, draw, edit info, info text) in the family file under `src/elements/`.
2. **Descriptor**: one entry in the family's `register(&mut Registry)` function (see data-model.md, Element Descriptor).
3. **Icon**: an SVG in `assets/icons/components/` named by the descriptor's `icon` field (optional; a generic icon is used when absent).
4. **Tests**: parse/dump round trip, stamping, and a golden or analytical result check.

A new family additionally adds one `mod` line, one `register` call and one Cargo feature gating both.

## What the author never touches

Simulation core (`src/solver`), application shell (`src/app`), menus, file loader, example catalog, inventory logic.

## System guarantees

- **M1 Discovery**: Draw menu entries, toolbar quick-insert buttons, factory dispatch and `inventory` output are generated from descriptors.
- **M2 Uniqueness**: duplicate `id` or `dump_code` fails the registry test at build time.
- **M3 Removal**: building with a family feature disabled succeeds; the Draw menu omits its entries; loading a circuit that uses it reports `LoadError { line, type }` and loads the remaining elements.
- **M4 Isolation test**: `tests/modularity.rs` registers a dummy element from the test file only and verifies M1 without editing any `src/` file.
- **M5 Documentation**: `docs/adding-a-component.md` contains a worked example verified by that test.

## Removal procedure

Disable the family's feature in `Cargo.toml` (or delete its file, its `mod` line, its `register` call and its feature). No other file changes; tests specific to the family are gated by the same feature.
