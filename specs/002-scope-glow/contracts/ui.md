# UI contract: Scope glow

- With the pointer away and nothing selected, no scope panel is tinted.
- Selecting element E tints every scope panel that plots E; the tint covers the panel background
  only (grid, traces, text, wheel icon unchanged).
- Hovering E tints the same panels; leaving removes the tint (unless E is selected).
- Several selected elements: any match tints.
- Dark background: tinted panel (14, 28, 60) in place of (0, 0, 0). Light background: (222, 232, 255)
  in place of (255, 255, 255).
- Applies to docked scopes and to scopes placed in the circuit (undocked, any size, compact).

Public API additions (documented with `///`): `ScopeSpot::glow`, `ScopeScales::glow`,
`Session::scope_glow`.
