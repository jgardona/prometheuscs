# Contract: format of an example document

File: `docs/exemplos/<stem>.md`. Language: Portuguese.

```markdown
---
exemplo: filt-lopass.txt
titulo: Low-Pass Filter (RC)
categoria: Passive Filters
verificacoes:
  - t=0.02 elemento=2:CapacitorElm grandeza=vd fn=rms de=0.01 esperado=0.707 tol=0.03
sem-verificacao:            # only when there is nothing to check; then the reason, and no list
---

# Filtro passa-baixas RC

<!-- navegacao:inicio -->
[Índice](README.md) · [Anterior](filt-hipass.md) · [Próximo](filt-hipass-l.md)
<!-- navegacao:fim -->

## Objetivo
## O circuito
<!-- componentes:inicio -->
(generated table)
<!-- componentes:fim -->
## Teoria
## Resultados esperados
## O que observar
## Limites do modelo
## Referências
```

Rules checked by `exdocs check` and `tests/example_docs.rs`:

0. There is one document per distinct example file: a file listed under two categories has one
   document, its `categoria` and its navigation line follow the first category in the menu, and
   the other category links to the same document.
1. The front matter has `exemplo`, `titulo`, `categoria` and exactly one of `verificacoes` (not
   empty) or `sem-verificacao` (a reason); `exemplo` is the entry of the menu this file is for.
2. The seven sections are present, in this order, each with text.
3. The navigation region (index, previous and next of the same category) and the component region exist and are identical to what the tool generates (the component region is skipped when the example
   does not load in this build).
4. `$$` delimiters are balanced; `$` delimiters are balanced in each paragraph; no
   `\usepackage` or `tabular`.
5. Every ` ```mermaid ` block starts with a known diagram keyword and is not empty.
6. Every relative link points to an existing file.
7. Every check of `verificacoes` runs and agrees with the expected value within its tolerance
   (skipped when the example does not load in this build).

8. The schematic region (`<!-- esquema:inicio -->` … `<!-- esquema:fim -->`, under `## O circuito`)
   holds `![Esquema do circuito](img/<example>.svg)` and `img/<example>.svg` is identical to the
   picture the tool draws from the example file (skipped when the example does not load in this
   build).

The check line keys are `t`, `de` (start of a window), `elemento=N:Classe`, `grandeza` (`vd`, `i`,
`v<k>`), `fn` (`valor`, `max`, `min`, `rms`, `media`, `freq`), `esperado`, `tol` or `abs`, and
`alterna=N[@t],M` which flips elements N and M (a switch or a logic input) once, at the time
after `@` in seconds (0 when there is none), so that a document can state the result of a given
input combination or of a sequence of button presses.

A document without an entry in the menu, or an entry in a "documentado" category without a
document, is an error (FR-009).
