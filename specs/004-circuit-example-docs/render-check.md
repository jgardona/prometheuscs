# Render check (T052)

**Status: done by hand on 2026-11-12 (user, VS Code Markdown preview), five documents.** One
problem found and fixed: `jerk.md` used `\dddot`, which the preview does not know (it showed in
red); both equations now use `\overset{\cdots}{x}`. No other document in `docs/` uses `\dddot`.
The other four documents opened showed nothing raw. Not confirmed by the user: which documents
those were, the index, the two Mermaid diagrams and phone width. The text below is the
original note from before this check.

**Original note: not done as written.** The task asks to open the index and five documents on the
GitHub preview (or a Markdown preview with math and Mermaid) and look at them. That was not
possible in the session that wrote waves 9 and 10: there was no rendered preview to look at, and
nothing below was seen on a screen. What was and was not checked:

## Checked by machine (every document, `exdocs check`)

- `$$` delimiters balanced, `$` balanced per paragraph, no `\usepackage` or `tabular`;
- every ` ```mermaid ` block starts with a known diagram keyword and is not empty (two documents
  use Mermaid: `cmosff.md` and `cmosmsff.md`);
- every relative link (navigation line, index, schematic image) points to an existing file;
- `docs/exemplos/img/` has 365 pictures for 366 documents (the blank circuit has nothing to
  draw), and each picture is identical to what `exdocs figure` draws from the example;
- the index has every document linked, in the order of the menu (`tests/example_docs.rs`).

## Not seen

- whether GitHub renders each equation (for instance the `\text{...}` units inside `$...$`, the
  `\bigl[ ... \bigr]` pairs and the `\parallel` of the transmission-line and OTA documents);
- whether the tables with math in cells (the transmission-line, memristor and OTA documents)
  keep their columns;
- the two Mermaid diagrams;
- the SVG schematics at phone width.

Someone should open the index and a handful of documents from different categories on GitHub
(or in a preview with KaTeX and Mermaid) and record here what shows raw. This task stays open.
