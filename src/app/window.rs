// SPDX-License-Identifier: GPL-2.0-or-later
//! The main window: menu bar, toolbar, circuit canvas and the side panel.

use super::commands::{Action, Cmd, Outcome, apply, dispatch};
use super::consts;
use super::dialogs::{self, OtherOptions};
use super::edit::{Mods, ScopeMode, Tool};
use super::examples::{ExampleItem, ExampleList, parse_setup_list};
use super::menus::{self, ActionEntry};
use super::session::Session;
use super::settings::{AppSettings, settings_path};
use super::toolbar::{self, Toolbar};
use super::worker::Worker;
use crate::elements::element::EditInfo;
use crate::elements::registry::Registry;
use crate::elements::units::short_unit_text;
use crate::format::link::make_link;
use crate::locale;
use crate::render::cairo::CairoPainter;
use crate::render::export;
use crate::render::gl::{GlCanvas, GlPainter};
use crate::render::painter::Painter;
use crate::render::scene::{View, draw_overlay, draw_scene, post_draw_list};
use relm4::adw::{self, prelude::*};
use relm4::gtk::{self, gdk, glib};
use relm4::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

/// Seconds between automatic saves of a circuit with unsaved changes.
const AUTOSAVE_SECONDS: u32 = 30;

/// Folder with the bundled circuits and `setuplist.txt`.
fn assets_dir() -> PathBuf {
    consts::assets_dir()
}

/// Shown after another language was chosen.
/// Shown when the GPU canvas is running on a software renderer.
const SOFTWARE_GL_NOTICE: &str = "No graphics hardware acceleration was found: drawing is done by the CPU and large circuits may be slow.";
const LANGUAGE_NOTICE: &str = "The language will change the next time the program starts.";

/// Loads the Portuguese translations and the few strings the original does not translate.
fn load_translations(assets: &std::path::Path, setting: &str) {
    let code = if setting == "auto" { locale::detect_language() } else { setting };
    let mut map = locale::load_language(code, assets);
    if code == "pt" {
        for (k, v) in [
            ("Mode~", "Modo"),
            ("Show \u{3a9} Unit", "Mostrar Unidade \u{3a9}"),
            ("Drag", "Arrastar"),
            ("Delete", "Excluir"),
            (LANGUAGE_NOTICE, "O idioma será aplicado na próxima vez que o programa for aberto."),
            ("Language", "Idioma"),
        ] {
            map.entry(k.to_string()).or_insert_with(|| v.to_string());
        }
    }
    locale::set_translations(map);
    locale::set_current_language(code);
}

/// Runs a GTK callback body. A panic that escapes into GTK aborts the whole program, so it is
/// caught, logged, and the callback returns `fallback`.
fn guarded<T>(what: &str, fallback: T, f: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|m| (*m).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown".to_string());
            tracing::error!("panic in {what}: {msg}");
            fallback
        }
    }
}

/// Draws one frame of the session, whichever backend `g` draws with.
fn paint_frame(s: &mut Session, g: &mut dyn Painter, w: f64, h: f64) {
    s.canvas_size = (w, h);
    if s.fit_pending && w > 0.0 && h > 0.0 {
        s.view = View::fit(s.display_elements(), w, h, 2.0);
        s.fit_pending = false;
    }
    let posts = post_draw_list(s.display_elements());
    if s.inspecting() {
        // inside an instance there is nothing to select or place: no rubber band, no crosshair
        let (ctx, view) = (s.draw_ctx.clone(), s.view);
        draw_scene(g, s.display_elements_mut(), &posts, &ctx, view, (w, h));
        return;
    }
    draw_scene(g, &mut s.sim.elements, &posts, &s.draw_ctx, s.view, (w, h));
    let cross = if s.settings.crosshair { s.pointer } else { None };
    draw_overlay(g, &s.draw_ctx, s.view, (w, h), s.rubber_band(), cross);
    // with no scopes under the canvas, the information box is part of the canvas, as in the
    // original; with scopes it is the column to their right
    if s.scope_count() == 0 {
        let lines = s.info_lines();
        let tall = s.info_is_about_an_element();
        crate::render::scene::draw_info_box(g, &lines, (w, h), tall, s.settings.white_background);
    }
}

/// Translates the slider names used by the bundled circuits.
fn slider_title(name: &str) -> String {
    match name {
        "Capacitance" => "Capacitância",
        "Inductance" => "Indutância",
        "Resistance" => "Resistência",
        "Voltage" => "Tensão",
        "Frequency" => "Frequência",
        other => other,
    }
    .to_string()
}

/// Messages handled by the window.
#[derive(Debug)]
pub enum Msg {
    /// A menu entry, toolbar button or shortcut.
    Command(Cmd),
    /// Reset time and state.
    Reset,
    /// Start or stop time.
    ToggleRun,
    /// A circuit slider moved.
    Slider(usize, f64),
    /// A slider an element owns moved: element index and position.
    ElementSlider(usize, i32),
    /// The simulation speed slider moved.
    Speed(f64),
    /// The current speed slider moved.
    CurrentSpeed(f64),
    /// The brightness slider moved.
    Brightness(f64),
    /// Something changed outside the message flow; refresh the view.
    Refresh,
    /// Open a circuit file.
    OpenPath(PathBuf),
    /// Save to a file.
    SavePath(PathBuf),
    /// Load circuit text typed or pasted by the user.
    ImportText(String),
    /// Text read from the system clipboard for a paste.
    Paste(Option<String>),
    /// An element was picked in the Find dialog.
    Focus(usize),
    /// The edit dialog of an element was applied.
    ApplyEdit(usize, Vec<EditInfo>),
    /// The Other Options dialog was applied.
    ApplyOptions(OtherOptions),
    /// Show a message in the banner.
    Notice(String),
    /// Hide the banner.
    DismissBanner,
}

/// Window state.
pub struct App {
    session: Rc<RefCell<Session>>,
    window: adw::ApplicationWindow,
    assets: PathBuf,
    examples: ExampleList,
    registry: Registry,
    notice: Option<String>,
}

/// The GPU canvas and its font, once the GL context exists.
type GlSlot = Rc<RefCell<Option<(GlCanvas, Option<femtovg::FontId>)>>>;

/// Identity of the circuit sliders, to rebuild them only when they change.
/// (element, field, label, range and step as bits, logarithmic): a slider is rebuilt when any
/// of these change.
type SliderKey = Vec<(usize, usize, String, [u64; 3], bool)>;

/// Widgets that change after start-up.
pub struct Widgets {
    sliders: gtk::Box,
    run_button: gtk::Button,
    banner: adw::Banner,
    speed: gtk::Scale,
    current: gtk::Scale,
    brightness: gtk::Scale,
    circuit_label: gtk::Label,
    toolbar: Toolbar,
    actions: Vec<ActionEntry>,
    /// The part of the Draw > Subcircuits menu that lists the subcircuits made, and the names
    /// it shows now.
    subcircuit_menu: gtk::gio::Menu,
    shown_models: Vec<String>,
    shown_sliders: SliderKey,
    /// The play buttons of the audio outputs.
    audio: gtk::Box,
    /// The bar shown while a subcircuit model is edited, and the label with its name.
    context_bar: gtk::Box,
    context_label: gtk::Label,
    /// The buttons of the bar that only make sense while a model is being edited.
    context_save: [gtk::Button; 2],
    shown_audio: Vec<(usize, String)>,
}

/// Range of the simulation speed slider, as in the original: the speed grows exponentially,
/// so only a range this wide reaches both very slow and very fast runs.
const SPEED_RANGE: (f64, f64) = (0.0, 260.0);
/// Range of the current speed and power brightness sliders.
const BAR_RANGE: (f64, f64) = (1.0, 100.0);

fn slider_row(title: &str, value: f64, range: (f64, f64)) -> (gtk::Box, gtk::Scale, gtk::Label) {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let name = gtk::Label::new(Some(title));
    name.set_xalign(0.0);
    name.set_hexpand(true);
    let val = gtk::Label::new(None);
    val.add_css_class("dim-label");
    head.append(&name);
    head.append(&val);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, range.0, range.1, 1.0);
    scale.set_value(value);
    scale.set_draw_value(false);
    scale.set_hexpand(true);
    let minus = gtk::Button::from_icon_name("list-remove-symbolic");
    let plus = gtk::Button::from_icon_name("list-add-symbolic");
    for b in [&minus, &plus] {
        b.add_css_class("flat");
        b.add_css_class("circular");
        // the buttons are enabled and disabled together with their slider
        scale.bind_property("sensitive", b, "sensitive").sync_create().build();
    }
    let (s1, s2) = (scale.clone(), scale.clone());
    minus.connect_clicked(move |_| s1.set_value(s1.value() - 1.0));
    plus.connect_clicked(move |_| s2.set_value(s2.value() + 1.0));
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    line.append(&minus);
    line.append(&scale);
    line.append(&plus);
    row.append(&head);
    row.append(&line);
    (row, scale, val)
}

fn slider_key(s: &Session) -> SliderKey {
    let adjustable = s.adjustables.iter().map(|a| {
        (
            a.elm,
            a.edit_item,
            a.slider_text.clone(),
            [a.min_value.to_bits(), a.max_value.to_bits(), a.slider_step.to_bits()],
            a.logarithmic,
        )
    });
    // the sliders elements own have no field: `usize::MAX` stands for it
    let owned =
        s.element_sliders().into_iter().map(|(i, e)| (i, usize::MAX, e.text, [0; 3], false));
    adjustable.chain(owned).collect()
}

/// Opens the edit dialog of element `i`, with the buttons for its device model. A model
/// button closes the dialog, shows the model dialog and then opens this dialog again so it
/// shows the new model.
fn open_element_editor(
    win: &adw::ApplicationWindow,
    ses: &Rc<RefCell<Session>>,
    tx: &relm4::Sender<Msg>,
    i: usize,
) {
    let (title, fields, labels) = {
        let s = ses.borrow();
        (s.element_title(i), s.edit_fields(i), s.model_buttons(i))
    };
    let mut buttons: Vec<dialogs::DialogButton> = labels
        .into_iter()
        .enumerate()
        .map(|(b, label)| {
            let (win, ses, tx) = (win.clone(), ses.clone(), tx.clone());
            let action: Box<dyn Fn()> = Box::new(move || {
                let editor = ses.borrow().start_model_edit(i, b);
                match editor {
                    Some(editor) => open_model_editor(&win, &ses, &tx, i, editor, None),
                    None => open_element_editor(&win, &ses, &tx, i),
                }
            });
            (label, action)
        })
        .collect();
    // the parts of an instance can be looked at, whatever its model
    if ses.borrow().can_view_components(i) {
        let (s2, t) = (ses.clone(), tx.clone());
        buttons.push((
            "View Components".to_string(),
            Box::new(move || {
                let result = guarded("view components", Err("failed".to_string()), || {
                    s2.borrow_mut().view_components(i)
                });
                if let Err(e) = result {
                    let _ = t.send(Msg::Notice(locale::ls(&e)));
                }
                let _ = t.send(Msg::Refresh);
            }),
        ));
    }
    // an instance of a subcircuit model can have its pins and its circuit edited
    let model = ses.borrow().sim.elements.get(i).and_then(|e| e.subcircuit_model_name());
    if let Some(name) = model {
        let (w, s2, t) = (win.clone(), ses.clone(), tx.clone());
        buttons.push((
            "Edit Pin Layout".to_string(),
            Box::new(move || {
                match crate::elements::subcircuit::lookup(&name) {
                    Some(m) => open_subcircuit_dialog(&w, &s2, &t, &m, None),
                    None => {
                        let _ = t.send(Msg::Notice(locale::ls("Can't edit this model.")));
                    }
                };
            }),
        ));
        let (s2, t) = (ses.clone(), tx.clone());
        buttons.push((
            "Edit Model".to_string(),
            Box::new(move || {
                let result = guarded("edit model", Err("failed".to_string()), || {
                    s2.borrow_mut().edit_model(i)
                });
                if let Err(e) = result {
                    let _ = t.send(Msg::Notice(locale::ls(&e)));
                }
                let _ = t.send(Msg::Refresh);
            }),
        ));
    }
    let tx = tx.clone();
    dialogs::edit_element_with(win, &title, fields, buttons, None, move |f| {
        let _ = tx.send(Msg::ApplyEdit(i, f));
    });
}

/// Shows the pin layout dialog of a subcircuit being made or changed; a name or layout that
/// does not work shows it again with the reason.
fn open_subcircuit_dialog(
    win: &adw::ApplicationWindow,
    ses: &Rc<RefCell<Session>>,
    tx: &relm4::Sender<Msg>,
    model: &crate::elements::subcircuit::CompositeModel,
    error: Option<String>,
) {
    let (win2, ses2, tx2) = (win.clone(), ses.clone(), tx.clone());
    dialogs::subcircuit_pins(win, model, error.as_deref(), move |m, scope| {
        let r = guarded("subcircuit", Err("failed".to_string()), || {
            ses2.borrow_mut().finish_subcircuit(m.clone(), scope)
        });
        match r {
            Ok(()) => {
                let _ = tx2.send(Msg::Refresh);
            }
            Err(e) => open_subcircuit_dialog(&win2, &ses2, &tx2, &m, Some(e)),
        }
    });
}

/// Shows the subcircuit manager.
fn open_subcircuit_manager(
    win: &adw::ApplicationWindow,
    ses: &Rc<RefCell<Session>>,
    tx: &relm4::Sender<Msg>,
) {
    let names = ses.borrow().subcircuit_names();
    let (w1, s1, t1) = (win.clone(), ses.clone(), tx.clone());
    let (w2, s2, t2) = (win.clone(), ses.clone(), tx.clone());
    let (w3, s3, t3) = (win.clone(), ses.clone(), tx.clone());
    dialogs::subcircuit_manager(
        win,
        &names,
        move |n| {
            s1.borrow_mut().place_subcircuit(&n);
            let _ = t1.send(Msg::Refresh);
            let _ = &w1;
        },
        move |n| {
            let model = s2.borrow().subcircuit_model(&n);
            if let Some(m) = model {
                open_subcircuit_dialog(&w2, &s2, &t2, &m, None);
            }
        },
        move |n| {
            s3.borrow_mut().delete_subcircuit(&n);
            let _ = t3.send(Msg::Refresh);
            open_subcircuit_manager(&w3, &s3, &t3);
        },
    );
}

/// Shows the Add Sliders dialog of element `i`; values that are not allowed show it again.
fn open_slider_dialog(
    win: &adw::ApplicationWindow,
    ses: &Rc<RefCell<Session>>,
    tx: &relm4::Sender<Msg>,
    i: usize,
    error: Option<String>,
) {
    let (title, entries) = {
        let s = ses.borrow();
        (s.element_title(i), s.slider_entries(i))
    };
    let (win2, ses2, tx2) = (win.clone(), ses.clone(), tx.clone());
    dialogs::element_sliders(
        win,
        &format!("{} - {}", locale::ls("Add Sliders"), title),
        entries,
        error.as_deref(),
        move |choices| {
            let r = guarded("add sliders", Err("failed".to_string()), || {
                ses2.borrow_mut().apply_sliders(i, &choices)
            });
            match r {
                Ok(()) => {
                    let _ = tx2.send(Msg::Refresh);
                }
                Err(m) => open_slider_dialog(&win2, &ses2, &tx2, i, Some(locale::ls(&m))),
            }
        },
    );
}

/// Shows the properties dialog of scope `idx`; a value that is not allowed shows the dialog
/// again with the reason.
/// A scope the interface can act on: one of the diagrams under the canvas, or one placed in
/// the circuit as an element.
#[derive(Clone, Copy, Debug)]
enum ScopeTarget {
    /// The diagram with this index.
    Docked(usize),
    /// The scope element with this index.
    Embedded(usize),
}

fn open_scope_properties(
    win: &adw::ApplicationWindow,
    ses: &Rc<RefCell<Session>>,
    target: ScopeTarget,
    fields: Option<Vec<EditInfo>>,
    error: Option<String>,
) {
    let Some(fields) = fields.or_else(|| match target {
        ScopeTarget::Docked(i) => ses.borrow().scope_properties(i),
        ScopeTarget::Embedded(e) => ses.borrow().embedded_scope_properties(e),
    }) else {
        return;
    };
    let (win2, ses2) = (win.clone(), ses.clone());
    let shown = fields.clone();
    dialogs::edit_element_with(
        win,
        &locale::ls("Scope Properties"),
        shown,
        Vec::new(),
        error.as_deref(),
        move |f| {
            let result = guarded("scope properties", Err("failed".to_string()), || match target {
                ScopeTarget::Docked(i) => ses2.borrow_mut().apply_scope_properties(i, &f),
                ScopeTarget::Embedded(e) => {
                    ses2.borrow_mut().apply_embedded_scope_properties(e, &f)
                }
            });
            if let Err(message) = result {
                open_scope_properties(&win2, &ses2, target, Some(f), Some(locale::ls(&message)));
            }
        },
    );
}

/// The menu of a scope: what it plots, how it draws, and what can be done with it. It opens
/// over `parent` at (`x`, `y`).
fn show_scope_menu(
    win: &adw::ApplicationWindow,
    ses: &Rc<RefCell<Session>>,
    tx: &relm4::Sender<Msg>,
    parent: &gtk::Widget,
    target: ScopeTarget,
    (x, y): (f64, f64),
) {
    let items = match target {
        ScopeTarget::Docked(i) => ses.borrow().scope_menu(i),
        ScopeTarget::Embedded(e) => ses.borrow().embedded_scope_menu(e),
    };
    let body = gtk::Box::new(gtk::Orientation::Vertical, 2);
    for (value, key, active) in items {
        let check = gtk::CheckButton::with_label(&locale::ls(key));
        check.set_active(active);
        let ses = ses.clone();
        check.connect_toggled(move |c| {
            guarded("scope value", (), || match target {
                ScopeTarget::Docked(i) => {
                    ses.borrow_mut().toggle_scope_value(i, value, c.is_active());
                }
                ScopeTarget::Embedded(e) => {
                    ses.borrow_mut().toggle_embedded_value(e, value, c.is_active());
                }
            });
        });
        body.append(&check);
    }
    body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    {
        let check = gtk::CheckButton::with_label(&locale::ls("Max Scale"));
        check.set_active(match target {
            ScopeTarget::Docked(i) => ses.borrow().scope_max_scale(i),
            ScopeTarget::Embedded(e) => ses.borrow().embedded_max_scale(e),
        });
        let ses2 = ses.clone();
        check.connect_toggled(move |c| {
            guarded("max scale", (), || match target {
                ScopeTarget::Docked(i) => ses2.borrow_mut().set_scope_max_scale(i, c.is_active()),
                ScopeTarget::Embedded(e) => {
                    ses2.borrow_mut().set_embedded_max_scale(e, c.is_active());
                }
            });
        });
        body.append(&check);
    }
    for (mode, key) in [
        (ScopeMode::VoltageCurrent, "Show V vs I"),
        (ScopeMode::Xy, "Plot X/Y"),
        (ScopeMode::Spectrum, "Show Spectrum"),
    ] {
        let check = gtk::CheckButton::with_label(&locale::ls(key));
        check.set_active(match target {
            ScopeTarget::Docked(i) => ses.borrow().scope_mode(i, mode),
            ScopeTarget::Embedded(e) => ses.borrow().embedded_scope_mode(e, mode),
        });
        let ses = ses.clone();
        check.connect_toggled(move |c| {
            guarded("scope mode", (), || match target {
                ScopeTarget::Docked(i) => ses.borrow_mut().set_scope_mode(i, mode, c.is_active()),
                ScopeTarget::Embedded(e) => {
                    ses.borrow_mut().set_embedded_scope_mode(e, mode, c.is_active());
                }
            });
        });
        body.append(&check);
    }
    body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let popover = gtk::Popover::new();
    // one flat button that does `action` and closes the menu
    let entry = |label: &str, action: Box<dyn Fn()>| {
        let b = menu_button(&locale::ls(label));
        let pop = popover.clone();
        b.connect_clicked(move |_| {
            pop.popdown();
            action();
        });
        b
    };
    // placing the scope in a column with its neighbours, or combining it with the one before
    if let ScopeTarget::Docked(i) = target {
        type Can = fn(&Session, usize) -> bool;
        type Do = fn(&mut Session, usize);
        let items: [(&str, Can, Do); 3] = [
            ("Stack", Session::can_stack_scope, Session::stack_scope),
            ("Unstack", Session::can_unstack_scope, Session::unstack_scope),
            ("Combine", Session::can_combine_scope, Session::combine_scope),
        ];
        for (label, can, act) in items {
            if !can(&ses.borrow(), i) {
                continue;
            }
            let ses = ses.clone();
            body.append(&entry(
                label,
                Box::new(move || guarded("scope layout", (), || act(&mut ses.borrow_mut(), i))),
            ));
        }
    }
    // a scope with several plots can drop one of them
    let plots: Vec<String> = match target {
        ScopeTarget::Docked(i) => ses
            .borrow()
            .scope_views()
            .get(i)
            .map(|v| {
                v.plots
                    .iter()
                    .map(|p| format!("{} {}", p.title, crate::render::scopes::plot_label(p)))
                    .collect()
            })
            .unwrap_or_default(),
        ScopeTarget::Embedded(e) => ses.borrow().embedded_plot_names(e),
    };
    if plots.len() > 1 {
        for (k, name) in plots.iter().enumerate() {
            let ses = ses.clone();
            let label = format!("{}: {name}", locale::ls("Remove Plot"));
            body.append(&entry(
                &label,
                Box::new(move || {
                    guarded("remove plot", (), || match target {
                        ScopeTarget::Docked(i) => ses.borrow_mut().remove_scope_plot(i, k),
                        ScopeTarget::Embedded(e) => ses.borrow_mut().remove_embedded_plot(e, k),
                    });
                }),
            ));
        }
    }
    // the history of a scope placed in the circuit starts again whenever its settings change,
    // so there is nothing to reset there
    if let ScopeTarget::Docked(i) = target {
        let ses = ses.clone();
        body.append(&entry(
            "Reset",
            Box::new(move || {
                guarded("reset scope", (), || ses.borrow_mut().reset_scope(i));
            }),
        ));
    }
    {
        let (ses, win) = (ses.clone(), win.clone());
        body.append(&entry(
            "Export CSV...",
            Box::new(move || {
                let csv = match target {
                    ScopeTarget::Docked(i) => ses.borrow().scope_csv(i),
                    ScopeTarget::Embedded(e) => ses.borrow().embedded_scope_csv(e),
                };
                let Some(csv) = csv else { return };
                dialogs::choose_save(
                    &win,
                    "Exportar dados do osciloscópio",
                    consts::SCOPE_DATA_FILE,
                    consts::GLOB_CSV,
                    move |path| {
                        if let Err(e) = std::fs::write(&path, &csv) {
                            tracing::warn!("cannot write {}: {e}", path.display());
                        }
                    },
                );
            }),
        ));
    }
    {
        let (ses, win) = (ses.clone(), win.clone());
        body.append(&entry(
            "Properties...",
            Box::new(move || open_scope_properties(&win, &ses, target, None, None)),
        ));
    }
    {
        let (ses, tx) = (ses.clone(), tx.clone());
        let label = match target {
            ScopeTarget::Docked(_) => "Undock Scope",
            ScopeTarget::Embedded(_) => "Dock Scope",
        };
        body.append(&entry(
            label,
            Box::new(move || {
                guarded("dock scope", false, || match target {
                    ScopeTarget::Docked(i) => ses.borrow_mut().undock_scope(i),
                    ScopeTarget::Embedded(e) => ses.borrow_mut().dock_scope(e),
                });
                let _ = tx.send(Msg::Refresh);
            }),
        ));
    }
    let remove = menu_button(&locale::ls("Remove Scope"));
    {
        let (ses, pop, tx) = (ses.clone(), popover.clone(), tx.clone());
        remove.connect_clicked(move |_| {
            guarded("remove scope", (), || match target {
                ScopeTarget::Docked(i) => ses.borrow_mut().remove_scope(i),
                ScopeTarget::Embedded(e) => ses.borrow_mut().remove_embedded_scope(e),
            });
            pop.popdown();
            let _ = tx.send(Msg::Refresh);
        });
    }
    body.append(&remove);
    popover.set_child(Some(&body));
    popover.set_parent(parent);
    popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    popover.connect_closed(|p| {
        let p = p.clone();
        glib::idle_add_local_once(move || p.unparent());
    });
    popover.popup();
}

/// Shows the dialog of a device model being created or edited for element `i`.
fn open_model_editor(
    win: &adw::ApplicationWindow,
    ses: &Rc<RefCell<Session>>,
    tx: &relm4::Sender<Msg>,
    i: usize,
    editor: crate::elements::model_edit::ModelEditor,
    error: Option<String>,
) {
    let title = locale::ls(editor.title());
    let fields = editor.fields();
    let (win2, ses2, tx2) = (win.clone(), ses.clone(), tx.clone());
    dialogs::edit_element_with(win, &title, fields, Vec::new(), error.as_deref(), move |mut f| {
        let mut editor = editor.clone();
        let result = ses2.borrow_mut().finish_model_edit(i, &mut editor, &mut f);
        match result {
            // the options changed: show the dialog again with what was typed
            Ok(true) => open_model_editor(&win2, &ses2, &tx2, i, editor, None),
            Ok(false) => {
                let _ = tx2.send(Msg::Refresh);
                open_element_editor(&win2, &ses2, &tx2, i);
            }
            Err(message) => {
                open_model_editor(&win2, &ses2, &tx2, i, editor, Some(locale::ls(&message)));
            }
        }
    });
}

/// The row of an audio output in the side panel: a button that plays what it recorded and one
/// that saves it as a WAV file.
fn audio_row(
    win: &adw::ApplicationWindow,
    ses: &Rc<RefCell<Session>>,
    tx: &relm4::Sender<Msg>,
    elm: usize,
    label: &str,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let play = gtk::Button::with_label(&format!("\u{25b6} {} {label}", locale::ls("Play Audio")));
    play.set_hexpand(true);
    {
        let (ses, tx) = (ses.clone(), tx.clone());
        play.connect_clicked(move |_| {
            let wav = guarded("audio", None, || ses.borrow().audio_wav(elm));
            let result = match wav {
                Some(bytes) => super::playback::play(&bytes),
                None => Err(locale::ls(
                    "Audio data is not ready yet.  Increase simulation speed to make data ready sooner.",
                )),
            };
            if let Err(e) = result {
                let _ = tx.send(Msg::Notice(e));
            }
        });
    }
    let save = gtk::Button::from_icon_name("document-save-symbolic");
    save.set_tooltip_text(Some(&locale::ls("Download last played audio")));
    {
        let (ses, win, tx) = (ses.clone(), win.clone(), tx.clone());
        save.connect_clicked(move |_| {
            let Some(bytes) = guarded("audio", None, || ses.borrow().audio_wav(elm)) else {
                let _ = tx.send(Msg::Notice(locale::ls(
                    "Audio data is not ready yet.  Increase simulation speed to make data ready sooner.",
                )));
                return;
            };
            dialogs::choose_save(&win, "Salvar áudio", consts::AUDIO_FILE, consts::GLOB_WAV, move |path| {
                if let Err(e) = std::fs::write(&path, &bytes) {
                    tracing::warn!("cannot write {}: {e}", path.display());
                }
            });
        });
    }
    row.append(&play);
    row.append(&save);
    row
}

/// A popover with one flat button per command of `nodes`, a separator between sections. Unlike
/// `PopoverMenu` it always takes the height of all its entries.
fn flat_menu_popover(nodes: &[crate::app::commands::Node]) -> gtk::Popover {
    use crate::app::commands::Node;
    let popover = gtk::Popover::new();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        }
        let Node::Section(items) = node else { continue };
        for item in items {
            let Node::Item(cmd, label, _, accel) = item else { continue };
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 24);
            let text = gtk::Label::new(Some(&locale::ls(label)));
            text.set_xalign(0.0);
            text.set_hexpand(true);
            row.append(&text);
            if let Some(accel) = accel {
                let hint = gtk::Label::new(Some(accel));
                hint.add_css_class("dim-label");
                row.append(&hint);
            }
            let b = gtk::Button::new();
            b.set_child(Some(&row));
            b.add_css_class("flat");
            let (pop, name, target) = (
                popover.clone(),
                format!("win.{}", cmd.action_name()),
                cmd.target().map(|t| t.to_variant()),
            );
            b.connect_clicked(move |b| {
                pop.popdown();
                let _ = b.activate_action(&name, target.as_ref());
            });
            body.append(&b);
        }
    }
    popover.set_child(Some(&body));
    popover
}

/// A flat button for a popover menu: its text is left aligned and in the normal weight, like
/// the check buttons around it.
fn menu_button(text: &str) -> gtk::Button {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class("body");
    let b = gtk::Button::new();
    b.set_child(Some(&label));
    b.add_css_class("flat");
    b
}

fn example_title(list: &[ExampleItem], file: &str) -> Option<String> {
    list.iter().find_map(|i| match i {
        ExampleItem::Circuit { file: f, title } if f == file => Some(title.clone()),
        ExampleItem::Group { items, .. } => example_title(items, file),
        _ => None,
    })
}

impl App {
    /// Tells the user about the lines of the file that could not be read.
    fn show_load_problems(&self) {
        let errors = self.session.borrow().load_errors.clone();
        if !errors.is_empty() {
            dialogs::load_problems(&self.window, &errors);
        }
    }

    fn load_example(&mut self, file: &str) {
        let path = self.assets.join("circuits").join(file);
        let title = example_title(&self.examples.items, file).unwrap_or_else(|| file.to_string());
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let r = self.session.borrow_mut().open_text(&text, &locale::ls(&title), None);
                self.notice = r.err().map(|e| e.to_string());
                self.show_load_problems();
            }
            Err(e) => self.notice = Some(format!("{}: {e}", path.display())),
        }
    }

    fn handle_ui(&mut self, cmd: Cmd, sender: &ComponentSender<Self>) {
        let w = self.window.clone();
        match cmd {
            Cmd::NewWindow => {
                let started =
                    std::env::current_exe().and_then(|exe| std::process::Command::new(exe).spawn());
                if let Err(e) = started {
                    self.notice = Some(format!("Não foi possível abrir outra janela: {e}"));
                }
            }
            Cmd::Open => {
                let s = sender.input_sender().clone();
                dialogs::choose_open(&w, move |p| {
                    let _ = s.send(Msg::OpenPath(p));
                });
            }
            Cmd::ImportText => {
                let (s1, s2) = (sender.input_sender().clone(), sender.input_sender().clone());
                dialogs::import_text(
                    &w,
                    move |t| {
                        let _ = s1.send(Msg::ImportText(t));
                    },
                    move || {
                        let _ = s2.send(Msg::Command(Cmd::Open));
                    },
                );
            }
            Cmd::Save => {
                let path = self.session.borrow().path.clone();
                match path {
                    Some(p) => sender.input(Msg::SavePath(p)),
                    None => sender.input(Msg::Command(Cmd::SaveAs)),
                }
            }
            Cmd::SaveAs => {
                let name = format!("{}.xml", self.session.borrow().title);
                let s = sender.input_sender().clone();
                dialogs::choose_save(&w, "Salvar circuito", &name, consts::GLOB_XML, move |p| {
                    let _ = s.send(Msg::SavePath(p));
                });
            }
            Cmd::ExportLink => {
                let text = self.session.borrow_mut().export_text();
                dialogs::link(&w, &make_link(&text));
            }
            Cmd::ExportText => {
                let text = self.session.borrow_mut().export_text();
                dialogs::text_view(&w, "Exportar em Formato Texto", &text);
            }
            Cmd::ExportImage => self.export_picture(
                sender,
                "Exportar como Imagem",
                consts::IMAGE_FILE,
                consts::GLOB_PNG,
                false,
            ),
            Cmd::ExportSvg => self.export_picture(
                sender,
                "Exportar como SVG",
                consts::SVG_FILE,
                consts::GLOB_SVG,
                true,
            ),
            Cmd::CopyImage => {
                let result = export::png_bytes(&mut self.session.borrow_mut())
                    .and_then(|png| dialogs::copy_png(&w, &png));
                if let Err(e) = result {
                    self.notice = Some(e);
                }
            }
            Cmd::Recover => {
                let r = self.session.borrow_mut().recover_autosave();
                self.notice = r.err();
                self.show_load_problems();
            }
            Cmd::Print => dialogs::print(&w, self.session.clone()),
            Cmd::Fullscreen => w.set_fullscreened(!w.is_fullscreen()),
            Cmd::About => dialogs::about(&w),
            Cmd::Shortcuts => dialogs::shortcuts(&w, &self.registry),
            Cmd::CreateSubcircuit => {
                let made = self.session.borrow_mut().start_subcircuit();
                match made {
                    Ok(model) => open_subcircuit_dialog(
                        &w,
                        &self.session,
                        sender.input_sender(),
                        &model,
                        None,
                    ),
                    Err(e) => self.notice = Some(locale::ls(&e)),
                }
            }
            Cmd::SubcircuitManager => {
                open_subcircuit_manager(&w, &self.session, sender.input_sender());
            }
            Cmd::AddSliders => {
                let target = self.session.borrow().selected_slider_element();
                if let Some(i) = target {
                    open_slider_dialog(&w, &self.session, sender.input_sender(), i, None);
                }
            }
            Cmd::Find => {
                let labels = self.session.borrow().element_labels();
                let s = sender.input_sender().clone();
                dialogs::find(&w, labels, move |i| {
                    let _ = s.send(Msg::Focus(i));
                });
            }
            Cmd::Paste => {
                let s = sender.input_sender().clone();
                let clipboard = w.clipboard();
                glib::spawn_future_local(async move {
                    let text =
                        clipboard.read_text_future().await.ok().flatten().map(|t| t.to_string());
                    let _ = s.send(Msg::Paste(text));
                });
            }
            Cmd::OtherOptions => {
                let (o, st) = {
                    let ses = self.session.borrow();
                    (ses.options.clone(), ses.settings.clone())
                };
                let now = OtherOptions {
                    max_time_step: o.max_time_step,
                    voltage_range: o.voltage_range,
                    adjust_time_step: o.adjust_time_step,
                    auto_dc_on_reset: o.auto_dc_on_reset,
                    min_time_step: o.min_time_step,
                    solver: o.solver,
                    colors: [
                        st.positive_color,
                        st.negative_color,
                        st.neutral_color,
                        st.selection_color,
                        st.current_color,
                    ],
                    decimals_short: st.decimals_short,
                    decimals_long: st.decimals_long,
                    developer_mode: st.developer_mode,
                    min_frame_rate: st.min_frame_rate,
                    wheel_sensitivity: st.wheel_sensitivity,
                    value_font_size: st.value_font_size,
                    language: None,
                };
                let s = sender.input_sender().clone();
                dialogs::other_options(&w, now, move |v| {
                    let _ = s.send(Msg::ApplyOptions(v));
                });
            }
            Cmd::Example(file) => self.load_example(&file),
            Cmd::Language(code) => {
                self.session.borrow_mut().set_language(&code);
                self.notice = Some(locale::ls(LANGUAGE_NOTICE));
            }
            other => self.notice = Some(format!("Recurso ainda não disponível: {other:?}")),
        }
    }

    fn export_picture(
        &self,
        sender: &ComponentSender<Self>,
        title: &str,
        name: &str,
        pattern: &str,
        svg: bool,
    ) {
        let ses = self.session.clone();
        let s = sender.input_sender().clone();
        dialogs::choose_save(&self.window, title, name, pattern, move |p| {
            let r = if svg {
                export::save_svg(&mut ses.borrow_mut(), &p)
            } else {
                export::save_png(&mut ses.borrow_mut(), &p)
            };
            if let Err(e) = r {
                let _ = s.send(Msg::Notice(e));
            }
        });
    }
}

impl SimpleComponent for App {
    type Init = ();
    type Input = Msg;
    type Output = ();
    type Root = adw::ApplicationWindow;
    type Widgets = Widgets;

    fn init_root() -> Self::Root {
        adw::ApplicationWindow::builder()
            .title(consts::APP_NAME)
            .default_width(consts::DEFAULT_WINDOW_WIDTH)
            .default_height(consts::DEFAULT_WINDOW_HEIGHT)
            .build()
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        tracing::debug!(ms = super::bench::since_start(), "init begins");
        let assets = assets_dir();
        load_translations(&assets, &AppSettings::load(&settings_path()).language);
        let registry = Registry::standard();
        let examples = std::fs::read_to_string(assets.join(consts::SETUPLIST_FILE))
            .map(|t| parse_setup_list(&t))
            .unwrap_or_default();
        let settings = AppSettings::load(&settings_path());
        let dark = adw::StyleManager::default().is_dark();

        // the subcircuits kept for every circuit
        super::subcircuits::load_saved();
        let mut session = Session::empty_with(&registry, settings.clone());
        // the solver runs on its own thread; the window only draws what it publishes
        session.attach_worker(Worker::spawn(registry.clone(), settings));
        session.settings_path = Some(settings_path());
        let startup = examples.startup.clone().unwrap_or_else(|| consts::FALLBACK_STARTUP.into());
        let mut notice = None;
        // a render benchmark draws the circuit it was given instead of the start-up example
        let bench_path = super::bench::config().map(|c| c.path.clone());
        let startup_path = bench_path.unwrap_or_else(|| assets.join("circuits").join(&startup));
        match std::fs::read_to_string(&startup_path) {
            Ok(text) => {
                let title = example_title(&examples.items, &startup).unwrap_or(startup.clone());
                if let Err(e) = session.open_text(&text, &locale::ls(&title), None) {
                    notice = Some(e.to_string());
                }
            }
            Err(e) => notice = Some(format!("{startup}: {e}")),
        }
        tracing::debug!(ms = super::bench::since_start(), "session loaded");
        let session = Rc::new(RefCell::new(session));
        // the render benchmark, when asked for on the command line
        let bench: Rc<RefCell<Option<super::bench::Bench>>> =
            Rc::new(RefCell::new(super::bench::config().map(|c| {
                let n = session.borrow().sim.elements.len();
                super::bench::Bench::new(c.clone(), n)
            })));

        // --- canvas: a GPU canvas (GtkGLArea + femtovg) with the Cairo canvas as fallback
        let cairo_area = gtk::DrawingArea::new();
        cairo_area.set_hexpand(true);
        cairo_area.set_vexpand(true);
        let draw_session = session.clone();
        let cairo_bench = bench.clone();
        cairo_area.set_draw_func(move |_, cr, w, h| {
            guarded("drawing", (), || {
                let began = std::time::Instant::now();
                let mut g = CairoPainter::new(cr);
                paint_frame(&mut draw_session.borrow_mut(), &mut g, f64::from(w), f64::from(h));
                if let Some(b) = cairo_bench.borrow_mut().as_mut() {
                    b.on_draw(began.elapsed());
                }
            });
        });
        let gl_area = gtk::GLArea::new();
        gl_area.set_hexpand(true);
        gl_area.set_vexpand(true);
        gl_area.set_has_stencil_buffer(true);
        gl_area.set_auto_render(false);
        let area = gtk::Stack::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_focusable(true);
        area.add_named(&gl_area, Some("gl"));
        area.add_named(&cairo_area, Some("cairo"));
        // PROMETHEUSCS_RENDERER=cairo forces the CPU canvas
        let force_cpu = std::env::var(consts::ENV_RENDERER).is_ok_and(|v| v == "cairo");
        area.set_visible_child_name(if force_cpu { "cairo" } else { "gl" });
        let gl: GlSlot = Rc::new(RefCell::new(None));
        {
            let (gl_slot, stack) = (gl.clone(), area.clone());
            let notice = sender.input_sender().clone();
            gl_area.connect_realize(move |a| {
                a.make_current();
                a.attach_buffers();
                let created = match a.error() {
                    Some(e) => Err(e.to_string()),
                    None => GlCanvas::new(),
                };
                tracing::debug!(ms = super::bench::since_start(), "GL canvas created");
                match created {
                    Ok(mut c) => {
                        let renderer = c.renderer_name();
                        tracing::info!(renderer, "GPU canvas ready");
                        if crate::render::gl::is_software_renderer(&renderer) {
                            let _ = notice.send(Msg::Notice(locale::ls(SOFTWARE_GL_NOTICE)));
                        }
                        let font = c.load_font();
                        if font.is_none() {
                            tracing::warn!("no system font found; text will not be drawn");
                        }
                        tracing::debug!(ms = super::bench::since_start(), "font loaded");
                        *gl_slot.borrow_mut() = Some((c, font));
                    }
                    Err(e) => {
                        tracing::warn!("GPU canvas unavailable ({e}); using the CPU canvas");
                        stack.set_visible_child_name("cairo");
                    }
                }
            });
            let gl_slot = gl.clone();
            gl_area.connect_unrealize(move |a| {
                a.make_current();
                *gl_slot.borrow_mut() = None;
            });
            let (gl_slot, ses) = (gl.clone(), session.clone());
            let gl_bench = bench.clone();
            let frames = Cell::new(0_u32);
            gl_area.connect_render(move |a, _| {
                guarded("GPU drawing", (), || {
                    if let Some((c, font)) = gl_slot.borrow_mut().as_mut() {
                        let began = std::time::Instant::now();
                        let (w, h) = (a.width(), a.height());
                        // the buffer GTK made, which with fractional scaling is the rounded-up
                        // scale times the size; the canvas scale is what that makes of the width
                        let scale = a.scale_factor();
                        let (bw, bh) = c.viewport_size().unwrap_or_else(|| {
                            let px = |v: i32| u32::try_from(v * scale).unwrap_or(1).max(1);
                            (px(w), px(h))
                        });
                        let factor = bw as f32 / w.max(1) as f32;
                        c.begin(bw, bh, factor, femtovg::Color::rgb(0, 0, 0));
                        let mut g = GlPainter::new(&mut c.canvas, *font);
                        paint_frame(&mut ses.borrow_mut(), &mut g, f64::from(w), f64::from(h));
                        c.end();
                        if let Some(b) = gl_bench.borrow_mut().as_mut() {
                            b.on_draw(began.elapsed());
                        }
                        // a diagnostic for GPU problems: once, read back how much was drawn
                        frames.set(frames.get() + 1);
                        if frames.get() == 60 && tracing::enabled!(tracing::Level::DEBUG) {
                            let lit = c.non_black_pixels(bw, bh);
                            tracing::debug!(
                                lit,
                                total = bw * bh,
                                scale = factor,
                                w,
                                h,
                                "GPU canvas drew pixels"
                            );
                        }
                    }
                });
                glib::Propagation::Stop
            });
        }

        // --- mouse
        let space = Rc::new(Cell::new(false));
        let last_pointer = Rc::new(Cell::new((0.0_f64, 0.0_f64)));
        let start = Rc::new(Cell::new((0.0_f64, 0.0_f64)));
        let drag = gtk::GestureDrag::builder().button(0).build();
        {
            let (ses, tx, sp, st) =
                (session.clone(), sender.input_sender().clone(), space.clone(), start.clone());
            let (win_w, tx_w) = (root.clone(), tx.clone());
            drag.connect_drag_begin(move |g, x, y| {
                st.set((x, y));
                // the settings wheel of a scope placed in the circuit opens its properties
                let wheel = ses.borrow().embedded_wheel_at(x, y);
                if let Some(e) = wheel {
                    open_scope_properties(&win_w, &ses, ScopeTarget::Embedded(e), None, None);
                    let _ = tx_w.send(Msg::Refresh);
                    return;
                }
                let state = g.current_event_state();
                let mods = Mods {
                    shift: state.contains(gdk::ModifierType::SHIFT_MASK) || sp.get(),
                    ctrl: state.contains(gdk::ModifierType::CONTROL_MASK),
                };
                let button = g.current_button();
                guarded("mouse press", false, || ses.borrow_mut().mouse_down(x, y, mods, button));
                let _ = tx.send(Msg::Refresh);
            });
            let (ses, st) = (session.clone(), start.clone());
            drag.connect_drag_update(move |_, dx, dy| {
                let (sx, sy) = st.get();
                guarded("mouse drag", false, || ses.borrow_mut().mouse_drag(sx + dx, sy + dy));
            });
            let (ses, tx) = (session.clone(), sender.input_sender().clone());
            drag.connect_drag_end(move |_, _, _| {
                guarded("mouse release", false, || ses.borrow_mut().mouse_up());
                let _ = tx.send(Msg::Refresh);
            });
        }
        area.add_controller(drag);
        let click = gtk::GestureClick::builder().button(1).build();
        {
            let (ses, win, tx) = (session.clone(), root.clone(), sender.input_sender().clone());
            click.connect_pressed(move |_, n, x, y| {
                if n != 2 {
                    return;
                }
                let target = guarded("double click", None, || ses.borrow().element_for_edit(x, y));
                if let Some(i) = target {
                    if ses.borrow().embedded_scope(i).is_some() {
                        open_scope_properties(&win, &ses, ScopeTarget::Embedded(i), None, None);
                    } else {
                        open_element_editor(&win, &ses, &tx, i);
                    }
                }
            });
        }
        area.add_controller(click);
        // right click on an element: a small menu of what can be done with it
        let context = gtk::GestureClick::builder().button(3).build();
        {
            let (ses, area_ctx) = (session.clone(), area.clone());
            let (tx_ctx, win_ctx) = (sender.input_sender().clone(), root.clone());
            context.connect_pressed(move |_, _, x, y| {
                let hit =
                    guarded("context menu", false, || ses.borrow_mut().select_for_context(x, y));
                if !hit {
                    return;
                }
                // the entries are enabled according to the new selection
                let _ = tx_ctx.send(Msg::Refresh);
                // a scope placed in the circuit has the menu of a diagram
                let scope_element = ses.borrow().selected_scope_element();
                if let Some(elm) = scope_element {
                    show_scope_menu(
                        &win_ctx,
                        &ses,
                        &tx_ctx,
                        area_ctx.upcast_ref(),
                        ScopeTarget::Embedded(elm),
                        (x, y),
                    );
                    return;
                }
                let on_scope_element = false;
                let first = if on_scope_element {
                    crate::app::commands::Node::Item(
                        Cmd::DockScope,
                        "Dock Scope".to_string(),
                        None,
                        None,
                    )
                } else {
                    crate::app::commands::Node::Item(
                        Cmd::ViewInScope,
                        "View in New Scope".to_string(),
                        None,
                        None,
                    )
                };
                let mut view = vec![first];
                #[cfg(feature = "outputs")]
                view.push(crate::app::commands::Node::Item(
                    Cmd::ViewInUndockedScope,
                    "View in New Undocked Scope".to_string(),
                    None,
                    None,
                ));
                let nodes = vec![
                    crate::app::commands::Node::Section(view),
                    crate::app::commands::Node::Section(vec![crate::app::commands::Node::Item(
                        Cmd::AddSliders,
                        "Add Sliders".to_string(),
                        None,
                        None,
                    )]),
                    crate::app::commands::Node::Section(vec![
                        crate::app::commands::Node::Item(
                            Cmd::Duplicate,
                            "Duplicate".to_string(),
                            None,
                            None,
                        ),
                        crate::app::commands::Node::Item(
                            Cmd::Delete,
                            "Delete".to_string(),
                            None,
                            None,
                        ),
                    ]),
                ];
                let popover = flat_menu_popover(&nodes);
                popover.set_parent(&area_ctx);
                popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                popover.connect_closed(|p| {
                    // the popover is rebuilt for every click
                    let p = p.clone();
                    glib::idle_add_local_once(move || p.unparent());
                });
                popover.popup();
            });
        }
        area.add_controller(context);
        let motion = gtk::EventControllerMotion::new();
        {
            let (ses, lp) = (session.clone(), last_pointer.clone());
            motion.connect_motion(move |_, x, y| {
                lp.set((x, y));
                guarded("hover", (), || ses.borrow_mut().hover_at(Some((x, y))));
            });
            let ses = session.clone();
            motion.connect_leave(move |_| {
                guarded("hover", (), || ses.borrow_mut().hover_at(None));
            });
        }
        area.add_controller(motion);
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        {
            let (ses, lp) = (session.clone(), last_pointer.clone());
            scroll.connect_scroll(move |_, _, dy| {
                let (x, y) = lp.get();
                guarded("wheel", (), || {
                    let mut s = ses.borrow_mut();
                    if !s.wheel_at(x, y, dy < 0.0) {
                        let step = 1.1_f64.powf(s.settings.wheel_sensitivity);
                        s.zoom_at(x, y, if dy < 0.0 { step } else { 1.0 / step });
                    }
                });
                glib::Propagation::Stop
            });
        }
        area.add_controller(scroll);

        // --- keyboard: Escape, Space-to-pan, and switch shortcut keys
        let keys = gtk::EventControllerKey::new();
        {
            let (ses, tx, sp) = (session.clone(), sender.input_sender().clone(), space.clone());
            keys.connect_key_pressed(move |_, key, _, state| {
                if key == gdk::Key::Escape {
                    let _ = tx.send(Msg::Command(Cmd::SetTool(Tool::Select)));
                    return glib::Propagation::Stop;
                }
                if key == gdk::Key::space {
                    sp.set(true);
                    return glib::Propagation::Stop;
                }
                let plain = !state
                    .intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK);
                if plain {
                    if let Some(c) = key.to_unicode() {
                        let key = c.to_lowercase().to_string();
                        if guarded("key", false, || ses.borrow_mut().key_toggle(&key)) {
                            let _ = tx.send(Msg::Refresh);
                            return glib::Propagation::Stop;
                        }
                    }
                }
                glib::Propagation::Proceed
            });
            let sp = space.clone();
            keys.connect_key_released(move |_, key, _, _| {
                if key == gdk::Key::space {
                    sp.set(false);
                }
            });
        }
        root.add_controller(keys);

        // --- side panel
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
        panel.set_width_request(consts::SIDE_PANEL_WIDTH);
        panel.set_margin_top(12);
        panel.set_margin_bottom(12);
        panel.set_margin_start(12);
        panel.set_margin_end(12);
        let reset = gtk::Button::with_label("Reiniciar");
        let run_button = gtk::Button::with_label("Parar");
        {
            let s = sender.clone();
            reset.connect_clicked(move |_| s.input(Msg::Reset));
            let s = sender.clone();
            run_button.connect_clicked(move |_| s.input(Msg::ToggleRun));
        }
        panel.append(&reset);
        panel.append(&run_button);
        let (speed_row, speed, speed_val) = slider_row(
            "Velocidade da Simulação",
            f64::from(session.borrow().options.speed_bar()),
            SPEED_RANGE,
        );
        let (current_row, current, current_val) = slider_row(
            "Velocidade da Corrente",
            f64::from(session.borrow().options.current_bar),
            BAR_RANGE,
        );
        let (bright_row, brightness, bright_val) = slider_row(
            "Brilho da Potência",
            f64::from(session.borrow().options.power_bar),
            BAR_RANGE,
        );
        for v in [&speed_val, &current_val, &bright_val] {
            v.set_text("");
        }
        {
            let s = sender.clone();
            speed.connect_value_changed(move |sc| s.input(Msg::Speed(sc.value())));
            let s = sender.clone();
            current.connect_value_changed(move |sc| s.input(Msg::CurrentSpeed(sc.value())));
            let s = sender.clone();
            brightness.connect_value_changed(move |sc| s.input(Msg::Brightness(sc.value())));
        }
        panel.append(&speed_row);
        panel.append(&current_row);
        panel.append(&bright_row);
        let circuit_label = gtk::Label::new(None);
        circuit_label.set_xalign(0.0);
        circuit_label.set_wrap(true);
        circuit_label.add_css_class("heading");
        panel.append(&circuit_label);
        let sliders = gtk::Box::new(gtk::Orientation::Vertical, 12);
        panel.append(&sliders);
        let audio = gtk::Box::new(gtk::Orientation::Vertical, 6);
        panel.append(&audio);
        let panel_scroll = gtk::ScrolledWindow::new();
        panel_scroll.set_child(Some(&panel));
        panel_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);

        // --- menus, toolbar, actions
        let title = adw::WindowTitle::new(consts::APP_NAME, "");
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&title));
        let (model, subcircuit_menu) = menus::menu_bar_model(&registry, &examples, &assets, dark);
        let menu_bar = gtk::PopoverMenuBar::from_model(Some(&model));
        let tb = toolbar::build(&registry, &assets, dark);
        let command_sender = sender.input_sender().clone();
        let actions = menus::install_actions(
            &root,
            &registry,
            Rc::new(move |cmd| {
                let _ = command_sender.send(Msg::Command(cmd));
            }),
        );

        menus::install_shortcuts(&root, &registry);

        // --- layout
        let banner = adw::Banner::new("");
        banner.set_button_label(Some("Fechar"));
        {
            let s = sender.clone();
            banner.connect_button_clicked(move |_| s.input(Msg::DismissBanner));
        }
        // the time diagrams go under the canvas; the area has no height without scopes
        let scope_area = gtk::DrawingArea::new();
        scope_area.set_hexpand(true);
        scope_area.set_content_height(0);
        scope_area.set_visible(false);
        let scope_session = session.clone();
        // where the pointer is over the diagrams, for the readout under it
        let scope_pointer = Rc::new(Cell::new(None::<(f64, f64)>));
        let draw_pointer = scope_pointer.clone();
        scope_area.set_draw_func(move |_, cr, w, h| {
            guarded("scope drawing", (), || {
                let mut g = CairoPainter::new(cr);
                let mut s = scope_session.borrow_mut();
                let views = s.scope_views();
                let light = s.settings.white_background;
                let mut scales = std::mem::take(&mut s.scope_scales);
                scales.info = s.info_lines();
                scales.glow = s.scope_glow();
                crate::render::scopes::draw_scopes(
                    &mut g,
                    &views,
                    &mut scales,
                    f64::from(w),
                    f64::from(h),
                    light,
                    draw_pointer.get(),
                );
                s.scope_scales = scales;
            });
        });
        let motion = gtk::EventControllerMotion::new();
        {
            let (pointer, area_m) = (scope_pointer.clone(), scope_area.clone());
            let ses_m = session.clone();
            motion.connect_motion(move |_, x, y| {
                pointer.set(Some((x, y)));
                guarded("scope hover", (), || {
                    let mut s = ses_m.borrow_mut();
                    let under = crate::render::scopes::scope_at(
                        &s.scope_views(),
                        f64::from(area_m.width()),
                        f64::from(area_m.height()),
                        x,
                        y,
                    );
                    s.set_scope_hover(under);
                });
                area_m.queue_draw();
            });
            let (pointer, area_m) = (scope_pointer.clone(), scope_area.clone());
            let ses_m = session.clone();
            motion.connect_leave(move |_| {
                pointer.set(None);
                // GTK sends this synchronously when the area is hidden, possibly while the
                // frame callback still holds the session: then there is nothing to update
                guarded("scope leave", (), || {
                    let _ = crate::app::session::try_with_session(&ses_m, |s| {
                        s.set_scope_hover(None);
                    });
                });
                area_m.queue_draw();
            });
        }
        scope_area.add_controller(motion);
        // right click on a diagram: choose what it plots
        let scope_menu = gtk::GestureClick::builder().button(3).build();
        {
            let (ses, area_s) = (session.clone(), scope_area.clone());
            let (win_s, tx_s) = (root.clone(), sender.input_sender().clone());
            scope_menu.connect_pressed(move |_, _, x, y| {
                let picked = crate::render::scopes::scope_at(
                    &ses.borrow().scope_views(),
                    f64::from(area_s.width()),
                    f64::from(area_s.height()),
                    x,
                    y,
                );
                let Some(idx) = picked else { return };
                show_scope_menu(
                    &win_s,
                    &ses,
                    &tx_s,
                    area_s.upcast_ref(),
                    ScopeTarget::Docked(idx),
                    (x, y),
                );
            });
        }
        scope_area.add_controller(scope_menu);
        // the wheel over a scope changes its time scale: down is slower, up is faster
        let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        {
            let (ses, area_w, pointer) =
                (session.clone(), scope_area.clone(), scope_pointer.clone());
            let delta = Cell::new(0.0_f64);
            wheel.connect_scroll(move |_, _, dy| {
                let sensitivity = ses.borrow().settings.wheel_sensitivity;
                delta.set(delta.get() + dy * sensitivity);
                if delta.get().abs() < 1.0 {
                    return glib::Propagation::Stop;
                }
                let faster = delta.get() < 0.0;
                delta.set(0.0);
                let Some((x, y)) = pointer.get() else { return glib::Propagation::Stop };
                guarded("scope speed", (), || {
                    let mut s = ses.borrow_mut();
                    let (w, h) = (f64::from(area_w.width()), f64::from(area_w.height()));
                    if let Some(idx) = crate::render::scopes::scope_at(&s.scope_views(), w, h, x, y)
                    {
                        let now = s.scope_speed(idx);
                        s.set_scope_speed(idx, if faster { now / 2 } else { now * 2 });
                    }
                });
                area_w.queue_draw();
                glib::Propagation::Stop
            });
        }
        scope_area.add_controller(wheel);
        // in manual scale mode, dragging the plot under the pointer moves it up or down
        let plot_drag = gtk::GestureDrag::builder().button(1).build();
        {
            // (scope, plot, position when the drag began, half height of the diagram)
            let state = Rc::new(Cell::new(None::<(usize, usize, i32, f64)>));
            let (ses, area_d, st) = (session.clone(), scope_area.clone(), state.clone());
            let win_wheel = root.clone();
            plot_drag.connect_drag_begin(move |_, x, y| {
                // the settings wheel in the corner of a scope opens its properties
                let wheel = {
                    let s = ses.borrow();
                    let views = s.scope_views();
                    let (aw, ah) = (f64::from(area_d.width()), f64::from(area_d.height()));
                    crate::render::scopes::scope_at(&views, aw, ah, x, y).filter(|&i| {
                        let r = crate::render::scopes::layout(&views, aw, ah)[i];
                        crate::render::scopes::on_wheel(r, x - r.0, y - r.1)
                    })
                };
                if let Some(i) = wheel {
                    open_scope_properties(&win_wheel, &ses, ScopeTarget::Docked(i), None, None);
                    return;
                }
                guarded("plot drag", (), || {
                    let mut s = ses.borrow_mut();
                    let views = s.scope_views();
                    let (aw, ah) = (f64::from(area_d.width()), f64::from(area_d.height()));
                    let rects = crate::render::scopes::layout(&views, aw, ah);
                    // dragging along a plot in manual scale moves it; anywhere else it
                    // measures the stretch of the signal that is dragged over
                    if let Some((i, p)) = s.scope_scales.hover
                        && let Some((true, pos)) = s.plot_position(i, p)
                    {
                        let half = ((rects.get(i).map_or(ah, |r| r.3) - 1.0) / 2.0).max(1.0);
                        s.push_undo();
                        s.scope_scales.dragging = true;
                        st.set(Some((i, p, pos, half)));
                        return;
                    }
                    let Some(i) = crate::render::scopes::scope_at(&views, aw, ah, x, y) else {
                        return;
                    };
                    let (rx, _, rw, _) = rects[i];
                    let from = crate::render::scopes::time_at(&views[i], rw, x - rx);
                    s.scope_scales.drag_from = from.map(|t| (i, t));
                });
            });
            let (ses, area_d, st) = (session.clone(), scope_area.clone(), state.clone());
            plot_drag.connect_drag_update(move |_, _, dy| {
                let Some((i, p, from, half)) = st.get() else { return };
                guarded("plot drag", (), || {
                    let steps = f64::from(crate::scope::V_POSITION_STEPS);
                    let pos = from - (dy * steps / (2.0 * half)).round() as i32;
                    ses.borrow_mut().set_plot_position(i, p, pos);
                });
                area_d.queue_draw();
            });
            let (ses, area_d) = (session.clone(), scope_area.clone());
            plot_drag.connect_drag_end(move |_, _, _| {
                guarded("plot drag end", (), || {
                    let _ = crate::app::session::try_with_session(&ses, |s| {
                        s.scope_scales.drag_from = None;
                        if state.take().is_some() {
                            s.scope_scales.dragging = false;
                        }
                    });
                });
                area_d.queue_draw();
            });
        }
        scope_area.add_controller(plot_drag);
        // the bar of a subcircuit model being edited: its name, back, save and save a copy
        let context_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        context_bar.set_margin_start(8);
        context_bar.set_margin_end(8);
        context_bar.set_margin_top(4);
        context_bar.set_margin_bottom(4);
        context_bar.set_visible(false);
        let context_save: [gtk::Button; 2];
        let context_label = gtk::Label::new(None);
        context_label.set_hexpand(true);
        context_label.set_xalign(0.0);
        context_bar.append(&context_label);
        {
            let back = gtk::Button::with_label(&format!("\u{25c0} {}", locale::ls("Back")));
            let (ses, tx) = (session.clone(), sender.input_sender().clone());
            back.connect_clicked(move |_| {
                guarded("leave model", (), || {
                    let mut s = ses.borrow_mut();
                    if s.inspecting() {
                        s.leave_components();
                    } else {
                        s.leave_context();
                    }
                });
                let _ = tx.send(Msg::Refresh);
            });
            context_bar.append(&back);
            let save = gtk::Button::with_label(&locale::ls("Save"));
            save.add_css_class("suggested-action");
            let (ses, tx) = (session.clone(), sender.input_sender().clone());
            save.connect_clicked(move |_| {
                let r = guarded("save model", Err("failed".to_string()), || {
                    ses.borrow_mut().save_context(None)
                });
                if let Err(e) = r {
                    let _ = tx.send(Msg::Notice(locale::ls(&e)));
                }
                let _ = tx.send(Msg::Refresh);
            });
            context_bar.append(&save);
            let copy = gtk::Button::with_label(&locale::ls("Save Copy"));
            let (ses, tx, win) = (session.clone(), sender.input_sender().clone(), root.clone());
            copy.connect_clicked(move |_| {
                let (ses, tx) = (ses.clone(), tx.clone());
                dialogs::edit_element(
                    &win,
                    &locale::ls("Save Copy"),
                    vec![EditInfo::text("Model Name", "")],
                    move |f| {
                        let name = f[0].text.clone().unwrap_or_default();
                        let r = guarded("save model copy", Err("failed".to_string()), || {
                            ses.borrow_mut().save_context(Some(&name))
                        });
                        if let Err(e) = r {
                            let _ = tx.send(Msg::Notice(locale::ls(&e)));
                        }
                        let _ = tx.send(Msg::Refresh);
                    },
                );
            });
            context_bar.append(&copy);
            context_save = [save, copy];
        }
        let canvas_column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        canvas_column.set_hexpand(true);
        canvas_column.append(&context_bar);
        canvas_column.append(&area);
        canvas_column.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        canvas_column.append(&scope_area);
        // the panel starts 350 px wide and can be resized by dragging the divider
        let body = gtk::Paned::new(gtk::Orientation::Horizontal);
        body.set_start_child(Some(&canvas_column));
        body.set_end_child(Some(&panel_scroll));
        body.set_resize_start_child(true);
        body.set_resize_end_child(false);
        body.set_shrink_start_child(false);
        body.set_shrink_end_child(false);
        body.set_vexpand(true);
        {
            // wait for a real width: the first notifications come before the window is laid out
            let placed = std::cell::Cell::new(false);
            body.connect_notify_local(Some("width"), move |paned, _| {
                let width = paned.width();
                if !placed.get() && width >= 700 {
                    placed.set(true);
                    paned.set_position(width - 350);
                }
            });
        }
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&banner);
        content.append(&body);
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.add_top_bar(&menu_bar);
        toolbar_view.add_top_bar(&tb.widget);
        toolbar_view.set_content(Some(&content));
        root.set_content(Some(&toolbar_view));

        // --- animation: advance the simulation once per display frame
        let tick_session = session.clone();
        let (gl_tick, cairo_tick) = (gl_area.clone(), cairo_area.clone());
        let scope_tick = scope_area.clone();
        let tick_bench = bench.clone();
        let last = Cell::new(None::<i64>);
        area.add_tick_callback(move |a, clock| {
            let now = clock.frame_time();
            let dt = last.get().map_or(Duration::ZERO, |l| {
                Duration::from_micros(u64::try_from(now - l).unwrap_or(0))
            });
            last.set(Some(now));
            let frame = guarded("frame update", None, || {
                let mut s = tick_session.borrow_mut();
                // an error stops the simulation; the message is shown by the next view update
                let began = std::time::Instant::now();
                let _ = s.update(dt);
                if let Some(b) = tick_bench.borrow_mut().as_mut() {
                    b.on_update(now, began.elapsed());
                }
                Some(s.scope_area_state())
            });
            // the widgets are changed once the session is free again: hiding the scope area
            // makes GTK send "leave" to its pointer controller, whose handler needs the session
            if let Some((height, visible)) = frame {
                if scope_tick.content_height() != height {
                    scope_tick.set_content_height(height);
                }
                scope_tick.set_visible(visible);
            }
            if let Some(b) = tick_bench.borrow_mut().as_mut() {
                let done =
                    guarded("benchmark", false, || b.on_tick(now, &mut tick_session.borrow_mut()));
                if done {
                    let backend =
                        a.visible_child_name().map_or_else(String::new, |n| n.to_string());
                    print!("{}", b.report(&backend));
                    relm4::main_application().quit();
                    return glib::ControlFlow::Break;
                }
            }
            scope_tick.queue_draw();
            if a.visible_child_name().as_deref() == Some("gl") {
                gl_tick.queue_render();
            } else {
                cairo_tick.queue_draw();
            }
            glib::ControlFlow::Continue
        });

        // --- automatic save
        let auto = session.clone();
        glib::timeout_add_seconds_local(AUTOSAVE_SECONDS, move || {
            // a failed automatic save is not worth interrupting the user for
            let _ = auto.borrow_mut().autosave();
            glib::ControlFlow::Continue
        });
        let closing = session.clone();
        root.connect_close_request(move |_| {
            super::playback::stop();
            let _ = closing.borrow_mut().autosave();
            glib::Propagation::Proceed
        });

        let model = App { session, window: root.clone(), assets, examples, registry, notice };
        let widgets = Widgets {
            sliders,
            run_button,
            banner,
            speed,
            current,
            brightness,
            circuit_label,
            toolbar: tb,
            actions,
            subcircuit_menu,
            shown_models: Vec::new(),
            shown_sliders: Vec::new(),
            audio,
            shown_audio: Vec::new(),
            context_bar,
            context_label,
            context_save,
        };
        // the actions start unchecked and enabled; show the loaded state before the first message
        menus::sync_actions(&widgets.actions, &model.session.borrow());
        tracing::debug!(ms = super::bench::since_start(), "init done");
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Msg, sender: ComponentSender<Self>) {
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.update_inner(msg, &sender);
        }));
        if panicked.is_err() {
            tracing::error!("panic while handling a message");
            self.notice = Some("Erro interno ao tratar o comando".into());
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.update_view_inner(widgets, &sender);
        }));
        if panicked.is_err() {
            tracing::error!("panic while refreshing the window");
        }
    }
}

impl App {
    fn update_inner(&mut self, msg: Msg, sender: &ComponentSender<Self>) {
        match msg {
            Msg::Command(cmd) => {
                let outcome = dispatch(&mut self.session.borrow_mut(), &cmd);
                match outcome {
                    Outcome::Done => {}
                    Outcome::Clipboard(text) => self.window.clipboard().set_text(&text),
                    Outcome::Ui(cmd) => self.handle_ui(cmd, sender),
                }
            }
            Msg::Reset => apply(&mut self.session.borrow_mut(), Action::Restart),
            Msg::ToggleRun => apply(&mut self.session.borrow_mut(), Action::ToggleRun),
            Msg::Slider(index, position) => {
                apply(&mut self.session.borrow_mut(), Action::Slider { index, position });
            }
            Msg::ElementSlider(elm, position) => {
                apply(&mut self.session.borrow_mut(), Action::ElementSlider { elm, position });
            }
            Msg::Speed(v) => apply(&mut self.session.borrow_mut(), Action::Speed(v)),
            Msg::CurrentSpeed(v) => apply(&mut self.session.borrow_mut(), Action::CurrentSpeed(v)),
            Msg::Brightness(v) => apply(&mut self.session.borrow_mut(), Action::Brightness(v)),
            Msg::Refresh => {}
            Msg::OpenPath(p) => {
                let r = self.session.borrow_mut().open_file(&p);
                self.notice = r.err();
                self.show_load_problems();
            }
            Msg::SavePath(p) => {
                let r = self.session.borrow_mut().save_to(&p);
                self.notice = r.err().map(|e| format!("{}: {e}", p.display()));
            }
            Msg::ImportText(text) => {
                let r = self.session.borrow_mut().open_text(&text, "Importado", None);
                self.notice = r.err().map(|e| e.to_string());
                self.show_load_problems();
            }
            Msg::Paste(text) => {
                let mut s = self.session.borrow_mut();
                let pasted = match text {
                    Some(t) if t.contains("<cir") || t.starts_with('$') => s.paste_from_text(&t),
                    _ => s.paste(),
                };
                if !pasted {
                    self.notice = Some("Nada para colar".into());
                }
            }
            Msg::Focus(i) => self.session.borrow_mut().focus_element(i),
            Msg::ApplyEdit(i, fields) => self.session.borrow_mut().apply_edit_fields(i, fields),
            Msg::ApplyOptions(o) => {
                let mut ses = self.session.borrow_mut();
                ses.apply_other_options(
                    o.max_time_step,
                    o.voltage_range,
                    o.adjust_time_step,
                    o.auto_dc_on_reset,
                    o.solver,
                    o.min_time_step,
                );
                ses.change_settings(|st| {
                    [
                        st.positive_color,
                        st.negative_color,
                        st.neutral_color,
                        st.selection_color,
                        st.current_color,
                    ] = o.colors;
                    st.decimals_short = o.decimals_short;
                    st.decimals_long = o.decimals_long;
                    st.developer_mode = o.developer_mode;
                    st.min_frame_rate = o.min_frame_rate;
                    st.wheel_sensitivity = o.wheel_sensitivity;
                    st.value_font_size = o.value_font_size;
                });
                if let Some(code) = &o.language {
                    ses.set_language(code);
                    drop(ses);
                    self.notice = Some(locale::ls(LANGUAGE_NOTICE));
                }
            }
            Msg::Notice(text) => self.notice = Some(text),
            Msg::DismissBanner => {
                self.notice = None;
                self.session.borrow_mut().clear_stop();
            }
        }
    }

    fn update_view_inner(&self, widgets: &mut Widgets, sender: &ComponentSender<Self>) {
        let s = self.session.borrow();
        menus::sync_actions(&widgets.actions, &s);
        let models = s.subcircuit_names();
        if models != widgets.shown_models {
            menus::refresh_subcircuit_menu(&widgets.subcircuit_menu, &models);
            widgets.shown_models = models;
        }
        widgets.run_button.set_label(if s.running { "Parar" } else { "Executar" });
        let message = self.notice.clone().or_else(|| s.stop_message.clone());
        widgets.banner.set_title(message.as_deref().unwrap_or(""));
        widgets.banner.set_revealed(message.is_some());
        widgets.toolbar.widget.set_visible(s.settings.show_toolbar);
        widgets.toolbar.mode.set_text(&s.mode_text());
        widgets.brightness.set_sensitive(s.options.show_power);
        widgets.circuit_label.set_text(&format!("Circuito Atual: {}", s.title));
        let inside = s.inspecting();
        match s.editing_model() {
            _ if inside => {
                let path = s.inspect_names().join(" \u{25b8} ");
                widgets.context_label.set_text(&format!("{} {path}", locale::ls("Subcircuit:")));
                widgets.context_bar.set_visible(true);
            }
            Some(name) => {
                widgets.context_label.set_text(&format!("{} {name}", locale::ls("Editing: ")));
                widgets.context_bar.set_visible(true);
            }
            None => widgets.context_bar.set_visible(false),
        }
        for b in &widgets.context_save {
            b.set_visible(!inside);
        }
        let outputs = s.audio_outputs();
        if widgets.shown_audio != outputs {
            while let Some(child) = widgets.audio.first_child() {
                widgets.audio.remove(&child);
            }
            for (elm, label) in &outputs {
                widgets.audio.append(&audio_row(
                    &self.window,
                    &self.session,
                    sender.input_sender(),
                    *elm,
                    label,
                ));
            }
            widgets.shown_audio = outputs;
        }
        let key = slider_key(&s);
        if widgets.shown_sliders == key {
            return;
        }
        widgets.shown_sliders = key;
        widgets.speed.set_value(f64::from(s.options.speed_bar()));
        widgets.current.set_value(f64::from(s.options.current_bar));
        widgets.brightness.set_value(f64::from(s.options.power_bar));
        while let Some(child) = widgets.sliders.first_child() {
            widgets.sliders.remove(&child);
        }
        for (i, a) in s.adjustables.iter().enumerate() {
            let value = s
                .sim
                .elements
                .get(a.elm)
                .and_then(|e| e.get_edit_info(a.edit_item))
                .map_or(0.0, |ei| ei.value);
            let pos = s.slider_positions.get(i).copied().unwrap_or(0.0);
            let (row, scale, label) = slider_row(&slider_title(&a.slider_text), pos, (0.0, 100.0));
            label.set_text(&short_unit_text(value, ""));
            let tx = sender.input_sender().clone();
            let adj = a.clone();
            scale.connect_value_changed(move |sc| {
                label.set_text(&short_unit_text(adj.position_to_value(sc.value()), ""));
                let _ = tx.send(Msg::Slider(i, sc.value()));
            });
            widgets.sliders.append(&row);
        }
        for (elm, e) in s.element_sliders() {
            let (row, scale, label) =
                slider_row(&slider_title(&e.text), f64::from(e.value), (0.0, 100.0));
            label.set_text(&format!("{}", e.value));
            let tx = sender.input_sender().clone();
            scale.connect_value_changed(move |sc| {
                let v = sc.value().round() as i32;
                label.set_text(&format!("{v}"));
                let _ = tx.send(Msg::ElementSlider(elm, v));
            });
            widgets.sliders.append(&row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_buttons_follow_the_slider_state() {
        // needs a display; without one there is nothing to check
        if gtk::init().is_err() {
            return;
        }
        let (row, scale, _) = slider_row("t", 5.0, BAR_RANGE);
        let line = row.last_child().unwrap();
        let minus = line.first_child().unwrap();
        let plus = line.last_child().unwrap();
        assert!(minus.is_sensitive() && plus.is_sensitive());
        scale.set_sensitive(false);
        assert!(!minus.is_sensitive() && !plus.is_sensitive());
        scale.set_sensitive(true);
        assert!(minus.is_sensitive() && plus.is_sensitive());
    }
}
