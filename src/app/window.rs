// SPDX-License-Identifier: GPL-2.0-or-later
//! The main window: menu bar, toolbar, circuit canvas and the side panel.

use super::commands::{Action, Cmd, Outcome, apply, dispatch};
use super::dialogs::{self, OtherOptions};
use super::edit::{Mods, Tool};
use super::examples::{ExampleItem, ExampleList, parse_setup_list};
use super::menus::{self, ActionEntry};
use super::session::Session;
use super::settings::{AppSettings, settings_path};
use super::toolbar::{self, Toolbar};
use super::worker::Worker;
use crate::elements::element::EditInfo;
use crate::elements::registry::Registry;
use crate::elements::units::{short_unit_text, time_text};
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
    std::env::var_os("PROMETHEUSCS_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"))
}

/// Loads the Portuguese translations and the few strings the original does not translate.
fn load_translations(assets: &std::path::Path) {
    let text =
        std::fs::read_to_string(assets.join("locale").join("locale_pt.txt")).unwrap_or_default();
    let mut map = locale::parse_translations(&text);
    for (k, v) in [
        ("Mode~", "Modo"),
        ("Show \u{3a9} Unit", "Mostrar Unidade \u{3a9}"),
        ("Drag", "Arrastar"),
        ("Delete", "Excluir"),
    ] {
        map.entry(k.to_string()).or_insert_with(|| v.to_string());
    }
    locale::set_translations(map);
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
        s.view = View::fit(&s.sim.elements, w, h, 2.0);
        s.fit_pending = false;
    }
    let posts = post_draw_list(&s.sim.elements);
    draw_scene(g, &mut s.sim.elements, &posts, &s.draw_ctx, s.view, (w, h));
    let cross = if s.settings.crosshair { s.pointer } else { None };
    draw_overlay(g, &s.draw_ctx, s.view, (w, h), s.rubber_band(), cross);
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
type SliderKey = Vec<(usize, usize, String)>;

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
    shown_sliders: SliderKey,
}

fn slider_row(title: &str, value: f64) -> (gtk::Box, gtk::Scale, gtk::Label) {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let name = gtk::Label::new(Some(title));
    name.set_xalign(0.0);
    name.set_hexpand(true);
    let val = gtk::Label::new(None);
    val.add_css_class("dim-label");
    head.append(&name);
    head.append(&val);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    scale.set_value(value);
    scale.set_draw_value(false);
    scale.set_hexpand(true);
    let minus = gtk::Button::from_icon_name("list-remove-symbolic");
    let plus = gtk::Button::from_icon_name("list-add-symbolic");
    for b in [&minus, &plus] {
        b.add_css_class("flat");
        b.add_css_class("circular");
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
    s.adjustables.iter().map(|a| (a.elm, a.edit_item, a.slider_text.clone())).collect()
}

fn example_title(list: &[ExampleItem], file: &str) -> Option<String> {
    list.iter().find_map(|i| match i {
        ExampleItem::Circuit { file: f, title } if f == file => Some(title.clone()),
        ExampleItem::Group { items, .. } => example_title(items, file),
        _ => None,
    })
}

impl App {
    fn load_example(&mut self, file: &str) {
        let path = self.assets.join("circuits").join(file);
        let title = example_title(&self.examples.items, file).unwrap_or_else(|| file.to_string());
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let r = self.session.borrow_mut().open_text(&text, &locale::ls(&title), None);
                self.notice = r.err().map(|e| e.to_string());
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
                dialogs::choose_save(&w, "Salvar circuito", &name, "*.xml", move |p| {
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
            Cmd::ExportImage => {
                self.export_picture(sender, "Exportar como Imagem", "circuito.png", "*.png", false)
            }
            Cmd::ExportSvg => {
                self.export_picture(sender, "Exportar como SVG", "circuito.svg", "*.svg", true)
            }
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
            }
            Cmd::Print => dialogs::print(&w, self.session.clone()),
            Cmd::Fullscreen => w.set_fullscreened(!w.is_fullscreen()),
            Cmd::About => dialogs::about(&w),
            Cmd::Shortcuts => dialogs::shortcuts(&w, &self.registry),
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
                let o = self.session.borrow().options.clone();
                let now = OtherOptions {
                    max_time_step: o.max_time_step,
                    voltage_range: o.voltage_range,
                    adjust_time_step: o.adjust_time_step,
                    auto_dc_on_reset: o.auto_dc_on_reset,
                };
                let s = sender.input_sender().clone();
                dialogs::other_options(&w, now, move |v| {
                    let _ = s.send(Msg::ApplyOptions(v));
                });
            }
            Cmd::Example(file) => self.load_example(&file),
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
            .title("prometheuscs")
            .default_width(1280)
            .default_height(800)
            .build()
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let assets = assets_dir();
        load_translations(&assets);
        let registry = Registry::standard();
        let examples = std::fs::read_to_string(assets.join("setuplist.txt"))
            .map(|t| parse_setup_list(&t))
            .unwrap_or_default();
        let settings = AppSettings::load(&settings_path());
        let dark = adw::StyleManager::default().is_dark();

        let mut session = Session::empty_with(&registry, settings.clone());
        // the solver runs on its own thread; the window only draws what it publishes
        session.attach_worker(Worker::spawn(registry.clone(), settings));
        session.settings_path = Some(settings_path());
        let startup = examples.startup.clone().unwrap_or_else(|| "lrc.txt".into());
        let mut notice = None;
        match std::fs::read_to_string(assets.join("circuits").join(&startup)) {
            Ok(text) => {
                let title = example_title(&examples.items, &startup).unwrap_or(startup.clone());
                if let Err(e) = session.open_text(&text, &locale::ls(&title), None) {
                    notice = Some(e.to_string());
                }
            }
            Err(e) => notice = Some(format!("{startup}: {e}")),
        }
        let session = Rc::new(RefCell::new(session));

        // --- canvas: a GPU canvas (GtkGLArea + femtovg) with the Cairo canvas as fallback
        let cairo_area = gtk::DrawingArea::new();
        cairo_area.set_hexpand(true);
        cairo_area.set_vexpand(true);
        let draw_session = session.clone();
        cairo_area.set_draw_func(move |_, cr, w, h| {
            guarded("drawing", (), || {
                let mut g = CairoPainter::new(cr);
                paint_frame(&mut draw_session.borrow_mut(), &mut g, f64::from(w), f64::from(h));
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
        let force_cpu = std::env::var("PROMETHEUSCS_RENDERER").is_ok_and(|v| v == "cairo");
        area.set_visible_child_name(if force_cpu { "cairo" } else { "gl" });
        let gl: GlSlot = Rc::new(RefCell::new(None));
        {
            let (gl_slot, stack) = (gl.clone(), area.clone());
            gl_area.connect_realize(move |a| {
                a.make_current();
                a.attach_buffers();
                let created = match a.error() {
                    Some(e) => Err(e.to_string()),
                    None => GlCanvas::new(),
                };
                match created {
                    Ok(mut c) => {
                        let font = c.load_font();
                        if font.is_none() {
                            tracing::warn!("no system font found; text will not be drawn");
                        }
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
            let frames = Cell::new(0_u32);
            gl_area.connect_render(move |a, _| {
                guarded("GPU drawing", (), || {
                    if let Some((c, font)) = gl_slot.borrow_mut().as_mut() {
                        let scale = a.scale_factor();
                        let (w, h) = (a.width(), a.height());
                        let px = |v: i32| u32::try_from(v * scale).unwrap_or(1).max(1);
                        c.begin(px(w), px(h), scale as f32, femtovg::Color::rgb(0, 0, 0));
                        let mut g = GlPainter::new(&mut c.canvas, *font);
                        paint_frame(&mut ses.borrow_mut(), &mut g, f64::from(w), f64::from(h));
                        c.end();
                        // a diagnostic for GPU problems: once, read back how much was drawn
                        frames.set(frames.get() + 1);
                        if frames.get() == 60 && tracing::enabled!(tracing::Level::DEBUG) {
                            let lit = c.non_black_pixels(px(w), px(h));
                            tracing::debug!(lit, total = px(w) * px(h), "GPU canvas drew pixels");
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
            drag.connect_drag_begin(move |g, x, y| {
                st.set((x, y));
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
                    let (title, fields) = {
                        let s = ses.borrow();
                        (s.element_title(i), s.edit_fields(i))
                    };
                    let tx = tx.clone();
                    dialogs::edit_element(&win, &title, fields, move |f| {
                        let _ = tx.send(Msg::ApplyEdit(i, f));
                    });
                }
            });
        }
        area.add_controller(click);
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
                        s.zoom_at(x, y, if dy < 0.0 { 1.1 } else { 1.0 / 1.1 });
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
        panel.set_width_request(270);
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
        let (speed_row, speed, speed_val) =
            slider_row("Velocidade da Simulação", f64::from(session.borrow().options.speed_bar()));
        let (current_row, current, current_val) =
            slider_row("Velocidade da Corrente", f64::from(session.borrow().options.current_bar));
        let (bright_row, brightness, bright_val) =
            slider_row("Brilho da Potência", f64::from(session.borrow().options.power_bar));
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
        let panel_scroll = gtk::ScrolledWindow::new();
        panel_scroll.set_child(Some(&panel));
        panel_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);

        // --- menus, toolbar, actions
        let title = adw::WindowTitle::new("prometheuscs", "");
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&title));
        let model = menus::menu_bar_model(&registry, &examples, &assets, dark);
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
        menus::install_accels(&relm4::main_application(), &registry);

        // --- layout
        let banner = adw::Banner::new("");
        banner.set_button_label(Some("Fechar"));
        {
            let s = sender.clone();
            banner.connect_button_clicked(move |_| s.input(Msg::DismissBanner));
        }
        // the time diagrams will go under the canvas; until they exist the place is kept
        let scope_area = gtk::Label::new(Some("Diagramas de tempo: ainda não disponíveis"));
        scope_area.add_css_class("dim-label");
        scope_area.set_height_request(96);
        let canvas_column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        canvas_column.set_hexpand(true);
        canvas_column.append(&area);
        canvas_column.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        canvas_column.append(&scope_area);
        let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        body.append(&canvas_column);
        body.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        body.append(&panel_scroll);
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
        let tick_title = title.clone();
        let (gl_tick, cairo_tick) = (gl_area.clone(), cairo_area.clone());
        let last = Cell::new(None::<i64>);
        area.add_tick_callback(move |a, clock| {
            let now = clock.frame_time();
            let dt = last.get().map_or(Duration::ZERO, |l| {
                Duration::from_micros(u64::try_from(now - l).unwrap_or(0))
            });
            last.set(Some(now));
            guarded("frame update", (), || {
                let mut s = tick_session.borrow_mut();
                // an error stops the simulation; the message is shown by the next view update
                let _ = s.update(dt);
                tick_title.set_subtitle(&format!("t = {}", time_text(s.time())));
            });
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
            shown_sliders: Vec::new(),
        };
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
            Msg::Speed(v) => apply(&mut self.session.borrow_mut(), Action::Speed(v)),
            Msg::CurrentSpeed(v) => apply(&mut self.session.borrow_mut(), Action::CurrentSpeed(v)),
            Msg::Brightness(v) => apply(&mut self.session.borrow_mut(), Action::Brightness(v)),
            Msg::Refresh => {}
            Msg::OpenPath(p) => {
                let r = self.session.borrow_mut().open_file(&p);
                self.notice = r.err();
            }
            Msg::SavePath(p) => {
                let r = self.session.borrow_mut().save_to(&p);
                self.notice = r.err().map(|e| format!("{}: {e}", p.display()));
            }
            Msg::ImportText(text) => {
                let r = self.session.borrow_mut().open_text(&text, "Importado", None);
                self.notice = r.err().map(|e| e.to_string());
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
            Msg::ApplyOptions(o) => self.session.borrow_mut().apply_other_options(
                o.max_time_step,
                o.voltage_range,
                o.adjust_time_step,
                o.auto_dc_on_reset,
            ),
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
        widgets.run_button.set_label(if s.running { "Parar" } else { "Executar" });
        let message = self.notice.clone().or_else(|| s.stop_message.clone());
        widgets.banner.set_title(message.as_deref().unwrap_or(""));
        widgets.banner.set_revealed(message.is_some());
        widgets.toolbar.widget.set_visible(s.settings.show_toolbar);
        widgets.toolbar.mode.set_text(&s.mode_text());
        widgets.brightness.set_sensitive(s.options.show_power);
        widgets.circuit_label.set_text(&format!("Circuito Atual: {}", s.title));
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
            let (row, scale, label) = slider_row(&slider_title(&a.slider_text), pos);
            label.set_text(&short_unit_text(value, ""));
            let tx = sender.input_sender().clone();
            let adj = a.clone();
            scale.connect_value_changed(move |sc| {
                label.set_text(&short_unit_text(adj.position_to_value(sc.value()), ""));
                let _ = tx.send(Msg::Slider(i, sc.value()));
            });
            widgets.sliders.append(&row);
        }
    }
}
