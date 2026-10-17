// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (CircuitElm).
//! The [`Element`] trait, shared element state ([`Base`]) and drawing helpers.

use super::geometry::{
    Point, Polygon, Rect, distance, interp_point, interp_point_g, interp_point2, sign, snap_grid,
};
use super::units;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::render::painter::{Color, LineCap, Painter, TextAlign, TextBaseline};
use crate::solver::path::Validator;
use crate::solver::stamper::{NodeId, Stamper, VsId};
use std::any::Any;

/// Number of entries in the voltage colour scale (odd, so ground is gray).
pub const COLOR_SCALE_COUNT: usize = 201;

/// Marker current value meaning "moving too fast to animate".
pub const CURRENT_TOO_FAST: f64 = 100.0;

/// The settings of a scope placed in the circuit, as the file wrote them.
#[derive(Clone, Debug)]
pub enum ScopeSource {
    /// Fields of a text-format entry, joined by `_`.
    Line(String),
    /// The `o` tag of an XML entry.
    Xml(XmlNode),
}

/// Options and colours that influence how elements are drawn.
#[derive(Clone, Debug)]
pub struct DrawCtx {
    /// Colour wires by voltage.
    pub show_volts: bool,
    /// Draw component values next to elements.
    pub show_values: bool,
    /// Colour elements by power.
    pub show_power: bool,
    /// Draw moving current dots.
    pub show_dots: bool,
    /// Draw resistors with the European box symbol.
    pub euro_resistors: bool,
    /// Add the ohm symbol to resistance values.
    pub show_ohm: bool,
    /// Draw logic gates with IEC rectangles.
    pub iec_gates: bool,
    /// Use the small grid.
    pub small_grid: bool,
    /// The simulation is running (current dots animate only then).
    pub sim_running: bool,
    /// Drawing for print or export (white background).
    pub printable: bool,
    /// Voltage mapped to the end of the colour scale.
    pub voltage_range: f64,
    /// Scale from current to dot movement per frame.
    pub current_mult: f64,
    /// Scale from power to colour.
    pub power_mult: f64,
    /// True while the user drags a whole row or column.
    pub dragging_row_or_column: bool,
    /// Voltage colour scale, from most negative to most positive.
    pub color_scale: Vec<Color>,
    /// Colour of selected elements.
    pub select_color: Color,
    /// Colour of ordinary text and posts.
    pub white: Color,
    /// Colour of secondary text and outlines (black on a white background).
    pub light_gray: Color,
    /// Colour of current dots.
    pub current_color: Color,
    /// Font size of values.
    pub value_font_size: i32,
    /// Simulated time, for elements whose drawing depends on it.
    pub sim_time: f64,
}

impl Default for DrawCtx {
    fn default() -> Self {
        let mut ctx = DrawCtx {
            show_volts: true,
            show_values: true,
            show_power: false,
            show_dots: true,
            euro_resistors: false,
            show_ohm: false,
            iec_gates: false,
            small_grid: false,
            sim_running: true,
            printable: false,
            voltage_range: 5.0,
            current_mult: 0.0,
            power_mult: 0.0,
            dragging_row_or_column: false,
            color_scale: Vec::new(),
            select_color: Color::CYAN,
            white: Color::WHITE,
            light_gray: Color::LIGHT_GRAY,
            current_color: Color::YELLOW,
            value_font_size: 12,
            sim_time: 0.0,
        };
        ctx.set_color_scale(Color::GREEN, Color::RED, Color::GRAY);
        ctx
    }
}

impl DrawCtx {
    /// Rebuilds the voltage colour scale from the three anchor colours.
    pub fn set_color_scale(&mut self, positive: Color, negative: Color, neutral: Color) {
        self.color_scale = (0..COLOR_SCALE_COUNT)
            .map(|i| {
                let v = i as f64 * 2.0 / COLOR_SCALE_COUNT as f64 - 1.0;
                if v < 0.0 {
                    Color::mix(neutral, negative, -v)
                } else {
                    Color::mix(neutral, positive, v)
                }
            })
            .collect();
    }

    /// Colour for a voltage, honouring the "show voltage" option.
    pub fn voltage_color(&self, volts: f64, highlight: bool) -> Color {
        if highlight {
            return self.select_color;
        }
        if !self.show_volts {
            return self.white;
        }
        let volts = if volts.is_nan() { 0.0 } else { volts };
        let c = ((volts + self.voltage_range) * (COLOR_SCALE_COUNT - 1) as f64
            / (self.voltage_range * 2.0)) as i64;
        self.color_scale[c.clamp(0, COLOR_SCALE_COUNT as i64 - 1) as usize]
    }

    /// Colour for a power value (call only when power display is on).
    pub fn power_color(&self, w: f64, highlight: bool) -> Color {
        if highlight {
            return self.select_color;
        }
        let w = w * self.power_mult;
        let half = (COLOR_SCALE_COUNT / 2) as f64;
        let i = (half + half * -w) as i64;
        self.color_scale[i.clamp(0, COLOR_SCALE_COUNT as i64 - 1) as usize]
    }
}

/// Which mirror operation was applied to an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flip {
    /// Left to right.
    X,
    /// Top to bottom.
    Y,
    /// About the diagonal.
    Xy,
}

/// How the dialog should show a field of an element.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditInfo {
    /// Label.
    pub name: String,
    /// Text value (for text fields).
    pub text: Option<String>,
    /// Numeric value.
    pub value: f64,
    /// Smallest allowed value (slider range).
    pub min_val: f64,
    /// Largest allowed value (slider range).
    pub max_val: f64,
    /// Options for a drop-down choice.
    pub choice: Option<Vec<String>>,
    /// Selected option of the choice.
    pub choice_index: usize,
    /// State of a checkbox field.
    pub checkbox: Option<bool>,
    /// Label of a button field.
    pub button: Option<String>,
    /// The text field holds several lines.
    pub multiline: bool,
    /// Value has no unit.
    pub dimensionless: bool,
    /// Step by plain +/-1 instead of E12 values.
    pub unit_step: bool,
    /// Do not offer a slider for this field.
    pub no_sliders: bool,
    /// The text is a colour.
    pub is_color: bool,
    /// Start a new dialog column.
    pub new_column: bool,
    /// Value must be positive.
    pub positive: bool,
    /// Value must not be negative.
    pub non_negative: bool,
    /// Error message set by the element after validation.
    pub error: Option<String>,
    /// Set by the element when the edit changes which fields exist, so the dialog rebuilds.
    pub new_dialog: bool,
    /// The text is a file path and the dialog offers a file chooser for it.
    pub file_chooser: Option<FileChoice>,
}

/// What a file chooser next to a path field asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileChoice {
    /// An existing file; the patterns (`*.wav`) of the files to show, besides all files.
    Open(Vec<String>),
    /// A file to write, with a suggested name.
    Save(String),
}

impl EditInfo {
    /// Numeric field with a slider range.
    pub fn new(name: &str, value: f64, min_val: f64, max_val: f64) -> Self {
        EditInfo { name: name.into(), value, min_val, max_val, ..Default::default() }
    }

    /// Numeric field without a slider range.
    pub fn number(name: &str, value: f64) -> Self {
        EditInfo { name: name.into(), value, ..Default::default() }
    }

    /// Text field.
    pub fn text(name: &str, text: &str) -> Self {
        EditInfo {
            name: name.into(),
            text: Some(text.into()),
            dimensionless: true,
            no_sliders: true,
            ..Default::default()
        }
    }

    /// Multi-line text field.
    pub fn text_area(name: &str, text: &str) -> Self {
        EditInfo { multiline: true, ..EditInfo::text(name, text) }
    }

    /// Checkbox field.
    pub fn checkbox(name: &str, state: bool) -> Self {
        EditInfo {
            name: String::new(),
            checkbox: Some(state),
            min_val: -1.0,
            max_val: -1.0,
            ..Default::default()
        }
        .with_checkbox_label(name)
    }

    fn with_checkbox_label(mut self, label: &str) -> Self {
        self.text = Some(label.into());
        self
    }

    /// Drop-down field.
    pub fn choice(name: &str, options: Vec<String>, selected: usize) -> Self {
        EditInfo {
            name: name.into(),
            choice: Some(options),
            choice_index: selected,
            ..Default::default()
        }
    }

    /// Marks the value as dimensionless.
    pub fn set_dimensionless(mut self) -> Self {
        self.dimensionless = true;
        self
    }

    /// Offers a chooser for an existing file whose name matches one of `patterns`.
    pub fn open_file(mut self, patterns: &[&str]) -> Self {
        self.file_chooser = Some(FileChoice::Open(patterns.iter().map(|p| (*p).into()).collect()));
        self
    }

    /// Offers a chooser for a file to write, suggesting `name`.
    pub fn save_file(mut self, name: &str) -> Self {
        self.file_chooser = Some(FileChoice::Save(name.into()));
        self
    }

    /// Steps by plain +/-1.
    pub fn set_unit_step(mut self) -> Self {
        self.unit_step = true;
        self
    }

    /// Disallows sliders.
    pub fn disallow_sliders(mut self) -> Self {
        self.no_sliders = true;
        self
    }

    /// Marks the field as a colour.
    pub fn set_is_color(mut self) -> Self {
        self.is_color = true;
        self
    }

    /// Starts a new column before this field.
    pub fn new_column(mut self) -> Self {
        self.new_column = true;
        self
    }

    /// Requires a positive value.
    pub fn set_positive(mut self) -> Self {
        self.positive = true;
        self
    }

    /// Requires a non-negative value.
    pub fn set_non_negative(mut self) -> Self {
        self.non_negative = true;
        self
    }

    /// Sets or clears `bit` in `flags` according to the checkbox.
    pub fn change_flag(&self, flags: i32, bit: i32) -> i32 {
        if self.checkbox.unwrap_or(false) { flags | bit } else { flags & !bit }
    }

    /// Like [`EditInfo::change_flag`] with the checkbox meaning inverted.
    pub fn change_flag_inverted(&self, flags: i32, bit: i32) -> i32 {
        if !self.checkbox.unwrap_or(false) { flags | bit } else { flags & !bit }
    }

    /// True when a slider can be created for this field.
    pub fn can_create_adjustable(&self) -> bool {
        self.choice.is_none()
            && self.checkbox.is_none()
            && self.button.is_none()
            && !self.no_sliders
    }
}

/// A slider an element adds to the side panel (for example a variable voltage source).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementSlider {
    /// Label shown above the slider.
    pub text: String,
    /// Position from 0 to 100.
    pub value: i32,
}

/// State shared by all elements: geometry, terminal voltages and current.
#[derive(Clone, Debug, Default)]
pub struct Base {
    /// First post (x).
    pub x: i32,
    /// First post (y).
    pub y: i32,
    /// Second post (x).
    pub x2: i32,
    /// Second post (y).
    pub y2: i32,
    /// Element flag bits.
    pub flags: i32,
    /// Horizontal extent.
    pub dx: i32,
    /// Vertical extent.
    pub dy: i32,
    /// Sign of the dominant extent.
    pub dsign: i32,
    /// Element length.
    pub dn: f64,
    /// Unit vector perpendicular to the element (x).
    pub dpx1: f64,
    /// Unit vector perpendicular to the element (y).
    pub dpy1: f64,
    /// First post as a point.
    pub point1: Point,
    /// Second post as a point.
    pub point2: Point,
    /// End of the first lead stub.
    pub lead1: Point,
    /// End of the second lead stub.
    pub lead2: Point,
    /// Voltage at each node (posts first, then internal nodes).
    pub volts: Vec<f64>,
    /// Solver node of each post and internal node.
    pub nodes: Vec<NodeId>,
    /// The element's voltage source, for elements that have exactly one.
    pub volt_source: Option<VsId>,
    /// Current through a two-terminal element.
    pub current: f64,
    /// Animated current dot position.
    pub curcount: f64,
    /// Bounding box used for selection.
    pub bbox: Rect,
    /// Restrict the element to horizontal or vertical placement.
    pub no_diagonal: bool,
    /// The user has selected the element.
    pub selected: bool,
    /// Highlighted by hover, scope role or net highlight (set by the application).
    pub highlight: bool,
    /// The element is plotted by the scope the pointer is over: what it is there ("X", "Y",
    /// or empty when the scope has no axes). It is drawn highlighted and its terminals are
    /// marked.
    pub scope_role: Option<String>,
    /// The user is dragging the element out right now.
    pub creating: bool,
    /// Which end handle was last grabbed, or -1.
    pub last_handle_grabbed: i32,
}

impl Base {
    /// Element with one post at (`xx`, `yy`), to be dragged out by the user.
    pub fn new(xx: i32, yy: i32) -> Self {
        let mut b =
            Base { x: xx, y: yy, x2: xx, y2: yy, last_handle_grabbed: -1, ..Base::default() };
        b.init_bounding_box();
        b
    }

    /// Element between two posts, as read from a file.
    pub fn from_dump(xa: i32, ya: i32, xb: i32, yb: i32, f: i32) -> Self {
        let mut b = Base {
            x: xa,
            y: ya,
            x2: xb,
            y2: yb,
            flags: f,
            last_handle_grabbed: -1,
            ..Base::default()
        };
        b.init_bounding_box();
        b
    }

    /// Resets the selection box to the rectangle spanned by the two posts.
    pub fn init_bounding_box(&mut self) {
        self.bbox.set_bounds(
            self.x.min(self.x2),
            self.y.min(self.y2),
            (self.x2 - self.x).abs() + 1,
            (self.y2 - self.y).abs() + 1,
        );
    }

    /// Allocates node and voltage arrays for `n` nodes, keeping them when the size is unchanged.
    pub fn alloc_nodes(&mut self, n: usize) {
        if self.nodes.len() != n {
            self.nodes = vec![NodeId::GROUND; n];
            self.volts = vec![0.0; n];
        }
    }

    /// Calculates post positions and the convenience values used for drawing.
    pub fn set_points(&mut self) {
        self.dx = self.x2 - self.x;
        self.dy = self.y2 - self.y;
        self.dn = f64::from(self.dx * self.dx + self.dy * self.dy).sqrt();
        self.dpx1 = f64::from(self.dy) / self.dn;
        self.dpy1 = -f64::from(self.dx) / self.dn;
        self.dsign = if self.dy == 0 { sign(self.dx) } else { sign(self.dy) };
        self.point1 = Point::new(self.x, self.y);
        self.point2 = Point::new(self.x2, self.y2);
    }

    /// Calculates lead points for an element of length `len`.
    pub fn calc_leads(&mut self, len: i32) {
        if self.dn < f64::from(len) || len == 0 {
            self.lead1 = self.point1;
            self.lead2 = self.point2;
            return;
        }
        let len = f64::from(len);
        self.lead1 = interp_point(self.point1, self.point2, (self.dn - len) / (2.0 * self.dn));
        self.lead2 = interp_point(self.point1, self.point2, (self.dn + len) / (2.0 * self.dn));
    }

    /// Moves both leads so an element with an odd lead length sits on the grid.
    pub fn adjust_leads_to_grid(&mut self, flip_x: bool, flip_y: bool) {
        let cx = (self.point1.x + self.point2.x) / 2;
        let cy = (self.point1.y + self.point2.y) / 2;
        let round_x = if flip_x { 1 } else { -1 };
        let round_y = if flip_y { 1 } else { -1 };
        let adj_x = snap_grid(cx + round_x) - cx;
        let adj_y = snap_grid(cy + round_y) - cy;
        for l in [&mut self.lead1, &mut self.lead2] {
            l.x += adj_x;
            l.y += adj_y;
        }
    }

    /// Sets the selection box to the rectangle through the two points.
    pub fn set_bbox(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let (x1, x2) = if x1 > x2 { (x2, x1) } else { (x1, x2) };
        let (y1, y2) = if y1 > y2 { (y2, y1) } else { (y1, y2) };
        self.bbox.set_bounds(x1, y1, x2 - x1 + 1, y2 - y1 + 1);
    }

    /// Sets the selection box for an element from `p1` to `p2` with half-width `w`.
    pub fn set_bbox_pts(&mut self, p1: Point, p2: Point, w: f64) {
        self.set_bbox(p1.x, p1.y, p2.x, p2.y);
        let dpx = (self.dpx1 * w) as i32;
        let dpy = (self.dpy1 * w) as i32;
        self.adjust_bbox(p1.x + dpx, p1.y + dpy, p1.x - dpx, p1.y - dpy);
    }

    /// Grows the selection box to contain the rectangle through the two points.
    pub fn adjust_bbox(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let (x1, x2) = if x1 > x2 { (x2, x1) } else { (x1, x2) };
        let (y1, y2) = if y1 > y2 { (y2, y1) } else { (y1, y2) };
        let x1 = self.bbox.x.min(x1);
        let y1 = self.bbox.y.min(y1);
        let x2 = (self.bbox.x + self.bbox.width).max(x2);
        let y2 = (self.bbox.y + self.bbox.height).max(y2);
        self.bbox.set_bounds(x1, y1, x2 - x1, y2 - y1);
    }

    /// Grows the selection box to contain two points.
    pub fn adjust_bbox_pts(&mut self, p1: Point, p2: Point) {
        self.adjust_bbox(p1.x, p1.y, p2.x, p2.y);
    }

    /// Voltage difference between the first two nodes; an element with a single post is
    /// measured against ground.
    pub fn voltage_diff(&self) -> f64 {
        self.volts.first().copied().unwrap_or(0.0) - self.volts.get(1).copied().unwrap_or(0.0)
    }

    /// True when the element must be drawn highlighted.
    pub fn needs_highlight(&self) -> bool {
        self.selected || self.highlight || self.scope_role.is_some()
    }

    /// Squared distance from grid point (`gx`, `gy`) to the line through the two posts.
    pub fn line_distance_sq(&self, gx: i32, gy: i32) -> i32 {
        let (xa, ya, xb, yb) =
            (i64::from(self.x), i64::from(self.y), i64::from(self.x2), i64::from(self.y2));
        let (gx, gy) = (i64::from(gx), i64::from(gy));
        let dtop = (yb - ya) * gx - (xb - xa) * gy + xb * ya - yb * xa;
        let dbot = (yb - ya) * (yb - ya) + (xb - xa) * (xb - xa);
        // 64-bit arithmetic: the original's 32-bit product overflows for far-away points
        if dbot == 0 {
            return i32::MAX;
        }
        i32::try_from(dtop * dtop / dbot).unwrap_or(i32::MAX)
    }

    /// Updates current dot positions for a current `cur` and previous position `cc`.
    pub fn update_dot_count_for(&self, cur: f64, cc: f64, ctx: &DrawCtx) -> f64 {
        if !ctx.sim_running {
            return cc;
        }
        let mut cadd = cur * ctx.current_mult;
        if !(-6.0..=6.0).contains(&cadd) {
            return CURRENT_TOO_FAST;
        }
        let cc = if cc == CURRENT_TOO_FAST { 0.0 } else { cc };
        cadd %= 8.0;
        cc + cadd
    }

    /// Updates [`Base::curcount`] from [`Base::current`].
    pub fn update_dot_count(&mut self, ctx: &DrawCtx) {
        self.curcount = self.update_dot_count_for(self.current, self.curcount, ctx);
    }

    /// Adds `a` to the dot position `c` unless it is marked too fast.
    pub fn add_cur_count(c: f64, a: f64) -> f64 {
        if c == CURRENT_TOO_FAST || c == -CURRENT_TOO_FAST { c } else { c + a }
    }

    /// Moves the element by (`dx`, `dy`).
    pub fn move_by(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
        self.x2 += dx;
        self.y2 += dy;
        self.bbox.translate(dx, dy);
        self.set_points();
    }

    /// Moves the second post to (`xx`, `yy`), already snapped to the grid.
    pub fn drag(&mut self, xx: i32, yy: i32) {
        let (mut xx, mut yy) = (xx, yy);
        if self.no_diagonal {
            if (self.x - xx).abs() < (self.y - yy).abs() {
                xx = self.x;
            } else {
                yy = self.y;
            }
        }
        self.x2 = xx;
        self.y2 = yy;
        self.set_points();
    }

    /// Swaps the two posts.
    pub fn flip_posts(&mut self) {
        std::mem::swap(&mut self.x, &mut self.x2);
        std::mem::swap(&mut self.y, &mut self.y2);
        self.set_points();
    }

    /// Mirrors the element about a vertical axis.
    pub fn flip_x(&mut self, center2: i32) {
        self.x = center2 - self.x;
        self.x2 = center2 - self.x2;
        self.init_bounding_box();
        self.set_points();
    }

    /// Mirrors the element about a horizontal axis.
    pub fn flip_y(&mut self, center2: i32) {
        self.y = center2 - self.y;
        self.y2 = center2 - self.y2;
        self.init_bounding_box();
        self.set_points();
    }

    /// Mirrors the element about the diagonal x - y = `xmy`.
    pub fn flip_xy(&mut self, xmy: i32) {
        let nx = self.y + xmy;
        let ny = self.x - xmy;
        let nx2 = self.y2 + xmy;
        let ny2 = self.x2 - xmy;
        self.x = nx;
        self.y = ny;
        self.x2 = nx2;
        self.y2 = ny2;
        self.init_bounding_box();
        self.set_points();
    }

    /// Sets the position of an internal element, so it can be drawn inside its parent.
    pub fn set_position(&mut self, x: i32, y: i32, x2: i32, y2: i32) {
        self.x = x;
        self.y = y;
        self.x2 = x2;
        self.y2 = y2;
        self.init_bounding_box();
        self.set_points();
    }

    /// Dump prefix shared by every element: `type x y x2 y2 flags`.
    pub fn dump_prefix(&self, dump_type: i32) -> String {
        let head = if dump_type < 127 {
            format!("{} ", char::from(dump_type as u8))
        } else {
            format!("{dump_type} ")
        };
        format!("{head}{} {} {} {} {}", self.x, self.y, self.x2, self.y2, self.flags)
    }
}

/// One end of a wire segment, used to work out wire currents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Endpoint {
    /// Not connected through to anything.
    None,
    /// Connected at a grid point.
    Point(Point),
    /// Connected by sharing a label name with other labelled nodes.
    Label(String),
}

/// Registry of labelled-node and ground positions used while merging wires into nodes.
#[derive(Default, Debug)]
pub struct LabelRegistry {
    points: std::collections::HashMap<String, Point>,
    first_ground: Option<Point>,
}

impl LabelRegistry {
    /// Returns the first post registered under `name`, registering `p` when it is the first.
    pub fn connect(&mut self, name: &str, p: Point) -> Option<Point> {
        match self.points.get(name) {
            Some(first) => Some(*first),
            None => {
                self.points.insert(name.to_string(), p);
                None
            }
        }
    }

    /// Same as [`LabelRegistry::connect`] for ground elements: all grounds share one node.
    pub fn connect_ground(&mut self, p: Point) -> Option<Point> {
        match self.first_ground {
            Some(first) => Some(first),
            None => {
                self.first_ground = Some(p);
                None
            }
        }
    }

    /// Forgets all names and the first ground.
    pub fn clear(&mut self) {
        self.points.clear();
        self.first_ground = None;
    }
}

/// What an element is for the detection of bus widths (see `Simulation::analyze`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusRole {
    /// Not part of the propagation.
    None,
    /// A wire: takes the widest bus at either end.
    Wire,
    /// A labeled node (with its name): takes the widest bus at its post or under its name.
    Label(String),
    /// A bus splitter: its bus side is at `anchor` and has `bits` bits.
    Splitter {
        /// Position of the bus side.
        anchor: Point,
        /// Number of bits.
        bits: usize,
    },
}

/// Behaviour of a circuit element. Defaults mirror the original `CircuitElm`.
pub trait Element: Any {
    /// Shared state.
    fn base(&self) -> &Base;
    /// Shared state, mutable.
    fn base_mut(&mut self) -> &mut Base;
    /// The element as [`Any`], for downcasting.
    fn as_any(&self) -> &dyn Any;
    /// The element as mutable [`Any`], for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Class name used by the original (for example `ResistorElm`).
    fn class_name(&self) -> &'static str;
    /// Type code used in circuit files.
    fn dump_type(&self) -> i32 {
        0
    }

    /// Name of the tag used in XML circuit files.
    fn xml_dump_type(&self) -> String {
        let t = self.dump_type();
        if t > 64 && t < 127 {
            char::from(t as u8).to_string()
        } else {
            self.class_name().replace("Elm", "")
        }
    }

    /// Flags a freshly placed element starts with.
    fn default_flags(&self) -> i32 {
        0
    }

    /// Text line describing the element for saving.
    fn dump(&self) -> String {
        self.base().dump_prefix(self.dump_type())
    }

    /// Writes the element's persistent fields (position, flags and parameters) to an XML tag.
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(self.base(), out);
    }

    /// The XML tag of the device model this element uses, when it is a model that files must
    /// carry (not a built-in one).
    fn dump_model_xml(&self) -> Option<XmlOut> {
        None
    }

    /// Writes the element's runtime state (for example capacitor voltage) to an XML tag.
    fn dump_xml_state(&self, _out: &mut XmlOut) {}

    /// Reads element state from an XML tag (the part after position and flags).
    fn undump_xml(&mut self, xml: &XmlNode) {
        let f = xml.int_attr("f", self.base().flags);
        self.base_mut().flags = f;
    }

    /// For a scope placed in the circuit: how its settings are written in the file.
    fn scope_source(&self) -> Option<ScopeSource> {
        None
    }
    /// Receives what the scope placed in the circuit shows (see [`Element::scope_source`]).
    fn set_scope_view(&mut self, _view: crate::scope::ScopeView) {}
    /// What the scope placed in the circuit showed last.
    fn scope_view(&self) -> Option<&crate::scope::ScopeView> {
        None
    }
    /// Replaces the settings of the scope placed in the circuit.
    fn set_scope_source(&mut self, _source: ScopeSource) {}

    /// For an instance of a subcircuit model that can be edited: the name of the model.
    fn subcircuit_model_name(&self) -> Option<String> {
        None
    }

    /// The elements this one is made of, for composites (subcircuits and the like).
    fn child_elements(&self) -> &[Box<dyn Element>] {
        &[]
    }
    /// The same, to draw or to hand state to.
    fn child_elements_mut(&mut self) -> &mut [Box<dyn Element>] {
        &mut []
    }

    /// Number of input pairs of a current-controlled source in "spice style", which senses
    /// the current through voltage sources of its own circuit instead of creating its own.
    fn spice_pairs(&self) -> usize {
        0
    }
    /// Gives input pair `i` the voltage source it senses (see [`Element::spice_pairs`]).
    fn set_spice_source(&mut self, _i: usize, _vs: VsId) {}
    /// Gives input pair `i` the current of the voltage source it senses.
    fn set_spice_current(&mut self, _i: usize, _current: f64) {}

    /// Sets the high logic voltage of a digital element (used by subcircuit instances).
    fn set_high_voltage(&mut self, _hv: f64) {}

    /// Number of posts the user can connect to.
    fn post_count(&self) -> usize {
        2
    }
    /// Width in bits of post `n` (1 for ordinary posts).
    fn post_width(&self, _n: usize) -> usize {
        1
    }
    /// Number of bits when the element is a bus wire.
    fn bus_width(&self) -> usize {
        1
    }
    /// Labels of the buttons the edit dialog shows for the element's device model (create a
    /// new model, edit this one).
    fn model_buttons(&self) -> Vec<String> {
        Vec::new()
    }
    /// Starts the model dialog for button `button` of [`Element::model_buttons`].
    fn start_model_edit(&self, _button: usize) -> Option<super::model_edit::ModelEditor> {
        None
    }
    /// Makes the element use the model called `name` (after the model dialog).
    fn use_model(&mut self, _name: &str) {}
    /// Looks the element's model up again after a model of the library changed.
    fn update_models(&mut self) {}
    /// How the element takes part in the detection of bus widths.
    fn bus_role(&self) -> BusRole {
        BusRole::None
    }
    /// Gives a bus wire or bus label the width the detection found; true when it changed.
    fn apply_bus_width(&mut self, _width: usize) -> bool {
        false
    }
    /// Number of voltage sources (extra matrix rows) the element needs.
    fn voltage_source_count(&self) -> usize {
        0
    }
    /// Number of internal nodes.
    fn internal_node_count(&self) -> usize {
        0
    }
    /// Total node count.
    fn node_count(&self) -> usize {
        self.post_count() + self.internal_node_count()
    }
    /// True if the matrix must be restamped every iteration.
    fn non_linear(&self) -> bool {
        false
    }

    /// Allocates node arrays after construction or after the node count changes.
    fn alloc_nodes(&mut self) {
        let n = self.node_count();
        self.base_mut().alloc_nodes(n);
    }

    /// Calculates post positions and drawing helpers; call after any geometry change.
    fn set_points(&mut self) {
        self.base_mut().set_points();
    }

    /// Position of post `n`.
    fn get_post(&self, n: usize) -> Option<Point> {
        match n {
            0 => Some(self.base().point1),
            1 => Some(self.base().point2),
            _ => None,
        }
    }

    /// The post that post `bit` is wired through to, for wire-like elements.
    fn connected_post(&self, _bit: usize, _labels: &mut LabelRegistry) -> Option<Point> {
        Some(self.base().point2)
    }

    /// Both ends of wire segment `bit`, for the wire current calculation.
    fn wire_endpoints(&self, bit: usize, labels: &mut LabelRegistry) -> (Endpoint, Endpoint) {
        let p0 = self.get_post(bit);
        let p1 = self.connected_post(bit, labels);
        let ep0 = p0.map_or(Endpoint::None, Endpoint::Point);
        let ep1 = match p1 {
            Some(p) if Some(p) != p0 => Endpoint::Point(p),
            _ => Endpoint::None,
        };
        (ep0, ep1)
    }

    /// Index of the post located at `pt` (0 when none matches).
    fn node_at_point(&self, pt: Point) -> usize {
        (0..self.post_count()).find(|&i| self.get_post(i) == Some(pt)).unwrap_or(0)
    }

    /// Called before nodes are allocated so elements can set up internal counts.
    fn pre_stamp(&mut self) {}
    /// Stamps the constant part of the matrix.
    fn stamp(&mut self, _s: &mut Stamper) {}
    /// Stamps the nonlinear part; called every iteration.
    fn do_step(&mut self, _s: &mut Stamper) {}
    /// Called at the start of every time step.
    fn start_iteration(&mut self, _s: &Stamper) {}
    /// Called after every successful time step.
    fn step_finished(&mut self, _s: &mut Stamper) {}
    /// Handles the reset button.
    fn reset(&mut self) {
        let b = self.base_mut();
        b.volts.iter_mut().for_each(|v| *v = 0.0);
        b.curcount = 0.0;
    }

    /// Tells the element which solver node its post `p` is.
    fn set_node(&mut self, p: usize, n: NodeId) {
        self.base_mut().nodes[p] = n;
    }
    /// The two nodes the element's voltage source connects, for elements whose source moves
    /// between nodes (a switch); the solver needs them to put the source in the right matrix
    /// before any stamping happens.
    fn voltage_source_nodes(&self) -> Option<(NodeId, NodeId)> {
        None
    }
    /// Like [`Element::voltage_source_nodes`] for the element's `j`th source, for elements
    /// that own several.
    fn voltage_source_nodes_at(&self, _j: usize) -> Option<(NodeId, NodeId)> {
        self.voltage_source_nodes()
    }
    /// Tells the element about its `n`th voltage source.
    fn set_voltage_source(&mut self, _n: usize, v: VsId) {
        self.base_mut().volt_source = Some(v);
    }
    /// Sets the voltage of node `n`, called by the solver.
    fn set_node_voltage(&mut self, n: usize, v: f64) {
        self.base_mut().volts[n] = v;
        self.calculate_current();
    }
    /// Recomputes current after the node voltages changed.
    fn calculate_current(&mut self) {}
    /// Sets the current of voltage source `vs`.
    fn set_current(&mut self, _vs: VsId, c: f64) {
        self.base_mut().current = c;
    }
    /// Sets the current of a wire-like element.
    fn set_wire_current(&mut self, _bit: usize, c: f64) {
        self.base_mut().current = c;
    }
    /// Current through a one- or two-terminal element.
    fn get_current(&self) -> f64 {
        self.base().current
    }
    /// Current flowing into node `n` out of this element.
    fn get_current_into_node(&self, n: usize) -> f64 {
        if n == 0 && self.post_count() == 2 { -self.base().current } else { self.base().current }
    }
    /// Voltage at post `n`.
    fn get_post_voltage(&self, n: usize) -> f64 {
        self.base().volts[n]
    }
    /// Voltage across the element.
    fn get_voltage_diff(&self) -> f64 {
        self.base().voltage_diff()
    }
    /// Power dissipated.
    fn get_power(&self) -> f64 {
        self.get_voltage_diff() * self.base().current
    }

    /// Are nodes `n1` and `n2` connected through this element?
    fn get_connection(&self, _n1: usize, _n2: usize) -> bool {
        true
    }
    /// Are nodes `n1` and `n2` in the same matrix?
    fn get_matrix_connection(&self, n1: usize, n2: usize) -> bool {
        self.get_connection(n1, n2)
    }
    /// Is node `n1` connected to ground by this element?
    fn has_ground_connection(&self, _n1: usize) -> bool {
        false
    }
    /// Is this a wire or equivalent (for circuit validation)?
    fn is_wire_equivalent(&self) -> bool {
        false
    }
    /// Can the node merging step remove this element from the matrix?
    fn is_removable_wire(&self) -> bool {
        false
    }
    /// Is this a ground element?
    fn is_ground(&self) -> bool {
        false
    }
    /// Is this a rail (voltage source to ground)?
    fn is_rail(&self) -> bool {
        false
    }
    /// Is this a plain voltage source?
    fn is_voltage_elm(&self) -> bool {
        false
    }
    /// Is this a battery?
    fn is_battery(&self) -> bool {
        false
    }
    /// Is this an ideal capacitor?
    fn is_ideal_capacitor(&self) -> bool {
        false
    }
    /// Checks the element is usable. Returns false to make the solver analyse the circuit
    /// again (after the element changed itself) or after it asked the solver to stop.
    /// `idx` is the element's index in the circuit.
    fn validate(&mut self, _idx: usize, _v: &mut Validator<'_>) -> bool {
        true
    }

    /// State the drawing depends on that is not a voltage or a current (a lamp's temperature,
    /// a relay's armature position). The worker publishes it with every frame.
    fn visual_state(&self) -> Vec<f64> {
        Vec::new()
    }
    /// Receives the state published by [`Element::visual_state`] on the worker's copy.
    fn set_visual_state(&mut self, _state: &[f64]) {}

    /// Draws the element.
    fn draw(&mut self, _g: &mut dyn Painter, _c: &DrawCtx) {}

    /// Value plotted by a scope for value kind `x` (see the `scope::VAL_*` constants).
    fn scope_value(&self, x: i32) -> f64 {
        match x {
            crate::scope::VAL_CURRENT => self.get_current(),
            crate::scope::VAL_POWER => self.get_power(),
            _ => self.get_voltage_diff(),
        }
    }
    /// Units of the value plotted for value kind `x`.
    fn scope_units(&self, x: i32) -> i32 {
        match x {
            crate::scope::VAL_CURRENT => crate::scope::UNITS_A,
            crate::scope::VAL_POWER => crate::scope::UNITS_W,
            _ => crate::scope::UNITS_V,
        }
    }
    /// Can a scope show value kind `v` for this element?
    fn can_show_value_in_scope(&self, _v: i32) -> bool {
        false
    }
    /// The value a new scope plots first (the voltage unless the element has a better one).
    fn default_scope_value(&self) -> i32 {
        crate::scope::VAL_VOLTAGE
    }
    /// Values besides voltage, current and power that a scope can plot for the element.
    fn scope_values(&self) -> Vec<i32> {
        if self.can_show_value_in_scope(crate::scope::VAL_R) {
            vec![crate::scope::VAL_R]
        } else {
            Vec::new()
        }
    }
    /// Can the element be viewed in a scope?
    fn can_view_in_scope(&self) -> bool {
        self.post_count() <= 2
    }

    /// Component information lines shown in the lower right corner.
    fn get_info(&self, _arr: &mut Vec<String>) {}
    /// Element type name for dialog titles.
    fn elm_type(&self) -> Option<String> {
        let mut info = Vec::new();
        self.get_info(&mut info);
        info.into_iter().next()
    }
    /// Text describing the element in a scope.
    fn scope_text(&self, _v: i32) -> Option<String> {
        self.elm_type()
    }

    /// Edit dialog field `n`, or none past the last.
    fn get_edit_info(&self, _n: usize) -> Option<EditInfo> {
        None
    }
    /// Applies edit dialog field `n`.
    fn set_edit_value(&mut self, _n: usize, _ei: &mut EditInfo) {}
    /// The slider this element wants in the side panel, if any.
    fn slider(&self) -> Option<ElementSlider> {
        None
    }
    /// Moves the element's slider (0 to 100).
    fn set_slider_value(&mut self, _v: i32) {}

    /// Key that inserts this element, or 0.
    fn shortcut(&self) -> i32 {
        0
    }

    /// True when the element can be mirrored horizontally.
    fn can_flip_x(&self) -> bool {
        true
    }
    /// True when the element can be mirrored vertically.
    fn can_flip_y(&self) -> bool {
        true
    }
    /// Number of end handles the user can drag.
    fn num_handles(&self) -> usize {
        self.post_count()
    }
    /// Where handle `n` is, for the user to grab and drag: a post by default.
    fn handle_point(&self, n: usize) -> Option<super::geometry::Point> {
        self.get_post(n)
    }
    /// Default length when placed from the toolbar.
    fn drag_length(&self) -> i32 {
        64
    }
    /// Is this element a graphic annotation with no electrical function?
    fn is_graphic(&self) -> bool {
        false
    }
    /// Called when the user finishes dragging out the element; true if it must be discarded.
    fn creation_failed(&self) -> bool {
        let b = self.base();
        b.x == b.x2 && b.y == b.y2
    }
    /// Moves the element's second post while the user drags the element out or moves a handle.
    fn drag(&mut self, x: i32, y: i32) {
        self.base_mut().drag(x, y);
        self.set_points();
    }
    /// True when a selection rectangle drawn by the user selects this element.
    fn selected_by_rect(&self, r: &super::geometry::Rect) -> bool {
        r.intersects(&self.base().bbox)
    }
    /// Area (circuit coordinates) where a click operates the element, for elements such as
    /// switches that react to the mouse.
    fn click_rect(&self) -> Option<super::geometry::Rect> {
        None
    }
    /// Flips an interactive element to its other state, without touching linked elements.
    fn simple_toggle(&mut self) {}
    /// Name shared by interactive elements that must flip together.
    fn link_label(&self) -> Option<&str> {
        None
    }
    /// Group number shared by multi-throw switches that must move together (0 for none).
    fn link_group(&self) -> i32 {
        0
    }
    /// Position, number of positions and whether the element is mirrored, for switches.
    fn switch_state(&self) -> Option<(i32, i32, bool)> {
        None
    }
    /// Moves a switch to position `pos` without touching linked switches.
    fn set_switch_position(&mut self, _pos: i32) {}
    /// Mirrors the element left to right about `center2 / 2`. `count` is how many elements
    /// are being transformed together (chips only move their position when it is not 1).
    fn flip_x(&mut self, center2: i32, _count: usize) {
        self.on_flipped(Flip::X);
        self.base_mut().flip_x(center2);
        self.set_points();
    }
    /// Mirrors the element top to bottom about `center2 / 2`.
    fn flip_y(&mut self, center2: i32, _count: usize) {
        self.on_flipped(Flip::Y);
        self.base_mut().flip_y(center2);
        self.set_points();
    }
    /// Mirrors the element about the diagonal x - y = `xmy`.
    fn flip_xy(&mut self, xmy: i32, _count: usize) {
        self.on_flipped(Flip::Xy);
        self.base_mut().flip_xy(xmy);
        self.set_points();
    }
    /// Called when the element was moved by (`dx`, `dy`), before its points are recomputed.
    fn moved(&mut self, _dx: i32, _dy: i32) {}
    /// Called after the element was mirrored along `axis`, so it can keep its own state
    /// consistent (a switch swaps its position, an op-amp swaps its inputs).
    fn on_flipped(&mut self, _axis: Flip) {}
    /// Key that operates the element from the keyboard.
    fn toggle_key(&self) -> Option<&str> {
        None
    }
    /// True when the element returns to its rest state as soon as the mouse is released.
    fn is_momentary(&self) -> bool {
        false
    }
    /// Squared distance from a mouse position to the element.
    fn mouse_distance(&self, gx: i32, gy: i32) -> i32 {
        let b = self.base();
        if self.post_count() == 0 {
            return super::geometry::distance_sq(gx, gy, (b.x2 + b.x) / 2, (b.y2 + b.y) / 2);
        }
        b.line_distance_sq(gx, gy)
    }
}

/// Validation shared by one-terminal drivers: stops the simulation when node `n` is tied to
/// ground through nothing but wires and voltage sources. Returns false after stopping.
pub fn validate_rail_node(
    v: &mut crate::solver::path::Validator<'_>,
    idx: usize,
    n: NodeId,
) -> bool {
    if v.find_path(crate::solver::path::PathKind::Voltage, idx, n, NodeId::GROUND) {
        v.stop("Path to ground with no resistance!", idx);
        return false;
    }
    true
}

/// Writes the attributes every element has: position and flags.
pub fn default_dump_xml(b: &Base, out: &mut XmlOut) {
    out.str_attr("x", &format!("{} {} {} {}", b.x, b.y, b.x2, b.y2));
    out.int_attr("f", b.flags);
}

/// Implements the boilerplate accessors of [`Element`] for a type with a `base: Base` field.
/// With a class name literal it also implements `class_name`; without one the type provides
/// `class_name` itself (for types that stand for several classes).
#[macro_export]
macro_rules! impl_element_base {
    () => {
        fn base(&self) -> &$crate::elements::element::Base {
            &self.base
        }
        fn base_mut(&mut self) -> &mut $crate::elements::element::Base {
            &mut self.base
        }
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn ::std::any::Any {
            self
        }
    };
    ($class:literal) => {
        $crate::impl_element_base!();
        fn class_name(&self) -> &'static str {
            $class
        }
    };
}

/// Draws a thick line between two points.
pub fn draw_thick_line(g: &mut dyn Painter, a: Point, b: Point) {
    draw_thick_line_w(g, a, b, 3.0);
}

/// Draws a line of the given width between two points, then restores width 1.
pub fn draw_thick_line_w(g: &mut dyn Painter, a: Point, b: Point, width: f64) {
    g.set_line_width(width);
    g.draw_line(a.x, a.y, b.x, b.y);
    g.set_line_width(1.0);
}

/// Draws a thick closed polyline through the polygon's points.
pub fn draw_thick_polygon(g: &mut dyn Painter, p: &Polygon) {
    g.set_line_width(3.0);
    let pts: Vec<(i32, i32)> = p.points.iter().map(|q| (q.x, q.y)).collect();
    g.draw_polyline(&pts);
    g.set_line_width(1.0);
}

/// Draws a thin closed polyline through the polygon's points.
pub fn draw_polygon(g: &mut dyn Painter, p: &Polygon) {
    let pts: Vec<(i32, i32)> = p.points.iter().map(|q| (q.x, q.y)).collect();
    g.draw_polyline(&pts);
}

/// Fills the polygon.
pub fn fill_polygon(g: &mut dyn Painter, p: &Polygon) {
    let pts: Vec<(i32, i32)> = p.points.iter().map(|q| (q.x, q.y)).collect();
    g.fill_polygon(&pts);
}

/// Draws a thick circle outline.
pub fn draw_thick_circle(g: &mut dyn Painter, cx: i32, cy: i32, ri: i32) {
    g.set_line_width(3.0);
    g.begin_path();
    g.arc(f64::from(cx), f64::from(cy), f64::from(ri) * 0.98, 0.0, 2.0 * std::f64::consts::PI);
    g.stroke();
    g.set_line_width(1.0);
}

/// Draws a post (connection dot).
pub fn draw_post(g: &mut dyn Painter, c: &DrawCtx, pt: Point) {
    g.set_color(c.white);
    g.fill_oval(pt.x - 3, pt.y - 3, 7, 7);
}

/// Colour for an element terminal at `volts`.
pub fn voltage_color(base: &Base, c: &DrawCtx, volts: f64) -> Color {
    c.voltage_color(volts, base.needs_highlight())
}

/// Sets the drawing colour for a voltage.
pub fn set_voltage_color(g: &mut dyn Painter, base: &Base, c: &DrawCtx, volts: f64) {
    g.set_color(voltage_color(base, c, volts));
}

/// Sets the drawing colour for power `w0` when power display is on.
pub fn set_power_color_w(g: &mut dyn Painter, base: &Base, c: &DrawCtx, w0: f64) {
    if !c.show_power {
        return;
    }
    g.set_color(c.power_color(w0, base.needs_highlight()));
}

/// Draws both leads of a two-terminal element, coloured by terminal voltage.
pub fn draw_2_leads(g: &mut dyn Painter, base: &Base, c: &DrawCtx) {
    set_voltage_color(g, base, c, base.volts[0]);
    draw_thick_line(g, base.point1, base.lead1);
    set_voltage_color(g, base, c, base.volts[1]);
    draw_thick_line(g, base.lead2, base.point2);
}

/// Draws current dots from `pa` to `pb` at animation offset `pos`.
pub fn draw_dots(g: &mut dyn Painter, c: &DrawCtx, pa: Point, pb: Point, pos: f64) {
    if !c.sim_running || pos == 0.0 || !c.show_dots {
        return;
    }
    let dx = pb.x - pa.x;
    let dy = pb.y - pa.y;
    let dn = f64::from(dx * dx + dy * dy).sqrt();
    g.set_color(c.current_color);
    let ds = 16.0;
    let mut pos = pos;
    if pos == CURRENT_TOO_FAST || pos == -CURRENT_TOO_FAST {
        // too fast to follow: draw a translucent line and dots at a pseudo-random offset
        g.save();
        g.set_line_width(4.0);
        g.set_alpha(0.5);
        g.begin_path();
        g.move_to(f64::from(pa.x), f64::from(pa.y));
        g.line_to(f64::from(pb.x), f64::from(pb.y));
        g.stroke();
        g.restore();
        pos = pseudo_random() * ds;
    }
    pos %= ds;
    if pos < 0.0 {
        pos += ds;
    }
    let mut di = pos;
    while di < dn {
        let x0 = (f64::from(pa.x) + di * f64::from(dx) / dn) as i32;
        let y0 = (f64::from(pa.y) + di * f64::from(dy) / dn) as i32;
        g.fill_rect(f64::from(x0 - 2), f64::from(y0 - 2), 4.0, 4.0);
        di += ds;
    }
}

fn pseudo_random() -> f64 {
    use std::cell::Cell;
    thread_local!(static STATE: Cell<u64> = const { Cell::new(0x2545_F491_4F6C_DD1D) });
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}

/// Updates current dots and draws them for a simple two-terminal element.
pub fn do_dots(g: &mut dyn Painter, base: &mut Base, c: &DrawCtx) {
    base.update_dot_count(c);
    if !base.creating {
        draw_dots(g, c, base.point1, base.point2, base.curcount);
    }
}

/// Draws the posts of an element that is selected or being created.
pub fn draw_posts(g: &mut dyn Painter, base: &Base, elm: &dyn Element, c: &DrawCtx) {
    if !base.creating && !base.needs_highlight() {
        return;
    }
    if c.dragging_row_or_column {
        return;
    }
    draw_scope_terminal_labels(g, base, elm, c);
    for i in 0..elm.post_count() {
        if let Some(p) = elm.get_post(i) {
            draw_post(g, c, p);
        }
    }
}

/// While the pointer is over a scope that plots this element: marks its two terminals with
/// "+" and a minus sign beside them, and the role it has in an X/Y plot in the middle.
fn draw_scope_terminal_labels(g: &mut dyn Painter, base: &Base, elm: &dyn Element, c: &DrawCtx) {
    let Some(role) = &base.scope_role else { return };
    if elm.post_count() != 2 || base.dn == 0.0 {
        return;
    }
    let (Some(a), Some(b)) = (elm.get_post(0), elm.get_post(1)) else { return };
    g.save();
    g.set_color(c.select_color);
    g.set_font_size(12);
    g.set_text_align(crate::render::painter::TextAlign::Center);
    g.set_text_baseline(crate::render::painter::TextBaseline::Middle);
    let swap = elm.class_name() == "VoltageElm";
    let (plus, minus) = if swap { ("\u{2212}", "+") } else { ("+", "\u{2212}") };
    let at = |f: f64, side: f64| super::geometry::interp_point_g(a, b, f, side);
    let (pp, mp) = (at(10.0 / base.dn, 8.0), at(1.0 - 10.0 / base.dn, 8.0));
    g.draw_string(plus, f64::from(pp.x), f64::from(pp.y));
    g.draw_string(minus, f64::from(mp.x), f64::from(mp.y));
    if !role.is_empty() {
        g.set_font_size(16);
        let mid = at(0.5, 0.0);
        g.draw_string(role, f64::from(mid.x), f64::from(mid.y));
    }
    g.restore();
}

/// Draws a component value next to the element. `hs` is the offset from the element axis.
pub fn draw_values(
    g: &mut dyn Painter,
    base: &Base,
    c: &DrawCtx,
    s: Option<&str>,
    hs: f64,
    rail_like: bool,
    voltage_like: bool,
) {
    let Some(s) = s else { return };
    g.save();
    g.set_font_size(c.value_font_size);
    let w = g.measure_text(s) as i32;
    g.set_color(c.white);
    let ya = g.font_size() / 2;
    let (xc, yc) = if rail_like {
        (base.x2, base.y2)
    } else {
        ((base.x2 + base.x) / 2, (base.y2 + base.y) / 2)
    };
    let dpx = (base.dpx1 * hs) as i32;
    let dpy = (base.dpy1 * hs) as i32;
    if dpx == 0 {
        g.draw_string(s, f64::from(xc - w / 2), f64::from(yc - dpy.abs() - 2));
    } else {
        let mut xx = xc + dpx.abs() + 2;
        if voltage_like || (base.x < base.x2 && base.y > base.y2) {
            xx = xc - (w + dpx.abs() + 2);
        }
        g.draw_string(s, f64::from(xx), f64::from(yc + dpy + ya));
    }
    g.restore();
}

/// Draws a name next to a labelled node.
pub fn draw_labeled_node(
    g: &mut dyn Painter,
    base: &mut Base,
    c: &DrawCtx,
    text: &str,
    pt1: Point,
    pt2: Point,
) {
    g.save();
    g.set_font_size(c.value_font_size);
    let (line_over, text) = match text.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let w = g.measure_text(text) as i32;
    let h = g.font_size();
    g.set_text_baseline(TextBaseline::Middle);
    let mut x = pt2.x;
    let mut y = pt2.y;
    if pt1.y != pt2.y {
        x -= w / 2;
        y += sign(pt2.y - pt1.y) * h;
    } else if pt2.x > pt1.x {
        x += 4;
    } else {
        x -= 4 + w;
    }
    g.draw_string(text, f64::from(x), f64::from(y));
    base.adjust_bbox(x, y - h / 2, x + w, y + h / 2);
    g.restore();
    if line_over {
        let ya = y - h / 2 - 1;
        g.draw_line(x, ya, x + w, ya);
    }
}

/// Draws text centred (or left aligned) at a point and grows the selection box around it.
pub fn draw_centered_text(g: &mut dyn Painter, base: &mut Base, s: &str, x: i32, y: i32, cx: bool) {
    let w = g.measure_text(s) as i32;
    let h2 = g.font_size() / 2;
    g.save();
    g.set_text_baseline(TextBaseline::Middle);
    if cx {
        g.set_text_align(TextAlign::Center);
        base.adjust_bbox(x - w / 2, y - h2, x + w / 2, y + h2);
    } else {
        base.adjust_bbox(x, y - h2, x + w, y + h2);
    }
    g.draw_string(s, f64::from(x), f64::from(y));
    g.restore();
}

/// Draws a coil (inductor symbol) from `p1` to `p2` with loops of height `hs`.
pub fn draw_coil(
    g: &mut dyn Painter,
    base: &Base,
    c: &DrawCtx,
    hs: i32,
    p1: Point,
    p2: Point,
    v1: f64,
    v2: f64,
) {
    let len = distance(p1, p2);
    g.save();
    g.set_line_width(3.0);
    let (ex, ey) = (f64::from(p2.x - p1.x), f64::from(p2.y - p1.y));
    g.transform(ex / len, ey / len, -ey / len, ex / len, f64::from(p1.x), f64::from(p1.y));
    if c.show_volts {
        g.set_stroke_gradient(len, voltage_color(base, c, v1), voltage_color(base, c, v2));
    }
    g.set_line_cap(LineCap::Round);
    g.scale(1.0, if hs > 0 { 1.0 } else { -1.0 });
    let loop_ct = (len / 11.0).ceil() as i32;
    for lp in 0..loop_ct {
        g.begin_path();
        let start = len * f64::from(lp) / f64::from(loop_ct);
        g.move_to(start, 0.0);
        g.arc(
            len * (f64::from(lp) + 0.5) / f64::from(loop_ct),
            0.0,
            len / (2.0 * f64::from(loop_ct)),
            std::f64::consts::PI,
            std::f64::consts::PI * 2.0,
        );
        g.line_to(len * (f64::from(lp) + 1.0) / f64::from(loop_ct), 0.0);
        g.stroke();
    }
    g.restore();
}

/// The Schmitt-trigger hysteresis symbol polygon.
pub fn schmitt_polygon(base: &Base, gsize: f32, ctr: f32) -> Polygon {
    let hs = f64::from(3.0 * gsize);
    let h1 = f64::from(3.0 * gsize);
    let h2 = h1 * 2.0;
    let len = distance(base.lead1, base.lead2);
    let ctr = f64::from(ctr);
    Polygon::from_points(&[
        interp_point_g(base.lead1, base.lead2, ctr - h2 / len, hs),
        interp_point_g(base.lead1, base.lead2, ctr + h1 / len, hs),
        interp_point_g(base.lead1, base.lead2, ctr + h1 / len, -hs),
        interp_point_g(base.lead1, base.lead2, ctr + h2 / len, -hs),
        interp_point_g(base.lead1, base.lead2, ctr - h1 / len, -hs),
        interp_point_g(base.lead1, base.lead2, ctr - h1 / len, hs),
    ])
}

/// Basic information lines shared by most elements (current and voltage across).
pub fn basic_info(e: &dyn Element, arr: &mut Vec<String>) {
    arr.push(format!("I = {}", units::current_d_text(e.get_current())));
    arr.push(format!("Vd = {}", units::voltage_d_text(e.get_voltage_diff())));
}

/// Two points perpendicular to the element at fraction `f`, offset by `+-g`.
pub fn interp2(base: &Base, f: f64, g: f64) -> (Point, Point) {
    interp_point2(base.lead1, base.lead2, f, g)
}

/// Reads the five-integer file prefix `x1 y1 x2 y2 f` handled by the loader; helper for tests.
pub fn read_prefix(
    st: &mut Tokenizer<'_>,
) -> Result<(i32, i32, i32, i32, i32), crate::format::tokenizer::TokenError> {
    Ok((st.next_i32()?, st.next_i32()?, st.next_i32()?, st.next_i32()?, st.next_i32()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_points_computes_axis_vectors() {
        let mut b = Base::from_dump(0, 0, 0, 64, 0);
        b.set_points();
        assert_eq!((b.dx, b.dy, b.dsign), (0, 64, 1));
        assert!((b.dn - 64.0).abs() < 1e-12);
        assert!((b.dpx1 - 1.0).abs() < 1e-12 && b.dpy1.abs() < 1e-12);
    }

    #[test]
    fn leads_are_centred_and_shortened() {
        let mut b = Base::from_dump(0, 0, 96, 0, 0);
        b.set_points();
        b.calc_leads(32);
        assert_eq!((b.lead1.x, b.lead2.x), (32, 64));
        let mut short = Base::from_dump(0, 0, 16, 0, 0);
        short.set_points();
        short.calc_leads(32);
        assert_eq!((short.lead1, short.lead2), (short.point1, short.point2));
    }

    #[test]
    fn dump_prefix_uses_letter_or_number() {
        let b = Base::from_dump(1, 2, 3, 4, 5);
        assert_eq!(b.dump_prefix(b'r' as i32), "r 1 2 3 4 5");
        assert_eq!(b.dump_prefix(150), "150 1 2 3 4 5");
    }

    #[test]
    fn voltage_color_maps_range_ends_and_ground() {
        let ctx = DrawCtx::default();
        assert_eq!(ctx.voltage_color(5.0, false), ctx.color_scale[COLOR_SCALE_COUNT - 1]);
        assert_eq!(ctx.voltage_color(-50.0, false), ctx.color_scale[0]);
        assert_eq!(ctx.voltage_color(0.0, false), ctx.color_scale[COLOR_SCALE_COUNT / 2]);
        assert_eq!(ctx.voltage_color(f64::NAN, false), ctx.color_scale[COLOR_SCALE_COUNT / 2]);
    }

    #[test]
    fn voltage_diff_of_a_single_post_is_measured_against_ground() {
        let mut b = Base::new(0, 0);
        b.volts = vec![5.0];
        assert_eq!(b.voltage_diff(), 5.0);
        b.volts = vec![];
        assert_eq!(b.voltage_diff(), 0.0);
        b.volts = vec![5.0, 2.0];
        assert_eq!(b.voltage_diff(), 3.0);
    }

    #[test]
    fn dot_count_flags_fast_currents() {
        let ctx = DrawCtx { current_mult: 1.0, ..DrawCtx::default() };
        let b = Base::default();
        assert_eq!(b.update_dot_count_for(10.0, 3.0, &ctx), CURRENT_TOO_FAST);
        assert!((b.update_dot_count_for(2.0, 1.0, &ctx) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn registry_returns_first_post_for_repeated_names() {
        let mut r = LabelRegistry::default();
        assert_eq!(r.connect("a", Point::new(1, 1)), None);
        assert_eq!(r.connect("a", Point::new(5, 5)), Some(Point::new(1, 1)));
    }

    #[test]
    fn edit_info_flag_helpers() {
        let on = EditInfo::checkbox("x", true);
        assert_eq!(on.change_flag(0, 4), 4);
        assert_eq!(on.change_flag_inverted(4, 4), 0);
    }
}
