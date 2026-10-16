// SPDX-License-Identifier: GPL-2.0-or-later
//! GTK glue for the command tables: menu models, window actions and accelerators.

use super::commands::{Cmd, Node, accelerators, all_commands};
use super::icons::{self, Icon};
use super::session::Session;
use crate::elements::registry::Registry;
use crate::locale::ls;
use relm4::gtk::{self, gio, glib, prelude::*};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// A window action together with the command it stands for.
pub struct ActionEntry {
    /// The GTK action.
    pub action: gio::SimpleAction,
    /// The command of a plain action; parameterised actions keep their own copy per entry.
    pub cmd: Cmd,
}

/// Icon for a glyph name: the symbolic theme icon, or the imported SVG recoloured for the theme.
pub fn gicon(glyph: &str, assets: &Path, dark: bool) -> Option<gio::Icon> {
    match icons::resolve(glyph, assets)? {
        Icon::Symbolic(name) => Some(gio::ThemedIcon::new(name).upcast()),
        Icon::File(path) => {
            let themed = icons::themed_svg(&path, dark).unwrap_or(path);
            Some(gio::FileIcon::new(&gio::File::for_path(themed)).upcast())
        }
    }
}

/// An image widget for a glyph, or an empty one when the glyph is unknown.
pub fn glyph_image(glyph: &str, assets: &Path, dark: bool) -> gtk::Image {
    match gicon(glyph, assets, dark) {
        Some(icon) => gtk::Image::from_gicon(&icon),
        None => gtk::Image::new(),
    }
}

fn detailed(cmd: &Cmd) -> String {
    let name = format!("win.{}", cmd.action_name());
    match cmd.target() {
        Some(t) => gio::Action::print_detailed_name(&name, Some(&t.to_variant())).to_string(),
        None => name,
    }
}

/// Builds the menu model for a list of nodes. Submenu titles that repeat get invisible
/// zero-width spaces appended, because GTK names each submenu page after its label.
pub fn build_menu(
    nodes: &[Node],
    assets: &Path,
    dark: bool,
    seen: &mut HashMap<String, usize>,
) -> gio::Menu {
    let menu = gio::Menu::new();
    for node in nodes {
        match node {
            Node::Item(cmd, label, glyph, _) => {
                let entry = gio::MenuItem::new(Some(&ls(label)), None);
                match cmd.target() {
                    Some(t) => entry.set_action_and_target_value(
                        Some(&format!("win.{}", cmd.action_name())),
                        Some(&t.to_variant()),
                    ),
                    None => entry.set_detailed_action(&format!("win.{}", cmd.action_name())),
                }
                if let Some(icon) = glyph.and_then(|g| gicon(g, assets, dark)) {
                    entry.set_icon(&icon);
                }
                menu.append_item(&entry);
            }
            Node::Check(toggle, label) => {
                let cmd = Cmd::Toggle(*toggle);
                menu.append(Some(&ls(label)), Some(&detailed(&cmd)));
            }
            Node::Section(inner) => {
                menu.append_section(None, &build_menu(inner, assets, dark, seen));
            }
            Node::Sub(title, inner) => {
                let n = seen.entry(title.clone()).or_insert(0);
                let unique = format!("{}{}", ls(title), "\u{200b}".repeat(*n));
                *n += 1;
                menu.append_submenu(Some(&unique), &build_menu(inner, assets, dark, seen));
            }
        }
    }
    menu
}

/// The model of the whole menu bar.
pub fn menu_bar_model(
    registry: &Registry,
    examples: &super::examples::ExampleList,
    assets: &Path,
    dark: bool,
) -> gio::Menu {
    use super::commands::{
        draw_menu, edit_menu, examples_menu, file_menu, options_menu, scopes_menu, tools_menu,
    };
    let bar = gio::Menu::new();
    let mut seen = HashMap::new();
    let tops: [(&str, Vec<Node>); 7] = [
        ("File", file_menu()),
        ("Edit", edit_menu()),
        ("Draw", draw_menu(registry)),
        ("Scopes", scopes_menu()),
        ("Options", options_menu()),
        ("Tools", tools_menu()),
        ("Circuits", examples_menu(examples)),
    ];
    for (title, nodes) in tops {
        let menu = build_menu(&nodes, assets, dark, &mut seen);
        bar.append_submenu(Some(ls(title).trim()), &menu);
    }
    bar
}

/// Creates one window action per command and routes activations to `on_command`.
pub fn install_actions(
    window: &impl IsA<gio::ActionMap>,
    registry: &Registry,
    on_command: Rc<dyn Fn(Cmd)>,
) -> Vec<ActionEntry> {
    let mut entries = Vec::new();
    let mut made: Vec<String> = Vec::new();
    let mut cmds = all_commands(registry);
    // the examples menu is built from the catalogue, so its action is not in the tables
    cmds.extend([Cmd::Fullscreen, Cmd::Undo, Cmd::Example(String::new())]);
    for cmd in cmds {
        let name = cmd.action_name();
        if made.contains(&name) {
            continue;
        }
        made.push(name.clone());
        let action = match &cmd {
            Cmd::Place(_) | Cmd::Example(_) => {
                gio::SimpleAction::new(&name, Some(glib::VariantTy::STRING))
            }
            Cmd::Toggle(_) | Cmd::SetTool(_) => {
                gio::SimpleAction::new_stateful(&name, None, &false.to_variant())
            }
            _ => gio::SimpleAction::new(&name, None),
        };
        let handler = on_command.clone();
        let action_name = name.clone();
        action.connect_activate(move |_, param| {
            let target = param.and_then(|v| v.get::<String>());
            if let Some(cmd) = Cmd::from_action(&action_name, target.as_deref()) {
                handler(cmd);
            }
        });
        window.add_action(&action);
        entries.push(ActionEntry { action, cmd });
    }
    entries
}

/// Registers every keyboard accelerator of the menus.
pub fn install_accels(app: &impl IsA<gtk::Application>, registry: &Registry) {
    for (cmd, accel, _) in accelerators(registry) {
        // a shortcut GTK cannot parse is left out instead of raising a critical warning
        if gtk::accelerator_parse(&accel).is_none() {
            tracing::warn!(accel, "ignoring an unparsable accelerator");
            continue;
        }
        app.set_accels_for_action(&detailed(&cmd), &[accel.as_str()]);
    }
}

/// Brings enabled and checked states of the actions in line with the session.
pub fn sync_actions(entries: &[ActionEntry], s: &Session) {
    for e in entries {
        match &e.cmd {
            Cmd::Toggle(t) => {
                let on = s.toggle_state(*t);
                e.action.set_state(&on.to_variant());
                e.action.set_enabled(true);
            }
            Cmd::SetTool(t) => {
                e.action.set_state(&(s.tool == *t).to_variant());
                e.action.set_enabled(s.editing_allowed());
            }
            Cmd::Place(_) => e.action.set_enabled(s.editing_allowed()),
            Cmd::Example(_) => e.action.set_enabled(true),
            cmd => e.action.set_enabled(cmd.enabled(s)),
        }
    }
}
