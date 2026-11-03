# Contract: Expression Reference Functions

Additions to the expression language used by controlled sources (FR-002). Everything else in the language is unchanged.

## Grammar additions

```text
reference   := ( "v" | "dvdt" ) "(" name [ "," name ] ")"
             | ( "i" | "didt" ) "(" name ")"
name        := characters up to "," or ")" with surrounding spaces trimmed,
               or a double-quoted string (which may contain "," and ")");
               case is kept and names are compared exactly
```

| Form | Value | Unit |
|------|-------|------|
| `v(name)` | Voltage of the labeled node `name`, or across the labeled voltmeter `name` | V |
| `v(a, b)` | `v(a) - v(b)` | V |
| `i(name)` | Current through the labeled ammeter `name` | A |
| `dvdt(name)` | `(v(name) − v(name) at previous step) / timestep` | V/s |
| `didt(name)` | `(i(name) − i(name) at previous step) / timestep` | A/s |

## Disambiguation (must match upstream)

- `i` followed directly by `(` is a current reference; any other `i` is the 9th input.
- `didt` not followed by `(` keeps its old meaning (derivative of input `i`). `dvdt` without `(` is a parse error.
- A labeled node wins over a meter with the same name for `v()`.
- `i()` never reads a labeled node: `i(x)` where `x` is only a labeled node is an unknown name.
- Each distinct quantity has one slot: `v(x)` and `dvdt(x)` share it, `v(x)` and `i(x)` do not.
- Inside a subcircuit, names resolve only within that subcircuit.

## Errors

| Situation | Where it shows | Text (localized) |
|-----------|----------------|------------------|
| Syntax error when editing | Edit panel or dialog; the old value is kept | `Parse error in expression: <text>: <detail>` |
| Name not found | Element info box; the reference reads 0 | `unknown name: v(<name>)` or `unknown name: i(<name>)` |
| `i()` names a labeled element that is not a current meter (a voltmeter) | Element info box; the reference reads 0 | `not a current meter: <name>` |

## Numeric behavior

The output is linearized each iteration with a numeric derivative per input: step = change since the last iteration, at least 1e-6; derivative magnitude at least 1e-6. Convergence limits widen with sub-iterations (0.001 under 10, 0.01 under 200, then 0.1), and current inputs use a limit ten times tighter. These constants are what make results match the 5.0.4 recordings.

## Examples

| Expression | Meaning |
|------------|---------|
| `2*v(in)` | Twice the voltage at node `in` |
| `10*i(ib)` | 10 Ω transresistance from ammeter `ib` |
| `v(a)-v(b)` | Difference of two labeled nodes |
| `1e-6*dvdt(vc)` | Current of a 1 µF capacitor across voltmeter `vc` |
