# Contract: the developer tool `exdocs`

`cargo run --release --bin exdocs -- <command>`

| Command | What it does | Exit status |
|---------|--------------|-------------|
| `sync` | rewrites the generated parts: the index, the schematic (`img/<stem>.svg` and its `esquema` region) and the component region of every document | 0 |
| `check` | runs every check of `document-format.md` and the freshness of the index; prints one line per problem | 0 when none, 1 otherwise |
| `facts <example.txt>` | prints what the example contains (title, category, each element with its values, the scopes) as text for the author to read | 0 |
| `skeleton <example.txt>` | writes a new document (`docs/exemplos/<stem>.md`) with the front matter, the navigation line, the sections empty and the component region generated; refuses to overwrite | 0 |
| `probe <example.txt> <t>` | prints, for the instant `t`, the value of every element (voltage difference, current, node voltages) so that the author can choose the expected values of a check; reads the example only | 0 |
| `figure <example.txt> <out.svg>` | writes the schematic of one example (what `sync` puts in `img/`) | 0 |

Output is plain text; problems are `path: message`. The tool changes only `docs/exemplos/`.
The same checks are run by `cargo test --test example_docs`.
