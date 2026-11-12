# Data Model: Smooth scope waveforms

## ScopePlot (src/scope/mod.rs)

| Field | Meaning | Change |
|-------|---------|--------|
| `min[POINTS]`, `max[POINTS]` | smallest and largest value of each sample | unchanged |
| `last[POINTS]` | value at the end of each sample (the newest value recorded in it) | **new**, written on each `record`; set with `min`/`max` when a sample starts and in `clear` |

Rules:
- `min[k] <= last[k] <= max[k]` for every recorded sample.
- `last[k]` of the sample being filled is the newest value, so a live trace advances smoothly.
- `ordered()` returns the three buffers oldest first.

## Sweep (held trigger capture)

Per plot: `(min, max, last, filled)`: `last` added so a held display is drawn like a live one.

## PlotView (what the painter receives)

Adds `last: Vec<f64>` alongside `min` and `max`, same length (`POINTS`), oldest first.

## Curve (drawing result, not stored)

An ordered list of vertices built per plot for the visible samples; see
[contracts/curve-drawing.md](./contracts/curve-drawing.md).
