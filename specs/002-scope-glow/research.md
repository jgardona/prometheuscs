# Research: Scope glow

## R1. What the reference image shows

- **Decision**: reproduce the tint as a flat background colour, not a halo around the panel.
- **Evidence**: sampled the image (1198×834): glow panel pixels are all (14, 52, 52) across the
  panel, plain panels (17, 17, 17); the plain canvas is (0, 0, 0). The tint stops at the panel
  edges and grid/traces are drawn over it.
- **Rationale**: matches "brilho ameno" and the spec (FR-002).
- **Alternatives**: an outer glow/blur — not in the image, costs more, would need clipping changes.

## R2. Colour for the blue version

- **Decision**: dark background: **(14, 28, 60)**; light background: the plain white blended toward
  blue by the same fraction, **(222, 232, 255)**.
- **Rationale**: the green tint lifts the plain (17, 17, 17) by +35 in two channels (about 20% of
  full). Blue keeps a +35..+43 lift in the blue channel, a smaller one in green, so it reads as
  blue and not as the original teal. Final values are checked by eye against the image in
  quickstart step 5; they are constants in one place.
- **Alternatives**: pure (14, 14, 52) — reads as purple; the teal itself — not what was asked.

## R3. What counts as "selected"

- **Decision**: the set of the selected elements plus the element under the pointer; a scope glows
  if any of its plots' elements is in the set. Uses `selected_indices()` and `hover`, the same
  state as the information box (`Session::info_lines`).
- **Rationale**: consistent with the info box and the original (hover and selection both glow).
- **Alternatives**: selection only (drops story 2); the plot under the scope pointer (the opposite
  direction, already handled by `scope_hover`).

## R4. Where the flag travels

- **Decision**: docked scopes: `ScopeScales.glow: Vec<bool>` set by the window before
  `draw_scopes`, like `info`; the scales struct already lives in the session and is taken for
  drawing. Placed scopes: `Session` writes the flag into the element with the view in
  `Simulation::set_scope_glow`, called from `Session::pump` every frame with or without a worker (the views of placed scopes arrive from the snapshot when there is a worker, so `push_embedded_views`, which runs only without one, cannot carry the flag), and `scope_elm.draw` reads it. The worker thread's
  copy never needs it (drawing happens on the window side, as in `tests/worker_draw.rs`).
- **Rationale**: no change to `ScopeView` (compared with `PartialEq`, comes from the worker) so the
  worker protocol and snapshots stay untouched.
- **Alternatives**: a field in `ScopeView` — mixes UI state into the snapshot; a field in
  `DrawCtx` — it is per element, not per frame.

## R5. Scopes sharing a place

- **Decision**: only `slot == 0` draws the background today, and nothing creates other slots
  (all callers pass 0). The tint is applied where the background is drawn; no other work.
