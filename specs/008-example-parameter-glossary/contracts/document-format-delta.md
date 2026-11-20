# Contract delta: example document format

Adds to `specs/004-circuit-example-docs/contracts/document-format.md`:

```markdown
## O circuito
...
## Símbolos e parâmetros

Os símbolos usados neste documento (definições completas no [glossário](../glossario-simbolos.md)):

| Símbolo | Nome | Significado | Unidade |
|---------|------|-------------|---------|
| $V_t$ | tensão de limiar | $V_{GS}$ a partir da qual o canal começa a conduzir | V |
| $I_D$ | corrente de dreno | corrente que entra pelo dreno | A |
| $V_{GS}$ | tensão porta-fonte | tensão da porta em relação à fonte | V |

## Teoria
```

Sections order becomes: Objetivo, O circuito, **Símbolos e parâmetros**, Teoria, Resultados
esperados, O que observar, Limites do modelo, Referências.

New rules (documents that have the section; all documents once the final switch is on):

9. The section exists, in that position, with a table of the four columns and at least one row.
10. Every symbol token used in the document's math is a row of the table (ignore list in
    research R2); every row is used somewhere in the document (no dead rows).
11. Every row matches a glossary entry (same notation, name and unit).
12. Every field name of the generated component table has an entry in the glossary.
13. The section links the glossary and the link resolves (existing rule 6).

Untouched: front matter, `verificacoes`, generated regions (navigation, schematic, components),
and all numbers in "Resultados esperados".
