# Render check of the PDF

- `python3 docs/livro/build_book.py` ends with the PDF path and prints no `aviso:` line; xelatex
  runs to the end (no undefined references: the log has no `Undefined`).
- A MOSFET example (`nmosfet`, page 6 of a partial build) shows the symbols table between the
  components table and "Teoria", with the glossary reference "(apêndice A, p. N)".
- The glossary appendix (page 1109 onward) has readable tables; the column headers fit after the
  header-width fix in `tabela()` (`docs/livro/build_book.py`).
- Two bugs found by this check and fixed: a Greek or `\infty` subscript wrapped in `\text` by the
  authoring aid (`L_\sigma`, `I_\infty`), which xelatex rejected.
