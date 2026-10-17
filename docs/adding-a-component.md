# Adding a component

A component is one type that implements `Element` plus one registry entry. Nothing in
`src/solver`, `src/app`, the menus, the file loader or the example catalog changes: the Draw
menu, the toolbar, the factory and the `inventory` tool are all generated from the registry.

[`tests/modularity.rs`](../tests/modularity.rs) does exactly what this guide describes, from
a test file, without editing anything under `src/`. If this guide and that test disagree, the
test is right.

## 1. The element

A two-terminal element with one parameter, the least a new component needs:

```rust
use prometheuscs::elements::element::{Base, Element, default_dump_xml};
use prometheuscs::format::xml::{XmlNode, XmlOut};
use prometheuscs::impl_element_base;

#[derive(Clone, Debug)]
struct DummyElm {
    base: Base,   // position, flags, post voltages, current
    gain: f64,    // the parameter
}

impl Element for DummyElm {
    impl_element_base!("DummyElm");        // class name, base(), as_any() and the like

    fn dump_type(&self) -> i32 { 9001 }    // the code a text-format file uses
    fn dump(&self) -> String {             // one line of the text format
        format!("{} {}", self.base.dump_prefix(self.dump_type()), self.gain)
    }
    fn dump_xml(&self, out: &mut XmlOut) { // the XML format
        default_dump_xml(&self.base, out);
        out.double_attr("g", self.gain);
    }
    fn undump_xml(&mut self, xml: &XmlNode) {
        self.base.flags = xml.int_attr("f", self.base.flags);
        self.gain = xml.double_attr("g", self.gain);
    }
}
```

Every other method of `Element` has a default, so this already loads, saves and draws as a
plain two-post element. Override what the part really does:

| To do this | Override |
|---|---|
| put a resistance, source or conductance in the matrix | `stamp` (once) and `do_step` (every iteration, for nonlinear parts) |
| keep state between time steps | `step_finished`, `reset`, and `visual_state` / `set_visual_state` if the drawing depends on it |
| draw it | `draw` (the default draws a line between the posts) |
| make fields editable | `get_edit_info` and `set_edit_value` |
| say what it is in the info box | `get_info` |
| add more than two posts | `post_count`, `get_post`, `set_points` |
| show a number on a scope | `scope_value`, `scope_units` |

Two rules that bite:

- **The window draws from a copy.** The simulation runs on a worker thread and the window
  keeps its own copy of the elements, which only receives voltages, currents and
  `visual_state` each frame. If `draw` uses anything else learned while stepping (a position,
  a buffer, a temperature), put it in `visual_state`. `tests/worker_draw.rs` checks every
  bundled example for this.
- **No panics.** Return `Result` or an error message; `unwrap`, `expect` and `panic!` are
  warnings in the crate and an error in CI.

## 2. The registry entry

```rust
use prometheuscs::elements::registry::{ElementDescriptor, Registry};

pub fn register(r: &mut Registry) {
    r.add(
        ElementDescriptor::new("DummyElm", "myfamily", "Add Dummy", "My Parts", |x, y| {
            Box::new(DummyElm { base: Base::new(x, y), gain: 1.0 })
        })
        .dump(9001, |a, b, c, d, f, st| {
            Ok(Box::new(DummyElm { base: Base::from_dump(a, b, c, d, f), gain: st.next_f64()? }))
        })
        .shortcut('d'),
    );
}
```

The arguments of `new` are the class name, the family (the Cargo feature that owns it), the
menu text, the Draw submenu, and the factory used for placement. `.dump` gives the code and the
reader for files. Options: `.shortcut(char)`, `.icon("name")` (an SVG in
`assets/icons/components/`; a generic icon is used without one), `.quick()` (a toolbar button)
and `.hidden()` (not in the menu).

A duplicate class name or dump code is reported by `Registry::errors()` and fails the registry
test.

## 3. Putting it in the program

In a family file under `src/elements/`:

1. add the type and its `register` function (as above);
2. for a new family, add one `pub mod`, one `register` call in `elements::register_all`, and
   one Cargo feature gating both lines (and add the feature to `default`).

Then add the tests the element needs: a text and XML round trip, a stamping check against a
number you can work out by hand (see `tests/solver_basic.rs`), and, for a family, the
`#[cfg(feature = "...")]` on every test that needs it.

## 4. Removing a component or a family

Disable the family's feature in `Cargo.toml`, or delete its file, its `mod` line, its
`register` call and its feature. Nothing else changes. A circuit that uses the missing part
loads with an `UnknownType` error naming it and the rest of the circuit loads.

CI builds and tests the program with each family left out in turn (the `families` job in
`.github/workflows/ci.yml`). To run one configuration yourself:

```bash
cargo test --no-default-features --features passive,reactive,sources,switches,logic,diodes,analog,transistors,mosfets,digital,subcircuits   # without `outputs`
```

Code outside `src/elements/` that needs a family must be behind the same feature; that is
what the job catches.
