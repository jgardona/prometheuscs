// SPDX-License-Identifier: GPL-2.0-or-later
//! Icon layer: maps the glyph names of the original's menus and toolbar to GTK/Adwaita
//! symbolic icons, falling back to the SVG files imported from the original.

use std::path::{Path, PathBuf};

/// Where an icon comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Icon {
    /// A symbolic icon of the GTK icon theme.
    Symbolic(&'static str),
    /// An SVG file imported from the original.
    File(PathBuf),
}

/// Every glyph the menus and the toolbar use.
pub const GLYPHS: &[&str] = &[
    "window",
    "doc-new",
    "folder",
    "doc-text",
    "floppy",
    "export",
    "image",
    "microchip",
    "magic",
    "back-in-time",
    "print",
    "resize-full-alt",
    "info-circled",
    "reply",
    "forward",
    "scissors",
    "copy",
    "paste",
    "clone",
    "select-all",
    "search",
    "target",
    "zoom-11",
    "zoom-in",
    "zoom-out",
    "flip-x",
    "flip-y",
    "ccw",
    "cw",
    "lines",
    "columns",
    "object-group",
    "object-ungroup",
];

/// The GTK symbolic icon that stands in for a glyph, when the theme has a good equivalent.
pub fn symbolic_name(glyph: &str) -> Option<&'static str> {
    Some(match glyph {
        "window" => "window-new-symbolic",
        "doc-new" => "document-new-symbolic",
        "folder" => "document-open-symbolic",
        "doc-text" => "text-x-generic-symbolic",
        "floppy" => "document-save-symbolic",
        "export" => "document-send-symbolic",
        "image" => "image-x-generic-symbolic",
        "print" => "document-print-symbolic",
        "resize-full-alt" => "view-fullscreen-symbolic",
        "info-circled" => "help-about-symbolic",
        "reply" => "edit-undo-symbolic",
        "forward" => "edit-redo-symbolic",
        "scissors" => "edit-cut-symbolic",
        "copy" => "edit-copy-symbolic",
        "paste" => "edit-paste-symbolic",
        "select-all" => "edit-select-all-symbolic",
        "search" => "edit-find-symbolic",
        "zoom-11" => "zoom-original-symbolic",
        "zoom-in" => "zoom-in-symbolic",
        "zoom-out" => "zoom-out-symbolic",
        "flip-x" => "object-flip-horizontal-symbolic",
        "flip-y" => "object-flip-vertical-symbolic",
        "ccw" => "object-rotate-left-symbolic",
        "cw" => "object-rotate-right-symbolic",
        _ => return None,
    })
}

/// Path of the imported SVG of a glyph under `assets`.
pub fn glyph_svg(glyph: &str, assets: &Path) -> PathBuf {
    assets.join("icons").join("ui").join(format!("{glyph}.svg"))
}

/// Resolves a glyph to an icon: the symbolic theme icon if there is one, else the imported
/// SVG, else nothing.
pub fn resolve(glyph: &str, assets: &Path) -> Option<Icon> {
    if let Some(name) = symbolic_name(glyph) {
        return Some(Icon::Symbolic(name));
    }
    let svg = glyph_svg(glyph, assets);
    svg.is_file().then_some(Icon::File(svg))
}

/// A copy of the SVG at `path` with its fixed colours replaced by one that is visible on the
/// current theme, cached under the configuration folder. Imported icons are drawn in black or
/// in `currentColor`, which GTK does not recolour for non-symbolic images.
pub fn themed_svg(path: &Path, dark: bool) -> Option<PathBuf> {
    let text = std::fs::read_to_string(path).ok()?;
    let color = if dark { "#e6e6e6" } else { "#2e3436" };
    let text = text.replace("currentColor", color).replace("#000000", color);
    let dir = super::settings::config_dir().join("icon-cache");
    std::fs::create_dir_all(&dir).ok()?;
    let name =
        format!("{}-{}", if dark { "dark" } else { "light" }, path.file_name()?.to_string_lossy());
    let out = dir.join(name);
    if std::fs::read_to_string(&out).ok().as_deref() != Some(text.as_str()) {
        std::fs::write(&out, text).ok()?;
    }
    Some(out)
}

/// Path of a component icon (for example `resistor`) under `assets`, when it was imported.
pub fn component_svg(name: &str, assets: &Path) -> Option<PathBuf> {
    let p = assets.join("icons").join("components").join(format!("{name}.svg"));
    p.is_file().then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
    }

    #[test]
    fn every_menu_and_toolbar_glyph_resolves() {
        for g in GLYPHS {
            assert!(resolve(g, &assets()).is_some(), "no icon for `{g}`");
        }
    }

    #[test]
    fn glyphs_without_a_symbolic_equivalent_use_the_imported_svg() {
        assert!(matches!(resolve("magic", &assets()), Some(Icon::File(_))));
        assert!(resolve("undo-missing", &assets()).is_none());
        assert!(matches!(resolve("reply", &assets()), Some(Icon::Symbolic("edit-undo-symbolic"))));
    }

    #[test]
    fn themed_copies_replace_the_fixed_colours() {
        let src = component_svg("resistor", &assets()).unwrap();
        let dark = themed_svg(&src, true).unwrap();
        let text = std::fs::read_to_string(dark).unwrap();
        assert!(
            text.contains("#e6e6e6") && !text.contains("currentColor") && !text.contains("#000000")
        );
        let light = themed_svg(&src, false).unwrap();
        assert!(std::fs::read_to_string(light).unwrap().contains("#2e3436"));
    }

    #[test]
    fn component_icons_are_found() {
        assert!(component_svg("resistor", &assets()).is_some());
        assert!(component_svg("no-such-part", &assets()).is_none());
    }
}
