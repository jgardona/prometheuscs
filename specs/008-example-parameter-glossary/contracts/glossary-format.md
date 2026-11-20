# Contract: `docs/glossario-simbolos.md`

Language: Portuguese. One file. Structure:

```markdown
# Glossário de símbolos e parâmetros

<intro: how to read; link to the index and the book>

## Tensões e correntes
| Símbolo | Grafias | Nome | Significado | Unidade | Exemplo |
|---------|---------|------|-------------|---------|---------|
| $V_{GS}$ | `Vgs` | tensão porta-fonte | ... | V | [nmosinv](exemplos/nmosinv.md) |

## ... one `##` per area ...

## Campos dos componentes
| Campo | Componentes | Unidade | Efeito | Exemplo |
|-------|-------------|---------|--------|---------|
| Threshold Voltage | `MosfetElm` | V | ... | [...](exemplos/...md) |
```

Rules:

1. Symbol tables have the six columns, field tables the five; no empty cell.
2. Key uniqueness: normalised notation + name for symbols; field name for fields.
3. A spelling belongs to exactly one entry.
4. The example exists and uses the symbol (symbols) or has the field (fields).
5. Every symbol and field used in any document has an entry (no missing); every entry is used by
   at least one document (no orphans).
6. Links relative to `docs/`; `$` balanced; same LaTeX subset as the documents.
