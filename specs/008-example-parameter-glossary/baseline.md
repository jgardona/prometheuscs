# Baseline (before the feature)

On the untouched tree (branch `feature/enhance-docs`, commit 6eec91f): `exdocs check` printed
`tudo certo` (2 min 27 s in release mode) and `cargo test` passed. The committed PDF had 923
pages. After the feature: 365 documents with the symbols section, `exdocs check` green, the PDF
has 1155 pages (glossary as appendix A, from page 1109).
