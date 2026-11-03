# Contract: User-Facing Commands, Options and Gestures

What a user can invoke after this feature. Menu text is the English source string, which the locale files translate (FR-027). The command ids are the names used by the Shortcuts dialog and the settings file; they must stay stable once released.

## Menus and commands

| Command id | Menu text | Location | Default shortcut | Story |
|------------|-----------|----------|------------------|-------|
| `find` (extended) | Find Component... | Edit menu, toolbar | `/`, double Shift | 3 |
| `swap-terminals` | Swap Terminals | Element context menu, Edit menu | — | 3 |
| `scope-export-image` | Export Image... | Scope menu | — | 4 |
| `subcircuits-load-only` | Load Subcircuits Only... | File menu | — | 5 |
| `subcircuits-save-library` | Save Library... | Subcircuit Manager | — | 5 |
| `sram-load-contents` | Load Contents From File... | SRAM edit panel button | — | 2 |
| (element) | Add Controlled Voltage Source | Draw > Active Building Blocks | — | 2 |
| (element) | Add Controlled Current Source | Draw > Active Building Blocks | — | 2 |

## Options

| Setting key | Text | Location | Default |
|-------------|------|----------|---------|
| `show-properties-on-create` | Show Properties on Create | Other Options | on |
| `show-properties-on-click` | Show Properties on Click | Other Options | off |

## Edit panel fields added

| Element | Field | Type |
|---------|-------|------|
| Voltmeter / probe, ammeter | Label | text (empty allowed) |
| Controlled voltage source | Output Voltage | text (expression) |
| Controlled current source | Output Current | text (expression) |
| Monostable | Invert Trigger | checkbox |
| Text | Color | color |

## Gestures

| Gesture | Where | Result |
|---------|-------|--------|
| Drag a part button onto the canvas | Toolbar | Places the part at the drop point |
| Drop a terminal on the interior of a wire or lead | Canvas, when placing or moving | Splits the wire and connects; one undo step |
| One wheel notch | Over a value-bearing element | Value moves one step; popup shows it |
| One wheel notch | Over a transistor, MOSFET or JFET | Toggles polarity or channel |
| One wheel notch | Over a slider element | Moves the slider one step |
| Double Shift within 400 ms | Anywhere except a text entry | Opens Find Component |
| Drag inside a scope | Scope area | Moves only scope content (never circuit elements) |
| Click a checkbox's label | Any dialog | Toggles the checkbox |

## Find Component result groups

In this order, each filtered by case-insensitive substring of the localized name:

1. Recent (last 10 picks, newest first)
2. Components to place (every Draw menu entry, including subcircuits)
3. Commands (every menu command)
4. Elements in this circuit (the current prometheuscs list)
