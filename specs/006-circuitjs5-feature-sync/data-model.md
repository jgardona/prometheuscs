# Data Model: CircuitJS 5.x Feature Sync

Phase 1 output for [plan.md](plan.md). Describes the state each feature adds and the rules it must keep. File-level formats are in [contracts/](contracts/).

## Expression reference

A name used inside an expression.

| Field | Meaning | Rules |
|-------|---------|-------|
| `name` | Text between the parentheses | Non-empty; compared exactly (case-sensitive), as upstream |
| `current` | `true` for `i()` / `didt()`, `false` for `v()` / `dvdt()` | Fixed at parse time |
| `derivative` | Per use, not per slot: `dvdt()` / `didt()` read `(x − last x) / timestep` of the slot | — |
| `slot` | Index k; owner node slots `ref_base + 2k` and `ref_base + 2k + 1` | Assigned in order of first use; one slot per (`name`, `current`): `v(x)` and `dvdt(x)` share a slot |
| `resolved` | Labeled node, meter element, or nothing | Set by the resolve pass on every analysis |

Resolution order: (1) a labeled node with that name (voltage references only); (2) the first element in circuit order whose reference name equals `name`; (3) unresolved, both slots on ground and the info box shows `unknown name: v(name)`. A current reference resolved to an element without a voltage source shows `not a current meter: name`.

## Expression owner (extended)

Elements that evaluate an expression: the new controlled voltage and current sources, and the existing VCVS, VCCS, CCVS and CCCS.

| Field | Meaning |
|-------|---------|
| `expr_string`, `expr` | Text and parsed form (existing) |
| `pin_inputs` | Number of inputs from the element's own pins, `a`..`i` (0 for the two-terminal sources) |
| `refs` | List of expression references |
| `ref_values`, `last_ref_values` | Current and previous-step values of each reference |
| `last_inputs` | Previous iteration's input values, for the numeric derivative and convergence check |

Node count of an owner = posts + internal nodes + 2 × `refs.len()`. Editing the expression can change the node count, so it triggers a full re-analysis.

## Controlled source (new: `cv`, `ci`)

| Field | Meaning | Default |
|-------|---------|---------|
| posts | 2 (negative, positive) | — |
| `expr_string` | Output voltage (`cv`) or current (`ci`) | Upstream defaults (`getDefaultExpr`) |
| flags | Standard element flags | 0 |

`cv` owns one voltage source; `ci` stamps a current. Both are nonlinear. Lifecycle: parse at load and on edit → resolve on analysis → evaluate and stamp each iteration → roll last values in `step_finished` → clear in `reset`.

## Meter label (extended: `ProbeElm`, `AmmeterElm`)

| Field | Meaning | Rules |
|-------|---------|-------|
| `label` | Optional reference name | Empty means "not referenceable"; saved only when non-empty (`lb`); drawn next to the meter |

## Subcircuit model (extended)

| Field | Meaning | Rules |
|-------|---------|-------|
| `scope` | `Local` (this circuit), `Global` (this session), `Persistent` (saved in the user library) | Persistent ⊂ Global; changing the scope never changes the model's contents |
| child `ref_nodes` | Per child with references: model node numbers for each slot (`rn`) | Written when the model is built; read when an instance expands |

Transitions: Local → Global → Persistent and back, from the Subcircuit Manager. Deleting is refused while any instance in the current circuit uses the model.

## Editor preferences (new settings)

| Key | Type | Default |
|-----|------|---------|
| `show_properties_on_create` | bool | true |
| `show_properties_on_click` | bool | false |
| `palette_recent` | list of command or component ids, newest first | empty, at most 10 |

Registry descriptor gains `quiet_on_create: bool` (true for elements rarely edited when placed).

## Scope image export request

| Field | Rules |
|-------|-------|
| `format` | PNG or SVG |
| `width`, `height` | Integers, 1 to 8192 |
| `transparent` | bool; when true the background is not filled |

## Element additions (small fields)

| Element | Field | File attribute |
|---------|-------|----------------|
| Monostable | invert trigger (flag bit) | as upstream (`f`) |
| Text | color | as upstream |
| SRAM | — (action only: load contents from a file of numbers) | contents written as today |
| Stop Trigger | "armed" state, cleared on load and reset | runtime only |
