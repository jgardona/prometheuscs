# Contract: Circuit File Additions (CircuitJS 5.0.4 XML)

prometheuscs MUST read and write these exactly as CircuitJS 5.0.4 does, so files round-trip in both directions (FR-001, FR-003, FR-005, FR-008, SC-002). Everything not listed here is unchanged from the format prometheuscs already supports.

## New element tags

| Tag | Element | Attributes beyond `x` and `f` | Default when created |
|-----|---------|-------------------------------|----------------------|
| `cv` | Controlled voltage source (diamond, two posts) | `ex`: output-voltage expression | `ex="v(in)"` |
| `ci` | Controlled current source (diamond, two posts) | `ex`: output-current expression | `ex=".001*v(in)"` |

These elements have no legacy text-format line (upstream dump type 0). When the user saves a circuit containing them in the legacy text format, prometheuscs MUST either save as XML or warn before saving (spec edge case).

Example (upstream `cs-resistor.txt`, 5.0.4):

```xml
<ci x="560 240 560 336" f="0" ex="v(vd)/1000"/>
<p x="464 240 464 384" f="3" me="0" sc="0" re="0" lb="vd"/>
```

## New attributes on existing tags

| Tag | Attribute | Meaning | Written when |
|-----|-----------|---------|--------------|
| `p` (voltmeter / probe) | `lb` | Reference name | Label non-empty |
| ammeter tag | `lb` | Reference name | Label non-empty |
| text tag | `co` | Text color | A custom color is set |
| monostable tag | flag bit 2 in `f` | Invert trigger | Always part of `f` |
| any child inside a subcircuit model | `rn` | Model node numbers of the child's expression references, two per reference | The child has references |

## Expressions in `ex`

Any `ex` attribute (the new tags and the existing VCVS, VCCS, CCVS and CCCS tags) may contain the reference functions in [expression-references.md](expression-references.md).

## Subcircuit library files

A file whose root is `<cir … sublib="1">` contains only subcircuit model definitions. Loading it with "Load Subcircuits Only" adds those models and does not touch the current circuit. "Save library" writes the selected models in this form. The user's persistent library uses the same format.

## Compatibility rules

- A 4.x file using the custom composite (`cc`) loads unchanged; `cc` is also the 5.x subcircuit tag.
- An unknown attribute on a known tag is ignored and reported once in the load-problems list (current behavior).
- Saving must not write attributes upstream would read differently, for example a transformer saturation of `undefined` (N20).
