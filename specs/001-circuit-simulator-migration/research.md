# Phase 0 Research: Circuit Simulator Migration

All items from the Technical Context are resolved here. No `NEEDS CLARIFICATION` remains. Findings about the reference project were verified by inspecting `/home/jgardona/Workspace/Rust/circuitjs1`.

## Facts about the reference project

- About 58k lines of Java in `src/com/lushprojects/circuitjs1/client`, with 153 `*Elm.java` element classes, a `matrix/` package (sparse LU ported from EJML), `Scope*` classes, `Expr.java` (expression evaluator) and `CircuitLoader.java`.
- 373 example circuits as text files in `src/com/lushprojects/circuitjs1/public/circuits`, listed in `public/setuplist.txt` (482 lines: `+Category` starts a submenu, `-` ends one, `>` marks the default entry, other lines are `file title`).
- Circuit file format: first line `$ flags timeStep ...`, then one element per line (`r`, `c`, `l`, `w`, `v`, `R`, and so on, and `o` for scopes). The same text is used for undo, export and share links.
- 15 locale files (`locale_pt.txt` exists, which matches the Portuguese UI in the screenshots).
- Toolbar component icons are inline SVG strings inside `Toolbar.java` (for example `resistorIcon`, `wireIcon`); menu and general icons are glyphs of the Fontello font in `war/font` (Font Awesome and similar, see `war/font/LICENSE.txt`).
- Automated tests: `auto-tests/*.txt` (7 files, XML with recorded scope data, for example `lrc.txt`) plus a `TestManager` hook. Only these 7 have recorded expected output; the 373 examples do not.
- Licence: GPL-2.0-or-later (file headers and `COPYING.txt`).
- Tooling available locally: Java, Node, Firefox, Rust 1.99, GTK 4.22.5, libadwaita 1.9.4.
- A built desktop (Electron) copy of the original is at `/home/jgardona/Bin/circuitjs1/circuitjs1` (the user-given path `.../circuitjs` does not exist; `circuitjs1` is the real name).

## Decisions

### R-1: Architecture and thread model

- **Decision**: Single Cargo package at the repository root with `format`, `elements`, `solver`, `render` and `app` modules; the engine modules never depend on `app` or GTK. The solver runs on a dedicated worker thread. The UI thread holds the editable circuit; the worker holds its own copy built from the same circuit text. Edits are sent as the serialized circuit (the original already uses this for undo) or as small parameter messages (slider changes, switch toggles). The worker publishes immutable frame snapshots (time, node voltages, element currents, scope samples) through a lock-free latest-value slot; the UI draws the newest snapshot each frame.
- **Rationale**: Satisfies FR-019 and SC-006 (no input blocking), keeps GTK out of the engine, reuses an existing, proven interchange format, and avoids shared mutable state between threads.
- **Alternatives**: Multi-crate workspace (rejected by the user in favor of simplicity; can be introduced later); solver on the UI thread via idle callbacks (simple, but input stalls on large circuits); one shared circuit behind a mutex (contention between draw and solve, hard to reason about).

### R-2: GPU rendering

- **Decision**: A custom canvas widget built on `GtkGLArea` (OpenGL) with the `femtovg` 2D vector renderer, drawing through a `Painter` trait.
- **Rationale**: `GtkGLArea` is the supported way to get a hardware-accelerated surface in GTK4 and is available through gtk4-rs. femtovg gives antialiased paths, strokes, dashed lines and text on OpenGL with a small API, matching the Canvas2D calls the original uses. Mesa software GL provides the no-GPU fallback required by the edge cases.
- **Alternatives**: GTK snapshot/GSK nodes (accelerated, but per-frame node-tree cost grows with thousands of dashed line segments and has no direct dash-offset animation); wgpu with vello (strong, but needs a texture bridge into GTK and a heavier dependency); Cairo drawing area (CPU only, fails FR-011).
- **Risk**: femtovg inside `GtkGLArea` must be proven early. Mitigation: first task is a spike drawing 500 animated elements at 60 fps; if it fails, switch the backend behind the `Painter` trait to wgpu/vello without touching element code.

- **Spike result (T044, 2026-10-15)**: GO. `src/bin/gl_spike.rs` draws N resistor-like elements (two leads, a zigzag body, five animated current dots each) in a `GtkGLArea` through femtovg 0.27 on Wayland, AMD Radeon RX 7600 (radeonsi, Mesa 26.2.4), release build. The spike reads the framebuffer back to prove pixels were drawn (about 15% of a 1700x900 window at 500 elements). Time of one whole frame, CPU submission plus `glFinish` for the GPU: 100 elements 3.2 ms, **500 elements 7.2 ms**, 2000 elements 11.0 ms, 5000 elements 14.8 ms, all inside the 16.7 ms of a 60 Hz frame. CPU time of the render call alone (no GPU wait): 1.2, 5.3, 9.1 and 12.9 ms. For comparison the Cairo backend costs about 27 microseconds per element on the CPU (2.0 ms for the 76-element ladder), about 13 ms at 500 elements and 130 ms at 5000, so the GPU backend is about 2x faster at 500 elements and 9x at 5000. Frames per second measured on a window that has no focus are throttled by the compositor and mean nothing; use the per-frame times. Two things the spike taught: (1) `GtkGLArea` draws into its own framebuffer, so its id must be queried after `attach_buffers` in `realize` and given to femtovg (`set_screen_target`); without it every draw fails with `GL_INVALID_FRAMEBUFFER_OPERATION` (femtovg reports that through `tracing`, so a subscriber must be installed to see it, and an early version of this spike measured a failing run); (2) OpenGL entry points come from libepoxy by `dlsym` of `epoxy_<name>`, which are function-pointer variables, so the variable must be read, not called. All `unsafe` is confined to `src/render/gl.rs`. Not measured: software GL (llvmpipe) and text drawing cost.

### R-3: Animation approach

- **Decision**: Drive frames with the widget frame clock (`add_tick_callback`) and render the current-flow dots from simulation time and a per-wire phase offset, so animation costs no extra solving.
- **Rationale**: Frames track display refresh, and rendering stays decoupled from the solver rate, as in the original's current-flow offset logic.
- **Alternatives**: Fixed timer (judder and wasted frames).

### R-4: Solver and numerical parity

- **Decision**: Port the original MNA (stamping, nonlinear convergence loop, subiterations, matrix simplification) and its sparse LU directly in `src/solver`, using `f64`. Optimize after parity using benchmarks (reusing the symbolic factorization, avoiding allocation in the step loop).
- **Rationale**: Identical algorithms produce comparable results (FR-008) and make failures easy to bisect against the Java source.
- **Alternatives**: `faer` or `sprs` solvers (different pivoting changes low-order digits, which complicates golden comparison; kept as a later optimization candidate behind benchmarks).

### R-5: Element port strategy

- **Decision**: One Rust trait `Element` with defaults mirroring `CircuitElm` (parse, dump, setup, stamp, step, draw, edit info, info text). Elements are grouped into modules by family. The component inventory is generated by `src/bin/inventory.rs` from the original `*Elm.java` files plus the dump-type table, and a test fails if any inventory entry lacks a Rust implementation (FR-022, SC-010).
- **Modularity (FR-025 to FR-027)**: Each family module exposes a single `register` function that declares its elements and their metadata (name, dump code, category, icon, shortcut, constructor). Menus, toolbar entries, file loading and the inventory read the registry, so there is no central list or `match` to edit. One Cargo feature per family allows build-time removal. A test-only dummy element proves that adding a component touches no file in `src/`.
- **Rationale**: A machine-checked inventory is the only reliable way to guarantee "all components present".
- **Alternatives**: Manual checklist (drifts); code generation of elements from Java (fragile, produces unmaintainable Rust, violates the maintainability principle).

### R-6: Golden data for compatibility

- **Decision**: Use the already built original at `/home/jgardona/Bin/circuitjs1/circuitjs1` (Electron; launch with a remote debugging port), load each example, run a fixed number of steps through its JavaScript interface, and record node voltages and element currents at checkpoints into `tests/compat/golden/*.json`. The 7 `auto-tests` files are used directly first, as they already hold scope data. Tolerance defaults to 1e-6 relative with an absolute floor, relaxed per circuit only with a written justification in the golden file.
- **Rationale**: The 373 examples have no recorded expectations; generating them from the actual reference gives real parity evidence (SC-001, SC-011).
- **Alternatives**: Analytical oracles only (cover simple circuits, not ICs or logic); comparing only "runs without error" (does not prove behavior).
- **Fallback**: If the Electron build cannot be driven programmatically, build the original from source (Gradle/GWT) and use headless Firefox, or capture results manually through `JSInterface` calls, documented in the quickstart.

### R-7: Icons and assets

- **Decision**: UI icons (menus, dialogs, window actions) use the symbolic icon names of the GTK/Adwaita icon theme, with a bundled mapping from each original Fontello glyph to the closest symbolic icon. Component and tool icons are extracted from the inline SVG strings in `Toolbar.java` by `src/bin/import_assets.rs` into `assets/icons/*.svg` and loaded as resources, recolored for light and dark themes. Where no Adwaita equivalent exists for a glyph, the Fontello SVG glyph is imported and credited in `assets/ATTRIBUTION.md`.
- **Rationale**: Implements FR-023 and FR-024 with native look for generic actions and familiar symbols for electrical components.
- **Alternatives**: Redraw all icons (wasted effort, unfamiliar); use the Fontello font at runtime (adds a font dependency and a non-native look).

### R-8: Internationalization and settings

- **Decision**: Load the original `locale_*.txt` translation maps at runtime (key = English string) with Portuguese as the default per the screenshots. Settings are persisted as a small file in the XDG config directory. Autosave goes to the XDG data directory.
- **Rationale**: Reuses 15 existing translations without a conversion step (FR-018), and follows desktop conventions.
- **Alternatives**: gettext (requires re-extracting all strings and losing parity with the original's keys).

### R-9: Licensing

- **Decision (user, 2026-10-12)**: GPL-2.0-or-later, the same terms as the original CircuitJS1 (file headers say "version 2 of the License, or (at your option) any later version"). An earlier Apache-2.0 choice was reverted: the original is GPL, this project is a translation of it, and the project is non-commercial.
- **Rationale**: The Rust code, imported example circuits and toolbar SVG icons are derived from GPL-2.0-or-later material, so GPL-2.0-or-later allows direct porting and publication without permission from the original authors.
- **Obligations**: Include the full licence text as `LICENSE`, keep the original copyright notices (Paul Falstad and Iain Sharp) in translated files, add an SPDX header `GPL-2.0-or-later` to source files, publish source with any distributed binary, and record asset provenance in `assets/ATTRIBUTION.md` (including the Fontello/Font Awesome glyph licences, which are compatible with distribution).
- **Alternatives considered**: Apache-2.0 (rejected: not allowed for derived GPL code without the authors' permission); GPL-2.0-only (rejected: narrower than the original's "or later" grant for no benefit).

### R-10: Subcircuits, scripting and browser-only features

- **Decision**: Custom composite elements and subcircuits are ported (they are components and examples depend on them). Browser-only features (Dropbox import, JS interface, embedded mode, websocket, "Browser Developer Tools" menu entry) are excluded, matching the spec assumptions. Audio input/output elements are ported using a native audio API, in the last implementation phase.
- **Rationale**: Required for FR-007, FR-021 and FR-022 without carrying web-only dependencies.
- **Alternatives**: Dropping audio elements (would break "all components present").

### R-11: Delivery order

- **Decision**: Build bottom-up: (1) format and solver with the passive elements, proven on the 7 auto-tests; (2) renderer spike and canvas; (3) editing and menus; (4) remaining element families in batches driven by the inventory; (5) scopes; (6) export, subcircuits, options; (7) performance tuning against benchmarks.
- **Rationale**: Each step produces an independently testable increment aligned with the story priorities.

## Audio playback (T120)

- **Decision**: the audio output plays through the sound server's own command line players (`pw-play`, then `paplay`, then `aplay`), run on a temporary WAV file (`src/app/playback.rs`). No audio crate was added. The audio input reads WAV files itself (`src/elements/samples.rs`: PCM 8 to 32 bits, float 32 and 64 bits, plain and extensible headers, first channel).
- **Why**: the original plays a finished recording on a button press (it builds a WAV and hands it to an `<audio>` element); it has no live stream. A native crate (cpal, rodio) would add a build dependency on the ALSA development files (`libasound2-dev`) to every build, the CI job and every user who compiles it, for a feature that plays a few seconds of sound on request. Neither the development machine nor the CI image has them installed, and the `gui` build already depends on GTK, which the players sit beside on every desktop that has one.
- **Cost**: playback needs one of the three players installed (all three are on a normal Linux desktop); without one the button reports it in the banner. A live, low-latency stream would need a native crate, and nothing in the original needs one.
- **Alternatives**: cpal (rejected above, revisit if live audio is ever wanted); rodio (same ALSA dependency); GStreamer through GTK (heavier, and not a dependency yet).

