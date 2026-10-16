// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (MouseManager, CommandManager).
//! Editing a circuit: tools, mouse handling, selection, clipboard, undo, transforms and view.
//!
//! Everything here works on the [`Session`] without GTK, so the editing rules are unit tested.

use super::session::Session;
use crate::elements::element::{EditInfo, Element};
use crate::elements::geometry::{Rect, distance_sq};
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
        !self.settings.disable_editing
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
                    // a negative distance means the element is not under the mouse
                    if !(0..=HOVER_DISTANCE_SQ).contains(&d) {
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
                    (0..e.post_count().min(2)).find(|&n| {
                        e.get_post(n).is_some_and(|p| distance_sq(gx, gy, p.x, p.y) <= POST_GRAB_SQ)
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

    /// The element under surface position (`px`, `py`) that a double click should edit.
    pub fn element_for_edit(&self, px: f64, py: f64) -> Option<usize> {
        let (gx, gy) = self.view.to_circuit(px, py);
        self.element_at(gx, gy).filter(|&i| !self.edit_fields(i).is_empty())
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

    #[test]
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
