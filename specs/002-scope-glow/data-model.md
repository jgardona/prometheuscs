# Data Model: Scope glow

No stored data. Transient state only:

| Name | Where | Meaning |
|------|-------|---------|
| `glow` (bool) | `ScopeSpot` | draw this scope with the tinted background |
| `glow` (`Vec<bool>`) | `ScopeScales` | one flag per docked scope, set before drawing; missing entries mean `false` |
| glow flag | `ScopeElm` | the same for a scope placed in the circuit; set with the view, cleared with it (`reset`) |
| glow set | computed in `Session::scope_glow()` | selected elements ∪ hovered element |

Rule: scope `i` glows ⇔ some plot of `i` has `elm` in the glow set. A plot whose element index no
longer exists never matches. No state change is saved, undone or sent to the worker.

Colours (constants in `render/scopes.rs`): `GLOW_DARK = (14, 28, 60)`, `GLOW_LIGHT = (222, 232, 255)`.
