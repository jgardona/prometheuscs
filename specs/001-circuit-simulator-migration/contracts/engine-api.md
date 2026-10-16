# Contract: Engine API (`src/format`, `src/elements`, `src/solver`, `src/scope`)

The engine modules are the stable boundary between the solver and everything else. They have no GUI dependencies.

## Operations

| Operation | Input | Output | Notes |
|-----------|-------|--------|-------|
| `parse` | circuit text | `Circuit` or list of `LoadError` (line, type, message) | Partial load allowed; errors listed |
| `dump` | `Circuit` | circuit text | See circuit-file-format.md |
| `analyze` | `Circuit` | `Simulation` or `AnalysisError` | Assigns nodes, builds matrix, finds floating nodes and shorts |
| `step` | `&mut Simulation` | `StepResult` (Ok, NotConverged, Singular(element)) | One time step with nonlinear iteration |
| `run_for` | `&mut Simulation`, duration or step count | `StepResult` | Headless runs for tests and benchmarks |
| `set_param` | element index, parameter id, value | updated simulation | Live slider and mouse-wheel edits without reset |
| `toggle_switch` | element index | updated simulation | Switch interaction |
| `reset` | none | time = 0, state cleared | Restart button |
| `snapshot` | `&Simulation` | `FrameSnapshot` | Immutable view for the UI |
| `dc_operating_point` | `Circuit` | node voltages | File menu entry "Find DC operating point" |

## Error model

Typed errors only (`thiserror`); the engine never panics on user input. `Singular` and `NotConverged` carry the offending element index so the UI can highlight it (FR-016).

## Determinism

Given the same circuit, settings, and step count, results are bit-identical across runs on the same machine. Randomness (noise elements) uses a seedable generator that tests can fix.

## Performance contract

- `step` does not allocate in steady state after the first call.
- Benchmarks: steps per second for RLC (small), a 500-element ladder, and a 1000+ element mixed circuit; regression thresholds are recorded in `tests/bench`.

## Rendering interface (`src/render`)

| Operation | Description |
|-----------|-------------|
| `Painter` trait | `line`, `polyline`, `polygon`, `ellipse`, `arc`, `text`, `set_stroke`, `set_color`, `dash`, `push/pop transform` |
| `draw_circuit(painter, circuit, snapshot, view, options)` | Draws every element, current dots, voltage colors, labels |
| Backends | GL (live canvas), SVG (export), PNG (export and golden-image tests) |
