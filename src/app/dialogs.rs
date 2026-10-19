// SPDX-License-Identifier: GPL-2.0-or-later
//! Dialogs and file choosers of the main window.

use super::commands::accelerators;
use super::edit::SliderEntry;
use super::session::Session;
use super::subcircuits::ModelScope;
use crate::elements::element::EditInfo;
use crate::elements::registry::Registry;
use crate::elements::subcircuit::CompositeModel;
use crate::elements::units::{edit_value_text, parse_edit_value};
use crate::locale::ls;
use crate::render::painter::Color;
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

/// The About dialog.
pub fn about(parent: &impl IsA<gtk::Widget>) {
    let d = adw::AboutDialog::new();
    d.set_application_name(super::consts::APP_NAME);
    d.set_application_icon("applications-engineering-symbolic");
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
    dialog("Atalhos de teclado", 420, 560, &scroll).present(Some(parent));
}

/// One line of the list of loading problems, in the interface language.
fn load_problem_text(e: &crate::error::LoadError) -> (String, String) {
    use crate::error::LoadError;
    match e {
        LoadError::UnknownType { line, type_code } => (
            format!("Linha {line}: elemento desconhecido"),
            format!("O tipo `{type_code}` não existe nesta versão; a linha foi ignorada."),
        ),
        LoadError::Malformed { line, type_code, message } => (
            format!("Linha {line}: entrada `{type_code}` malformada"),
            format!("{message}; a linha foi ignorada."),
        ),
        LoadError::Xml(message) => ("O arquivo XML é inválido".into(), message.clone()),
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
    let intro = gtk::Label::new(Some(&format!(
        "{} {} não puderam ser lidas. O restante do circuito foi carregado.",
        errors.len(),
        if errors.len() == 1 { "entrada" } else { "entradas" }
    )));
    intro.set_wrap(true);
    intro.set_xalign(0.0);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&intro);
    body.append(&list);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&padded(&body)));
    scroll.set_vexpand(true);
    dialog("Problemas ao carregar o circuito", 520, 420, &scroll).present(Some(parent));
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
    let copy = gtk::Button::with_label("Copiar");
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
    let copy = gtk::Button::with_label("Copiar link");
    let (u, p) = (url.to_string(), parent.clone().upcast::<gtk::Widget>());
    copy.connect_clicked(move |_| p.clipboard().set_text(&u));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&entry);
    body.append(&copy);
    dialog("Exportar como Link", 520, 160, &padded(&body)).present(Some(parent));
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
    let file = gtk::Button::with_label("Abrir arquivo...");
    let go = gtk::Button::with_label("Importar");
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
    let d = dialog("Importar de Arquivo Texto", 640, 480, &body);
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
    let d = dialog("Encontrar Componente", 420, 480, &body);
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
    let apply = gtk::Button::with_label("Aplicar");
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
                                message.set_text(&format!("{}: valor inválido", ls(&ei.name)));
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
    let apply = gtk::Button::with_label("Aplicar");
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
                    message.set_text(&format!("{}: valor inválido", e.name));
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
    let apply = gtk::Button::with_label("Aplicar");
    apply.add_css_class("suggested-action");
    apply.set_halign(gtk::Align::End);
    let message = gtk::Label::new(error.map(ls).as_deref());
    message.add_css_class("error");
    message.set_xalign(0.0);
    message.set_wrap(true);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&group);
    body.append(&pins);
    body.append(&message);
    body.append(&apply);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&padded(&body)));
    scroll.set_propagate_natural_height(true);
    let d = dialog(&ls("Edit Subcircuit Pin Layout"), 460, 560, &scroll);
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
#[derive(Clone, Copy, Debug)]
pub struct OtherOptions {
    /// Largest time step in seconds.
    pub max_time_step: f64,
    /// Voltage at the ends of the colour scale.
    pub voltage_range: f64,
    /// Reduce the step automatically when convergence fails.
    pub adjust_time_step: bool,
    /// Find the DC operating point on reset.
    pub auto_dc_on_reset: bool,
}

/// The Other Options dialog.
pub fn other_options(
    parent: &impl IsA<gtk::Widget>,
    now: OtherOptions,
    on_apply: impl Fn(OtherOptions) + 'static,
) {
    let group = adw::PreferencesGroup::new();
    let step = adw::EntryRow::new();
    step.set_title("Passo de tempo (s)");
    step.set_text(&edit_value_text(now.max_time_step, false));
    let range = adw::EntryRow::new();
    range.set_title("Faixa de tensão para cores (V)");
    range.set_text(&edit_value_text(now.voltage_range, false));
    let adjust = adw::SwitchRow::new();
    adjust.set_title("Ajustar o passo de tempo automaticamente");
    adjust.set_active(now.adjust_time_step);
    let dc = adw::SwitchRow::new();
    dc.set_title("Encontrar o ponto CC ao reiniciar");
    dc.set_active(now.auto_dc_on_reset);
    for w in [&step, &range] {
        group.add(w);
    }
    group.add(&adjust);
    group.add(&dc);
    let apply = gtk::Button::with_label("Aplicar");
    apply.add_css_class("suggested-action");
    apply.set_halign(gtk::Align::End);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&group);
    body.append(&apply);
    let d = dialog("Outras Opções", 420, 360, &padded(&body));
    let d2 = d.clone();
    apply.connect_clicked(move |_| {
        let out = OtherOptions {
            max_time_step: parse_edit_value(&step.text()).unwrap_or(now.max_time_step),
            voltage_range: parse_edit_value(&range.text()).unwrap_or(now.voltage_range),
            adjust_time_step: adjust.is_active(),
            auto_dc_on_reset: dc.is_active(),
        };
        d2.close();
        on_apply(out);
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
            d.set_title("Escolher arquivo");
            let refs: Vec<&str> = patterns.iter().map(String::as_str).collect();
            let store = filter("Arquivos de dados", &refs);
            let all = gtk::FileFilter::new();
            all.set_name(Some("Todos os arquivos"));
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
            d.set_title("Salvar como");
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
    d.set_title("Abrir circuito");
    d.set_filters(Some(&filter("Circuitos", &super::consts::CIRCUIT_GLOBS)));
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
