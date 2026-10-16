// SPDX-License-Identifier: GPL-2.0-or-later
//! Dialogs and file choosers of the main window.

use super::commands::accelerators;
use super::session::Session;
use crate::elements::element::EditInfo;
use crate::elements::registry::Registry;
use crate::elements::units::{edit_value_text, parse_edit_value};
use crate::locale::ls;
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
    d.set_application_name("prometheuscs");
    d.set_application_icon("applications-engineering-symbolic");
    d.set_version(env!("CARGO_PKG_VERSION"));
    d.set_comments(
        "Simulador de circuitos: um porte em Rust do CircuitJS1 com interface GTK4 e libadwaita.",
    );
    d.set_license_type(gtk::License::Gpl20);
    d.set_copyright("© Paul Falstad e Iain Sharp (CircuitJS1); porte prometheuscs");
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
    Fixed,
}

/// The element edit dialog, generated from the element's edit fields.
pub fn edit_element(
    parent: &impl IsA<gtk::Widget>,
    title: &str,
    fields: Vec<EditInfo>,
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
            row.set_title(&name);
            row.set_active(state);
            group.add(&row);
            widgets.push(Field::Switch(row));
        } else if ei.button.is_some() {
            widgets.push(Field::Fixed);
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
            group.add(&row);
            widgets.push(Field::Entry(row));
        }
    }
    let apply = gtk::Button::with_label("Aplicar");
    apply.add_css_class("suggested-action");
    apply.set_halign(gtk::Align::End);
    let message = gtk::Label::new(None);
    message.add_css_class("error");
    message.set_xalign(0.0);
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
                Field::Fixed => {}
            }
        }
        d2.close();
        on_apply(out);
    });
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

/// Asks for a circuit file to open.
pub fn choose_open(parent: &impl IsA<gtk::Window>, on_path: impl Fn(PathBuf) + 'static) {
    let d = gtk::FileDialog::new();
    d.set_title("Abrir circuito");
    d.set_filters(Some(&filter("Circuitos", &["*.txt", "*.xml", "*.circuitjs"])));
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
    op.set_job_name("prometheuscs");
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
