# Data Model: Scientific documentation for every circuit example

No program data changes. The documents and the tool work with these entities.

## ExampleEntry (from `assets/setuplist.txt`)

| Field | Meaning |
|-------|---------|
| `file` | circuit file name inside `assets/circuits`, for example `filt-lopass.txt` |
| `title` | the menu title |
| `path` | the groups it sits in, for example `Passive Filters` or `Logic Families > RTL` |
| `stem` | `file` without `.txt`: the document is `docs/exemplos/<stem>.md` |

Rules: the entries come from `app::examples::parse_setup_list`; the order is the menu's. The
same `file` can appear in two places of the menu (at this writing `cmosinverter.txt`, under
*MOSFETs* and *Logic Families > CMOS*): there are 367 entries and 366 distinct files. The **first**
occurrence in menu order is the entry's *home*: the document's `categoria`, previous and next are
those of the home; every other occurrence only links to the same document.

## CircuitDocument (`docs/exemplos/<stem>.md`)

| Part | Rule |
|------|------|
| front matter | `exemplo` (the `file`), `titulo`, `categoria` (the `path`), `verificacoes` (list) or `sem-verificacao` (a reason) |
| title (`# …`) | the menu title |
| navigation line | links to the index, the previous and the next example of the same category |
| sections | `Objetivo`, `O circuito`, `Teoria`, `Resultados esperados`, `O que observar`, `Limites do modelo`, `Referências` |
| component region | between `<!-- componentes:inicio -->` and `<!-- componentes:fim -->`, generated |
| math | `$…$` and `$$…$$`, balanced |
| diagrams | zero or more ` ```mermaid ` blocks, optional |

## Check (an item of `verificacoes`)

| Field | Meaning |
|-------|---------|
| `t` | simulated time in seconds at which the value is read (or the end of the window) |
| `elemento` | `N:Tipo`, the element number in file order and its class name (the class guards against a renumbering) |
| `grandeza` | `vd` (voltage across), `i` (current), `v<k>` (voltage of terminal k) |
| `fn` | `valor` (default, at `t`), or over the window `de`..`t`: `max`, `min`, `rms`, `media`, `freq` |
| `esperado` | the expected value in SI units |
| `tol` | relative tolerance (default 0.02), or `abs` for an absolute one |

## Index (`docs/exemplos/README.md`)

| Part | Rule |
|------|------|
| summary | how many entries, how many documented, per top-level category |
| category block | heading with the status `documentado` or `pendente (n de m)`; the entries in menu order; a link when the document exists, plain title otherwise; a duplicated file is listed in both categories with the same link, and counts in both |
| freshness | equals the text `exdocs sync` would write (checked) |

## State of a category

`documentado` ⇔ every entry under it (sub-groups included) has a document (a duplicated file counts in each category where it is listed); otherwise
`pendente (n de m)`. There is no other state (spec FR-011).
