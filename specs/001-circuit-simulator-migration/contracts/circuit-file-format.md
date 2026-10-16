# Contract: Circuit File Format

prometheuscs reads and writes the original simulator's text format (FR-009). This contract states what the port guarantees; the Java sources (`CircuitLoader.java`, each element's `dump`/constructor) are the normative reference.

## Structure

1. **Header line**: begins with `$` followed by flags, time step, speed parameters, current speed and power range. Older files may begin with `%` options lines; both are accepted.
2. **Element lines**: one per element. Format: `<type> <x1> <y1> <x2> <y2> <flags> <kind-specific fields...>`. The type is a letter or number (for example `r` resistor, `c` capacitor, `l` inductor, `w` wire, `v` voltage source, `R` rail, `o` scope).
3. **Special lines**: scopes (`o`), adjustables (`38`), hints (`h`), custom models (`.`), text and box elements.
4. Fields are space separated. Text with spaces uses the original escaping (`\s`, `\n`, `\\` style escapes, as implemented by `CustomLogicModel.escape`).

## Guarantees

- **G1 Read compatibility**: Every file in `assets/circuits` loads without error.
- **G2 Round trip**: Load then save of an unmodified circuit yields text that reloads to an equal circuit; for the 373 bundled examples the saved text is byte-identical except for documented whitespace/float formatting normalizations.
- **G3 Forward behavior**: Unknown element type or malformed line produces an error that names the line number and type; remaining lines still load.
- **G4 Text export**: "Export as Text" output equals the save text and can be re-imported through "Import from Text".
- **G5 Share link**: "Export as Link" produces a URL-safe compressed encoding of the same text; importing the link yields an identical circuit. The compression scheme matches the original so existing links import.
- **G6 Numerical text**: Floating-point values are written with enough digits to round-trip `f64`.

## Verification

- `tests/compat` loads every bundled file and checks G1 to G3.
- A proptest generates random valid element lines and checks parse(dump(x)) equals x.
- Example example lines used as fixtures: `r 176 80 384 80 0 10`, `w 176 80 176 352 0`.
