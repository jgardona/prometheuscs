# Circuits in the CircuitJS 5.x format

Test circuits for the features of CircuitJS 5.x (spec 006). They are written by hand in the
5.x XML format (see `specs/006-circuitjs5-feature-sync/contracts/file-format.md`) and checked
by `tests/compat_5x.rs` against values worked out by hand, listed in `expected.json`.

## `expected.json`

One entry per circuit, keyed by file name without `.txt`:

- `time`: the simulated time at which the values are checked.
- `points`: `[x, y, volts]`, the voltage of the node at that grid point.
- `currents`: `[element index, amps]`, `get_current()` of the element (index in file order).
- `info`: `[element index, [text, ...]]`, lines the element's info box must contain.

The tolerance is the one of `tests/compat_golden.rs` (relative 1e-3, absolute 1e-6).

## Derivations

- **cv-vnode**: a 5 V source puts 5 V on the labeled node `in`. The controlled voltage source
  (+ at its second post, `224,192`) outputs `2*v(in)` = 10 V across a 1 kΩ resistor to ground,
  so 10 mA flows through the resistor.
- **ci-didt**: a 1 V source drives a 1 H inductor through the ammeter `ia`. The inductor current
  rises at V/L = 1 A/s, so `didt(ia)` = 1 and the controlled current source (into its second
  post, `320,192`) drives 1/1000 A through 1 kΩ: 1 V. With the trapezoidal rule the inductor
  current lags the exact `t` by half a time step (5 µs): at t = 1 ms the ammeter reads
  1 ms − 2.5 µs = 0.9975 mA, while its derivative stays exactly 1 A/s.
- **vccs-vref**: the old `cs-resistor` example (a VCCS that makes a 1 kΩ resistor out of
  `-(a-b)/1000`) with the expression changed to `-v(vd)/1000`, where `vd` is a voltmeter
  across the same two nodes. The rail must still deliver 5 V / 1 kΩ = 5 mA, as the original
  recorded for the old circuit (`tests/compat/golden/cs-resistor.json`).
- **unknown-ref**: `1+v(nothere)` names nothing, so the reference reads 0 and the output is 1 V;
  the info box reports `unknown name: v(nothere)`.
- **not-current-meter**: `i(vp)` names a voltmeter, which has no current to read, so the
  reference reads 0, the source drives 1 mA through 1 kΩ (1 V), and the info box reports
  `not a current meter: vp`.

- **sub-scoped-ref**: two instances of the model `doubler` (written by prometheuscs's Create
  Subcircuit, so the model's `cv` carries `rn="1 0"`), each with its own label `in` read by
  `2*v(in)`. The inputs are driven with 1 V and 2 V, so the outputs are 2 V and 4 V: each
  instance reads its own `in`, never the other's.
- **mono-invert**: a monostable with flag bit 2 ("Invert Trigger", pulse 1 ms) triggered by a
  100 Hz square wave that is high for the first 5 ms. It fires on the falling edge at 5 ms, so
  at 5.5 ms its output is high (5 V); without the flag it would wait for the rising edge at
  10 ms.
- **scope-ac**: a 1 V, 100 Hz sine on a 5 V offset into two 1 kΩ resistors in series, with
  three stacked scopes (on the source and on each resistor). At t = 2.5 ms (a quarter period)
  the source is at its peak, 6 V, and the midpoint of the divider at 3 V. The scopes are for
  the AC-coupling and stacked-label checks of user story 4.

Two more circuits have no entry here because their checks are not node values:
`text-color.txt` (the `co` attribute round-trips, `tests/more_elements.rs`) and
`stop-trigger-load.txt` (a Stop Trigger whose condition is already true at t = 0 does not stop
the simulation on load or Reset, `tests/more_elements.rs`).

## `old/`

The 12 controlled-source examples as they were before CircuitJS 5.x rewrote them with the
two-terminal sources. `tests/compat/golden/cs-*.json` were recorded from these versions.
`tests/compat_5x.rs` runs each new `assets/circuits/cs-*.txt` and compares it with the
recording of its old version: an element of the new circuit is compared with the recorded
element of the same type at the same coordinates in the old circuit (elements present in only
one version, such as the replaced VCCS and the new probes, are skipped).

| Example | Old circuit | Recording | Changed in 5.x |
|---------|-------------|-----------|----------------|
| cs-currentadder | old/cs-currentadder.txt | golden/cs-currentadder.json | yes |
| cs-diff | old/cs-diff.txt | golden/cs-diff.json | yes |
| cs-fullrectifier | old/cs-fullrectifier.txt | golden/cs-fullrectifier.json | yes |
| cs-integrator | old/cs-integrator.txt | golden/cs-integrator.json | no |
| cs-multiplier | old/cs-multiplier.txt | golden/cs-multiplier.json | no |
| cs-opamp | old/cs-opamp.txt | golden/cs-opamp.json | yes |
| cs-opamprail | old/cs-opamprail.txt | golden/cs-opamprail.json | no |
| cs-ramp | old/cs-ramp.txt | golden/cs-ramp.json | yes |
| cs-resistor | old/cs-resistor.txt | golden/cs-resistor.json | yes |
| cs-varicap | old/cs-varicap.txt | golden/cs-varicap.json | yes |
| cs-varinduct | old/cs-varinduct.txt | golden/cs-varinduct.json | yes |
| cs-varyresistor | old/cs-varyresistor.txt | golden/cs-varyresistor.json | yes |

Building CircuitJS 5.0.4 to record these circuits directly is a fallback only; see
`tests/compat/README.md`.
