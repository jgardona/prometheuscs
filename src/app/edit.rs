// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (MouseManager, CommandManager).
//! Editing a circuit: tools, mouse handling, selection, clipboard, undo, transforms and view.
//!
//! Everything here works on the [`Session`] without GTK, so the editing rules are unit tested.

use super::session::Session;
use crate::elements::element::{EditInfo, Element};
use crate::elements::geometry::{Rect, distance_sq};
use crate::elements::model_edit::ModelEditor;
use crate::format::parser::parse;
use crate::format::writer::write_elements_xml;
use crate::locale::ls;

/// Longest undo history kept.
const MAX_UNDO: usize = 100;
/// Squared distance in circuit pixels within which the mouse grabs a post.
const POST_GRAB_SQ: i32 = 100;
/// Squared distance in circuit pixels within which the mouse counts as over an element.
const HOVER_DISTANCE_SQ: i32 = 64;
/// Zoom limits.
const MIN_SCALE: f64 = 0.1;
const MAX_SCALE: f64 = 10.0;

/// What a mouse press on the canvas does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Select, move and operate elements (the edit mode).
    Select,
    /// Drag out a new element of the given registry id.
    Place(String),
    /// Move every element.
    DragAll,
    /// Move the posts on one row vertically.
    DragRow,
    /// Move the posts on one column horizontally.
    DragColumn,
    /// Move the selected elements.
    DragSelected,
    /// Move all posts at one grid point.
    DragPost,
}

/// Modifier keys held during a mouse event.
#[derive(Clone, Copy, Debug, Default)]
pub struct Mods {
    /// Shift is held: pan instead of edit.
    pub shift: bool,
    /// Control is held: add to the selection.
    pub ctrl: bool,
}

/// How many fields of the scope properties dialog come after the measurements: two checkboxes,
/// six plot choices and the trail persistence.
const XY_FIELDS: usize = 9;

/// A scope's samples as a CSV file: the time, then the smallest and largest value of each plot.
fn csv_of(view: &crate::scope::ScopeView) -> Option<String> {
    use crate::format::number::js_number;
    let first = view.plots.first()?;
    let unit = |u: i32| match u {
        crate::scope::UNITS_V => "V",
        crate::scope::UNITS_A => "A",
        crate::scope::UNITS_W => "W",
        crate::scope::UNITS_OHMS => "ohm",
        crate::scope::UNITS_C => "C",
        _ => "",
    };
    let mut out = String::from("time");
    for p in &view.plots {
        for edge in ["min", "max"] {
            out += &format!(",\"{} {} {edge}\"", p.title, unit(p.units));
        }
    }
    out.push('\n');
    let n = first.min.len();
    for i in 0..n {
        let t = view.time - (n - 1 - i) as f64 * view.time_per_sample;
        if t < 0.0 {
            continue;
        }
        out += &js_number(t);
        for p in &view.plots {
            out += &format!(",{},{}", js_number(p.min[i]), js_number(p.max[i]));
        }
        out.push('\n');
    }
    Some(out)
}

/// The fields of the properties dialog of `sc` (see [`Session::scope_properties`]).
fn scope_fields(sc: &crate::scope::Scope, elements: &[Box<dyn Element>]) -> Vec<EditInfo> {
    use crate::scope::{FLAG_LOG_SPECTRUM, FLAG_PHASE_ANGLE, TRIGGER_EDGE_FALLING};
    let titles: Vec<String> = sc
        .view(elements, 0.0, 0.0)
        .plots
        .iter()
        .map(|p| format!("{} {}", p.title, crate::render::scopes::plot_label(p)))
        .collect();
    let mut f = vec![
        EditInfo::checkbox("Manual Scale", sc.is_manual()),
        EditInfo::checkbox("Max Scale", sc.flags & crate::scope::FLAG_MAX_SCALE != 0),
        EditInfo::number("# of Divisions", f64::from(sc.man_divisions)).set_dimensionless(),
        EditInfo::number("Time Steps per Sample", f64::from(sc.speed)).set_dimensionless(),
        EditInfo::choice(
            "Trigger Mode",
            vec!["Free Run".into(), "Normal".into(), "Auto".into()],
            sc.trigger.mode.clamp(0, 2) as usize,
        ),
        EditInfo::choice(
            "Trigger Edge",
            vec!["Rising".into(), "Falling".into()],
            usize::from(sc.trigger.edge == TRIGGER_EDGE_FALLING),
        ),
        EditInfo::number("Trigger Level", sc.trigger.level),
    ];
    for (i, p) in sc.plots.iter().enumerate() {
        let name = titles.get(i).cloned().unwrap_or_default();
        let scale = if p.man_set { p.man_scale } else { sc.man_scale_from_max(p.units, true) };
        let div = f64::from(sc.man_divisions.max(1));
        let pos = f64::from(p.man_pos) * div / f64::from(crate::scope::V_POSITION_STEPS);
        f.push(
            EditInfo::number(&format!("{}: {name}", ls("Scale per Division")), scale)
                .set_positive(),
        );
        f.push(EditInfo::number(&format!("{}: {name}", ls("Position (divisions)")), pos));
        if p.can_ac_couple() {
            f.push(EditInfo::checkbox(&format!("{}: {name}", ls("AC Coupled")), p.ac_coupled));
        }
    }
    for (label, bit, inverted) in INFO_FLAGS {
        f.push(EditInfo::checkbox(label, (sc.flags & bit != 0) != inverted));
    }
    f.push(EditInfo::checkbox("Show Phase Angle", sc.flags & FLAG_PHASE_ANGLE != 0));
    f.push(EditInfo::checkbox("Log Spectrum", sc.flags & FLAG_LOG_SPECTRUM != 0));
    // the axes only matter to an X/Y plot, so only that one is offered them
    if !(sc.xy && sc.plot_xy) {
        return f;
    }
    // which plots are the axes of an X/Y plot, and which ones modulate its trace
    let axes = sc.axes;
    let count = sc.plots.len();
    let named = |label: &str, current: i32| {
        EditInfo::choice(
            label,
            (0..count).map(|i| titles.get(i).cloned().unwrap_or_default()).collect(),
            crate::scope::XyAxes::valid(current, 0, count),
        )
    };
    let optional = |label: &str, current: i32| {
        let mut names = vec![ls("None")];
        names.extend((0..count).map(|i| titles.get(i).cloned().unwrap_or_default()));
        let at = crate::scope::XyAxes::modulator(current, count).map_or(0, |i| i + 1);
        EditInfo::choice(label, names, at)
    };
    f.push(named("X Axis", axes.x));
    f.push(named("Y Axis", axes.y));
    f.push(optional("Brightness", axes.brightness));
    f.push(optional("Red", axes.red));
    f.push(optional("Green", axes.green));
    f.push(optional("Blue", axes.blue));
    f.push(
        EditInfo::number("Trail Persistence (time steps)", f64::from(axes.trail))
            .set_dimensionless(),
    );
    f
}

/// Checks the fields of the properties dialog of `sc`; the first invalid one is reported.
fn check_scope_fields(sc: &crate::scope::Scope, fields: &[EditInfo]) -> Result<(), String> {
    let per_plot: usize = sc.plots.iter().map(|p| 2 + usize::from(p.can_ac_couple())).sum();
    if fields.len() < 7 + per_plot + INFO_FLAGS.len() + 2 {
        return Err("missing fields".into());
    }
    let divisions = fields[2].value;
    if divisions.fract() != 0.0 || !(2.0..=20.0).contains(&divisions) {
        return Err("# of Divisions: must be a whole number from 2 to 20".into());
    }
    let speed = fields[3].value;
    if speed.fract() != 0.0 || !(1.0..=1024.0).contains(&speed) {
        return Err("Time Steps per Sample: must be a whole number from 1 to 1024".into());
    }
    let mut at = 7;
    for p in &sc.plots {
        if fields[at].value <= 0.0 {
            return Err("Scale per Division: must be > 0".into());
        }
        at += 2 + usize::from(p.can_ac_couple());
    }
    Ok(())
}

/// Puts the fields of the properties dialog (already checked) into `sc`.
fn apply_scope_fields(sc: &mut crate::scope::Scope, fields: &[EditInfo]) {
    use crate::scope::{
        FLAG_LOG_SPECTRUM, FLAG_MAX_SCALE, FLAG_PHASE_ANGLE, TRIGGER_EDGE_FALLING,
        TRIGGER_EDGE_RISING,
    };
    let divisions = fields[2].value;
    sc.speed = fields[3].value as i32;
    sc.man_divisions = divisions as i32;
    if fields[1].checkbox.unwrap_or(false) {
        sc.flags |= FLAG_MAX_SCALE;
    } else {
        sc.flags &= !FLAG_MAX_SCALE;
    }
    sc.trigger.mode = fields[4].choice_index.min(2) as i32;
    sc.trigger.edge =
        if fields[5].choice_index == 1 { TRIGGER_EDGE_FALLING } else { TRIGGER_EDGE_RISING };
    sc.trigger.level = fields[6].value;
    let steps = f64::from(crate::scope::V_POSITION_STEPS);
    let mut at = 7;
    for p in &mut sc.plots {
        p.man_scale = fields[at].value;
        p.man_pos = (fields[at + 1].value * steps / divisions).round().clamp(-steps, steps) as i32;
        p.man_set = true;
        at += 2;
        if p.can_ac_couple() {
            p.ac_coupled = fields[at].checkbox.unwrap_or(false);
            at += 1;
        }
    }
    for (k, (_, bit, inverted)) in INFO_FLAGS.iter().enumerate() {
        let on = fields[at + k].checkbox.unwrap_or(false) != *inverted;
        if on {
            sc.flags |= bit;
        } else {
            sc.flags &= !bit;
        }
    }
    let at = at + INFO_FLAGS.len();
    for (k, bit) in [FLAG_PHASE_ANGLE, FLAG_LOG_SPECTRUM].into_iter().enumerate() {
        if fields[at + k].checkbox.unwrap_or(false) {
            sc.flags |= bit;
        } else {
            sc.flags &= !bit;
        }
    }
    // the axes are in the dialog of an X/Y plot only
    if fields.len() >= at + XY_FIELDS {
        let pick = |k: usize| fields[at + 2 + k].choice_index as i32;
        sc.axes = crate::scope::XyAxes {
            x: pick(0),
            y: pick(1),
            brightness: pick(2) - 1,
            red: pick(3) - 1,
            green: pick(4) - 1,
            blue: pick(5) - 1,
            trail: fields[at + 8].value.round().clamp(0.0, 1e9) as i32,
        };
    }
    let manual = fields[0].checkbox.unwrap_or(false);
    if manual {
        sc.flags |= crate::scope::FLAG_MAN_SCALE;
    } else {
        sc.flags &= !crate::scope::FLAG_MAN_SCALE;
    }
}

pub use crate::scope::ScopeMode;

/// The measurements a scope can show: the label, the flag bit, and whether the bit being set
/// means "do not show".
const INFO_FLAGS: [(&str, i32, bool); 9] = [
    ("Show Peak Value", crate::scope::FLAG_NO_MAX, true),
    ("Show Negative Peak Value", crate::scope::FLAG_MIN, false),
    ("Show Peak to Peak", crate::scope::FLAG_P2P, false),
    ("Show Frequency", crate::scope::FLAG_FREQ, false),
    ("Show RMS Average", crate::scope::FLAG_RMS, false),
    ("Show Average", crate::scope::FLAG_AVERAGE, false),
    ("Show Duty Cycle", crate::scope::FLAG_DUTY, false),
    ("Show Scale", crate::scope::FLAG_SCALE, false),
    ("Show Element Info", crate::scope::FLAG_ELM_INFO, false),
];

/// One field of the Add Sliders dialog: whether it has a slider and how the slider is set.
#[derive(Clone, Debug, PartialEq)]
pub struct SliderEntry {
    /// Index of the field in the element's edit list.
    pub item: usize,
    /// Name of the field.
    pub name: String,
    /// The field has a slider.
    pub enabled: bool,
    /// Value at the left end.
    pub min: f64,
    /// Value at the right end.
    pub max: f64,
    /// Step of the slider (0 for continuous).
    pub step: f64,
    /// Logarithmic slider.
    pub logarithmic: bool,
    /// Label above the slider.
    pub label: String,
}

/// Which elements a move gesture affects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoveScope {
    All,
    Selected,
}

/// The mouse gesture in progress.
pub(crate) enum Drag {
    None,
    Pan { px: f64, py: f64 },
    Place { idx: usize },
    Move { gx: i32, gy: i32, scope: MoveScope, pushed: bool },
    Post { idx: usize, first: bool, pushed: bool },
    Row { gy: i32, pushed: bool },
    Column { gx: i32, pushed: bool },
    PostsAt { gx: i32, gy: i32, pushed: bool },
    Rubber { x0: i32, y0: i32, x1: i32, y1: i32, additive: bool },
}

/// Where pasted elements go.
enum PasteAt {
    Point(i32, i32),
    Offset(i32, i32),
}

/// An Options-menu switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    /// Current dots.
    ShowCurrent,
    /// Voltage colours.
    ShowVoltage,
    /// Power colours.
    ShowPower,
    /// Component values.
    ShowValues,
    /// Small grid.
    SmallGrid,
    /// Toolbar.
    Toolbar,
    /// Cursor crosshair.
    Crosshair,
    /// European resistor symbol.
    EuroResistors,
    /// Ohm symbol in values.
    ShowOhm,
    /// IEC gate symbols.
    IecGates,
    /// White background.
    WhiteBackground,
    /// Conventional current direction.
    ConventionalCurrent,
    /// Disable editing.
    DisableEditing,
    /// Edit values with the mouse wheel.
    MouseWheelEdit,
}

fn move_element(e: &mut dyn Element, dx: i32, dy: i32) {
    e.moved(dx, dy);
    e.base_mut().move_by(dx, dy);
    e.set_points();
}

fn move_post(e: &mut dyn Element, first: bool, dx: i32, dy: i32) {
    let b = e.base_mut();
    if first {
        b.x += dx;
        b.y += dy;
    } else {
        b.x2 += dx;
        b.y2 += dy;
    }
    b.init_bounding_box();
    e.set_points();
}

fn drag_second_post(e: &mut dyn Element, x: i32, y: i32) {
    e.drag(x, y);
}

impl Session {
    /// Grid spacing in circuit pixels.
    pub fn grid_size(&self) -> i32 {
        if self.options.small_grid { 8 } else { 16 }
    }

    /// Rounds a coordinate to the grid.
    pub fn snap(&self, v: i32) -> i32 {
        let g = self.grid_size();
        (v + g / 2 - 1) & !(g - 1)
    }

    /// True unless editing is disabled in the options.
    pub fn editing_allowed(&self) -> bool {
        !self.settings.disable_editing && self.inspect.is_empty()
    }

    /// The toolbar's mode text, for example `Modo: Modo Edição`.
    pub fn mode_text(&self) -> String {
        let name = match &self.tool {
            Tool::Select => ls("Select/Drag Sel"),
            Tool::Place(id) => {
                let label = self.registry.get(id).map_or(id.as_str(), |d| d.display_name);
                ls(label)
            }
            Tool::DragAll => ls("Drag All"),
            Tool::DragRow => ls("Drag Row"),
            Tool::DragColumn => ls("Drag Column"),
            Tool::DragSelected => ls("Drag Selected"),
            Tool::DragPost => ls("Drag Post"),
        };
        format!("{}: {}", ls("Mode~"), name)
    }

    /// Chooses what mouse presses do. Placing deselects everything.
    pub fn set_tool(&mut self, tool: Tool) {
        if matches!(tool, Tool::Place(_)) {
            self.deselect_all();
        }
        self.tool = tool;
        self.drag = Drag::None;
    }

    // ---- hit testing -----------------------------------------------------------------------

    fn element_at(&self, gx: i32, gy: i32) -> Option<usize> {
        let mut best: Option<(usize, i32)> = None;
        for (i, e) in self.sim.elements.iter().enumerate() {
            let d = match e.click_rect() {
                Some(r) if r.contains(gx, gy) => 0,
                _ => {
                    let b = &e.base().bbox;
                    let near = Rect::new(b.x - 8, b.y - 8, b.width + 16, b.height + 16);
                    if !near.contains(gx, gy) {
                        continue;
                    }
                    let d = e.mouse_distance(gx, gy);
                    // inside its bounding box an element is hit wherever the pointer is, as in
                    // the original: the generic distance is to a line (or to the centre) and
                    // only ranks overlapping elements. The few elements with a rule of their
                    // own (wires, boxes) answer -1 when the pointer is not on them. Just
                    // outside the box the pointer must still be close. A negative distance
                    // means the element is not under the mouse.
                    let limit = if b.contains(gx, gy) { i32::MAX } else { HOVER_DISTANCE_SQ };
                    if !(0..=limit).contains(&d) {
                        continue;
                    }
                    d + 1
                }
            };
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Updates the highlighted element and the crosshair for the mouse at surface position
    /// (`px`, `py`), or when the mouse left the canvas.
    pub fn hover_at(&mut self, pos: Option<(f64, f64)>) {
        // what is shown inside an instance is not the circuit the pointer would find
        let pos = pos.filter(|_| self.inspect.is_empty());
        self.pointer = pos.map(|(px, py)| {
            let (gx, gy) = self.view.to_circuit(px, py);
            (self.snap(gx), self.snap(gy))
        });
        let found = pos.and_then(|(px, py)| {
            let (gx, gy) = self.view.to_circuit(px, py);
            self.element_at(gx, gy)
        });
        if found == self.hover {
            return;
        }
        if let Some(e) = self
            .hover
            .filter(|i| Some(*i) != self.stop_element)
            .and_then(|i| self.sim.elements.get_mut(i))
        {
            e.base_mut().highlight = false;
        }
        if let Some(e) = found.and_then(|i| self.sim.elements.get_mut(i)) {
            e.base_mut().highlight = true;
        }
        self.hover = found;
    }

    // ---- switches --------------------------------------------------------------------------

    fn toggle_linked(&mut self, i: usize) {
        let label = self.sim.elements.get(i).and_then(|e| e.link_label().map(str::to_string));
        if let Some(e) = self.sim.elements.get_mut(i) {
            e.simple_toggle();
        }
        if let Some(label) = label {
            for (j, e) in self.sim.elements.iter_mut().enumerate() {
                if j != i && e.link_label() == Some(label.as_str()) {
                    e.simple_toggle();
                }
            }
        }
        self.sync_link_group(i);
    }

    /// Moves every multi-throw switch of the same group to the position of switch `i`,
    /// mirrored when one of the two is flipped.
    fn sync_link_group(&mut self, i: usize) {
        let Some(group) = self.sim.elements.get(i).map(|e| e.link_group()).filter(|g| *g != 0)
        else {
            return;
        };
        let Some((position, _, flipped)) = self.sim.elements[i].switch_state() else { return };
        for (j, e) in self.sim.elements.iter_mut().enumerate() {
            if j == i || e.link_group() != group {
                continue;
            }
            if let Some((_, count, other_flipped)) = e.switch_state() {
                let pos = if other_flipped != flipped { count - 1 - position } else { position };
                if pos < count {
                    e.set_switch_position(pos);
                }
            }
        }
    }

    fn press_switch(&mut self, gx: i32, gy: i32) -> bool {
        let Some(i) = self.element_at(gx, gy) else { return false };
        let hit = self.sim.elements[i].click_rect().is_some_and(|r| r.contains(gx, gy));
        if !hit {
            return false;
        }
        // keep the live state (capacitor voltages, ...) across the restart below
        self.pull_state();
        self.toggle_linked(i);
        if self.sim.elements[i].is_momentary() {
            self.held = Some(i);
        }
        self.reanalyze();
        true
    }

    /// Operates a switch under surface position (`px`, `py`). Returns true when one was hit.
    pub fn press_at(&mut self, px: f64, py: f64) -> bool {
        let (gx, gy) = self.view.to_circuit(px, py);
        self.press_switch(gx, gy)
    }

    fn release_switch(&mut self) {
        if let Some(i) = self.held.take() {
            self.toggle_linked(i);
            self.reanalyze();
        }
    }

    // ---- mouse gestures --------------------------------------------------------------------

    /// A mouse button was pressed at surface position (`px`, `py`). Returns true when the
    /// circuit or the view changed.
    pub fn mouse_down(&mut self, px: f64, py: f64, mods: Mods, button: u32) -> bool {
        // looking inside an instance: the view can be moved, nothing else
        if !self.inspect.is_empty() && !(mods.shift || button == 2) {
            return false;
        }
        if button == 3 {
            let changed = self.tool != Tool::Select || !matches!(self.drag, Drag::None);
            self.set_tool(Tool::Select);
            return changed;
        }
        if mods.shift || button == 2 {
            self.drag = Drag::Pan { px, py };
            return false;
        }
        let (gx, gy) = self.view.to_circuit(px, py);
        let (sx, sy) = (self.snap(gx), self.snap(gy));
        if let Tool::Place(id) = self.tool.clone() {
            if !self.editing_allowed() {
                return false;
            }
            let Some(mut e) = self.registry.create(&id, sx, sy) else { return false };
            self.push_undo();
            e.base_mut().creating = true;
            e.set_points();
            self.sim.elements.push(e);
            self.drag = Drag::Place { idx: self.sim.elements.len() - 1 };
            return true;
        }
        if self.press_switch(gx, gy) {
            return true;
        }
        if !self.editing_allowed() {
            return false;
        }
        match self.tool {
            Tool::DragAll => {
                self.drag = Drag::Move { gx: sx, gy: sy, scope: MoveScope::All, pushed: false }
            }
            Tool::DragSelected => {
                self.drag = Drag::Move { gx: sx, gy: sy, scope: MoveScope::Selected, pushed: false }
            }
            Tool::DragRow => self.drag = Drag::Row { gy: sy, pushed: false },
            Tool::DragColumn => self.drag = Drag::Column { gx: sx, pushed: false },
            Tool::DragPost => self.drag = Drag::PostsAt { gx: sx, gy: sy, pushed: false },
            _ => return self.select_press(gx, gy, sx, sy, mods),
        }
        false
    }

    fn select_press(&mut self, gx: i32, gy: i32, sx: i32, sy: i32, mods: Mods) -> bool {
        match self.element_at(gx, gy) {
            Some(i) => {
                if mods.ctrl {
                    let b = self.sim.elements[i].base_mut();
                    b.selected = !b.selected;
                } else if !self.sim.elements[i].base().selected {
                    self.deselect_all();
                    self.sim.elements[i].base_mut().selected = true;
                }
                let sole = self.selected_indices() == vec![i];
                let grab = if sole {
                    let e = &self.sim.elements[i];
                    (0..e.num_handles().min(2)).find(|&n| {
                        e.handle_point(n)
                            .is_some_and(|p| distance_sq(gx, gy, p.x, p.y) <= POST_GRAB_SQ)
                    })
                } else {
                    None
                };
                self.drag = match grab {
                    Some(n) => Drag::Post { idx: i, first: n == 0, pushed: false },
                    None => {
                        Drag::Move { gx: sx, gy: sy, scope: MoveScope::Selected, pushed: false }
                    }
                };
                true
            }
            None => {
                let changed = !mods.ctrl && self.has_selection();
                if !mods.ctrl {
                    self.deselect_all();
                }
                self.drag = Drag::Rubber { x0: gx, y0: gy, x1: gx, y1: gy, additive: mods.ctrl };
                changed
            }
        }
    }

    /// The mouse moved with a button held. Returns true when something changed.
    pub fn mouse_drag(&mut self, px: f64, py: f64) -> bool {
        let (gx, gy) = self.view.to_circuit(px, py);
        let (sx, sy) = (self.snap(gx), self.snap(gy));
        let mut drag = std::mem::replace(&mut self.drag, Drag::None);
        let mut changed = false;
        match &mut drag {
            Drag::None => {}
            Drag::Pan { px: ox, py: oy } => {
                self.view.x += px - *ox;
                self.view.y += py - *oy;
                *ox = px;
                *oy = py;
                changed = true;
            }
            Drag::Place { idx } => {
                if let Some(e) = self.sim.elements.get_mut(*idx) {
                    drag_second_post(e.as_mut(), sx, sy);
                    changed = true;
                }
            }
            Drag::Move { gx: lx, gy: ly, scope, pushed } => {
                let (dx, dy) = (sx - *lx, sy - *ly);
                if dx != 0 || dy != 0 {
                    if !*pushed {
                        self.push_undo();
                        *pushed = true;
                    }
                    for e in &mut self.sim.elements {
                        if *scope == MoveScope::All || e.base().selected {
                            move_element(e.as_mut(), dx, dy);
                        }
                    }
                    (*lx, *ly) = (sx, sy);
                    changed = true;
                }
            }
            Drag::Post { idx, first, pushed } => {
                if !*pushed {
                    self.push_undo();
                    *pushed = true;
                }
                if let Some(e) = self.sim.elements.get_mut(*idx) {
                    if *first {
                        let (dx, dy) = (sx - e.base().x, sy - e.base().y);
                        move_post(e.as_mut(), true, dx, dy);
                    } else {
                        drag_second_post(e.as_mut(), sx, sy);
                    }
                    changed = true;
                }
            }
            Drag::Row { gy: row, pushed } => {
                let dy = sy - *row;
                if dy != 0 {
                    if !*pushed {
                        self.push_undo();
                        *pushed = true;
                    }
                    for e in &mut self.sim.elements {
                        if e.base().y == *row {
                            move_post(e.as_mut(), true, 0, dy);
                        }
                        if e.base().y2 == *row {
                            move_post(e.as_mut(), false, 0, dy);
                        }
                    }
                    *row = sy;
                    changed = true;
                }
            }
            Drag::Column { gx: col, pushed } => {
                let dx = sx - *col;
                if dx != 0 {
                    if !*pushed {
                        self.push_undo();
                        *pushed = true;
                    }
                    for e in &mut self.sim.elements {
                        if e.base().x == *col {
                            move_post(e.as_mut(), true, dx, 0);
                        }
                        if e.base().x2 == *col {
                            move_post(e.as_mut(), false, dx, 0);
                        }
                    }
                    *col = sx;
                    changed = true;
                }
            }
            Drag::PostsAt { gx: lx, gy: ly, pushed } => {
                let (dx, dy) = (sx - *lx, sy - *ly);
                if dx != 0 || dy != 0 {
                    if !*pushed {
                        self.push_undo();
                        *pushed = true;
                    }
                    for e in &mut self.sim.elements {
                        if (e.base().x, e.base().y) == (*lx, *ly) {
                            move_post(e.as_mut(), true, dx, dy);
                        }
                        if (e.base().x2, e.base().y2) == (*lx, *ly) {
                            move_post(e.as_mut(), false, dx, dy);
                        }
                    }
                    (*lx, *ly) = (sx, sy);
                    changed = true;
                }
            }
            Drag::Rubber { x1, y1, .. } => {
                (*x1, *y1) = (gx, gy);
                changed = true;
            }
        }
        self.drag = drag;
        changed
    }

    /// The mouse button was released. Returns true when the circuit changed and was analysed
    /// again.
    pub fn mouse_up(&mut self) -> bool {
        self.release_switch();
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        match drag {
            Drag::Place { idx } => {
                let failed = self.sim.elements.get(idx).is_none_or(|e| e.creation_failed());
                if failed {
                    if idx < self.sim.elements.len() {
                        self.sim.elements.remove(idx);
                    }
                    self.undo.pop();
                    return false;
                }
                if let Some(e) = self.sim.elements.get_mut(idx) {
                    e.base_mut().creating = false;
                }
                self.dirty = true;
                self.reanalyze();
                true
            }
            Drag::Move { pushed: true, .. }
            | Drag::Post { pushed: true, .. }
            | Drag::Row { pushed: true, .. }
            | Drag::Column { pushed: true, .. }
            | Drag::PostsAt { pushed: true, .. } => {
                self.remove_zero_length();
                self.dirty = true;
                self.reanalyze();
                true
            }
            Drag::Rubber { x0, y0, x1, y1, additive } => {
                let r = Rect::new(x0.min(x1), y0.min(y1), (x1 - x0).abs() + 1, (y1 - y0).abs() + 1);
                for e in &mut self.sim.elements {
                    if e.selected_by_rect(&r) {
                        e.base_mut().selected = true;
                    } else if !additive {
                        e.base_mut().selected = false;
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// The selection rectangle being dragged, in circuit coordinates, for drawing.
    pub fn rubber_band(&self) -> Option<Rect> {
        match self.drag {
            Drag::Rubber { x0, y0, x1, y1, .. } => {
                Some(Rect::new(x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()))
            }
            _ => None,
        }
    }

    fn remove_zero_length(&mut self) {
        let zero: Vec<usize> = self
            .sim
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.creation_failed())
            .map(|(i, _)| i)
            .collect();
        self.remove_elements(&zero);
    }

    // ---- selection and deletion ------------------------------------------------------------

    /// Indices of the selected elements.
    pub fn selected_indices(&self) -> Vec<usize> {
        self.sim
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.base().selected)
            .map(|(i, _)| i)
            .collect()
    }

    /// Elements a scope can be opened for: the selection, or else the element under the mouse.
    pub fn scope_targets(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self
            .selected_indices()
            .into_iter()
            .filter(|&i| self.sim.elements[i].can_view_in_scope())
            .collect();
        if v.is_empty() {
            v.extend(self.hover.filter(|&i| self.sim.elements[i].can_view_in_scope()));
        }
        v
    }

    /// Opens a scope for every element of [`Session::scope_targets`]; returns how many.
    pub fn view_in_scope(&mut self) -> usize {
        let targets = self.scope_targets();
        if targets.is_empty() {
            return 0;
        }
        self.push_undo();
        for i in &targets {
            let position = self.sim.scopes.len() as i32;
            if let Some(sc) = crate::scope::Scope::for_element(*i, &self.sim.elements, position) {
                self.sim.scopes.push(sc);
            }
        }
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
        targets.len()
    }

    /// The values scope `idx` can plot, with whether each is plotted now.
    pub fn scope_menu(&self, idx: usize) -> Vec<(i32, &'static str, bool)> {
        self.sim.scopes.get(idx).map(|s| s.menu_items(&self.sim.elements)).unwrap_or_default()
    }

    /// Starts or stops plotting `value` in scope `idx`.
    pub fn toggle_scope_value(&mut self, idx: usize, value: i32, on: bool) {
        if idx >= self.sim.scopes.len() {
            return;
        }
        self.push_undo();
        let elements = std::mem::take(&mut self.sim.elements);
        self.sim.scopes[idx].set_value(value, on, &elements);
        self.sim.elements = elements;
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
    }

    /// The pointer is over scope `idx` (or over none): the elements it plots are highlighted,
    /// and in an X/Y plot each is marked with the axis it is on.
    pub fn set_scope_hover(&mut self, idx: Option<usize>) {
        if self.scope_hover == idx {
            return;
        }
        self.scope_hover = idx;
        self.apply_scope_roles();
    }

    /// The elements a scope plots, each with the part it plays: "X", "Y", "Br", "R", "G" or
    /// "B" in an X/Y plot, empty otherwise.
    fn plot_roles(sc: &crate::scope::Scope) -> Vec<(usize, String)> {
        let n = sc.plots.len();
        let mut roles: Vec<(usize, &str)> = Vec::new();
        if sc.xy && sc.plot_xy {
            let a = sc.axes;
            roles.push((crate::scope::XyAxes::valid(a.x, 0, n), "X"));
            roles.push((crate::scope::XyAxes::valid(a.y, 1, n), "Y"));
            for (index, name) in [(a.brightness, "Br"), (a.red, "R"), (a.green, "G"), (a.blue, "B")]
            {
                if let Some(p) = crate::scope::XyAxes::modulator(index, n) {
                    roles.push((p, name));
                }
            }
        } else {
            roles.extend((0..n).map(|p| (p, "")));
        }
        roles.into_iter().map(|(p, r)| (sc.plots[p].elm, r.to_string())).collect()
    }

    /// Marks the elements of the scope the pointer is over: a scope under the diagrams, or a
    /// scope placed in the circuit that is under the pointer or selected.
    pub(crate) fn apply_scope_roles(&mut self) {
        let mut plotted: Vec<(usize, String)> = Vec::new();
        if let Some(sc) = self.scope_hover.and_then(|i| self.sim.scopes.get(i)) {
            plotted = Self::plot_roles(sc);
        } else {
            let on = |i: usize| self.sim.elements[i].base().selected || self.hover == Some(i);
            for (i, sc) in &self.sim.embedded {
                if on(*i) {
                    plotted.extend(Self::plot_roles(sc));
                }
            }
        }
        for e in &mut self.sim.elements {
            e.base_mut().scope_role = None;
        }
        for (elm, role) in plotted {
            if let Some(e) = self.sim.elements.get_mut(elm) {
                let b = e.base_mut();
                b.scope_role = Some(match b.scope_role.take() {
                    Some(old) if !old.is_empty() => format!("{old}/{role}"),
                    _ => role,
                });
            }
        }
    }

    /// The lines of the information box, in the order the original gives them: the voltage of
    /// a terminal when the pointer is on one; else what the element under the pointer (or the
    /// plotted element of the scope under it) says about itself; else the same for the one
    /// selected element; else the time and the time step.
    pub fn info_lines(&self) -> Vec<String> {
        use crate::elements::units::{time_text, unit_text, voltage_text};
        let scope_elm = self
            .scope_hover
            .and_then(|i| self.sim.scopes.get(i))
            .and_then(|sc| sc.plots.first())
            .map(|p| p.elm);
        let selected = self.selected_indices();
        let elm = scope_elm
            .or(self.hover)
            .or_else(|| (selected.len() == 1).then(|| selected[0]))
            .and_then(|i| self.sim.elements.get(i));
        let mut lines = Vec::new();
        match elm {
            Some(e) => {
                // the pointer on a terminal of the element under it: the voltage there
                let post = self
                    .pointer
                    .filter(|_| scope_elm.is_none() && self.hover.is_some())
                    .and_then(|(px, py)| {
                        (0..e.post_count())
                            .find(|&n| e.get_post(n).is_some_and(|p| (p.x, p.y) == (px, py)))
                    });
                match post {
                    Some(n) => lines.push(format!("V = {}", voltage_text(e.get_post_voltage(n)))),
                    None => e.get_info(&mut lines),
                }
            }
            None => {
                lines.push(format!("t = {}", time_text(self.time())));
                lines.push(format!(
                    "{}{}",
                    crate::locale::ls("time step = "),
                    unit_text(self.sim.time_step(), "s")
                ));
            }
        }
        lines
    }

    /// Whether the information box has an element to describe (and so is taller in the
    /// original, which gives it a fifth of the window then and a small strip otherwise).
    pub fn info_is_about_an_element(&self) -> bool {
        self.scope_hover.is_some() || self.hover.is_some() || self.selected_indices().len() == 1
    }

    /// Whether scope `idx` is in `mode`.
    pub fn scope_mode(&self, idx: usize, mode: ScopeMode) -> bool {
        self.sim.scopes.get(idx).is_some_and(|s| s.has_mode(mode))
    }

    /// Turns the X/Y plot, the plot of voltage against current or the spectrum of scope `idx`
    /// on or off.
    pub fn set_scope_mode(&mut self, idx: usize, mode: ScopeMode, on: bool) {
        if idx >= self.sim.scopes.len() {
            return;
        }
        self.push_undo();
        self.sim.scopes[idx].set_mode(mode, on);
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
    }

    /// Stops plotting plot `plot` of scope `idx`; a scope left with no plots goes away.
    pub fn remove_scope_plot(&mut self, idx: usize, plot: usize) {
        let Some(sc) = self.sim.scopes.get_mut(idx) else { return };
        if plot >= sc.plots.len() {
            return;
        }
        self.push_undo();
        if !self.sim.scopes[idx].remove_plot(plot) {
            self.sim.scopes.remove(idx);
        }
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
    }

    /// Empties what scope `idx` has recorded.
    pub fn reset_scope(&mut self, idx: usize) {
        let t = self.sim.time();
        if let Some(sc) = self.sim.scopes.get_mut(idx) {
            sc.reset(t);
        }
        if let Some(w) = &self.worker {
            w.send(super::worker::Command::ResetScope(idx));
        }
    }

    /// What scope `idx` shows now as a CSV file: the time, then the smallest and largest value
    /// of each plot in every sample, oldest first.
    pub fn scope_csv(&self, idx: usize) -> Option<String> {
        csv_of(&self.scope_views().into_iter().nth(idx)?)
    }

    /// The same for the scope placed in the circuit as element `elm`.
    pub fn embedded_scope_csv(&self, elm: usize) -> Option<String> {
        csv_of(self.sim.elements.get(elm)?.scope_view()?)
    }

    /// Turns docked scope `idx` into a scope element placed in the circuit, near the first
    /// element it plots.
    pub fn undock_scope(&mut self, idx: usize) -> bool {
        use crate::elements::geometry::snap_grid;
        if !self.editing_allowed() || idx >= self.sim.scopes.len() {
            return false;
        }
        let scope = &self.sim.scopes[idx];
        let Some(first) = scope.plots.first().and_then(|p| self.sim.elements.get(p.elm)) else {
            return false;
        };
        let (x, y) = (snap_grid(first.base().x + 50), snap_grid(first.base().y + 50));
        let Ok(node) = crate::format::xml::parse(&scope.to_xml().to_pretty_string()) else {
            return false;
        };
        let Some(mut elm) = self.registry.create("ScopeElm", x, y) else { return false };
        let holder = crate::format::xml::XmlNode {
            tag: "ScopeElm".into(),
            children: vec![node],
            ..Default::default()
        };
        elm.undump_xml(&holder);
        self.push_undo();
        self.sim.scopes.remove(idx);
        self.sim.elements.push(elm);
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
        true
    }

    /// Index of the selected element when it is the only one and has a field that can take a
    /// slider.
    pub fn selected_slider_element(&self) -> Option<usize> {
        match self.selected_indices()[..] {
            [i] if !self.slider_entries(i).is_empty() => Some(i),
            _ => None,
        }
    }

    /// The elements that make their scopes glow: the selected ones and the one under the pointer.
    fn glow_set(&self) -> Vec<usize> {
        let mut set = self.selected_indices();
        set.extend(self.hover);
        set
    }

    /// For each docked scope, whether it plots a selected or hovered element and is drawn with
    /// the tinted background.
    pub fn scope_glow(&self) -> Vec<bool> {
        let set = self.glow_set();
        self.sim.scopes.iter().map(|sc| sc.plots.iter().any(|p| set.contains(&p.elm))).collect()
    }

    /// Marks the scopes placed in the circuit that plot a selected or hovered element.
    pub(crate) fn mark_placed_scopes(&mut self) {
        let set = self.glow_set();
        self.sim.set_scope_glow(&set);
    }

    /// Index of the selected element when it is a scope placed in the circuit.
    pub fn selected_scope_element(&self) -> Option<usize> {
        let sel = self.selected_indices();
        match sel[..] {
            [i] if self.sim.elements[i].scope_source().is_some() => Some(i),
            _ => None,
        }
    }

    /// Turns the scope element `elm` back into a docked scope.
    pub fn dock_scope(&mut self, elm: usize) -> bool {
        if !self.editing_allowed() {
            return false;
        }
        let Some((_, mut scope)) =
            self.sim.embedded.iter().find(|(i, _)| *i == elm).map(|(i, s)| (*i, s.clone()))
        else {
            return false;
        };
        self.push_undo();
        // the scope plots element indices, which move when the scope element goes away
        let removed = [elm];
        if !scope.drop_elements(&removed) {
            return false;
        }
        scope.position = self.sim.scopes.iter().map(|s| s.position + 1).max().unwrap_or(0);
        self.remove_elements(&removed);
        self.sim.scopes.push(scope);
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
        true
    }

    /// The scope placed in the circuit as element `elm`, with the settings of the file.
    pub fn embedded_scope(&self, elm: usize) -> Option<&crate::scope::Scope> {
        self.sim.embedded.iter().find(|(i, _)| *i == elm).map(|(_, s)| s)
    }

    /// Changes the settings of the scope placed in the circuit as element `elm`: `edit` works
    /// on a copy, which is written back as the settings of the element. The scope starts its
    /// history again. Returns false when there is no such scope.
    pub fn edit_embedded_scope(
        &mut self,
        elm: usize,
        edit: impl FnOnce(&mut crate::scope::Scope, &[Box<dyn Element>]),
    ) -> bool {
        let Some(mut scope) = self.embedded_scope(elm).cloned() else { return false };
        edit(&mut scope, &self.sim.elements);
        let Ok(node) = crate::format::xml::parse(&scope.to_xml().to_pretty_string()) else {
            return false;
        };
        self.push_undo();
        self.sim.elements[elm].set_scope_source(crate::elements::element::ScopeSource::Xml(node));
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
        true
    }

    /// What the scope placed as element `elm` can plot: the value, its name and whether it is
    /// plotted.
    pub fn embedded_scope_menu(&self, elm: usize) -> Vec<(i32, &'static str, bool)> {
        self.embedded_scope(elm).map(|s| s.menu_items(&self.sim.elements)).unwrap_or_default()
    }

    /// Starts or stops plotting `value` in the scope placed as element `elm`.
    pub fn toggle_embedded_value(&mut self, elm: usize, value: i32, on: bool) {
        self.edit_embedded_scope(elm, |sc, els| sc.set_value(value, on, els));
    }

    /// Whether the scope placed as element `elm` is in `mode`.
    pub fn embedded_scope_mode(&self, elm: usize, mode: ScopeMode) -> bool {
        self.embedded_scope(elm).is_some_and(|s| s.has_mode(mode))
    }

    /// Turns `mode` on or off for the scope placed as element `elm`.
    pub fn set_embedded_scope_mode(&mut self, elm: usize, mode: ScopeMode, on: bool) {
        self.edit_embedded_scope(elm, |sc, _| sc.set_mode(mode, on));
    }

    /// The names of the plots of the scope placed as element `elm`.
    pub fn embedded_plot_names(&self, elm: usize) -> Vec<String> {
        let Some(sc) = self.embedded_scope(elm) else { return Vec::new() };
        sc.view(&self.sim.elements, 0.0, 0.0)
            .plots
            .iter()
            .map(|p| format!("{} {}", p.title, crate::render::scopes::plot_label(p)))
            .collect()
    }

    /// Stops plotting plot `plot` of the scope placed as element `elm`; the element goes away
    /// with its last plot.
    pub fn remove_embedded_plot(&mut self, elm: usize, plot: usize) {
        let Some(sc) = self.embedded_scope(elm) else { return };
        if plot >= sc.plots.len() {
            return;
        }
        if sc.plots.len() == 1 {
            self.push_undo();
            self.remove_elements(&[elm]);
            self.store_scopes();
            self.dirty = true;
            self.reanalyze();
        } else {
            self.edit_embedded_scope(elm, |sc, _| {
                sc.remove_plot(plot);
            });
        }
    }

    /// What the audio outputs have recorded: with a worker, what its circuit recorded.
    #[cfg(feature = "outputs")]
    pub fn audio_clips(&self) -> Vec<super::worker::AudioClip> {
        use crate::elements::audio::AudioOutput;
        self.sim
            .elements
            .iter()
            .enumerate()
            .filter_map(|(elm, e)| {
                let a = e.as_any().downcast_ref::<AudioOutput>()?;
                Some(super::worker::AudioClip {
                    elm,
                    label: a.label(),
                    rate: a.sampling_rate(),
                    samples: a.samples(),
                })
            })
            .collect()
    }

    /// The audio outputs of the circuit: the element and the name of its play button.
    #[cfg(feature = "outputs")]
    pub fn audio_outputs(&self) -> Vec<(usize, String)> {
        use crate::elements::audio::AudioOutput;
        self.sim
            .elements
            .iter()
            .enumerate()
            .filter_map(|(i, e)| Some((i, e.as_any().downcast_ref::<AudioOutput>()?.label())))
            .collect()
    }

    /// The recording of audio output `elm` as a WAV file; `None` when there is nothing to
    /// play yet.
    #[cfg(feature = "outputs")]
    pub fn audio_wav(&self, elm: usize) -> Option<Vec<u8>> {
        let clips = match &self.worker {
            Some(w) => w.audio()?,
            None => self.audio_clips(),
        };
        let clip = clips.into_iter().find(|c| c.elm == elm)?;
        (!clip.samples.is_empty())
            .then(|| crate::elements::audio::wav_bytes(&clip.samples, clip.rate))
    }

    /// Without the outputs family there are no audio outputs.
    #[cfg(not(feature = "outputs"))]
    pub fn audio_clips(&self) -> Vec<super::worker::AudioClip> {
        Vec::new()
    }

    /// Without the outputs family there are no audio outputs.
    #[cfg(not(feature = "outputs"))]
    pub fn audio_outputs(&self) -> Vec<(usize, String)> {
        Vec::new()
    }

    /// Without the outputs family there is nothing to play.
    #[cfg(not(feature = "outputs"))]
    pub fn audio_wav(&self, _elm: usize) -> Option<Vec<u8>> {
        None
    }

    /// Removes the scope placed in the circuit as element `elm`.
    pub fn remove_embedded_scope(&mut self, elm: usize) {
        if self.embedded_scope(elm).is_none() {
            return;
        }
        self.push_undo();
        self.remove_elements(&[elm]);
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
    }

    /// Sets how many time steps one sample of scope `idx` (and of the scopes stacked with it)
    /// covers, between 1 and 1024. The recorded history of those scopes starts again.
    pub fn set_scope_speed(&mut self, idx: usize, speed: i32) {
        let Some(place) = self.sim.scopes.get(idx).map(|s| s.position) else { return };
        let speed = speed.clamp(1, 1024);
        if self.sim.scopes.iter().filter(|s| s.position == place).all(|s| s.speed == speed) {
            return;
        }
        self.push_undo();
        for s in self.sim.scopes.iter_mut().filter(|s| s.position == place) {
            s.speed = speed;
        }
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
    }

    /// The fields of the properties dialog of scope `idx`: manual scale and its divisions, the
    /// time scale, the trigger, a scale, a position (and AC coupling for voltages) for each
    /// plot, and the measurements to show.
    pub fn scope_properties(&self, idx: usize) -> Option<Vec<EditInfo>> {
        Some(scope_fields(self.sim.scopes.get(idx)?, &self.sim.elements))
    }

    /// The same for the scope placed in the circuit as element `elm`.
    pub fn embedded_scope_properties(&self, elm: usize) -> Option<Vec<EditInfo>> {
        Some(scope_fields(self.embedded_scope(elm)?, &self.sim.elements))
    }

    /// Applies the fields of [`Session::scope_properties`]; an invalid value is reported and
    /// nothing changes.
    pub fn apply_scope_properties(
        &mut self,
        idx: usize,
        fields: &[EditInfo],
    ) -> Result<(), String> {
        let sc = self.sim.scopes.get(idx).ok_or("no such scope")?;
        check_scope_fields(sc, fields)?;
        self.push_undo();
        // scopes stacked in one place share their time scale
        let place = self.sim.scopes[idx].position;
        let speed = fields[3].value as i32;
        for s in self.sim.scopes.iter_mut().filter(|s| s.position == place) {
            s.speed = speed;
        }
        apply_scope_fields(&mut self.sim.scopes[idx], fields);
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
        Ok(())
    }

    /// Applies the fields of [`Session::embedded_scope_properties`].
    pub fn apply_embedded_scope_properties(
        &mut self,
        elm: usize,
        fields: &[EditInfo],
    ) -> Result<(), String> {
        check_scope_fields(self.embedded_scope(elm).ok_or("no such scope")?, fields)?;
        self.edit_embedded_scope(elm, |sc, _| apply_scope_fields(sc, fields));
        Ok(())
    }

    /// Moves plot `plot` of scope `idx` up or down (manual scale only) to `pos` steps of
    /// `V_POSITION_STEPS`, without restarting anything: this follows the mouse while dragging.
    pub fn set_plot_position(&mut self, idx: usize, plot: usize, pos: i32) {
        let steps = crate::scope::V_POSITION_STEPS;
        let Some(p) = self.sim.scopes.get_mut(idx).and_then(|s| s.plots.get_mut(plot)) else {
            return;
        };
        p.man_pos = pos.clamp(-steps, steps);
        p.man_set = true;
        self.dirty = true;
        if let Some(w) = &self.worker {
            w.send(super::worker::Command::PlotPosition { scope: idx, plot, pos });
        }
    }

    /// Whether scope `idx` is in manual scale mode, and where plot `plot` sits.
    pub fn plot_position(&self, idx: usize, plot: usize) -> Option<(bool, i32)> {
        let sc = self.sim.scopes.get(idx)?;
        Some((sc.is_manual(), sc.plots.get(plot)?.man_pos))
    }

    /// Time steps per sample of scope `idx`.
    pub fn scope_speed(&self, idx: usize) -> i32 {
        self.sim.scopes.get(idx).map_or(64, |s| s.speed)
    }

    /// Removes scope `idx`.
    pub fn remove_scope(&mut self, idx: usize) {
        if idx >= self.sim.scopes.len() {
            return;
        }
        self.push_undo();
        self.sim.scopes.remove(idx);
        self.store_scopes();
        self.scope_views.clear();
        self.dirty = true;
        self.reanalyze();
    }

    /// Removes every scope.
    pub fn remove_scopes(&mut self) {
        if self.sim.scopes.is_empty() {
            return;
        }
        self.push_undo();
        self.sim.scopes.clear();
        self.store_scopes();
        self.scope_views.clear();
        self.dirty = true;
        self.reanalyze();
    }

    /// Puts every scope in the same place so they are drawn over one another.
    pub fn stack_scopes(&mut self) {
        use crate::scope::{FLAG_MIN, FLAG_NO_MAX};
        // as in the original, stacked scopes show no peak values
        self.rearrange_scopes(|scopes| {
            for s in scopes.iter_mut() {
                s.position = 0;
                s.flags |= FLAG_NO_MAX;
                s.flags &= !FLAG_MIN;
            }
        });
    }

    /// Gives every scope a place of its own.
    pub fn unstack_scopes(&mut self) {
        self.rearrange_scopes(|scopes| {
            for (i, s) in scopes.iter_mut().enumerate() {
                s.position = i as i32;
                s.flags &= !crate::scope::FLAG_NO_MAX;
            }
        });
    }

    /// Whether scope `idx` can be stacked on the one before it (it is not in its column yet).
    pub fn can_stack_scope(&self, idx: usize) -> bool {
        let s = &self.sim.scopes;
        let i = idx.max(1);
        s.len() >= 2 && i < s.len() && s[i].position != s[i - 1].position
    }

    /// Whether scope `idx` can leave the column it shares.
    pub fn can_unstack_scope(&self, idx: usize) -> bool {
        let s = &self.sim.scopes;
        let i = idx.max(1);
        if s.len() < 2 || i >= s.len() {
            return false;
        }
        // from the top scope of a column too
        s[i].position == s[i - 1].position
            || s.get(i + 1).is_some_and(|n| n.position == s[i].position)
    }

    /// Whether scope `idx` can be combined into the one before it.
    pub fn can_combine_scope(&self, idx: usize) -> bool {
        self.sim.scopes.len() >= 2 && idx.max(1) < self.sim.scopes.len()
    }

    /// Puts scope `idx` in the column of the one before it, as the original's Stack.
    pub fn stack_scope(&mut self, idx: usize) {
        if !self.can_stack_scope(idx) {
            return;
        }
        self.rearrange_scopes(|scopes| {
            let i = idx.max(1);
            scopes[i].position = scopes[i - 1].position;
            for s in &mut scopes[i + 1..] {
                s.position -= 1;
            }
        });
    }

    /// Takes scope `idx` (and the ones after it) out of its column, as the original's Unstack.
    pub fn unstack_scope(&mut self, idx: usize) {
        if !self.can_unstack_scope(idx) {
            return;
        }
        self.rearrange_scopes(|scopes| {
            let mut i = idx.max(1);
            if scopes[i].position != scopes[i - 1].position {
                i += 1;
            }
            let from = i.min(scopes.len());
            for s in &mut scopes[from..] {
                s.position += 1;
            }
        });
    }

    /// Moves the plots of scope `idx` into the scope before it, as the original's Combine.
    pub fn combine_scope(&mut self, idx: usize) {
        if !self.can_combine_scope(idx) {
            return;
        }
        self.rearrange_scopes(|scopes| {
            let i = idx.max(1);
            let moved = scopes.remove(i);
            scopes[i - 1].combine(moved);
        });
    }

    /// The scope placed in the circuit whose settings wheel is at surface position
    /// (`px`, `py`): the element index.
    pub fn embedded_wheel_at(&self, px: f64, py: f64) -> Option<usize> {
        let (gx, gy) = self.view.to_circuit(px, py);
        self.sim.embedded.iter().map(|(i, _)| *i).find(|&i| {
            let b = self.sim.elements[i].base();
            let (x0, y0) = (b.x.min(b.x2), b.y.min(b.y2));
            let (w, h) = (f64::from((b.x2 - b.x).abs()), f64::from((b.y2 - b.y).abs()));
            crate::render::scopes::on_wheel(
                (f64::from(x0), f64::from(y0), w, h),
                f64::from(gx - x0),
                f64::from(gy - y0),
            )
        })
    }

    /// Whether the scope placed as element `elm` keeps its full scale at the largest value.
    pub fn embedded_max_scale(&self, elm: usize) -> bool {
        self.embedded_scope(elm).is_some_and(|s| s.flags & crate::scope::FLAG_MAX_SCALE != 0)
    }

    /// Turns "Max Scale" of the scope placed as element `elm` on or off.
    pub fn set_embedded_max_scale(&mut self, elm: usize, on: bool) {
        self.edit_embedded_scope(elm, |sc, _| {
            if on {
                sc.flags |= crate::scope::FLAG_MAX_SCALE;
            } else {
                sc.flags &= !crate::scope::FLAG_MAX_SCALE;
            }
        });
    }

    /// Whether scope `idx` keeps its full scale at the largest value it has seen.
    pub fn scope_max_scale(&self, idx: usize) -> bool {
        self.sim.scopes.get(idx).is_some_and(|s| s.flags & crate::scope::FLAG_MAX_SCALE != 0)
    }

    /// Turns the "Max Scale" of scope `idx` on or off.
    pub fn set_scope_max_scale(&mut self, idx: usize, on: bool) {
        if idx >= self.sim.scopes.len() {
            return;
        }
        self.push_undo();
        let s = &mut self.sim.scopes[idx];
        if on {
            s.flags |= crate::scope::FLAG_MAX_SCALE;
        } else {
            s.flags &= !crate::scope::FLAG_MAX_SCALE;
        }
        self.store_scopes();
        self.dirty = true;
        self.reanalyze();
    }

    /// Moves the plots of all scopes into the first one.
    pub fn combine_scopes(&mut self) {
        self.rearrange_scopes(|scopes| {
            let mut all = std::mem::take(scopes).into_iter();
            if let Some(mut first) = all.next() {
                for s in all {
                    first.combine(s);
                }
                scopes.push(first);
            }
        });
    }

    /// Gives every plot a scope of its own.
    pub fn separate_scopes(&mut self) {
        self.rearrange_scopes(|scopes| {
            let all: Vec<_> = std::mem::take(scopes)
                .into_iter()
                .flat_map(crate::scope::Scope::separate)
                .collect();
            *scopes = all;
            scopes.iter_mut().enumerate().for_each(|(i, s)| s.position = i as i32);
        });
    }

    fn rearrange_scopes(&mut self, f: impl FnOnce(&mut Vec<crate::scope::Scope>)) {
        if self.sim.scopes.is_empty() {
            return;
        }
        self.push_undo();
        f(&mut self.sim.scopes);
        self.store_scopes();
        self.scope_views.clear();
        self.dirty = true;
        self.reanalyze();
    }

    /// Keeps the scopes in the form that is saved and sent to the worker.
    pub(crate) fn store_scopes(&mut self) {
        self.sim.embedded = crate::scope::embedded_scopes(&self.sim.elements);
        self.extras.scope_xml = self
            .sim
            .scopes
            .iter()
            .filter_map(|s| crate::format::xml::parse(&s.to_xml().to_pretty_string()).ok())
            .collect();
        self.extras.scope_lines.clear();
    }

    /// Selects only the element under surface position (`px`, `py`), for a context menu.
    /// Returns false when there is none.
    pub fn select_for_context(&mut self, px: f64, py: f64) -> bool {
        if !self.inspect.is_empty() {
            return false;
        }
        let (gx, gy) = self.view.to_circuit(px, py);
        let Some(i) = self.element_at(gx, gy) else { return false };
        if !self.sim.elements[i].base().selected {
            self.deselect_all();
            self.sim.elements[i].base_mut().selected = true;
        }
        true
    }

    /// True when at least one element is selected.
    pub fn has_selection(&self) -> bool {
        self.sim.elements.iter().any(|e| e.base().selected)
    }

    /// Selects every element.
    pub fn select_all(&mut self) {
        for e in &mut self.sim.elements {
            e.base_mut().selected = true;
        }
    }

    /// Clears the selection.
    pub fn deselect_all(&mut self) {
        for e in &mut self.sim.elements {
            e.base_mut().selected = false;
        }
    }

    /// Removes elements by index, keeping the sliders pointing at the right elements.
    pub(crate) fn remove_elements(&mut self, indices: &[usize]) {
        if indices.is_empty() {
            return;
        }
        let mut sorted = indices.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        for &i in sorted.iter().rev() {
            if i < self.sim.elements.len() {
                self.sim.elements.remove(i);
            }
        }
        self.sim.scopes.retain_mut(|sc| sc.drop_elements(&sorted));
        self.store_scopes();
        let shift = |idx: usize| idx - sorted.iter().filter(|&&r| r < idx).count();
        let old = std::mem::take(&mut self.adjustables);
        let kept: Vec<usize> = (0..old.len()).filter(|&k| !sorted.contains(&old[k].elm)).collect();
        self.adjustables = kept
            .iter()
            .map(|&k| {
                let mut a = old[k].clone();
                a.elm = shift(a.elm);
                a.shared_slider = a.shared_slider.and_then(|s| kept.iter().position(|&x| x == s));
                a
            })
            .collect();
        self.refresh_slider_positions();
        self.held = None;
        self.hover = None;
    }

    /// Deletes the selected elements.
    pub fn delete_selected(&mut self) -> bool {
        if !self.editing_allowed() || !self.has_selection() {
            return false;
        }
        self.push_undo();
        let sel = self.selected_indices();
        self.remove_elements(&sel);
        self.dirty = true;
        self.reanalyze();
        true
    }

    // ---- clipboard -------------------------------------------------------------------------

    /// Copies the selected elements to the clipboard; returns the text, if anything was copied.
    pub fn copy_selected(&mut self) -> Option<String> {
        if !self.has_selection() {
            return None;
        }
        let text = write_elements_xml(
            self.sim.elements.iter().filter(|e| e.base().selected).map(|e| e.as_ref()),
        );
        self.clipboard = Some(text.clone());
        Some(text)
    }

    /// Cuts the selected elements.
    pub fn cut_selected(&mut self) -> Option<String> {
        let text = self.copy_selected()?;
        self.delete_selected();
        Some(text)
    }

    fn paste_text(&mut self, text: &str, at: PasteAt) -> bool {
        if !self.editing_allowed() {
            return false;
        }
        let parsed = parse(text, &self.registry);
        let mut elements = parsed.circuit.elements;
        if elements.is_empty() {
            return false;
        }
        let (mut min_x, mut min_y) = (i32::MAX, i32::MAX);
        for e in &elements {
            let b = e.base();
            min_x = min_x.min(b.x).min(b.x2);
            min_y = min_y.min(b.y).min(b.y2);
        }
        let (dx, dy) = match at {
            PasteAt::Point(x, y) => (self.snap(x) - min_x, self.snap(y) - min_y),
            PasteAt::Offset(dx, dy) => (dx, dy),
        };
        self.push_undo();
        self.deselect_all();
        for e in &mut elements {
            move_element(e.as_mut(), dx, dy);
            e.base_mut().selected = true;
        }
        self.sim.elements.extend(elements);
        self.dirty = true;
        self.reanalyze();
        true
    }

    /// Pastes the clipboard at the mouse position, or slightly offset when the mouse is away.
    pub fn paste(&mut self) -> bool {
        let Some(text) = self.clipboard.clone() else { return false };
        let at = match self.pointer {
            Some((x, y)) => PasteAt::Point(x, y),
            None => PasteAt::Offset(self.grid_size(), self.grid_size()),
        };
        self.paste_text(&text, at)
    }

    /// Pastes text from the system clipboard.
    pub fn paste_from_text(&mut self, text: &str) -> bool {
        self.clipboard = Some(text.to_string());
        self.paste()
    }

    /// Duplicates the selected elements next to the originals.
    pub fn duplicate(&mut self) -> bool {
        let Some(text) = self.copy_selected() else { return false };
        let g = self.grid_size();
        self.paste_text(&text, PasteAt::Offset(g, g))
    }

    // ---- undo and redo ---------------------------------------------------------------------

    /// Remembers the current circuit so the next change can be undone.
    pub fn push_undo(&mut self) {
        let snapshot = self.circuit_xml();
        if self.undo.last() != Some(&snapshot) {
            self.undo.push(snapshot);
            if self.undo.len() > MAX_UNDO {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
    }

    /// True when there is something to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// True when there is something to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn restore(&mut self, text: &str) {
        let parsed = parse(text, &self.registry);
        self.sim.elements = parsed.circuit.elements;
        self.adjustables = parsed.circuit.adjustables;
        self.sim.scopes = crate::scope::scopes_from_circuit(
            &parsed.circuit.scope_lines,
            &parsed.circuit.scope_xml,
            &self.sim.elements,
        );
        self.store_scopes();
        self.drag = Drag::None;
        self.held = None;
        self.hover = None;
        self.dirty = true;
        self.refresh_slider_positions();
        self.reanalyze();
    }

    /// Undoes the last change.
    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else { return false };
        let current = self.circuit_xml();
        self.redo.push(current);
        self.restore(&previous);
        true
    }

    /// Redoes the last undone change.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        let current = self.circuit_xml();
        self.undo.push(current);
        self.restore(&next);
        true
    }

    // ---- mirror and rotate -----------------------------------------------------------------

    /// Mirrors (`Mirror::X` or `Mirror::Y`) or rotates the selection, or the whole circuit when
    /// nothing is selected.
    pub fn transform(&mut self, op: Transform) -> bool {
        if !self.editing_allowed() || self.sim.elements.is_empty() {
            return false;
        }
        self.push_undo();
        let any = self.has_selection();
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for e in self.sim.elements.iter().filter(|e| e.base().selected || !any) {
            let b = e.base();
            min_x = min_x.min(b.x).min(b.x2);
            max_x = max_x.max(b.x).max(b.x2);
            min_y = min_y.min(b.y).min(b.y2);
            max_y = max_y.max(b.y).max(b.y2);
        }
        let (cx, cy) = ((min_x + max_x) / 2, (min_y + max_y) / 2);
        let xmy = self.snap(cx - cy);
        let count = self.selected_indices().len();
        for e in self.sim.elements.iter_mut().filter(|e| e.base().selected || !any) {
            let e = e.as_mut();
            match op {
                Transform::MirrorX if e.can_flip_x() => e.flip_x(cx * 2, count),
                Transform::MirrorY if e.can_flip_y() => e.flip_y(cy * 2, count),
                Transform::RotateCcw => {
                    e.flip_xy(xmy, count);
                    e.flip_y(cy * 2, count);
                }
                Transform::RotateCw => {
                    e.flip_y(cy * 2, count);
                    e.flip_xy(xmy, count);
                }
                _ => {}
            }
            e.set_points();
        }
        self.dirty = true;
        self.reanalyze();
        true
    }

    // ---- element properties ----------------------------------------------------------------

    /// Name of element `idx` for lists and dialog titles.
    pub fn element_title(&self, idx: usize) -> String {
        let Some(e) = self.sim.elements.get(idx) else { return String::new() };
        if let Some(t) = e.elm_type() {
            return ls(&t);
        }
        match self
            .registry
            .ids()
            .find(|id| *id == e.class_name())
            .and_then(|id| self.registry.get(id))
        {
            Some(d) => ls(d.display_name.strip_prefix("Add ").unwrap_or(d.display_name)),
            None => e.class_name().to_string(),
        }
    }

    /// The editable fields of element `idx`.
    pub fn edit_fields(&self, idx: usize) -> Vec<EditInfo> {
        let Some(e) = self.sim.elements.get(idx) else { return Vec::new() };
        (0..).map_while(|n| e.get_edit_info(n)).collect()
    }

    /// Applies edited fields to element `idx`.
    pub fn apply_edit_fields(&mut self, idx: usize, mut fields: Vec<EditInfo>) {
        if !self.editing_allowed() || idx >= self.sim.elements.len() {
            return;
        }
        self.push_undo();
        for (n, ei) in fields.iter_mut().enumerate() {
            self.sim.apply_edit(idx, n, ei);
        }
        self.sim.elements[idx].set_points();
        self.dirty = true;
        self.refresh_slider_positions();
        self.reanalyze();
    }

    /// The fields of element `idx` that can have a slider, with the slider each has now.
    pub fn slider_entries(&self, idx: usize) -> Vec<SliderEntry> {
        let Some(e) = self.sim.elements.get(idx) else { return Vec::new() };
        (0..)
            .map_while(|n| e.get_edit_info(n).map(|ei| (n, ei)))
            .filter(|(_, ei)| ei.can_create_adjustable() && ei.text.is_none())
            .map(|(item, ei)| {
                let found = self.adjustables.iter().find(|a| a.elm == idx && a.edit_item == item);
                // like the original: the field's own range when it has one, else 1 to 1000
                let (lo, hi) =
                    if ei.max_val > 0.0 { (ei.min_val, ei.max_val) } else { (1.0, 1000.0) };
                let name = ls(&ei.name);
                let label = match name.rfind(" (") {
                    Some(i) if name.ends_with(')') => name[..i].to_string(),
                    _ => name.clone(),
                };
                SliderEntry {
                    item,
                    name,
                    enabled: found.is_some(),
                    min: found.map_or(lo, |a| a.min_value),
                    max: found.map_or(hi, |a| a.max_value),
                    step: found.map_or(0.0, |a| a.slider_step),
                    logarithmic: found.is_some_and(|a| a.logarithmic),
                    label: found.map_or(label, |a| a.slider_text.clone()),
                }
            })
            .collect()
    }

    /// Creates, changes or removes the sliders of element `idx` as `choices` say.
    pub fn apply_sliders(&mut self, idx: usize, choices: &[SliderEntry]) -> Result<(), String> {
        use crate::format::circuit::Adjustable;
        if !self.editing_allowed() || idx >= self.sim.elements.len() {
            return Ok(());
        }
        for c in choices.iter().filter(|c| c.enabled) {
            if c.min.is_nan() || c.max.is_nan() || c.min >= c.max {
                return Err(format!("{}: the minimum must be below the maximum", c.name));
            }
            if c.step < 0.0 {
                return Err(format!("{}: the step cannot be negative", c.name));
            }
        }
        self.push_undo();
        for c in choices {
            let at = self.adjustables.iter().position(|a| a.elm == idx && a.edit_item == c.item);
            match (c.enabled, at) {
                (true, Some(k)) => {
                    let a = &mut self.adjustables[k];
                    a.min_value = c.min;
                    a.max_value = c.max;
                    a.slider_step = c.step;
                    a.logarithmic = c.logarithmic && c.min > 0.0;
                    a.slider_text.clone_from(&c.label);
                }
                (true, None) => self.adjustables.push(Adjustable {
                    elm: idx,
                    edit_item: c.item,
                    min_value: c.min,
                    max_value: c.max,
                    slider_text: c.label.clone(),
                    slider_step: c.step,
                    logarithmic: c.logarithmic && c.min > 0.0,
                    shared_slider: None,
                }),
                (false, Some(k)) => {
                    self.adjustables.remove(k);
                    // sliders that shared the removed one lose it; later indexes move up
                    for a in &mut self.adjustables {
                        a.shared_slider = match a.shared_slider {
                            Some(s) if s == k => None,
                            Some(s) if s > k => Some(s - 1),
                            other => other,
                        };
                    }
                }
                (false, None) => {}
            }
        }
        self.dirty = true;
        self.refresh_slider_positions();
        Ok(())
    }

    /// Labels of the model buttons of element `idx`'s edit dialog.
    pub fn model_buttons(&self, idx: usize) -> Vec<String> {
        self.sim.elements.get(idx).map(|e| e.model_buttons()).unwrap_or_default()
    }

    /// Starts the model dialog for model button `button` of element `idx`.
    pub fn start_model_edit(&self, idx: usize, button: usize) -> Option<ModelEditor> {
        self.sim.elements.get(idx)?.start_model_edit(button)
    }

    /// Reads the edited fields of a model dialog. Returns true when they change which fields
    /// the dialog has, so it must be shown again from `editor` (which keeps what was typed);
    /// otherwise the model is stored, element `idx` takes it, and every element looks its
    /// model up again.
    pub fn finish_model_edit(
        &mut self,
        idx: usize,
        editor: &mut ModelEditor,
        fields: &mut [EditInfo],
    ) -> Result<bool, String> {
        if !self.editing_allowed() || idx >= self.sim.elements.len() {
            return Ok(false);
        }
        editor.apply(fields)?;
        if ModelEditor::needs_new_dialog(fields) {
            return Ok(true);
        }
        self.push_undo();
        let name = editor.finish()?;
        self.sim.elements[idx].use_model(&name);
        for e in &mut self.sim.elements {
            e.update_models();
        }
        self.dirty = true;
        self.reanalyze();
        Ok(false)
    }

    /// The element under surface position (`px`, `py`) that a double click should edit.
    pub fn element_for_edit(&self, px: f64, py: f64) -> Option<usize> {
        if !self.inspect.is_empty() {
            return None;
        }
        let (gx, gy) = self.view.to_circuit(px, py);
        self.element_at(gx, gy)
            .filter(|&i| !self.edit_fields(i).is_empty() || self.embedded_scope(i).is_some())
    }

    /// Operates the switches whose keyboard shortcut is `key`. Returns true when one was found.
    pub fn key_toggle(&mut self, key: &str) -> bool {
        let hits: Vec<usize> = self
            .sim
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.toggle_key() == Some(key))
            .map(|(i, _)| i)
            .collect();
        for &i in &hits {
            self.toggle_linked(i);
        }
        if !hits.is_empty() {
            self.reanalyze();
        }
        !hits.is_empty()
    }

    /// Applies the settings of the Other Options dialog and analyses the circuit again.
    pub fn apply_other_options(
        &mut self,
        max_time_step: f64,
        voltage_range: f64,
        adjust_time_step: bool,
        auto_dc_on_reset: bool,
    ) {
        if max_time_step > 0.0 {
            self.options.max_time_step = max_time_step;
            self.options.min_time_step = self.options.min_time_step.min(max_time_step);
        }
        if voltage_range > 0.0 {
            self.options.voltage_range = voltage_range;
        }
        self.options.adjust_time_step = adjust_time_step;
        self.options.auto_dc_on_reset = auto_dc_on_reset;
        self.sim.set_settings(self.options.sim_settings());
        self.dirty = true;
        self.reanalyze();
        self.refresh_draw_ctx_now();
    }

    /// Nudges the first numeric field of the element under the pointer with the mouse wheel.
    /// `up` raises the value. Returns true when a value changed.
    pub fn wheel_at(&mut self, px: f64, py: f64, up: bool) -> bool {
        if !self.settings.mouse_wheel_edit || !self.editing_allowed() {
            return false;
        }
        let (gx, gy) = self.view.to_circuit(px, py);
        let Some(i) = self.element_at(gx, gy) else { return false };
        let fields = self.edit_fields(i);
        let Some((n, mut ei)) = fields.into_iter().enumerate().find(|(_, f)| {
            f.text.is_none() && f.choice.is_none() && f.checkbox.is_none() && f.button.is_none()
        }) else {
            return false;
        };
        let factor = if up { 1.1 } else { 1.0 / 1.1 };
        ei.value *= factor;
        let value = ei.value;
        self.sim.apply_edit(i, n, &mut ei);
        self.sim.elements[i].set_points();
        self.dirty = true;
        self.refresh_slider_positions();
        self.param_changed(i, n, value);
        true
    }

    /// Entries for the Find Component list: element index and a description.
    pub fn element_labels(&self) -> Vec<(usize, String)> {
        (0..self.sim.elements.len())
            .map(|i| {
                let b = self.sim.elements[i].base();
                (i, format!("{} ({}, {})", self.element_title(i), b.x, b.y))
            })
            .collect()
    }

    /// Selects only element `idx` and centres the view on it.
    pub fn focus_element(&mut self, idx: usize) {
        self.deselect_all();
        let Some(e) = self.sim.elements.get_mut(idx) else { return };
        e.base_mut().selected = true;
        let b = e.base();
        let (mx, my) = (f64::from(b.x + b.x2) / 2.0, f64::from(b.y + b.y2) / 2.0);
        self.view.x = self.canvas_size.0 / 2.0 - mx * self.view.scale;
        self.view.y = self.canvas_size.1 / 2.0 - my * self.view.scale;
    }

    // ---- view ------------------------------------------------------------------------------

    fn zoom_to(&mut self, scale: f64, cx: f64, cy: f64) {
        let scale = scale.clamp(MIN_SCALE, MAX_SCALE);
        let k = scale / self.view.scale;
        self.view.x = cx - (cx - self.view.x) * k;
        self.view.y = cy - (cy - self.view.y) * k;
        self.view.scale = scale;
    }

    /// Multiplies the zoom by `factor` about the canvas centre.
    pub fn zoom_by(&mut self, factor: f64) {
        let (cx, cy) = (self.canvas_size.0 / 2.0, self.canvas_size.1 / 2.0);
        self.zoom_to(self.view.scale * factor, cx, cy);
    }

    /// Multiplies the zoom by `factor` keeping the circuit point under (`px`, `py`) fixed.
    pub fn zoom_at(&mut self, px: f64, py: f64, factor: f64) {
        self.zoom_to(self.view.scale * factor, px, py);
    }

    /// Resets the zoom to 100% about the canvas centre.
    pub fn zoom_100(&mut self) {
        let (cx, cy) = (self.canvas_size.0 / 2.0, self.canvas_size.1 / 2.0);
        self.zoom_to(1.0, cx, cy);
    }

    /// Fits the circuit into the canvas on the next frame.
    pub fn center_circuit(&mut self) {
        self.fit_pending = true;
    }

    // ---- options ---------------------------------------------------------------------------

    /// Current state of an Options-menu switch.
    pub fn toggle_state(&self, t: Toggle) -> bool {
        let (o, s) = (&self.options, &self.settings);
        match t {
            Toggle::ShowCurrent => o.show_dots,
            Toggle::ShowVoltage => o.show_volts,
            Toggle::ShowPower => o.show_power,
            Toggle::ShowValues => o.show_values,
            Toggle::SmallGrid => o.small_grid,
            Toggle::Toolbar => s.show_toolbar,
            Toggle::Crosshair => s.crosshair,
            Toggle::EuroResistors => s.euro_resistors,
            Toggle::ShowOhm => s.show_ohm,
            Toggle::IecGates => s.iec_gates,
            Toggle::WhiteBackground => s.white_background,
            Toggle::ConventionalCurrent => s.conventional_current,
            Toggle::DisableEditing => s.disable_editing,
            Toggle::MouseWheelEdit => s.mouse_wheel_edit,
        }
    }

    /// Changes an Options-menu switch and saves the settings.
    pub fn set_toggle(&mut self, t: Toggle, on: bool) {
        let (o, s) = (&mut self.options, &mut self.settings);
        match t {
            Toggle::ShowCurrent => (o.show_dots, s.show_dots) = (on, on),
            Toggle::ShowVoltage => (o.show_volts, s.show_volts) = (on, on),
            Toggle::ShowPower => (o.show_power, s.show_power) = (on, on),
            Toggle::ShowValues => (o.show_values, s.show_values) = (on, on),
            Toggle::SmallGrid => {
                (o.small_grid, s.small_grid) = (on, on);
                crate::elements::geometry::set_grid_size(if on { 8 } else { 16 });
            }
            Toggle::Toolbar => s.show_toolbar = on,
            Toggle::Crosshair => s.crosshair = on,
            Toggle::EuroResistors => s.euro_resistors = on,
            Toggle::ShowOhm => s.show_ohm = on,
            Toggle::IecGates => s.iec_gates = on,
            Toggle::WhiteBackground => s.white_background = on,
            Toggle::ConventionalCurrent => s.conventional_current = on,
            Toggle::DisableEditing => s.disable_editing = on,
            Toggle::MouseWheelEdit => s.mouse_wheel_edit = on,
        }
        if let Some(path) = &self.settings_path {
            // a failure to save must not interrupt editing; the setting still applies
            let _ = self.settings.save(path);
        }
        self.refresh_draw_ctx_now();
    }

    /// Chooses the interface language; it is applied the next time the program starts.
    pub fn set_language(&mut self, code: &str) {
        self.settings.language = code.to_string();
        if let Some(path) = &self.settings_path {
            let _ = self.settings.save(path);
        }
    }
}

/// A geometric change applied to the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transform {
    /// Mirror left to right.
    MirrorX,
    /// Mirror top to bottom.
    MirrorY,
    /// Rotate 90 degrees counter-clockwise.
    RotateCcw,
    /// Rotate 90 degrees clockwise.
    RotateCw,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::Session;
    use crate::elements::registry::Registry;

    const RC: &str = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\nr 0 0 64 0 0 1000\nc 64 0 64 64 4 0.000001 0 0 0\nw 64 64 0 64 0\n";

    fn session() -> Session {
        let mut s = Session::load(RC, &Registry::standard()).unwrap();
        s.canvas_size = (800.0, 600.0);
        s.fit_pending = false;
        s
    }

    fn press(s: &mut Session, x: f64, y: f64) {
        s.mouse_down(x, y, Mods::default(), 1);
    }

    #[cfg(feature = "passive")]
    #[test]
    fn dragging_with_the_resistor_tool_places_a_resistor() {
        let mut s = session();
        let before = s.sim.elements.len();
        s.set_tool(Tool::Place("ResistorElm".into()));
        press(&mut s, 100.0, 200.0);
        s.mouse_drag(164.0, 200.0);
        assert!(s.mouse_up());
        assert_eq!(s.sim.elements.len(), before + 1);
        let r = s.sim.elements.last().unwrap();
        assert_eq!((r.base().x, r.base().y, r.base().x2, r.base().y2), (96, 192, 160, 192));
        assert!(s.dirty && s.can_undo());
        // a click without a drag discards the element
        press(&mut s, 300.0, 300.0);
        assert!(!s.mouse_up());
        assert_eq!(s.sim.elements.len(), before + 1);
    }

    #[cfg(feature = "passive")]
    #[test]
    fn undo_and_redo_restore_the_element_list() {
        let mut s = session();
        let n = s.sim.elements.len();
        s.set_tool(Tool::Place("ResistorElm".into()));
        press(&mut s, 100.0, 200.0);
        s.mouse_drag(164.0, 200.0);
        s.mouse_up();
        assert!(s.undo());
        assert_eq!(s.sim.elements.len(), n);
        assert!(s.can_redo());
        assert!(s.redo());
        assert_eq!(s.sim.elements.len(), n + 1);
    }

    #[test]
    fn rubber_band_selects_and_delete_removes() {
        let mut s = session();
        s.set_tool(Tool::Select);
        press(&mut s, -20.0, -20.0);
        s.mouse_drag(30.0, 30.0);
        assert!(s.rubber_band().is_some());
        s.mouse_up();
        // only the resistor's box touches the band's corner region
        assert!(s.has_selection());
        let n = s.sim.elements.len();
        let k = s.selected_indices().len();
        assert!(s.delete_selected());
        assert_eq!(s.sim.elements.len(), n - k);
        assert!(s.undo());
        assert_eq!(s.sim.elements.len(), n);
    }

    #[test]
    fn copy_paste_and_duplicate_add_selected_copies() {
        let mut s = session();
        s.select_all();
        let n = s.sim.elements.len();
        assert!(s.copy_selected().is_some());
        assert!(s.paste());
        assert_eq!(s.sim.elements.len(), 2 * n);
        assert!(s.duplicate());
        assert_eq!(s.sim.elements.len(), 3 * n);
        let cut = s.cut_selected();
        assert!(cut.is_some());
        assert_eq!(s.sim.elements.len(), 2 * n);
    }

    #[test]
    fn rotating_four_times_returns_to_the_start() {
        let mut s = session();
        let pos = |s: &Session| -> Vec<_> {
            s.sim
                .elements
                .iter()
                .map(|e| (e.base().x, e.base().y, e.base().x2, e.base().y2))
                .collect()
        };
        let start = pos(&s);
        for _ in 0..4 {
            assert!(s.transform(Transform::RotateCw));
        }
        assert_eq!(pos(&s), start);
        s.transform(Transform::MirrorX);
        s.transform(Transform::MirrorX);
        assert_eq!(pos(&s), start);
    }

    #[cfg(feature = "passive")]
    #[test]
    fn dragging_a_selected_element_moves_it_by_whole_grid_steps() {
        let mut s = session();
        press(&mut s, 32.0, 0.0);
        assert_eq!(s.selected_indices().len(), 1);
        s.mouse_drag(32.0, 32.0);
        s.mouse_up();
        let i = s.selected_indices()[0];
        assert_eq!(s.sim.elements[i].base().y, 32);
    }

    #[test]
    fn disabled_editing_blocks_changes_but_not_switches() {
        let mut s = session();
        s.set_toggle(Toggle::DisableEditing, true);
        s.set_tool(Tool::Place("ResistorElm".into()));
        let n = s.sim.elements.len();
        press(&mut s, 100.0, 200.0);
        s.mouse_drag(164.0, 200.0);
        s.mouse_up();
        assert_eq!(s.sim.elements.len(), n);
        s.select_all();
        assert!(!s.delete_selected());
    }

    #[test]
    fn zoom_keeps_the_centre_fixed_and_is_clamped() {
        let mut s = session();
        s.view = crate::render::scene::View { scale: 1.0, x: 0.0, y: 0.0 };
        let before = s.view.to_circuit(400.0, 300.0);
        s.zoom_by(2.0);
        assert_eq!(s.view.to_circuit(400.0, 300.0), before);
        for _ in 0..40 {
            s.zoom_by(2.0);
        }
        assert!(s.view.scale <= 10.0);
        s.zoom_100();
        assert_eq!(s.view.scale, 1.0);
    }

    #[test]
    fn toggles_update_options_and_settings() {
        let mut s = session();
        assert!(s.toggle_state(Toggle::ShowCurrent));
        s.set_toggle(Toggle::ShowCurrent, false);
        assert!(!s.toggle_state(Toggle::ShowCurrent));
        assert!(!s.draw_ctx.show_dots);
        s.set_toggle(Toggle::SmallGrid, true);
        assert_eq!(s.grid_size(), 8);
        s.set_toggle(Toggle::EuroResistors, true);
        assert!(s.draw_ctx.euro_resistors);
    }

    #[cfg(feature = "passive")]
    #[cfg(feature = "sources")]
    #[test]
    fn edit_fields_round_trip_through_apply() {
        let mut s = session();
        let mut f = s.edit_fields(1);
        assert!(!f.is_empty());
        f[0].value = 2200.0;
        s.apply_edit_fields(1, f);
        assert_eq!(s.edit_fields(1)[0].value, 2200.0);
        assert!(s.undo());
        assert_eq!(s.edit_fields(1)[0].value, 1000.0);
    }

    #[cfg(feature = "sources")]
    #[test]
    #[cfg(feature = "switches")]
    fn keyboard_shortcut_operates_the_matching_switch() {
        let text = "$ 1 0.000005 10 50 5 50 5e-11\nv 0 64 0 0 0 0 40 5 0 0 0.5\ns 0 0 64 0 0 0 false\nr 64 0 64 64 0 1000\nw 64 64 0 64 0\n";
        let mut s = Session::load(text, &Registry::standard()).unwrap();
        let mut f = s.edit_fields(1);
        f[3].text = Some("x".into());
        s.apply_edit_fields(1, f);
        let pos = |s: &Session| {
            s.sim.elements[1]
                .as_any()
                .downcast_ref::<crate::elements::switches::SwitchElm>()
                .unwrap()
                .position
        };
        assert_eq!(pos(&s), 0);
        assert!(!s.key_toggle("y"));
        assert!(s.key_toggle("x"));
        assert_eq!(pos(&s), 1);
    }

    #[test]
    fn other_options_change_the_time_step() {
        let mut s = session();
        s.apply_other_options(1e-6, 10.0, true, false);
        assert_eq!(s.sim.settings().max_time_step, 1e-6);
        assert_eq!(s.draw_ctx.voltage_range, 10.0);
        assert!(s.sim.settings().adjust_time_step);
    }

    #[test]
    fn mode_text_names_the_active_tool() {
        let mut s = session();
        assert!(s.mode_text().contains("Select/Drag Sel"));
        s.set_tool(Tool::DragAll);
        assert!(s.mode_text().contains("Drag All"));
    }
}
