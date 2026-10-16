# Data Model: Circuit Simulator Migration

Entities from the spec, with the fields and rules the engine and UI need. Names are domain terms; Rust types are decided during implementation.

## Circuit

| Field | Description |
|-------|-------------|
| `elements` | Ordered list of Element |
| `scopes` | Ordered list of Scope |
| `sim_settings` | Time step, speed, current speed, brightness, voltage range, power range |
| `display_flags` | Show current, voltage, power, values, small grid, European resistors, IEC gates, white background, conventional current |
| `adjustables` | Slider bindings shown in the side panel (element reference, label, min, max, value) |
| `title` | Circuit name shown in the side panel |
| `hints` | Optional text shown to the user (as in original examples) |

Rules: the first file line carries flags and simulation settings; element order is preserved on save so files round-trip byte-for-byte for unchanged circuits (SC-008).

## Element

| Field | Description |
|-------|-------------|
| `kind` | Dump type code or class (for example `r`, `c`, `l`, `w`, `v`, `R`, or numeric chip codes) |
| `x1, y1, x2, y2` | Grid endpoints |
| `flags` | Per-element flag bits |
| `params` | Kind-specific numeric/text parameters (editable) |
| `posts` | Terminal coordinates derived from geometry and rotation |
| `volts` | Terminal voltages (runtime) |
| `current` | Element current (runtime) |
| `node_ids` | Solver node indices (runtime) |
| `voltage_source_ids` | Extra matrix rows (runtime) |

Rules: every kind in the component inventory has one registered implementation; unknown kinds on load become a reported error naming the line, and loading continues (edge case: unknown element type).

State transitions: Created (placed) -> Edited (parameters or geometry) -> Analyzed (nodes assigned) -> Stepping (stamped each step) -> Removed.

## Element Descriptor (Registry Entry)

Declares one component type to the system (FR-025). Created by each family module's `register` function.

| Field | Description |
|-------|-------------|
| `id` | Unique stable identifier (the original class-style name, for example `ResistorElm`) |
| `dump_code` | File-format type code(s) used to load the element |
| `display_name` | Localizable label for the Draw menu and tooltips |
| `category` | Draw-menu path (for example Passive Components) |
| `icon` | Icon asset name |
| `shortcut` | Optional key binding |
| `quick_insert` | Whether it appears in the placement toolbar |
| `constructor` | Builds an element from placement coordinates or from a parsed line |
| `family` | Owning family and Cargo feature name |

Rules: `id` and `dump_code` are unique across the registry (duplicate registration is a startup test failure); the Draw menu, toolbar, file loader and inventory read only descriptors; a disabled family contributes no descriptors, and loading its element lines yields a `LoadError` naming the type.

## Registry

Collection of all Element Descriptors. Operations: register, look up by `dump_code`, look up by `id`, list by category, list by family. Immutable after startup.

## Node

| Field | Description |
|-------|-------------|
| `index` | Row in the matrix (0 is ground) |
| `links` | (element, post) pairs attached to the node |
| `voltage` | Solved value |
| `internal` | True for element-internal nodes |

## Scope

| Field | Description |
|-------|-------------|
| `plots` | One or more (element or node, quantity, units, scale) |
| `speed` | Samples per pixel / time scale |
| `position` | Slot in the scope panel |
| `display` | Waveform, X-Y, FFT, trigger and label options |
| `buffers` | Min/max sample history for drawing |

Rules: scope data is also recorded in the golden format for compatibility checks.

## Subcircuit (Composite Model)

| Field | Description |
|-------|-------------|
| `name` | Model name |
| `pins` | Named pins with positions |
| `body` | Circuit text of the contained elements |
| `extern_nodes` | Pin-to-node mapping |

Rules: instances are Elements of a composite kind; the model is stored in the circuit file ahead of use.

## Simulation State

| Field | Description |
|-------|-------------|
| `time` | Current simulated time |
| `time_step` | Current time step |
| `status` | Running, Stopped, Errored(reason, element) |
| `iteration_count` | Nonlinear iterations in the last step |
| `matrix` | Current system and factorization (worker-owned) |

State transitions: Stopped <-> Running (Stop/Run), any -> Reset (Restart sets time to 0), Running -> Errored (singular or non-convergent), Errored -> Stopped (after user edit or restart).

## Frame Snapshot (worker to UI)

| Field | Description |
|-------|-------------|
| `sequence` | Monotonic frame number |
| `time`, `time_step`, `status` | Copy of simulation fields |
| `node_voltages` | Vector indexed by node |
| `element_currents` | Vector indexed by element |
| `scope_samples` | New samples since the last snapshot |
| `error` | Optional error with offending element index |

Rules: immutable once published; the UI always draws the latest snapshot and never blocks the worker.

## Settings

Persisted options (display flags, toolbar visibility, language, mouse-wheel editing, window size). Stored per user; unknown keys are ignored.

## Example Entry (menu catalog)

| Field | Description |
|-------|-------------|
| `category` | Submenu path from `setuplist.txt` |
| `file` | Circuit file name |
| `title` | Menu label (localized) |
| `is_default` | Marked with `>` in the list |
