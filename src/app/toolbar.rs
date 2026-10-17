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

/// The parts of the toolbar, in the order of the original. A group with several parts is one
/// button that places the part chosen last and offers the others in a pop-up. Each entry is
/// the element id and the icon.
const PARTS: &[&[(&str, &str)]] = &[
    &[("WireElm", "wire")],
    &[("ResistorElm", "resistor")],
    &[("GroundElm", "ground")],
    &[("CapacitorElm", "capacitor")],
    &[("InductorElm", "induct")],
    &[("DiodeElm", "diode")],
    &[("DCVoltageElm", "voltage2"), ("ACVoltageElm", "acSrc")],
    &[("RailElm", "rail")],
    &[
        ("SwitchElm", "switch"),
        ("Switch2Elm", "spdt"),
        ("AnalogSwitchElm", "aswitch1"),
        ("AnalogSwitch2Elm", "aswitch2"),
    ],
    &[("OpAmpElm", "opAmpBot"), ("OpAmpSwapElm", "opAmpTop")],
    &[("NTransistorElm", "transistor"), ("PTransistorElm", "pnpTransistor")],
    &[("NMosfetElm", "fet"), ("PMosfetElm", "fet2")],
    &[("InverterElm", "inverter")],
    &[
        ("AndGateElm", "and"),
        ("NandGateElm", "nand"),
        ("OrGateElm", "or"),
        ("NorGateElm", "nor"),
        ("XorGateElm", "xor"),
        ("XnorGateElm", "xnor"),
    ],
];

/// The image of a component icon, or the empty icon when the file is missing.
fn part_image(icon: &str, assets: &Path, dark: bool) -> gtk::Image {
    let image = match icons::component_svg(icon, assets) {
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
    image
}

fn part_name(registry: &Registry, id: &str) -> String {
    registry.get(id).map_or_else(|| id.to_string(), |d| ls(d.display_name))
}

/// A button that places one part.
fn part_button(
    registry: &Registry,
    id: &str,
    icon: &str,
    assets: &Path,
    dark: bool,
) -> gtk::Button {
    let b = gtk::Button::new();
    b.set_child(Some(&part_image(icon, assets, dark)));
    b.add_css_class("flat");
    b.set_tooltip_text(Some(&part_name(registry, id)));
    b.set_action_name(Some("win.place"));
    b.set_action_target_value(Some(&id.to_variant()));
    b
}

/// A button for a group of parts: it places the part chosen last, and an arrow opens a list of
/// the others.
fn part_set(registry: &Registry, parts: &[(&str, &str)], assets: &Path, dark: bool) -> gtk::Box {
    let holder = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    holder.add_css_class("linked");
    let (first_id, first_icon) = parts[0];
    let main = part_button(registry, first_id, first_icon, assets, dark);
    let arrow = gtk::MenuButton::new();
    arrow.add_css_class("flat");
    arrow.set_tooltip_text(Some(&ls("More parts of this kind")));
    let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let popover = gtk::Popover::new();
    for &(id, icon) in parts {
        let row = gtk::Button::new();
        row.add_css_class("flat");
        let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        line.append(&part_image(icon, assets, dark));
        let label = gtk::Label::new(Some(&part_name(registry, id)));
        label.set_halign(gtk::Align::Start);
        line.append(&label);
        row.set_child(Some(&line));
        let (main, popover, name, icon_name) =
            (main.clone(), popover.clone(), part_name(registry, id), icon.to_string());
        let (assets, id) = (assets.to_path_buf(), id.to_string());
        row.connect_clicked(move |_| {
            main.set_child(Some(&part_image(&icon_name, &assets, dark)));
            main.set_tooltip_text(Some(&name));
            main.set_action_target_value(Some(&id.to_variant()));
            popover.popdown();
            main.emit_clicked();
        });
        list.append(&row);
    }
    popover.set_child(Some(&list));
    arrow.set_popover(Some(&popover));
    holder.append(&main);
    holder.append(&arrow);
    holder
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
    for group in PARTS {
        let available: Vec<(&str, &str)> =
            group.iter().copied().filter(|(id, _)| registry.contains(id)).collect();
        match available[..] {
            [] => {}
            [(id, icon)] => widget.append(&part_button(registry, id, icon, assets, dark)),
            _ => widget.append(&part_set(registry, &available, assets, dark)),
        }
    }
    let mode = gtk::Label::new(None);
    mode.set_hexpand(true);
    mode.set_halign(gtk::Align::End);
    mode.add_css_class("dim-label");
    widget.append(&mode);
    Toolbar { widget, mode }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_toolbar_part_exists_and_has_an_icon() {
        let registry = Registry::standard();
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for group in PARTS {
            for (id, icon) in *group {
                assert!(registry.contains(id), "no element {id}");
                assert!(icons::component_svg(icon, &assets).is_some(), "no icon {icon} for {id}");
            }
        }
    }
}
