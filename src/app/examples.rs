// SPDX-License-Identifier: GPL-2.0-or-later
//! The tree of bundled example circuits, read from `setuplist.txt`.

/// An entry of the examples menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExampleItem {
    /// A circuit file.
    Circuit {
        /// File name inside the circuits folder.
        file: String,
        /// Menu title.
        title: String,
    },
    /// A submenu.
    Group {
        /// Submenu title.
        title: String,
        /// Entries of the submenu.
        items: Vec<ExampleItem>,
    },
}

/// The parsed list and the circuit loaded at startup.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExampleList {
    /// Top-level entries.
    pub items: Vec<ExampleItem>,
    /// File of the circuit marked with `>`.
    pub startup: Option<String>,
}

/// Parses the text of `setuplist.txt`: `+Name` opens a submenu, `-` closes it, `#` starts a
/// comment, and `file title` is a circuit (`>file title` also marks the startup circuit).
pub fn parse_setup_list(text: &str) -> ExampleList {
    let mut stack: Vec<(String, Vec<ExampleItem>)> = vec![(String::new(), Vec::new())];
    let mut startup = None;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(title) = line.strip_prefix('+') {
            stack.push((title.to_string(), Vec::new()));
        } else if line.starts_with('-') {
            if stack.len() > 1 {
                if let Some((title, items)) = stack.pop() {
                    if let Some(top) = stack.last_mut() {
                        top.1.push(ExampleItem::Group { title, items });
                    }
                }
            }
        } else {
            let (is_startup, rest) = match line.strip_prefix('>') {
                Some(r) => (true, r),
                None => (false, line),
            };
            let (file, title) = rest.split_once(' ').unwrap_or((rest, rest));
            if is_startup {
                startup = Some(file.to_string());
            }
            if let Some(top) = stack.last_mut() {
                top.1.push(ExampleItem::Circuit { file: file.into(), title: title.into() });
            }
        }
    }
    while stack.len() > 1 {
        if let Some((title, items)) = stack.pop() {
            if let Some(top) = stack.last_mut() {
                top.1.push(ExampleItem::Group { title, items });
            }
        }
    }
    let items = stack.pop().map(|(_, i)| i).unwrap_or_default();
    ExampleList { items, startup }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_groups_and_the_startup_circuit() {
        let l = parse_setup_list("# c\n+Basics\na.txt Alpha\n>b.txt Beta\n-\nz.txt Zed\n");
        assert_eq!(l.startup.as_deref(), Some("b.txt"));
        assert_eq!(l.items.len(), 2);
        match &l.items[0] {
            ExampleItem::Group { title, items } => {
                assert_eq!(title, "Basics");
                assert_eq!(items.len(), 2);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bundled_list_has_lrc_as_startup_and_nested_groups() {
        let text = std::fs::read_to_string("assets/setuplist.txt").unwrap();
        let l = parse_setup_list(&text);
        assert_eq!(l.startup.as_deref(), Some("lrc.txt"));
        assert!(l.items.len() > 5);
    }
}
