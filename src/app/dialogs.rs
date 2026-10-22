// SPDX-License-Identifier: GPL-2.0-or-later
//! Dialogs and file choosers of the main window.

use super::commands::accelerators;
use super::edit::SliderEntry;
use super::icons;
use super::pin_layout::{Geometry, can_move};
use super::session::Session;
use super::settings::DEFAULT_COLORS;
use super::subcircuits::ModelScope;
use crate::elements::element::EditInfo;
use crate::elements::registry::Registry;
use crate::elements::subcircuit::CompositeModel;
use crate::elements::units::{edit_value_text, parse_edit_value};
use crate::locale::ls;
use crate::render::painter::Color;
use crate::solver::sim::SolverType;
use relm4::adw::{self, prelude::*};
use relm4::gtk::{self, gio, glib};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

fn dialog(title: &str, width: i32, height: i32, body: &impl IsA<gtk::Widget>) -> adw::Dialog {
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(body));
    let d = adw::Dialog::new();
    d.set_title(title);
    d.set_content_width(width);
    d.set_content_height(height);
    d.set_child(Some(&toolbar));
    d
}

fn padded(child: &impl IsA<gtk::Widget>) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 12);
    b.set_margin_top(12);
    b.set_margin_bottom(12);
    b.set_margin_start(12);
    b.set_margin_end(12);
    b.append(child);
    b
}

/// The icon name for the About dialog: the program's emblem when the icon theme can find it
/// under `assets`, the generic glyph otherwise. The assets folder joins the theme's search
/// path once per run.
fn about_icon(display: &gtk::gdk::Display, assets: &std::path::Path) -> &'static str {
    use std::sync::Once;
    static ADD_PATH: Once = Once::new();
    let theme = gtk::IconTheme::for_display(display);
    ADD_PATH.call_once(|| theme.add_search_path(assets.join("icons")));
    icons::app_icon_choice(
        icons::app_icon_svg(assets).is_file(),
        theme.has_icon(icons::APP_ICON_NAME),
    )
}

/// The About dialog.
pub fn about(parent: &impl IsA<gtk::Widget>) {
    let d = adw::AboutDialog::new();
    d.set_application_name(super::consts::APP_NAME);
    d.set_application_icon(about_icon(&parent.display(), &super::consts::assets_dir()));
    d.set_version(env!("CARGO_PKG_VERSION"));
    d.set_comments(
        "Simulador de circuitos: um porte em Rust do CircuitJS1 com interface GTK4 e libadwaita.",
    );
    d.set_license_type(gtk::License::Gpl20);
    d.set_copyright(&format!(
        "© Paul Falstad e Iain Sharp (CircuitJS1); porte {}",
        super::consts::PROGRAM_NAME
    ));
    d.set_website("https://github.com/pfalstad/circuitjs1");
    d.present(Some(parent));
}

/// The list of keyboard shortcuts (contract C4).
pub fn shortcuts(parent: &impl IsA<gtk::Widget>, registry: &Registry) {
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    for (_, accel, label) in accelerators(registry) {
        let row = adw::ActionRow::new();
        row.set_title(&ls(&label));
        let shown = match gtk::accelerator_parse(&accel) {
            Some((k, m)) => gtk::accelerator_get_label(k, m),
            None => accel.clone().into(),
        };
        let key = gtk::Label::new(Some(shown.as_str()));
        key.add_css_class("dim-label");
        row.add_suffix(&key);
        list.append(&row);
    }
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&padded(&list)));
    scroll.set_vexpand(true);
    dialog(&ls("Keyboard Shortcuts"), 420, 560, &scroll).present(Some(parent));
}

/// One line of the list of loading problems, in the interface language.
fn load_problem_text(e: &crate::error::LoadError) -> (String, String) {
    use crate::error::LoadError;
    match e {
        LoadError::UnknownType { line, type_code } => (
            ls("Line {line}: unknown element").replace("{line}", &line.to_string()),
            ls("The type `{type}` does not exist in this version; the line was ignored.")
                .replace("{type}", type_code),
        ),
        LoadError::Malformed { line, type_code, message } => (
            ls("Line {line}: malformed `{type}` entry")
                .replace("{line}", &line.to_string())
                .replace("{type}", type_code),
            ls("{message}; the line was ignored.").replace("{message}", message),
        ),
        LoadError::Xml(message) => (ls("The XML file is invalid"), message.clone()),
    }
}

/// Lists what could not be read from a circuit file (the rest of the circuit was loaded).
pub fn load_problems(parent: &impl IsA<gtk::Widget>, errors: &[crate::error::LoadError]) {
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    for e in errors {
        let (title, detail) = load_problem_text(e);
        let row = adw::ActionRow::new();
        row.set_title(&glib::markup_escape_text(&title));
        row.set_subtitle(&glib::markup_escape_text(&detail));
        row.set_subtitle_selectable(true);
        list.append(&row);
    }
    let intro = gtk::Label::new(Some(
        &ls(if errors.len() == 1 {
            "{count} entry could not be read. The rest of the circuit was loaded."
        } else {
            "{count} entries could not be read. The rest of the circuit was loaded."
        })
        .replace("{count}", &errors.len().to_string()),
    ));
    intro.set_wrap(true);
    intro.set_xalign(0.0);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&intro);
    body.append(&list);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&padded(&body)));
    scroll.set_vexpand(true);
    dialog(&ls("Problems Loading the Circuit"), 520, 420, &scroll).present(Some(parent));
}

/// Shows text that can be selected and copied.
pub fn text_view(parent: &impl IsA<gtk::Widget>, title: &str, text: &str) {
    let view = gtk::TextView::new();
    view.set_editable(false);
    view.set_monospace(true);
    view.buffer().set_text(text);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&view));
    scroll.set_vexpand(true);
    let copy = gtk::Button::with_label(&ls("Copy"));
    let t = text.to_string();
    let p = parent.clone().upcast::<gtk::Widget>();
    copy.connect_clicked(move |_| p.clipboard().set_text(&t));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.set_margin_top(6);
    body.set_margin_bottom(6);
    body.set_margin_start(6);
    body.set_margin_end(6);
    body.append(&scroll);
    body.append(&copy);
    dialog(title, 640, 480, &body).present(Some(parent));
}

/// Shows a share link with a Copy button.
pub fn link(parent: &impl IsA<gtk::Widget>, url: &str) {
    let entry = gtk::Entry::new();
    entry.set_text(url);
    entry.set_editable(false);
    entry.set_hexpand(true);
    let copy = gtk::Button::with_label(&ls("Copy Link"));
    let (u, p) = (url.to_string(), parent.clone().upcast::<gtk::Widget>());
    copy.connect_clicked(move |_| p.clipboard().set_text(&u));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&entry);
    body.append(&copy);
    dialog(&ls("Export As Link"), 520, 160, &padded(&body)).present(Some(parent));
}

/// Import from text: paste circuit text or choose a file.
pub fn import_text(
    parent: &impl IsA<gtk::Widget>,
    on_text: impl Fn(String) + 'static,
    on_file: impl Fn() + 'static,
) {
    let view = gtk::TextView::new();
    view.set_monospace(true);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&view));
    scroll.set_vexpand(true);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let file = gtk::Button::with_label(&ls("Open File..."));
    let go = gtk::Button::with_label(&ls("Import"));
    go.add_css_class("suggested-action");
    buttons.append(&file);
    buttons.append(&go);
    buttons.set_halign(gtk::Align::End);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.set_margin_top(6);
    body.set_margin_bottom(6);
    body.set_margin_start(6);
    body.set_margin_end(6);
    body.append(&scroll);
    body.append(&buttons);
    let d = dialog(&ls("Import From Text"), 640, 480, &body);
    let (d1, d2) = (d.clone(), d.clone());
    go.connect_clicked(move |_| {
        let b = view.buffer();
        let text = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
        d1.close();
        on_text(text);
    });
    file.connect_clicked(move |_| {
        d2.close();
        on_file();
    });
    d.present(Some(parent));
}

/// Find Component: a filterable list of the circuit's elements.
pub fn find(
    parent: &impl IsA<gtk::Widget>,
    labels: Vec<(usize, String)>,
    on_pick: impl Fn(usize) + 'static,
) {
    let entry = gtk::SearchEntry::new();
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    let indices: Vec<usize> = labels.iter().map(|(i, _)| *i).collect();
    let texts: Rc<Vec<String>> = Rc::new(labels.iter().map(|(_, t)| t.to_lowercase()).collect());
    for (_, text) in &labels {
        let row = adw::ActionRow::new();
        row.set_title(text);
        row.set_activatable(true);
        list.append(&row);
    }
    let query = Rc::new(RefCell::new(String::new()));
    {
        let (q, t) = (query.clone(), texts.clone());
        list.set_filter_func(move |row| {
            let i = row.index() as usize;
            t.get(i).is_some_and(|s| s.contains(q.borrow().as_str()))
        });
        let l = list.clone();
        entry.connect_search_changed(move |e| {
            *query.borrow_mut() = e.text().to_lowercase();
            l.invalidate_filter();
        });
    }
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&list));
    scroll.set_vexpand(true);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.set_margin_top(6);
    body.set_margin_bottom(6);
    body.set_margin_start(6);
    body.set_margin_end(6);
    body.append(&entry);
    body.append(&scroll);
    let d = dialog(&ls("Find Component"), 420, 480, &body);
    let d2 = d.clone();
    list.connect_row_activated(move |_, row| {
        if let Some(&i) = indices.get(row.index() as usize) {
            d2.close();
            on_pick(i);
        }
    });
    d.present(Some(parent));
}

enum Field {
    Entry(adw::EntryRow),
    Area(gtk::TextView),
    Switch(adw::SwitchRow),
    Combo(adw::ComboRow),
    Color(gtk::ColorDialogButton),
    Fixed,
}

/// The chooser's form of a colour.
fn to_rgba(c: Color) -> gtk::gdk::RGBA {
    let [r, g, b] = c.to_unit();
    gtk::gdk::RGBA::new(r as f32, g as f32, b as f32, 1.0)
}

/// The colour a chooser shows (alpha is not offered).
fn from_rgba(c: &gtk::gdk::RGBA) -> Color {
    Color::from_unit([f64::from(c.red()), f64::from(c.green()), f64::from(c.blue())])
}

/// The element edit dialog, generated from the element's edit fields.
pub fn edit_element(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    fields: Vec<EditInfo>,
    on_apply: impl Fn(Vec<EditInfo>) + 'static,
) {
    edit_element_with(parent, title, fields, Vec::new(), None, on_apply);
}

/// What a button of an edit dialog does once the dialog has closed.
pub type DialogButton = (String, Box<dyn Fn()>);

/// [`edit_element`] with extra buttons under the fields (each closes the dialog and then runs
/// its action) and an optional error message shown when it opens.
pub fn edit_element_with(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    fields: Vec<EditInfo>,
    buttons: Vec<DialogButton>,
    error: Option<&str>,
    on_apply: impl Fn(Vec<EditInfo>) + 'static,
) {
    let group = adw::PreferencesGroup::new();
    let mut widgets: Vec<Field> = Vec::new();
    for ei in &fields {
        let name = ls(&ei.name);
        if let Some(options) = &ei.choice {
            let row = adw::ComboRow::new();
            row.set_title(&name);
            let items: Vec<String> = options.iter().map(|o| ls(o)).collect();
            let refs: Vec<&str> = items.iter().map(String::as_str).collect();
            row.set_model(Some(&gtk::StringList::new(&refs)));
            row.set_selected(ei.choice_index as u32);
            group.add(&row);
            widgets.push(Field::Combo(row));
        } else if let Some(state) = ei.checkbox {
            let row = adw::SwitchRow::new();
            // the label of a checkbox field is kept in its text
            let label = if name.is_empty() { ls(ei.text.as_deref().unwrap_or("")) } else { name };
            row.set_title(&label);
            row.set_active(state);
            group.add(&row);
            widgets.push(Field::Switch(row));
        } else if ei.button.is_some() || ei.hidden {
            widgets.push(Field::Fixed);
        } else if ei.is_color {
            let dialog = gtk::ColorDialog::new();
            dialog.set_with_alpha(false);
            dialog.set_title(&name);
            let button = gtk::ColorDialogButton::new(Some(dialog));
            button.set_valign(gtk::Align::Center);
            let current =
                ei.text.as_deref().and_then(Color::parse_hex).unwrap_or(Color::LIGHT_GRAY);
            button.set_rgba(&to_rgba(current));
            let row = adw::ActionRow::new();
            row.set_title(&name);
            row.add_suffix(&button);
            group.add(&row);
            widgets.push(Field::Color(button));
        } else if ei.multiline {
            let view = gtk::TextView::new();
            view.set_monospace(true);
            view.buffer().set_text(ei.text.as_deref().unwrap_or(""));
            let scroll = gtk::ScrolledWindow::new();
            scroll.set_min_content_height(160);
            scroll.set_child(Some(&view));
            let label = gtk::Label::new(Some(&name));
            label.set_xalign(0.0);
            let col = gtk::Box::new(gtk::Orientation::Vertical, 6);
            col.append(&label);
            col.append(&scroll);
            let row = adw::PreferencesRow::new();
            row.set_child(Some(&padded(&col)));
            group.add(&row);
            widgets.push(Field::Area(view));
        } else {
            let row = adw::EntryRow::new();
            row.set_title(&name);
            match &ei.text {
                Some(t) => row.set_text(t),
                None => row.set_text(&edit_value_text(ei.value, ei.dimensionless)),
            }
            if let Some(choice) = ei.file_chooser.clone() {
                let button = gtk::Button::from_icon_name("document-open-symbolic");
                button.set_valign(gtk::Align::Center);
                button.add_css_class("flat");
                button.set_tooltip_text(Some(&ls("Choose File...")));
                let (entry, anchor) = (row.clone(), parent.clone().upcast::<gtk::Widget>());
                button.connect_clicked(move |_| {
                    let Some(window) = anchor.root().and_downcast::<gtk::Window>() else { return };
                    let entry = entry.clone();
                    choose_path(&window, &choice, entry.text().as_str(), move |path| {
                        entry.set_text(&path.to_string_lossy());
                    });
                });
                row.add_suffix(&button);
            }
            group.add(&row);
            widgets.push(Field::Entry(row));
        }
    }
    let apply = gtk::Button::with_label(&ls("Apply"));
    apply.add_css_class("suggested-action");
    apply.set_halign(gtk::Align::End);
    let message = gtk::Label::new(error);
    message.add_css_class("error");
    message.set_xalign(0.0);
    message.set_wrap(true);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&group);
    let extra: Vec<(gtk::Button, Box<dyn Fn()>)> = buttons
        .into_iter()
        .map(|(label, action)| {
            let b = gtk::Button::with_label(&ls(&label));
            body.append(&b);
            (b, action)
        })
        .collect();
    body.append(&message);
    body.append(&apply);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&padded(&body)));
    scroll.set_propagate_natural_height(true);
    let d = dialog(title, 420, 480, &scroll);
    for (button, action) in extra {
        let d3 = d.clone();
        button.connect_clicked(move |_| {
            d3.close();
            action();
        });
    }
    let d2 = d.clone();
    apply.connect_clicked(move |_| {
        let mut out = fields.clone();
        for (ei, w) in out.iter_mut().zip(&widgets) {
            match w {
                Field::Entry(row) => {
                    let text = row.text().to_string();
                    if ei.text.is_some() {
                        ei.text = Some(text);
                    } else {
                        match parse_edit_value(&text) {
                            Some(v) => ei.value = v,
                            None => {
                                message.set_text(
                                    &ls("{name}: invalid value").replace("{name}", &ls(&ei.name)),
                                );
                                return;
                            }
                        }
                        if ei.positive && ei.value <= 0.0 {
                            message.set_text(&format!("{}: deve ser > 0", ls(&ei.name)));
                            return;
                        }
                        if ei.non_negative && ei.value < 0.0 {
                            message.set_text(&format!("{}: deve ser >= 0", ls(&ei.name)));
                            return;
                        }
                    }
                }
                Field::Area(view) => {
                    let b = view.buffer();
                    ei.text = Some(b.text(&b.start_iter(), &b.end_iter(), false).to_string());
                }
                Field::Switch(row) => ei.checkbox = Some(row.is_active()),
                Field::Combo(row) => ei.choice_index = row.selected() as usize,
                Field::Color(button) => ei.text = Some(from_rgba(&button.rgba()).hex()),
                Field::Fixed => {}
            }
        }
        d2.close();
        on_apply(out);
    });
    d.present(Some(parent));
}

/// The Add Sliders dialog of an element: for each field that can have a slider, a switch and,
/// when it is on, the range, step, scale type and label of the slider.
pub fn element_sliders(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    entries: Vec<SliderEntry>,
    error: Option<&str>,
    on_apply: impl Fn(Vec<SliderEntry>) + 'static,
) {
    struct Row {
        expander: adw::ExpanderRow,
        min: adw::EntryRow,
        max: adw::EntryRow,
        step: adw::EntryRow,
        label: adw::EntryRow,
        log: adw::SwitchRow,
    }
    let group = adw::PreferencesGroup::new();
    let mut rows = Vec::new();
    for e in &entries {
        let expander = adw::ExpanderRow::new();
        expander.set_title(&e.name);
        expander.set_show_enable_switch(true);
        expander.set_enable_expansion(e.enabled);
        expander.set_expanded(e.enabled);
        let entry = |title: &str, v: f64| {
            let r = adw::EntryRow::new();
            r.set_title(&ls(title));
            r.set_text(&edit_value_text(v, false));
            expander.add_row(&r);
            r
        };
        let min = entry("Min Value", e.min);
        let max = entry("Max Value", e.max);
        let step = entry("Step (0=continuous)", e.step);
        let label = adw::EntryRow::new();
        label.set_title(&ls("Label"));
        label.set_text(&e.label);
        expander.add_row(&label);
        let log = adw::SwitchRow::new();
        log.set_title(&ls("Logarithmic"));
        log.set_active(e.logarithmic);
        expander.add_row(&log);
        group.add(&expander);
        rows.push(Row { expander, min, max, step, label, log });
    }
    let apply = gtk::Button::with_label(&ls("Apply"));
    apply.add_css_class("suggested-action");
    apply.set_halign(gtk::Align::End);
    let message = gtk::Label::new(error);
    message.add_css_class("error");
    message.set_xalign(0.0);
    message.set_wrap(true);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&group);
    body.append(&message);
    body.append(&apply);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&padded(&body)));
    scroll.set_propagate_natural_height(true);
    let d = dialog(title, 420, 480, &scroll);
    let d2 = d.clone();
    apply.connect_clicked(move |_| {
        let mut out = entries.clone();
        for (e, r) in out.iter_mut().zip(&rows) {
            e.enabled = r.expander.enables_expansion();
            if !e.enabled {
                continue;
            }
            let read = |row: &adw::EntryRow| parse_edit_value(row.text().as_str());
            match (read(&r.min), read(&r.max), read(&r.step)) {
                (Some(min), Some(max), Some(step)) => {
                    e.min = min;
                    e.max = max;
                    e.step = step;
                }
                _ => {
                    message.set_text(&ls("{name}: invalid value").replace("{name}", &ls(&e.name)));
                    return;
                }
            }
            e.label = r.label.text().to_string();
            e.logarithmic = r.log.is_active();
        }
        d2.close();
        on_apply(out);
    });
    d.present(Some(parent));
}

/// Size of the pin layout preview.
const PREVIEW_SIZE: (i32, i32) = (400, 240);
/// Space around the chip in the preview, for the pins and the name.
const PREVIEW_MARGIN: f64 = 44.0;

/// Where the body of the preview chip starts, from the corner of the drawing area.
fn preview_origin(geo: &Geometry, width: f64, height: f64) -> (f64, f64) {
    let (w, h) = geo.body();
    ((width - w) / 2.0, (height - h) / 2.0)
}

/// Draws the chip of the pin layout dialog: the body, the pins with their names, and the name
/// of the model above it when the chip shows it.
fn draw_pin_preview(
    cr: &gtk::cairo::Context,
    (width, height): (f64, f64),
    geo: &Geometry,
    pins: &[(usize, i32)],
    names: &[String],
    label: Option<&str>,
) {
    use crate::elements::chip::{SIDE_E, SIDE_N, SIDE_S, SIDE_W};
    cr.set_source_rgb(0.0, 0.0, 0.0);
    let _ = cr.paint();
    let (ox, oy) = preview_origin(geo, width, height);
    let (w, h) = geo.body();
    cr.set_line_width(2.0);
    cr.set_source_rgb(0.75, 0.75, 0.75);
    cr.rectangle(ox, oy, w, h);
    let _ = cr.stroke();
    cr.set_font_size(12.0);
    for (&(side, pos), name) in pins.iter().zip(names) {
        let (ax, ay) = geo.anchor(side, pos);
        let (ax, ay) = (ox + ax, oy + ay);
        let (sx, sy) = match side {
            SIDE_N => (0.0, -12.0),
            SIDE_S => (0.0, 12.0),
            SIDE_W => (-12.0, 0.0),
            _ => (12.0, 0.0),
        };
        cr.set_source_rgb(0.5, 0.5, 0.5);
        cr.move_to(ax, ay);
        cr.line_to(ax + sx, ay + sy);
        let _ = cr.stroke();
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.arc(ax + sx, ay + sy, 3.5, 0.0, std::f64::consts::TAU);
        let _ = cr.fill();
        let text_width = cr.text_extents(name).map_or(0.0, |e| e.width());
        let (tx, ty) = match side {
            SIDE_N => (ax - text_width / 2.0, ay + 16.0),
            SIDE_S => (ax - text_width / 2.0, ay - 6.0),
            SIDE_W => (ax + 5.0, ay + 4.0),
            SIDE_E => (ax - 5.0 - text_width, ay + 4.0),
            _ => (ax, ay),
        };
        cr.move_to(tx, ty);
        let _ = cr.show_text(name);
    }
    if let Some(text) = label {
        let text_width = cr.text_extents(text).map_or(0.0, |e| e.width());
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.move_to(ox + (w - text_width) / 2.0, oy - 26.0);
        let _ = cr.show_text(text);
    }
}

/// The pin layout dialog of a subcircuit: its name, whether the chip shows it, the size of the
/// chip, where it is kept, and the side and place of every pin. `on_apply` gets the changed
/// model and where to keep it.
pub fn subcircuit_pins(
    parent: &impl IsA<gtk::Widget>,
    model: &CompositeModel,
    error: Option<&str>,
    on_apply: impl Fn(CompositeModel, ModelScope) + 'static,
) {
    let group = adw::PreferencesGroup::new();
    let name = adw::EntryRow::new();
    name.set_title(&ls("Model Name"));
    name.set_text(&model.name);
    group.add(&name);
    let label = adw::SwitchRow::new();
    label.set_title(&ls("Show Label"));
    label.set_active(model.show_label());
    group.add(&label);
    let spin = |title: &str, v: i32| {
        let r = adw::SpinRow::with_range(1.0, 40.0, 1.0);
        r.set_title(&ls(title));
        r.set_value(f64::from(v));
        group.add(&r);
        r
    };
    let width = spin("Width", model.size_x);
    let height = spin("Height", model.size_y);
    let scope = adw::ComboRow::new();
    scope.set_title(&ls("Scope:"));
    let items = [ls("This Circuit"), ls("This Session"), ls("Save Across Sessions")];
    let refs: Vec<&str> = items.iter().map(String::as_str).collect();
    scope.set_model(Some(&gtk::StringList::new(&refs)));
    group.add(&scope);

    // one row per pin (a bus is one row for all its bits)
    let pins = adw::PreferencesGroup::new();
    pins.set_title(&ls("Edit Subcircuit Pin Layout"));
    let sides = [ls("North"), ls("South"), ls("West"), ls("East")];
    let side_refs: Vec<&str> = sides.iter().map(String::as_str).collect();
    let mut rows = Vec::new();
    for (i, p) in model.ext.iter().enumerate().filter(|(_, p)| p.bus_z == 0) {
        let row = adw::ActionRow::new();
        row.set_title(&p.name);
        let side =
            gtk::DropDown::new(Some(gtk::StringList::new(&side_refs)), gtk::Expression::NONE);
        side.set_selected(p.side as u32);
        side.set_valign(gtk::Align::Center);
        let pos = gtk::SpinButton::with_range(0.0, 39.0, 1.0);
        pos.set_value(f64::from(p.pos));
        pos.set_valign(gtk::Align::Center);
        row.add_suffix(&side);
        row.add_suffix(&pos);
        pins.add(&row);
        rows.push((i, side, pos));
    }
    // the preview of the chip: the rows above are what it shows and what dragging a pin sets
    let names: Vec<String> = rows.iter().map(|(i, ..)| model.ext[*i].name.clone()).collect();
    let preview = gtk::DrawingArea::new();
    preview.set_content_width(PREVIEW_SIZE.0);
    preview.set_content_height(PREVIEW_SIZE.1);
    preview.add_css_class("card");
    let state = {
        let (width, height, rows) = (width.clone(), height.clone(), rows.clone());
        Rc::new(move || {
            let geo = (width.value() as i32, height.value() as i32);
            let pins: Vec<(usize, i32)> = rows
                .iter()
                .map(|(_, side, pos)| (side.selected() as usize, pos.value() as i32))
                .collect();
            (geo, pins)
        })
    };
    let geometry = {
        let state = state.clone();
        Rc::new(move |w: f64, h: f64| {
            let ((sx, sy), _) = state();
            Geometry::fitting(sx, sy, w, h, PREVIEW_MARGIN)
        })
    };
    {
        let (state, geometry, names, label, name) =
            (state.clone(), geometry.clone(), names.clone(), label.clone(), name.clone());
        preview.set_draw_func(move |_, cr, w, h| {
            let (w, h) = (f64::from(w), f64::from(h));
            let (_, pins) = state();
            let title = name.text().to_string();
            let shown = (label.is_active() && !title.is_empty()).then_some(title.as_str());
            draw_pin_preview(cr, (w, h), &geometry(w, h), &pins, &names, shown);
        });
    }
    let redraw = {
        let preview = preview.clone();
        move || preview.queue_draw()
    };
    {
        let r = redraw.clone();
        width.connect_value_notify(move |_| r());
        let r = redraw.clone();
        height.connect_value_notify(move |_| r());
        let r = redraw.clone();
        label.connect_active_notify(move |_| r());
        let r = redraw.clone();
        name.connect_changed(move |_| r());
        for (_, side, pos) in &rows {
            let r = redraw.clone();
            side.connect_selected_notify(move |_| r());
            let r = redraw.clone();
            pos.connect_value_changed(move |_| r());
        }
    }
    // dragging a pin puts it on the nearest free place of the chip
    let grabbed: Rc<std::cell::Cell<Option<usize>>> = Rc::new(std::cell::Cell::new(None));
    let drag = gtk::GestureDrag::new();
    {
        let (state, geometry, grabbed, preview) =
            (state.clone(), geometry.clone(), grabbed.clone(), preview.clone());
        drag.connect_drag_begin(move |_, x, y| {
            let (w, h) = (f64::from(preview.width()), f64::from(preview.height()));
            let geo = geometry(w, h);
            let (ox, oy) = preview_origin(&geo, w, h);
            let (_, pins) = state();
            grabbed.set(geo.pin_at(&pins, x - ox, y - oy, 18.0));
        });
    }
    {
        let (state, geometry, grabbed, preview, rows, drag_ref) = (
            state.clone(),
            geometry.clone(),
            grabbed.clone(),
            preview.clone(),
            rows.clone(),
            drag.clone(),
        );
        drag.connect_drag_update(move |_, dx, dy| {
            let Some(i) = grabbed.get() else { return };
            let Some((sx, sy)) = drag_ref.start_point() else { return };
            let (w, h) = (f64::from(preview.width()), f64::from(preview.height()));
            let geo = geometry(w, h);
            let (ox, oy) = preview_origin(&geo, w, h);
            let Some((side, pos)) = geo.nearest_place(sx + dx - ox, sy + dy - oy) else { return };
            let (_, pins) = state();
            if pins[i] != (side, pos) && can_move(&geo, &pins, i, side, pos) {
                rows[i].1.set_selected(side as u32);
                rows[i].2.set_value(f64::from(pos));
            }
        });
    }
    {
        let grabbed = grabbed.clone();
        drag.connect_drag_end(move |_, _, _| grabbed.set(None));
    }
    preview.add_controller(drag);
    let hint = gtk::Label::new(Some(&ls("Drag the pins to the desired position")));
    hint.set_xalign(0.0);

    let apply = gtk::Button::with_label(&ls("Apply"));
    apply.add_css_class("suggested-action");
    apply.set_halign(gtk::Align::End);
    let message = gtk::Label::new(error.map(ls).as_deref());
    message.add_css_class("error");
    message.set_xalign(0.0);
    message.set_wrap(true);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&hint);
    body.append(&preview);
    body.append(&group);
    body.append(&pins);
    body.append(&message);
    body.append(&apply);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&padded(&body)));
    scroll.set_propagate_natural_height(true);
    let d = dialog(&ls("Edit Subcircuit Pin Layout"), 460, 760, &scroll);
    let d2 = d.clone();
    let model = model.clone();
    apply.connect_clicked(move |_| {
        let mut m = model.clone();
        m.name = name.text().to_string();
        if label.is_active() {
            m.flags |= 1;
        } else {
            m.flags &= !1;
        }
        m.size_x = width.value() as i32;
        m.size_y = height.value() as i32;
        for (i, side, pos) in &rows {
            let (s, p) = (side.selected() as i32, pos.value() as i32);
            // the bits of a bus follow its first pin and share its place
            let first = &model.ext[*i];
            for pin in m.ext.iter_mut() {
                if pin.name == first.name && pin.bus_width == first.bus_width {
                    pin.side = s;
                    pin.pos = p;
                }
            }
        }
        let scope = match scope.selected() {
            1 => ModelScope::Session,
            2 => ModelScope::Saved,
            _ => ModelScope::Circuit,
        };
        d2.close();
        on_apply(m, scope);
    });
    d.present(Some(parent));
}

/// The subcircuit manager: the subcircuits there are, with buttons to place one, change its pin
/// layout or delete it.
pub fn subcircuit_manager(
    parent: &impl IsA<gtk::Widget>,
    names: &[String],
    on_place: impl Fn(String) + 'static,
    on_edit: impl Fn(String) + 'static,
    on_delete: impl Fn(String) + 'static,
) {
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    for n in names {
        let row = adw::ActionRow::new();
        row.set_title(n);
        list.append(&row);
    }
    let empty = gtk::Label::new(Some(&ls("No subcircuits yet. Use File > Create Subcircuit.")));
    empty.set_wrap(true);
    empty.set_xalign(0.0);
    empty.set_halign(gtk::Align::Start);
    empty.set_visible(names.is_empty());
    let (place, edit, delete) = (
        gtk::Button::with_label(&ls("Place")),
        gtk::Button::with_label(&ls("Edit Subcircuit Pin Layout")),
        gtk::Button::with_label(&ls("Delete")),
    );
    delete.add_css_class("destructive-action");
    // the labels are long once translated, so the buttons wrap instead of widening the dialog
    let buttons = gtk::FlowBox::new();
    buttons.set_selection_mode(gtk::SelectionMode::None);
    buttons.set_homogeneous(true);
    buttons.set_column_spacing(6);
    buttons.set_row_spacing(6);
    buttons.set_max_children_per_line(3);
    buttons.insert(&place, -1);
    buttons.insert(&edit, -1);
    buttons.insert(&delete, -1);
    buttons.set_halign(gtk::Align::Fill);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&list);
    body.append(&empty);
    body.append(&buttons);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&padded(&body)));
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    let d = dialog(&ls("Subcircuit Manager"), 560, 420, &scroll);
    let names: Vec<String> = names.to_vec();
    let chosen = {
        let (list, names) = (list.clone(), names.clone());
        move || list.selected_row().and_then(|r| names.get(r.index() as usize).cloned())
    };
    let on_place = Rc::new(on_place);
    let on_edit = Rc::new(on_edit);
    let on_delete = Rc::new(on_delete);
    {
        let (d2, ch) = (d.clone(), chosen.clone());
        place.connect_clicked(move |_| {
            if let Some(n) = ch() {
                d2.close();
                on_place(n);
            }
        });
    }
    {
        let (d2, ch) = (d.clone(), chosen.clone());
        edit.connect_clicked(move |_| {
            if let Some(n) = ch() {
                d2.close();
                on_edit(n);
            }
        });
    }
    {
        let (d2, parent2) = (d.clone(), parent.clone().upcast::<gtk::Widget>());
        delete.connect_clicked(move |_| {
            let Some(n) = chosen() else { return };
            // asks first; a deleted subcircuit cannot be brought back
            let alert = gtk::AlertDialog::builder()
                .message(format!("{} {n}?", ls("Are you sure you want to delete")))
                .buttons([ls("Cancel"), ls("Delete")])
                .cancel_button(0)
                .default_button(0)
                .build();
            let (d3, on_delete) = (d2.clone(), on_delete.clone());
            let window = parent2.root().and_downcast::<gtk::Window>();
            alert.choose(window.as_ref(), gio::Cancellable::NONE, move |r| {
                if r == Ok(1) {
                    d3.close();
                    on_delete(n.clone());
                }
            });
        });
    }
    d.present(Some(parent));
}

/// Values of the Other Options dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct OtherOptions {
    /// Largest time step in seconds.
    pub max_time_step: f64,
    /// Voltage at the ends of the colour scale.
    pub voltage_range: f64,
    /// Reduce the step automatically when convergence fails.
    pub adjust_time_step: bool,
    /// Find the DC operating point on reset.
    pub auto_dc_on_reset: bool,
    /// Smallest time step in seconds, used when the step adjusts itself.
    pub min_time_step: f64,
    /// Linear solver.
    pub solver: SolverType,
    /// Positive, negative, neutral, selection and current colours, in this order.
    pub colors: [[u8; 3]; 5],
    /// Decimals of short value text.
    pub decimals_short: usize,
    /// Decimals of normal value text.
    pub decimals_long: usize,
    /// Developer mode.
    pub developer_mode: bool,
    /// Frame rate below which the simulation does not keep up with real time.
    pub min_frame_rate: u32,
    /// How much a notch of the mouse wheel changes a value.
    pub wheel_sensitivity: f64,
    /// Size of the text of component values.
    pub value_font_size: i32,
    /// Language chosen in the dialog; `None` leaves it as it is.
    pub language: Option<String>,
}

const SOLVERS: [(SolverType, &str); 3] = [
    (SolverType::Auto, "Auto"),
    (SolverType::Dense, "Dense (Crout's LU)"),
    (SolverType::Sparse, "Sparse (CSC LU)"),
];

/// A caption above a control.
fn captioned(text: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let label = gtk::Label::new(Some(&ls(text)));
    label.set_xalign(0.0);
    b.append(&label);
    b.append(control);
    b
}

fn whole_number(min: f64, max: f64, value: f64) -> gtk::SpinButton {
    let spin = gtk::SpinButton::with_range(min, max, 1.0);
    spin.set_value(value);
    spin
}

fn color_button(rgb: [u8; 3]) -> gtk::ColorDialogButton {
    let button = gtk::ColorDialogButton::new(Some(gtk::ColorDialog::new()));
    if let Some(d) = button.dialog() {
        d.set_with_alpha(false);
    }
    button.set_rgba(&to_rgba(Color::new(rgb[0], rgb[1], rgb[2])));
    button
}

fn rgb_of(button: &gtk::ColorDialogButton) -> [u8; 3] {
    let c = from_rgba(&button.rgba());
    [c.r, c.g, c.b]
}

/// The Other Options dialog. Apply keeps it open, OK applies and closes it.
pub fn other_options(
    parent: &impl IsA<gtk::Widget>,
    now: OtherOptions,
    on_apply: impl Fn(OtherOptions) + 'static,
) {
    let step = gtk::Entry::new();
    step.set_text(&edit_value_text(now.max_time_step, false));
    let range = gtk::Entry::new();
    range.set_text(&edit_value_text(now.voltage_range, false));
    let mut names = vec![ls("(no change)")];
    names.extend(crate::locale::LANGUAGES.iter().map(|(_, n)| (*n).to_string()));
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let language = gtk::DropDown::from_strings(&names);

    let min_step = gtk::Entry::new();
    min_step.set_text(&edit_value_text(now.min_time_step, false));
    let colors: Vec<gtk::ColorDialogButton> = now.colors.iter().map(|c| color_button(*c)).collect();
    let reset = gtk::Button::with_label(&ls("Reset Colors to Default"));
    {
        let colors = colors.clone();
        reset.connect_clicked(move |_| {
            for (b, c) in colors.iter().zip(DEFAULT_COLORS) {
                b.set_rgba(&to_rgba(Color::new(c[0], c[1], c[2])));
            }
        });
    }
    let short = whole_number(0.0, 10.0, now.decimals_short as f64);
    let long = whole_number(0.0, 10.0, now.decimals_long as f64);
    let developer = gtk::CheckButton::with_label(&ls("Developer Mode"));
    developer.set_active(now.developer_mode);
    let frame_rate = whole_number(1.0, 120.0, f64::from(now.min_frame_rate));
    let wheel = gtk::SpinButton::with_range(0.0, 10.0, 0.5);
    wheel.set_value(now.wheel_sensitivity);
    let auto_dc = gtk::CheckButton::with_label(&ls("Auto-Run DC Operating Point on Reset"));
    auto_dc.set_active(now.auto_dc_on_reset);
    let adjust = gtk::CheckButton::with_label(&ls("Auto-Adjust Timestep"));
    adjust.set_active(now.adjust_time_step);
    let labels: Vec<String> = SOLVERS.iter().map(|(_, n)| ls(n)).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let solver = gtk::DropDown::from_strings(&labels);
    solver.set_selected(SOLVERS.iter().position(|(s, _)| *s == now.solver).unwrap_or(0) as u32);
    let font = whole_number(6.0, 48.0, f64::from(now.value_font_size));

    let column = || gtk::Box::new(gtk::Orientation::Vertical, 10);
    let (left, middle, right) = (column(), column(), column());
    left.append(&captioned("Time step size (s)", &step));
    left.append(&captioned("Range for voltage color (V)", &range));
    // like the original, the smallest step is only offered while the step adjusts itself
    let min_step_row = captioned("Minimum time step size (s)", &min_step);
    min_step_row.set_visible(now.adjust_time_step);
    {
        let row = min_step_row.clone();
        adjust.connect_toggled(move |a| row.set_visible(a.is_active()));
    }
    left.append(&min_step_row);
    left.append(&captioned("Change Language", &language));
    let color_titles =
        ["Positive Color", "Negative Color", "Neutral Color", "Selection Color", "Current Color"];
    for (title, button) in color_titles.iter().zip(&colors) {
        middle.append(&captioned(title, button));
    }
    middle.append(&reset);
    middle.append(&captioned("# of Decimal Digits (short format)", &short));
    right.append(&captioned("# of Decimal Digits (long format)", &long));
    right.append(&developer);
    right.append(&captioned("Minimum Target Frame Rate", &frame_rate));
    right.append(&captioned("Mouse Wheel Sensitivity", &wheel));
    right.append(&auto_dc);
    right.append(&adjust);
    right.append(&captioned("Matrix Solver", &solver));
    right.append(&captioned("Value Label Font Size", &font));
    let columns = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    for c in [&left, &middle, &right] {
        c.set_hexpand(true);
        c.set_valign(gtk::Align::Start);
        columns.append(c);
    }

    let apply = gtk::Button::with_label(&ls("Apply"));
    let ok = gtk::Button::with_label(&ls("OK"));
    ok.add_css_class("suggested-action");
    let cancel = gtk::Button::with_label(&ls("Cancel"));
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    for b in [&apply, &ok, &cancel] {
        buttons.append(b);
    }
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&columns);
    body.append(&buttons);
    let d = dialog(&ls("Other Options"), 760, 540, &padded(&body));

    let read = {
        let now = now.clone();
        move || {
            let mut out = now.clone();
            out.max_time_step = parse_edit_value(&step.text()).unwrap_or(now.max_time_step);
            out.voltage_range = parse_edit_value(&range.text()).unwrap_or(now.voltage_range);
            out.min_time_step = parse_edit_value(&min_step.text()).unwrap_or(now.min_time_step);
            out.adjust_time_step = adjust.is_active();
            out.auto_dc_on_reset = auto_dc.is_active();
            out.solver = SOLVERS.get(solver.selected() as usize).map_or(now.solver, |(s, _)| *s);
            for (slot, b) in out.colors.iter_mut().zip(&colors) {
                *slot = rgb_of(b);
            }
            out.decimals_short = short.value_as_int().max(0) as usize;
            out.decimals_long = long.value_as_int().max(0) as usize;
            out.developer_mode = developer.is_active();
            out.min_frame_rate = frame_rate.value_as_int().max(1) as u32;
            out.wheel_sensitivity = wheel.value();
            out.value_font_size = font.value_as_int();
            // entry 0 is "no change"; the others follow the list of languages
            out.language = (language.selected() as usize)
                .checked_sub(1)
                .and_then(|i| crate::locale::LANGUAGES.get(i))
                .map(|(code, _)| (*code).to_string());
            out
        }
    };
    let on_apply = Rc::new(on_apply);
    let read = Rc::new(read);
    {
        let (read, on_apply) = (read.clone(), on_apply.clone());
        apply.connect_clicked(move |_| on_apply(read()));
    }
    {
        let (d2, read, on_apply) = (d.clone(), read.clone(), on_apply.clone());
        ok.connect_clicked(move |_| {
            d2.close();
            on_apply(read());
        });
    }
    let d3 = d.clone();
    cancel.connect_clicked(move |_| {
        d3.close();
    });
    d.present(Some(parent));
}

fn filter(name: &str, patterns: &[&str]) -> gio::ListStore {
    let f = gtk::FileFilter::new();
    f.set_name(Some(name));
    for p in patterns {
        f.add_pattern(p);
    }
    let store = gio::ListStore::new::<gtk::FileFilter>();
    store.append(&f);
    store
}

/// Asks for the file a path field names: one to read or one to write. The folder of the path
/// already typed is where the chooser starts.
fn choose_path(
    parent: &gtk::Window,
    choice: &crate::elements::element::FileChoice,
    current: &str,
    on_path: impl Fn(PathBuf) + 'static,
) {
    use crate::elements::element::FileChoice;
    let d = gtk::FileDialog::new();
    let start = std::path::Path::new(current.trim());
    if let Some(dir) = start.parent().filter(|p| p.is_dir()) {
        d.set_initial_folder(Some(&gio::File::for_path(dir)));
    }
    match choice {
        FileChoice::Open(patterns) => {
            d.set_title(&ls("Choose File"));
            let refs: Vec<&str> = patterns.iter().map(String::as_str).collect();
            let store = filter(&ls("Data Files"), &refs);
            let all = gtk::FileFilter::new();
            all.set_name(Some(&ls("All Files")));
            all.add_pattern("*");
            store.append(&all);
            d.set_filters(Some(&store));
            d.open(Some(parent), gio::Cancellable::NONE, move |r| {
                if let Some(path) = r.ok().and_then(|f| f.path()) {
                    on_path(path);
                }
            });
        }
        FileChoice::Save(name) => {
            d.set_title(&ls("Save As"));
            let suggested = start.file_name().and_then(|n| n.to_str()).unwrap_or(name);
            d.set_initial_name(Some(suggested));
            d.save(Some(parent), gio::Cancellable::NONE, move |r| {
                if let Some(path) = r.ok().and_then(|f| f.path()) {
                    on_path(path);
                }
            });
        }
    }
}

/// Asks for a circuit file to open.
pub fn choose_open(parent: &impl IsA<gtk::Window>, on_path: impl Fn(PathBuf) + 'static) {
    let d = gtk::FileDialog::new();
    d.set_title(&ls("Open Circuit"));
    d.set_filters(Some(&filter(&ls("Circuit Files"), &super::consts::CIRCUIT_GLOBS)));
    d.open(Some(parent), gio::Cancellable::NONE, move |r| {
        if let Some(path) = r.ok().and_then(|f| f.path()) {
            on_path(path);
        }
    });
}

/// Asks where to save a file with the suggested name and extension filter.
pub fn choose_save(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    suggested: &str,
    pattern: &str,
    on_path: impl Fn(PathBuf) + 'static,
) {
    let d = gtk::FileDialog::new();
    d.set_title(title);
    d.set_initial_name(Some(suggested));
    d.set_filters(Some(&filter(pattern, &[pattern])));
    d.save(Some(parent), gio::Cancellable::NONE, move |r| {
        if let Some(path) = r.ok().and_then(|f| f.path()) {
            on_path(path);
        }
    });
}

/// Prints the circuit scaled to the page.
pub fn print(parent: &impl IsA<gtk::Window>, session: Rc<RefCell<Session>>) {
    let op = gtk::PrintOperation::new();
    op.set_n_pages(1);
    op.set_job_name(super::consts::PROGRAM_NAME);
    op.connect_draw_page(move |_, ctx, _| {
        let cr = ctx.cairo_context();
        crate::render::export::draw_page(&cr, &mut session.borrow_mut(), ctx.width(), ctx.height());
    });
    // a cancelled or failed job needs no message of ours; the print dialog reports its own errors
    let _ = op.run(gtk::PrintOperationAction::PrintDialog, Some(parent));
}

/// Puts a picture on the clipboard.
pub fn copy_png(parent: &impl IsA<gtk::Widget>, png: &[u8]) -> Result<(), String> {
    let tex = gtk::gdk::Texture::from_bytes(&glib::Bytes::from(png)).map_err(|e| e.to_string())?;
    parent.clipboard().set_texture(&tex);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LoadError;

    #[test]
    fn loading_problems_are_described_with_line_and_type() {
        let (t, d) =
            load_problem_text(&LoadError::UnknownType { line: 12, type_code: "zz".into() });
        assert!(t.contains("12") && d.contains("zz"));
        let (t, d) = load_problem_text(&LoadError::Malformed {
            line: 3,
            type_code: "r".into(),
            message: "missing field".into(),
        });
        assert!(t.contains('3') && t.contains('r') && d.contains("missing field"));
    }
}
