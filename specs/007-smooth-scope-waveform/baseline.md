# Baseline and results: smooth scope waveforms

## Before the change (T001)

- `cargo test --release --lib`: 372 passed. `--test scopes`: 44 passed.
- `tests/example_docs.rs`: `the_index_follows_the_order_and_the_categories_of_the_menu` fails
  ("Blank Circuit" outside any group), plus two slow doc tests that fail on stale component
  tables (11 problems from `exdocs check`). None is related to scopes.
- Drawing time (temporary test, 2048 samples with a 0.3 V span each, 1600x400 scope, SVG painter,
  best of 5 x 300 frames): **1330 us per frame**.

## Red (T016)

The three sine tests failed on the old drawing: 779 direction changes against 51 expected
(a needle on every sloping sample), and 779 against 133 at 6 samples per period. This confirms
research R1. The detail tests (noise, glitch, square edge, constant) passed on the old drawing
(T012), so they pin the behaviour that must not change.

## After the change

- `cargo test --release --lib`: 387 passed (15 new tests).
- All other test binaries: 710 passed in total, 1 failed: the pre-existing `example_docs` failure
  above (two slow doc tests were skipped, they were already failing).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and the same with
  `--features gui`: clean. (An unused `mut` in an old scope test that failed `-D warnings` was
  fixed on the way.)
- Drawing time, same temporary test: **715 us per frame**, 46% less than before (fewer vertical
  strokes), so SC-004 (no more than +10%) holds. The temporary test was removed.
- Documentation: `docs/arquitetura.md` (scope section) updated.
- Window check (T028): **not done**, needs a person at the screen (see quickstart step 5).
