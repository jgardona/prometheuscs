# Comparison with the original CircuitJS1

Two kinds of reference data check the simulation:

* **Recorded auto-tests** (`tests/compat/auto/*.txt`): circuits saved by the original with the
  samples its scopes recorded (`<scopedata>`); used by `tests/us1_rlc.rs` and friends.
* **Golden recordings** (`tests/compat/golden/<example>.json`): what the original computes for
  each bundled example, at a few simulated times. Compared by `tests/compat_golden.rs`.

## Recording the golden data

The Electron build of CircuitJS1 (default `/home/jgardona/Bin/circuitjs1`) does not accept
switches such as `--remote-debugging-port`, so the tool works on a private copy and never
touches the original installation:

```
cargo run --bin golden                      # all 373 examples, two runs each
cargo run --bin golden -- --only lrc,td4    # some examples
cargo run --bin golden -- --runs 1          # one run (no nondeterminism check)
cargo run --bin golden -- --steps 200,2000,10000 --only tl,delayrc   # checkpoints in time steps
cargo run --bin golden -- --app DIR --circuits DIR --out DIR --times 0.001,0.01 --limit-ms 30000
```

It copies the program to `$TMPDIR/prometheuscs-golden`, adds
`tests/compat/tools/golden-preload.js` to the copy's `preload.js` (and two lines to `main.js`),
and starts it. **A window opens and the program closes itself when it is done.** For every
example the copy loads the circuit through the `CircuitJS1` JavaScript API, lets it run, and at
each requested simulated time writes the voltage across, the current through and the post
voltages of every element. Examples that take longer than `--limit-ms` of wall-clock time are
recorded with `"timeout": true` and skipped by the test.

With two or more runs, examples whose runs differ are written to
`tests/compat/nondeterministic.json` (random sources, wall-clock circuits); the test skips them.

## Comparing

```
cargo test --test compat_golden -- --nocapture
```

The default tolerance is `1e-3` relative plus `1e-6` absolute. A circuit that legitimately
differs (chaotic, or a documented difference in the original's numerics) goes into
`tests/compat/tolerances.json` with the reason:

```json
{ "td4": { "rel": 0.05, "abs": 1e-3, "reason": "..." } }
```

Without recordings the test passes after checking only that its comparison works.

## Known differences

`tests/compat/known_differences.json` lists examples that still differ from the original, each
with a reason. The test reports them and does not fail on them; it says so when one starts to
agree, so it can be removed from the list. Wire, ground and cross-switch currents are not
compared (they are derived values), and the state saved in the file is kept when a circuit is
loaded, as the original does.

## Nondeterministic examples

`tests/compat/nondeterministic.json` lists the examples that use a noise source (`n`), which the
two programs cannot reproduce. The recorder flags every example whose two runs differ, but most
of those only differ in the time of a checkpoint; `GOLDEN_ALL=1 cargo test --test compat_golden`
compares the listed ones too, which is how the list was cut down to the real ones. A run with
`--only` keeps the list entries of the examples it did not run.

## Examples that time out

Examples with picosecond time steps (transmission lines, slow CMOS) never reach a millisecond
in the wall-clock limit. Record them with `--steps`, which puts the checkpoints at that many
time steps of each circuit instead of at fixed times.
