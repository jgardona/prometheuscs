// SPDX-License-Identifier: GPL-2.0-or-later
//! The toolbar under the menu bar: edit commands, zoom, quick-insert parts and the mode text.

use super::commands::Cmd;
use super::icons;
use super::menus::{gicon, glyph_image};
use crate::elements::registry::Registry;
use crate::locale::ls;
use relm4::gtk::{self, prelude::*};
use std::path::Path;

/// The toolbar widget and the label that shows the current mode.
pub struct Toolbar {
    /// The whole bar.
    pub widget: gtk::Box,
    /// Text such as `Modo: Modo Edição`.
    pub mode: gtk::Label,
}

fn glyph_button(glyph: &str, label: &str, cmd: &Cmd, assets: &Path, dark: bool) -> gtk::Button {
    let b = gtk::Button::new();
    b.set_child(Some(&glyph_image(glyph, assets, dark)));
    b.add_css_class("flat");
    b.set_tooltip_text(Some(&ls(label)));
    b.set_action_name(Some(&format!("win.{}", cmd.action_name())));
    b
}

/// Builds the toolbar. Buttons are bound to window actions, so their enabled states follow
/// the menus.
pub fn build(registry: &Registry, assets: &Path, dark: bool) -> Toolbar {
    let widget = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    widget.set_margin_start(6);
    widget.set_margin_end(6);
    widget.set_margin_top(2);
    widget.set_margin_bottom(2);
    let edit = [
        ("reply", "Undo", Cmd::Undo),
        ("forward", "Redo", Cmd::Redo),
        ("scissors", "Cut", Cmd::Cut),
        ("copy", "Copy", Cmd::Copy),
        ("paste", "Paste", Cmd::Paste),
        ("clone", "Duplicate", Cmd::Duplicate),
        ("search", "Find Component...", Cmd::Find),
        ("zoom-11", "Zoom 100%", Cmd::Zoom100),
        ("zoom-in", "Zoom In", Cmd::ZoomIn),
        ("zoom-out", "Zoom Out", Cmd::ZoomOut),
    ];
    for (glyph, label, cmd) in edit {
        widget.append(&glyph_button(glyph, label, &cmd, assets, dark));
    }
    widget.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    for d in registry.quick_insert() {
        let b = gtk::Button::new();
        let image = match d.icon.and_then(|n| icons::component_svg(n, assets)) {
            Some(svg) => {
                let themed = icons::themed_svg(&svg, dark).unwrap_or(svg);
                gtk::Image::from_gicon(&relm4::gtk::gio::FileIcon::new(
                    &relm4::gtk::gio::File::for_path(themed),
                ))
            }
            None => gtk::Image::from_gicon(&gicon("none", assets, dark).unwrap_or_else(|| {
                relm4::gtk::gio::ThemedIcon::new("image-missing-symbolic").upcast()
            })),
        };
        image.set_pixel_size(24);
        b.set_child(Some(&image));
        b.add_css_class("flat");
        b.set_tooltip_text(Some(&ls(d.display_name)));
        b.set_action_name(Some("win.place"));
        b.set_action_target_value(Some(&d.id.to_variant()));
        widget.append(&b);
    }
    let mode = gtk::Label::new(None);
    mode.set_hexpand(true);
    mode.set_halign(gtk::Align::End);
    mode.add_css_class("dim-label");
    widget.append(&mode);
    Toolbar { widget, mode }
}
