# Audit of the numbers (T054)

**Status: not done as written.** The task asks to run at least 20 examples in the window
(`cargo run --release --features gui`) and compare by eye what the document says with what the
window shows, spread over all categories. No window was opened in the session that wrote waves 9
and 10, so no by-eye comparison was made, and this audit covers only the categories those waves
documented.

## What was done instead (waves 9 and 10, 68 documents)

For 31 documents the value the document gives was **derived independently of the simulator**
(a closed formula or a small numerical model written for the purpose) before it was compared with
what `exdocs` measured in the simulator. They agree as shown (relative difference):

| Document | Quantity | Theory | Simulator |
|----------|----------|--------|-----------|
| `tl` | load amplitude, delay 30 ns | 5 V, −4.755 V at 100 ns | 5.000 V, −4.757 V |
| `tlterm` | load pulse, four terminations | 2.5, 4.96, 0.588, 8.76 V | 2.500, 4.963, 0.588, 8.758 V |
| `tlmismatch` | first and second pulse on the load | 1.135, 0.620 V | 1.134, 0.620 V |
| `tlmis1` | junction, input | 4.348, 3.011 V | 4.348, 3.014 V |
| `tlmatch1` | load, matched / mismatched | 4.975, 4.000 V | 4.975, 4.000 V |
| `tlmatch2` | load, with / without stub | 5.000, 4.000 V | 5.000, 4.000 V |
| `tllight` | load step after 70 ns | 62.14 V | 62.14 V |
| `jfetcurrentsrc` | drain current | 2.146 mA | 2.147 mA |
| `jfetfollower` | source DC, gain | 3.28 V, 0.90 | 3.29 V, 0.90 |
| `jfetfollower-nooff` | current of each JFET | 2.007 mA | 2.007 mA |
| `jfetamp` | bias, gain | 1.472 mA, −3.1 | 1.473 mA, −3.1 |
| `lambda-diode` | current at 5, 6 V | 1.41, 0.63 mA | 1.411, 0.628 mA |
| `tdrelax` | frequency | 42.9 Hz | 42.3 Hz (1.3 %) |
| `mr` | current at 10 µs | 0.508 mA | 0.505 mA |
| `mr-sine`, `mr-square`, `mr-triangle`, `mr-sine2`, `mr-sine3` | current at several instants | per the flux formula | within 1 % |
| `ujtosc` | frequency | 1397 Hz | 1392 Hz |
| `triode` | plate current | 18.33 mA | 18.33 mA |
| `scractrig` | firing voltage, RMS and mean | 15.6 V, 9.3 V, 5.2 V | 15.56, 8.84, 4.86 V (0.64 V SCR drop) |
| `ccinductor` | peak current | 89.4 mA | 89.35 mA |
| `ccint`, `ccdiff` | ramp slope, current | 5 V/ms, 8 mA | 5.000 V/ms, 8.000 mA |
| `spark-sawtooth` | frequency | 67.9 Hz | 67.74 Hz |
| `wienbridge` | frequency, lamp resistance | 159.2 Hz, 50 Ω | 159.5 Hz, 50.0 Ω |
| `varactor` | amplitude at three biases | 0.765, 0.600, 0.481 | 0.747, 0.590, 0.477 |
| `lightbulb` | steady RMS current | 0.833 A | 0.832 A |
| `triodeamp` | gain | −6.7 | −6.4 |
| `triacdimmer` | lamp RMS from the firing angle | 60.1 V | 60.4 V |

## Differences found and what was done

- `tllopass`: the swept simulation differs from the steady-state theory by up to ~20 % in the
  passband; the document says so and does not claim agreement.
- `triodeamp`: gain of −6.4 against −6.7 (small-signal estimate); the document says why.
- `gyratorelm`: the gyrator circuit and the real inductor differ by ~5 % at one instant; the
  document says the cause was not investigated.
- `3motor`, `ota-vcf-single`, `ota-ringmod`, `ota-gain`, `norton-noninvert`,
  `lambda-diode-osc`, `tdosc`: the document gives measured values and says plainly that they
  were not derived from the model.
- No document was changed to hide a difference.

## Still to do

A person should open a sample of at least 20 examples spread over **all** categories in the
window and compare what the document says with what is on the screen; this task stays open.
