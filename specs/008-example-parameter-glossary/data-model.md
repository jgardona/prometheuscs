# Data Model

## Symbol (row of a document's symbols table, and of the glossary)

| Field | Meaning | Rule |
|-------|---------|------|
| notation | the LaTeX as written (`V_{GS}`) | first column; unique per table after normalisation |
| spellings | plain-text forms found in prose (`Vgs`, `Id`) | glossary only; every spelling maps to one entry |
| name | short name ("tensão porta-fonte") | non-empty; equal in document and glossary (FR-007) |
| meaning | one plain-language sentence | non-empty |
| unit | `V`, `A`, `Ω`, `s`, `—` for dimensionless | non-empty; equal in document and glossary |
| example | stem of one document that uses it | glossary only; the file exists and uses the symbol |

A symbol with several meanings is several entries, distinguished by notation or by the context in
the name. Uniqueness key: normalised notation + name.

## Component field (row of the glossary area "Campos dos componentes")

| Field | Meaning | Rule |
|-------|---------|------|
| field | name as shown in the component table (`Threshold Voltage`) | exactly one entry per distinct name |
| components | element classes that have it | informative |
| unit | unit of the value, or `—` | non-empty |
| effect | what it changes in the circuit | non-empty |

## Example document (existing)

Gains one section `## Símbolos e parâmetros` holding a symbols table with the columns
`Símbolo | Nome | Significado | Unidade`. Relations: every symbol token used in its math is a row;
every row has a glossary entry; every field name of its generated component table has a
glossary field entry.

## Index progress (generated)

Per category: `símbolos` = documents with the section / documents of the category; shown as
`documentado` when equal, else `pendente (X de Y)`.
