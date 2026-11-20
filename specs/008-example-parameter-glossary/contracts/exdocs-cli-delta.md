# Contract delta: `exdocs`

- `exdocs check` prints one line per problem for the new rules 9–13 and the glossary rules, in the
  existing format (`<document>: <message>`; glossary problems use `glossario-simbolos.md:`).
- `exdocs sync` regenerates the index with, per category, `símbolos: documentado` or
  `símbolos: pendente (X de Y)`, and a link to the glossary in the index header; it does not write
  the glossary or the symbols sections (they are authored).
- `exdocs symbols <exemplo>` (developer aid, read-only): prints the symbol tokens extracted from a
  document, the field names of its component table, and what is missing from its table and the
  glossary, to help authoring a wave.
- Exit status unchanged: 1 when there are problems.
