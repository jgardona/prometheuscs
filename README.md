# prometheuscs

A native Rust port of the [CircuitJS1](https://github.com/pfalstad/circuitjs1) circuit
simulator, with a GTK4 / libadwaita (relm4) interface and a GPU-accelerated canvas.

## Licence

GPL-2.0-or-later, the same terms as the original CircuitJS1 (copyright Paul Falstad and
Iain Sharp), from which this project is derived. See `LICENSE`.

Source file policy:

- Every source file starts with `// SPDX-License-Identifier: GPL-2.0-or-later`.
- Files translated from CircuitJS1 keep a `// Derived from CircuitJS1 (C) Paul Falstad and
  Iain Sharp` line.
- Imported assets (examples, locales, icons) are listed in `assets/ATTRIBUTION.md`.

## Build prerequisites

- Rust 1.99 or newer (edition 2024)
- GTK 4 (4.12+) and libadwaita 1.x development files
- An OpenGL 3.3 / GLES 3 capable session (software GL works, slower)

## Quality gates

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See `specs/001-circuit-simulator-migration/quickstart.md` for the full validation guide.
