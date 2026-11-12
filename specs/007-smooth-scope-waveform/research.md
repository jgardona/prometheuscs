# Research: Smooth scope waveforms

## R1. Where the spikes come from

**Decision**: the spikes come from how the time-domain curve is drawn, not from the simulated
values or from graphics anti-aliasing.

**Evidence** (code reading; to be confirmed by the failing test of quickstart step 1):
- `ScopePlot::record` (`src/scope/mod.rs`) folds every simulation value of a sample into
  `min[ptr]` and `max[ptr]`; a sample lasts `time_step * speed` (Summing Amplifier: 5 us * 64 =
  320 us, so 64 values per sample and a 200 Hz period of about 15.6 samples).
- `draw_scope_inside` (`src/render/scopes.rs`, "curves") joins the **maxima** of consecutive
  samples and, where `max - min` is above a flat threshold, strokes down to the minimum and back.
- With so few samples per period the maxima of a rising sample sit at its end and those of a
  falling one at its start, so the line is shifted by half a sample on each side, the crest sample
  contributes the true peak as a one-pixel vertex and the trough sample is a downward needle. The
  shape changes with the phase between the period and the pixel grid, which is the uneven look in
  the report. Anti-aliasing of the strokes cannot change this: it only softens the edges of the
  same geometry.

**Rationale**: min/max per pixel is the standard peak-detect used to make sure glitches survive
decimation; the defect is that the line is built from one envelope instead of from the signal.

**Alternatives considered**:
- Enable multisampling in the GL/Cairo backends: does not alter vertices; rejected as not the cause.
- Draw the midpoint `(min + max) / 2` per sample: smooth for a sine but lowers peaks and halves
  glitches; violates FR-003/FR-004; rejected.
- Low-pass or resample the stored data: changes real values and hides noise; rejected.

## R2. What to draw instead

**Decision**: record the last value of each sample (`last`). The curve's vertices are `(right edge
of sample k, last[k])`, which lie on the true signal. A sample adds extra vertices only for the
part of its `[min, max]` that lies outside `[min(last[k-1], last[k]), max(last[k-1], last[k])]`:
- extremum above and below the end values (noise, a crest or trough inside the sample): the
  vertex is added at the sample centre; a crest therefore reaches its true maximum, with no
  needle because the curve goes up to it and down again;
- both excursions (noise): the vertical stroke min..max is drawn as today;
- no excursion (monotone slope, square edge inside the endpoints): nothing is added.

**Rationale**: a smooth signal is fully described by its end values plus at most one extremum per
sample, so the curve is smooth and reaches the true peaks; anything the end values do not explain
(glitch, noise, fast detail) is exactly what is added, at full height.

**Alternatives considered**: storing first and last (more memory, no gain: first of k is last of
k-1); cubic interpolation (invents values, rejected).

## R3. Edges and noise stay faithful

**Decision**: a square edge inside one sample has `min`/`max` equal to the neighbouring end
values, so the line goes from one level to the other over one sample, as steep as today (one
pixel). Noise whose range exceeds the end-value span keeps its vertical strokes at full
amplitude. A one-sample glitch is outside the span, so it is drawn at full height. These cases
become explicit tests (spec User Story 2).

## R4. Interaction with existing features

- **Trigger hold**: `Sweep` copies `(min, max, filled)` per plot; it must copy `last` too, or the
  held display would fall back to the old drawing.
- **Autoscale, cursor value, measurements, spectrum, X/Y**: use `min`/`max` only; unchanged
  (FR-004).
- **Selected plot (3 px line)** and **glow**: same path, only its vertices change.
- **First sample after a clear**: `last` is initialised with the value like `min`/`max`.
- **Samples with `valid` less than the window**: the curve starts at the first valid sample, as now.

## R5. Performance

**Decision**: one extra `f64` write per recorded value and one extra `Vec` copy per held sweep;
the drawing loop stays a single pass with constant work per sample. Measure with `render_bench`.
