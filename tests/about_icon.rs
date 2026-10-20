// SPDX-License-Identifier: GPL-2.0-or-later
// Integration tests may unwrap: a failure should fail the test.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! The emblem shown in the About dialog: the shipped file meets its contract and is credited.

use prometheuscs::app::consts::assets_dir;
use prometheuscs::app::icons::app_icon_svg;
use prometheuscs::format::xml::{XmlNode, parse};

/// Every element of the tree, the root included.
fn walk<'a>(node: &'a XmlNode, out: &mut Vec<&'a XmlNode>) {
    out.push(node);
    for child in &node.children {
        walk(child, out);
    }
}

#[test]
fn the_emblem_is_a_small_square_text_free_svg() {
    let path = app_icon_svg(&assets_dir());
    let text = std::fs::read_to_string(&path).expect("the emblem file exists");
    assert!(text.len() < 8192, "{} bytes", text.len());
    assert!(!text.contains("currentColor"), "the emblem keeps its own colours");
    assert!(!text.contains("data:"), "no embedded raster data");
    let root = parse(&text).expect("well-formed XML");
    assert_eq!(root.tag, "svg");
    let view_box = root.attr("viewBox").unwrap_or_default();
    let numbers: Vec<f64> = view_box.split_whitespace().filter_map(|n| n.parse().ok()).collect();
    assert_eq!(numbers.len(), 4, "viewBox `{view_box}`");
    assert_eq!(numbers[2], numbers[3], "the emblem is square");
    let mut all = Vec::new();
    walk(&root, &mut all);
    assert!(all.iter().all(|n| n.tag != "text"), "the emblem has no letters");
    for n in &all {
        for name in ["href", "xlink:href"] {
            let target = n.attr(name).unwrap_or("#");
            assert!(target.starts_with('#'), "`{}` refers to another file: {target}", n.tag);
        }
    }
}

#[test]
fn the_emblem_is_credited_as_the_projects_own_artwork() {
    let attribution = std::fs::read_to_string(assets_dir().join("ATTRIBUTION.md"))
        .expect("the attribution file exists");
    assert!(attribution.contains("org.prometheuscs.App.svg"));
}
