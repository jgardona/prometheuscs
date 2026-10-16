// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (ChipElm).
//! The framework shared by integrated-circuit style elements: a rectangle with pins on its
//! four sides, logic pins that read a threshold and output pins driven by voltage sources.
//!
//! A concrete chip supplies a [`ChipLogic`] (its pins and what it does each iteration); the
//! generic [`ChipElm`] provides everything else: geometry, drawing, files, editing.

use super::element::*;
use super::geometry::*;
use super::units::*;
use crate::format::tokenizer::Tokenizer;
use crate::format::xml::{XmlNode, XmlOut};
use crate::render::painter::{Color, Painter, TextAlign, TextBaseline};
use crate::solver::path::Validator;
use crate::solver::stamper::{NodeId, Stamper, VsId};

/// Pin on the north side.
pub const SIDE_N: usize = 0;
/// Pin on the south side.
pub const SIDE_S: usize = 1;
/// Pin on the west side.
pub const SIDE_W: usize = 2;
/// Pin on the east side.
pub const SIDE_E: usize = 3;
const SIDE_FLIP_XY: [usize; 4] = [SIDE_W, SIDE_E, SIDE_N, SIDE_S];

/// Chip flag: small drawing size.
pub const FLAG_SMALL: i32 = 1;
/// Chip flag: mirrored left to right.
pub const FLAG_FLIP_X: i32 = 1 << 10;
/// Chip flag: mirrored top to bottom.
pub const FLAG_FLIP_Y: i32 = 1 << 11;
/// Chip flag: mirrored about the diagonal.
pub const FLAG_FLIP_XY: i32 = 1 << 12;
/// Chip flag: the high logic voltage is stored in the file.
pub const FLAG_CUSTOM_VOLTAGE: i32 = 1 << 13;
/// Bit order: most significant bit first.
pub const BIT_ORDER_MSB_FIRST: i32 = 0;
/// Bit order: least significant bit first.
pub const BIT_ORDER_LSB_FIRST: i32 = 1;
/// Bit order: one bus pin instead of one pin per bit.
pub const BIT_ORDER_BUS: i32 = 2;

/// One pin of a chip.
#[derive(Clone, Debug)]
pub struct Pin {
    /// Position along its side, in pin spacings.
    pub pos: i32,
    /// Current side (after mirroring).
    pub side: usize,
    /// Side before mirroring.
    pub side0: usize,
    /// Label.
    pub text: String,
    /// Draw a bar over the label.
    pub line_over: bool,
    /// Draw an inversion bubble.
    pub bubble: bool,
    /// Draw the clock triangle.
    pub clock: bool,
    /// Driven by the chip.
    pub output: bool,
    /// Logic value (inputs: read at the start of an iteration; outputs: what to drive).
    pub value: bool,
    /// The pin's voltage is saved in files.
    pub state: bool,
    /// Highlighted.
    pub selected: bool,
    /// Animation position of the current dots.
    pub curcount: f64,
    /// Current through the pin.
    pub current: f64,
    /// Number of bits when the pin is a bus.
    pub bus_width: i32,
    /// Index of this bit inside its bus.
    pub bus_z: i32,
    /// The voltage source driving an output pin.
    pub volt_source: Option<VsId>,
    post: Point,
    stub: Point,
    textloc: Point,
    bubble_x: i32,
    bubble_y: i32,
    clock_points: Option<[Point; 3]>,
}

impl Pin {
    /// A pin at position `pos` on `side` labelled `text`.
    pub fn new(pos: i32, side: usize, text: &str) -> Pin {
        Pin {
            pos,
            side,
            side0: side,
            text: text.to_string(),
            line_over: false,
            bubble: false,
            clock: false,
            output: false,
            value: false,
            state: false,
            selected: false,
            curcount: 0.0,
            current: 0.0,
            bus_width: 1,
            bus_z: 0,
            volt_source: None,
            post: Point::default(),
            stub: Point::default(),
            textloc: Point::default(),
            bubble_x: 0,
            bubble_y: 0,
            clock_points: None,
        }
    }

    /// An output pin whose voltage is saved with the circuit.
    pub fn output_state(pos: i32, side: usize, text: &str) -> Pin {
        let mut p = Pin::new(pos, side, text);
        p.output = true;
        p.state = true;
        p
    }

    /// An output pin.
    pub fn output(pos: i32, side: usize, text: &str) -> Pin {
        let mut p = Pin::new(pos, side, text);
        p.output = true;
        p
    }

    /// The post (connection point) of the pin.
    pub fn post(&self) -> Point {
        self.post
    }

    /// Interprets the markers the original puts in pin names: `/` for a bar, `#` for a bubble,
    /// `CLK:` for a clock pin, `INV:` for a bubble.
    pub fn fix_name(&mut self) {
        if let Some(rest) = self.text.strip_prefix('/') {
            self.text = rest.to_string();
            self.line_over = true;
        } else if let Some(rest) = self.text.strip_prefix('#') {
            self.text = rest.to_string();
            self.bubble = true;
        }
        if self.text.contains("CLK:") {
            self.text = self.text.replace("CLK:", "");
            self.clock = true;
        }
        if self.text.contains("INV:") {
            self.text = self.text.replace("INV:", "");
            self.bubble = true;
        }
        if self.text.eq_ignore_ascii_case("clk") {
            self.text.clear();
            self.clock = true;
        }
    }

    fn to_grid(p: i32, s: usize, size_x: i32, size_y: i32) -> i32 {
        match s {
            SIDE_N => p,
            SIDE_S => p + size_x * (size_y - 1),
            SIDE_W => p * size_x,
            _ => p * size_x + size_x - 1,
        }
    }
}

/// The state every chip has, independent of what it does.
#[derive(Clone, Debug)]
pub struct ChipCore {
    /// Shared element state.
    pub base: Base,
    /// The pins, in post order.
    pub pins: Vec<Pin>,
    /// Drawing size (1 small, 2 normal).
    pub csize: i32,
    /// Pin spacing.
    pub cspc: i32,
    /// Twice the pin spacing.
    pub cspc2: i32,
    /// Width in pin spacings.
    pub size_x: i32,
    /// Height in pin spacings.
    pub size_y: i32,
    /// Width after mirroring about the diagonal.
    pub flipped_size_x: i32,
    /// Height after mirroring about the diagonal.
    pub flipped_size_y: i32,
    /// Number of bits of chips that have a bit count.
    pub bits: i32,
    /// Output voltage of a logic one.
    pub high_voltage: f64,
    /// Bit order of bussed chips.
    pub bit_order: i32,
    /// The clock level at the previous iteration.
    pub last_clock: bool,
    /// Corners of the chip rectangle.
    pub rect: [Point; 4],
    /// Where the chip's label is drawn.
    pub label: Point,
}

impl ChipCore {
    fn new(base: Base) -> Self {
        ChipCore {
            base,
            pins: Vec::new(),
            csize: 2,
            cspc: 16,
            cspc2: 32,
            size_x: 0,
            size_y: 0,
            flipped_size_x: 0,
            flipped_size_y: 0,
            bits: 0,
            high_voltage: 5.0,
            bit_order: 0,
            last_clock: false,
            rect: [Point::default(); 4],
            label: Point::default(),
        }
    }

    /// Sets the drawing size.
    pub fn set_size(&mut self, s: i32) {
        self.csize = s;
        self.cspc = 8 * s;
        self.cspc2 = self.cspc * 2;
        self.base.flags &= !FLAG_SMALL;
        if s == 1 {
            self.base.flags |= FLAG_SMALL;
        }
    }

    /// Logic threshold: half the high voltage.
    pub fn threshold(&self) -> f64 {
        self.high_voltage / 2.0
    }

    fn has_flag(&self, f: i32) -> bool {
        self.base.flags & f != 0
    }

    /// True when the chip is mirrored left to right.
    pub fn is_flipped_x(&self) -> bool {
        self.has_flag(FLAG_FLIP_X)
    }
    /// True when the chip is mirrored top to bottom.
    pub fn is_flipped_y(&self) -> bool {
        self.has_flag(FLAG_FLIP_Y)
    }
    /// True when the chip is mirrored about the diagonal.
    pub fn is_flipped_xy(&self) -> bool {
        self.has_flag(FLAG_FLIP_XY)
    }

    /// True when bits are bussed.
    pub fn use_bus(&self) -> bool {
        self.bit_order == BIT_ORDER_BUS
    }

    fn flipped_x_side(&self, s: usize) -> usize {
        if !self.is_flipped_x() {
            return s;
        }
        match s {
            SIDE_W => SIDE_E,
            SIDE_E => SIDE_W,
            other => other,
        }
    }

    /// Sets the logic value an output pin will drive.
    pub fn write_output(&mut self, n: usize, value: bool) {
        if let Some(p) = self.pins.get_mut(n) {
            p.value = value;
        }
    }

    /// Creates `count` pins for a bit field (the original's `makeBitPins`).
    #[allow(clippy::too_many_arguments)]
    pub fn make_bit_pins(
        &mut self,
        count: usize,
        pos: i32,
        side: usize,
        offset: usize,
        name: &str,
        output: bool,
        state: bool,
        reversed: bool,
    ) {
        for i in 0..count {
            let ii = if reversed { offset + count - 1 - i } else { offset + i };
            let mut pin;
            if self.use_bus() {
                pin = Pin::new(pos, side, name);
                pin.bus_width = count as i32;
                pin.bus_z = i as i32;
            } else if self.bit_order == BIT_ORDER_LSB_FIRST {
                pin = Pin::new(pos + i as i32, side, &format!("{name}{i}"));
            } else {
                pin = Pin::new(pos + (count - 1 - i) as i32, side, &format!("{name}{i}"));
            }
            pin.output = output;
            pin.state = state;
            if ii < self.pins.len() {
                self.pins[ii] = pin;
            }
        }
    }

    /// Positions the pins and the rectangle from the element's origin.
    pub fn place(&mut self) {
        let b = &self.base;
        if b.x2 - b.x > self.size_x * self.cspc2 && b.creating {
            self.set_size(2);
        }
        let (x0, y0) = (self.base.x + self.cspc2, self.base.y);
        let (xr, yr) = (x0 - self.cspc, y0 - self.cspc);
        self.flipped_size_x = self.size_x;
        self.flipped_size_y = self.size_y;
        if self.is_flipped_xy() {
            self.flipped_size_x = self.size_y;
            self.flipped_size_y = self.size_x;
        }
        let xs = self.flipped_size_x * self.cspc2;
        let ys = self.flipped_size_y * self.cspc2;
        let geometry = self.pin_geometry();
        let flip_xy = self.has_flag(FLAG_FLIP_XY);
        for p in &mut self.pins {
            p.side = if flip_xy { SIDE_FLIP_XY[p.side0] } else { p.side0 };
            let (px, py, dx, dy, dax, day, sx, sy) = match p.side {
                SIDE_N => (x0, y0, 1, 0, 0, -1, 0, 0),
                SIDE_S => (x0, y0, 1, 0, 0, 1, 0, ys - geometry.cspc2),
                SIDE_W => (x0, y0, 0, 1, -1, 0, 0, 0),
                _ => (x0, y0, 0, 1, 1, 0, xs - geometry.cspc2, 0),
            };
            Self::set_pin_point(p, &geometry, px, py, dx, dy, dax, day, sx, sy);
        }
        self.rect = [
            Point::new(xr, yr),
            Point::new(xr + xs, yr),
            Point::new(xr + xs, yr + ys),
            Point::new(xr, yr + ys),
        ];
        self.base.set_bbox(xr, yr, xr + xs, yr + ys);
        self.label = Point::new(xr + xs / 2, yr + ys / 2);
    }

    fn pin_geometry(&self) -> PinGeometry {
        PinGeometry {
            cspc: self.cspc,
            cspc2: self.cspc2,
            csize: self.csize,
            flipped_x: self.is_flipped_x(),
            flipped_y: self.is_flipped_y(),
            flipped_size_x: self.flipped_size_x,
            flipped_size_y: self.flipped_size_y,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn set_pin_point(
        p: &mut Pin,
        g: &PinGeometry,
        px: i32,
        py: i32,
        dx: i32,
        dy: i32,
        dax: i32,
        day: i32,
        sx: i32,
        sy: i32,
    ) {
        let (mut px, mut py, mut dx, mut dy, mut dax, mut day, mut sx, mut sy) =
            (px, py, dx, dy, dax, day, sx, sy);
        if g.flipped_x {
            dx = -dx;
            dax = -dax;
            px += g.cspc2 * (g.flipped_size_x - 1);
            sx = -sx;
        }
        if g.flipped_y {
            dy = -dy;
            day = -day;
            py += g.cspc2 * (g.flipped_size_y - 1);
            sy = -sy;
        }
        let xa = px + g.cspc2 * dx * p.pos + sx;
        let ya = py + g.cspc2 * dy * p.pos + sy;
        let bus_extra = if p.bus_width > 1 { 2 } else { 0 };
        p.post = Point { x: xa + dax * g.cspc2, y: ya + day * g.cspc2, z: p.bus_z };
        p.stub = Point::new(xa + dax * (g.cspc + bus_extra), ya + day * (g.cspc + bus_extra));
        p.textloc = Point::new(xa, ya);
        if p.bubble {
            p.bubble_x = xa + dax * 10 * g.csize;
            p.bubble_y = ya + day * 10 * g.csize;
        }
        if p.clock {
            let c0 = Point::new(
                xa + dax * g.cspc - dx * g.cspc / 2,
                ya + day * g.cspc - dy * g.cspc / 2,
            );
            let mut c1 = Point::new(xa, ya);
            let c2 = Point::new(
                xa + dax * g.cspc + dx * g.cspc / 2,
                ya + day * g.cspc + dy * g.cspc / 2,
            );
            if !p.text.is_empty() {
                c1.x += dax * g.cspc / 2;
                c1.y += day * g.cspc / 2;
                p.textloc.x -= dax * g.cspc / 2;
                p.textloc.y -= day * g.cspc / 4;
            }
            p.clock_points = Some([c0, c1, c2]);
        } else {
            p.clock_points = None;
        }
    }

    /// Index of the pin occupying grid cell (`p`, `s`), ignoring `pin`.
    pub fn overlapping_pin(&self, p: i32, s: usize, pin: usize) -> Option<usize> {
        let g = Pin::to_grid(p, s, self.size_x, self.size_y);
        self.pins.iter().enumerate().find_map(|(i, q)| {
            (i != pin && q.bus_z <= 0 && Pin::to_grid(q.pos, q.side, self.size_x, self.size_y) == g)
                .then_some(i)
        })
    }
}

#[derive(Clone, Copy)]
struct PinGeometry {
    cspc: i32,
    cspc2: i32,
    csize: i32,
    flipped_x: bool,
    flipped_y: bool,
    flipped_size_x: i32,
    flipped_size_y: i32,
}

/// What a particular chip is and does. Every method but the first three has a default, so a
/// simple chip only describes its pins and its logic.
pub trait ChipLogic: Clone + std::fmt::Debug + 'static {
    /// The original's class name.
    fn class_name(&self) -> &'static str;
    /// File-format type code.
    fn dump_type(&self) -> i32;
    /// Builds `c.pins` and sets `c.size_x` and `c.size_y`.
    fn setup_pins(&mut self, c: &mut ChipCore);

    /// Name shown in the info panel.
    fn chip_name(&self) -> &'static str {
        "chip"
    }
    /// Reads input pins and writes output pins, once per iteration.
    fn execute(&mut self, _c: &mut ChipCore, _s: &Stamper) {}
    /// True for chips with a configurable number of bits.
    fn needs_bits(&self) -> bool {
        false
    }
    /// Bit count of a new chip.
    fn default_bit_count(&self) -> i32 {
        4
    }
    /// True when the bit order can be chosen.
    fn allow_bus(&self) -> bool {
        false
    }
    /// True when pins are logic levels with a high logic voltage.
    fn is_digital(&self) -> bool {
        true
    }
    /// Number of voltage sources: one per output pin unless the chip says otherwise.
    fn voltage_source_count(&self, c: &ChipCore) -> usize {
        c.pins.iter().filter(|p| p.output).count()
    }
    /// True when the matrix must be restamped each iteration.
    fn non_linear(&self) -> bool {
        false
    }
    /// Fields after the common part of a text line.
    fn read_extra(&mut self, _c: &mut ChipCore, _st: &mut Tokenizer<'_>) {}
    /// Fields appended to a text line.
    fn write_extra(&self, _c: &ChipCore) -> String {
        String::new()
    }
    /// Reads chip-specific attributes of an XML tag.
    fn read_xml(&mut self, _c: &mut ChipCore, _xml: &XmlNode) {}
    /// Writes chip-specific attributes of an XML tag.
    fn write_xml(&self, _c: &ChipCore, _out: &mut XmlOut) {}
    /// Writes chip-specific state of an XML tag.
    fn write_xml_state(&self, _c: &ChipCore, _out: &mut XmlOut) {}
    /// Called once a chip was loaded from a file, after its pins exist.
    fn after_load(&mut self, _c: &mut ChipCore) {}
    /// Called after the standard reset.
    fn on_reset(&mut self, _c: &mut ChipCore) {}
    /// Draws text inside the rectangle.
    fn draw_label(
        &self,
        _g: &mut dyn Painter,
        _c: &mut ChipCore,
        _ctx: &DrawCtx,
        _x: i32,
        _y: i32,
    ) {
    }
    /// Edit field `n` of the chip (after the common fields).
    fn chip_edit_info(&self, _c: &ChipCore, _n: usize) -> Option<EditInfo> {
        None
    }
    /// Applies edit field `n` of the chip.
    fn set_chip_edit_value(&mut self, _c: &mut ChipCore, _n: usize, _ei: &mut EditInfo) {}
    /// Called after every step.
    fn step_finished(&mut self, _c: &mut ChipCore, _s: &mut Stamper) {}
    /// Replaces the standard output update; return true when handled.
    fn do_step(&mut self, _c: &mut ChipCore, _s: &mut Stamper) -> bool {
        false
    }
    /// Replaces the standard stamping; return true when handled.
    fn stamp(&mut self, _c: &mut ChipCore, _s: &mut Stamper) -> bool {
        false
    }
    /// Replaces the standard validation; return `Some` when handled.
    fn validate(&mut self, _c: &mut ChipCore, _idx: usize, _v: &mut Validator<'_>) -> Option<bool> {
        None
    }
    /// Are posts `n1` and `n2` connected through the chip?
    fn connection(&self, _n1: usize, _n2: usize) -> bool {
        false
    }
    /// Are posts `n1` and `n2` in the same matrix?
    fn matrix_connection(&self, n1: usize, n2: usize) -> bool {
        self.connection(n1, n2)
    }
    /// Does post `n` have a connection to ground (an output pin by default)?
    fn ground_connection(&self, c: &ChipCore, n: usize) -> bool {
        c.pins.get(n).is_some_and(|p| p.output)
    }
    /// Draws the chip's rectangle and pins; chips that draw themselves override this.
    fn custom_draw(&mut self, _g: &mut dyn Painter, _c: &mut ChipCore, _ctx: &DrawCtx) -> bool {
        false
    }
    /// Extra lines for the info panel.
    fn info_extra(&self, _c: &ChipCore, _arr: &mut Vec<String>) {}
    /// Keyboard shortcut character (0 for none).
    fn shortcut(&self) -> i32 {
        0
    }
    /// Number of internal nodes besides the pins.
    fn internal_nodes(&self) -> usize {
        0
    }
    /// True when the chip is only wiring (a bus splitter) and can be removed from the matrix.
    fn wire_equivalent(&self) -> bool {
        false
    }
    /// Pin that pin `n` is wired through to, for wire-equivalent chips.
    fn connected_pin(&self, _n: usize) -> Option<usize> {
        None
    }
    /// Sets the current of wire segment `bit` of a wire-equivalent chip.
    fn set_wire_current(&mut self, _c: &mut ChipCore, _bit: usize, _current: f64) {}
    /// Current into post `n` when the chip computes it itself.
    fn current_into_node(&self, _c: &ChipCore, _n: usize) -> Option<f64> {
        None
    }
    /// Bus width of a chip that is a bus wire.
    fn bus_width(&self, _c: &ChipCore) -> usize {
        1
    }
    /// Index into the node array of the node voltage source `j` drives, when it is not the
    /// node of its output pin.
    fn voltage_source_node(&self, _c: &ChipCore, _j: usize) -> Option<usize> {
        None
    }
    /// XML tag when it differs from the class name without `Elm`.
    fn xml_name(&self) -> Option<&'static str> {
        None
    }
    /// Called once a chip was loaded from XML, after the saved pin values were restored.
    fn after_xml(&mut self, _c: &mut ChipCore) {}
    /// Computes the pins' currents when the chip does not drive them through voltage sources.
    fn calculate_current(&mut self, _c: &mut ChipCore) {}
    /// Called for a chip being placed, before its pins are built.
    fn before_new(&mut self, _c: &mut ChipCore) {}
    /// Called for a chip being placed, once its pins exist.
    fn after_new(&mut self, _c: &mut ChipCore) {}
}

/// A chip: the shared framework plus the logic of one kind of chip.
#[derive(Clone, Debug)]
pub struct ChipElm<L: ChipLogic> {
    /// Framework state.
    pub core: ChipCore,
    /// The chip's own state and behaviour.
    pub logic: L,
}

impl<L: ChipLogic> ChipElm<L> {
    /// A chip being placed at (`xx`, `yy`).
    pub fn new(logic: L, xx: i32, yy: i32) -> Self {
        let mut c = ChipCore::new(Base::new(xx, yy));
        c.base.no_diagonal = true;
        let mut chip = ChipElm { core: c, logic };
        chip.logic.before_new(&mut chip.core);
        if chip.logic.needs_bits() {
            chip.core.bits = chip.logic.default_bit_count();
        }
        chip.rebuild_pins();
        chip.core.set_size(2);
        chip.logic.after_new(&mut chip.core);
        chip
    }

    /// A chip read from a file.
    pub fn from_dump(
        logic: L,
        xa: i32,
        ya: i32,
        xb: i32,
        yb: i32,
        f: i32,
        st: &mut Tokenizer<'_>,
    ) -> Self {
        let mut c = ChipCore::new(Base::from_dump(xa, ya, xb, yb, f));
        c.base.no_diagonal = true;
        let mut chip = ChipElm { core: c, logic };
        if chip.logic.needs_bits() {
            chip.core.bits = match st.next_i32() {
                Ok(b) => b,
                Err(_) => chip.logic.default_bit_count(),
            };
        }
        chip.core.high_voltage = if chip.core.has_flag(FLAG_CUSTOM_VOLTAGE) {
            st.next_f64().unwrap_or(5.0)
        } else {
            5.0
        };
        chip.rebuild_pins();
        chip.core.set_size(if f & FLAG_SMALL != 0 { 1 } else { 2 });
        let threshold = chip.core.threshold();
        for i in 0..chip.core.pins.len() {
            if chip.core.pins[i].state {
                let v = st.next_f64().unwrap_or(0.0);
                chip.core.base.volts[i] = v;
                chip.core.pins[i].value = v > threshold;
            }
        }
        chip.logic.read_extra(&mut chip.core, st);
        chip.logic.after_load(&mut chip.core);
        chip
    }

    /// Builds the pins again (after the bit count, flags or size changed) and reallocates
    /// the node arrays.
    pub fn rebuild_pins(&mut self) {
        self.logic.setup_pins(&mut self.core);
        let n = self.core.pins.len() + self.logic.internal_nodes();
        self.core.base.alloc_nodes(n);
    }

    fn output_pin_index(&self, j: usize) -> Option<usize> {
        self.core.pins.iter().enumerate().filter(|(_, p)| p.output).nth(j).map(|(i, _)| i)
    }

    fn draw_chip(&mut self, g: &mut dyn Painter, ctx: &DrawCtx) {
        g.save();
        let csize = self.core.csize;
        let has_vertical = self.core.pins.iter().any(|p| p.side == SIDE_N || p.side == SIDE_S);
        for i in 0..self.core.pins.len() {
            g.set_font_size(10 * csize);
            let (post, stub, bus_z, bus_width) = {
                let p = &self.core.pins[i];
                (p.post, p.stub, p.bus_z, p.bus_width)
            };
            if bus_z > 0 {
                continue;
            }
            let v = self.core.base.volts[i];
            set_voltage_color(g, &self.core.base, ctx, v);
            draw_thick_line_w(g, post, stub, if bus_width > 1 { 5.0 } else { 3.0 });
            let cur = self.core.pins[i].current;
            let cc = self.core.pins[i].curcount;
            let cc = self.core.base.update_dot_count_for(cur, cc, ctx);
            self.core.pins[i].curcount = cc;
            draw_dots(g, ctx, stub, post, cc);
            let p = self.core.pins[i].clone();
            if p.bubble {
                g.set_color(if ctx.printable { Color::WHITE } else { Color::BLACK });
                draw_thick_circle(g, p.bubble_x, p.bubble_y, 1);
                g.set_color(ctx.light_gray);
                draw_thick_circle(g, p.bubble_x, p.bubble_y, 3);
            }
            if let Some(cp) = p.clock_points {
                g.set_color(ctx.light_gray);
                g.begin_path();
                g.move_to(f64::from(cp[0].x), f64::from(cp[0].y));
                g.line_to(f64::from(cp[1].x), f64::from(cp[1].y));
                g.line_to(f64::from(cp[2].x), f64::from(cp[2].y));
                g.stroke();
            }
            g.set_color(if p.selected { ctx.select_color } else { ctx.white });
            let mut fsz = 10 * csize;
            let cspc = self.core.cspc;
            let mut avail = f64::from(cspc * 2 - 8);
            if !has_vertical && self.core.size_x > 2 {
                avail = f64::from(cspc) * 2.5 + f64::from(cspc * (self.core.size_x - 3));
            }
            let text = if p.bus_width > 1 {
                format!("{}/{}", p.text, p.bus_width)
            } else {
                p.text.clone()
            };
            let mut sw = g.measure_text(&text);
            while sw > avail && fsz > 3 {
                fsz -= 1;
                g.set_font_size(fsz);
                sw = g.measure_text(&text);
            }
            let sw = sw as i32;
            let asc = g.font_size();
            let tx = if p.side == self.core.flipped_x_side(SIDE_W) {
                p.textloc.x - (cspc - 5)
            } else if p.side == self.core.flipped_x_side(SIDE_E) {
                p.textloc.x + (cspc - 5) - sw
            } else {
                p.textloc.x - sw / 2
            };
            g.draw_string(&text, f64::from(tx), f64::from(p.textloc.y + asc / 3));
            if p.line_over {
                let ya = p.textloc.y - asc + asc / 3;
                g.draw_line(tx, ya, tx + sw, ya);
            }
        }
        let (lx, ly) = (self.core.label.x, self.core.label.y);
        self.logic.draw_label(g, &mut self.core, ctx, lx, ly);
        g.set_color(if self.core.base.needs_highlight() {
            ctx.select_color
        } else {
            ctx.light_gray
        });
        let r = &self.core.rect;
        draw_thick_polygon(g, &Polygon::from_points(r));
        g.restore();
    }
}

impl<L: ChipLogic> Element for ChipElm<L> {
    fn base(&self) -> &Base {
        &self.core.base
    }
    fn base_mut(&mut self) -> &mut Base {
        &mut self.core.base
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn class_name(&self) -> &'static str {
        self.logic.class_name()
    }
    fn dump_type(&self) -> i32 {
        self.logic.dump_type()
    }
    fn xml_dump_type(&self) -> String {
        if let Some(n) = self.logic.xml_name() {
            return n.to_string();
        }
        let t = self.logic.dump_type();
        if t > 64 && t < 127 {
            char::from(t as u8).to_string()
        } else {
            self.logic.class_name().replace("Elm", "")
        }
    }

    fn dump(&self) -> String {
        use crate::format::number::js_number;
        let mut b = self.core.base.clone();
        if self.core.high_voltage == 5.0 {
            b.flags &= !FLAG_CUSTOM_VOLTAGE;
        } else {
            b.flags |= FLAG_CUSTOM_VOLTAGE;
        }
        let mut s = b.dump_prefix(self.dump_type());
        if self.logic.needs_bits() {
            s += &format!(" {}", self.core.bits);
        }
        if b.flags & FLAG_CUSTOM_VOLTAGE != 0 {
            s += &format!(" {}", js_number(self.core.high_voltage));
        }
        for (i, p) in self.core.pins.iter().enumerate() {
            if p.state {
                s += &format!(" {}", js_number(self.core.base.volts[i]));
            }
        }
        s + &self.logic.write_extra(&self.core)
    }

    fn undump_xml(&mut self, xml: &XmlNode) {
        let state_restore = xml.attr("ix").is_some();
        if !state_restore {
            self.core.base.flags = 0;
        }
        self.core.base.flags = xml.int_attr("f", self.core.base.flags);
        self.core.bits = xml.int_attr("bi", self.core.bits);
        self.core.high_voltage = xml.double_attr("hv", self.core.high_voltage);
        self.core.bit_order = xml.int_attr("bo", self.core.bit_order);
        self.logic.read_xml(&mut self.core, xml);
        if !state_restore {
            self.rebuild_pins();
            let small = self.core.base.flags & FLAG_SMALL != 0;
            self.core.set_size(if small { 1 } else { 2 });
        }
        let threshold = self.core.threshold();
        for i in 0..self.core.pins.len() {
            let v = xml.double_attr(&format!("v{i}"), 0.0);
            self.core.base.volts[i] = v;
            self.core.pins[i].value = v > threshold;
        }
        self.logic.after_load(&mut self.core);
        self.logic.after_xml(&mut self.core);
    }
    fn dump_xml(&self, out: &mut XmlOut) {
        default_dump_xml(&self.core.base, out);
        if self.core.bits > 0 {
            out.int_attr("bi", self.core.bits);
        }
        if self.core.high_voltage != 5.0 {
            out.double_attr("hv", self.core.high_voltage);
        }
        if self.core.bit_order != 0 {
            out.int_attr("bo", self.core.bit_order);
        }
        self.logic.write_xml(&self.core, out);
    }
    fn dump_xml_state(&self, out: &mut XmlOut) {
        for (i, p) in self.core.pins.iter().enumerate() {
            if p.state && self.core.base.volts[i] > 0.0 {
                out.double_attr(&format!("v{i}"), self.core.base.volts[i]);
            }
        }
        self.logic.write_xml_state(&self.core, out);
    }

    fn post_count(&self) -> usize {
        self.core.pins.len()
    }
    fn post_width(&self, n: usize) -> usize {
        self.core.pins.get(n).map_or(1, |p| p.bus_width.max(1) as usize)
    }
    fn internal_node_count(&self) -> usize {
        self.logic.internal_nodes()
    }
    fn voltage_source_count(&self) -> usize {
        self.logic.voltage_source_count(&self.core)
    }
    fn non_linear(&self) -> bool {
        self.logic.non_linear()
    }
    fn get_post(&self, n: usize) -> Option<Point> {
        self.core.pins.get(n).map(Pin::post)
    }

    fn set_points(&mut self) {
        self.core.place();
    }

    fn drag(&mut self, xx: i32, yy: i32) {
        // chips are dragged out wider only, snapped to the grid
        let yy = (yy + 7) & !15;
        let b = &mut self.core.base;
        if xx < b.x {
            b.x2 = b.x;
            b.y2 = b.y;
        } else {
            b.y = yy;
            b.y2 = yy;
            b.x2 = ((xx + 7) & !15).min(b.x + (self.core.size_x + 1) * self.core.cspc2);
        }
        self.set_points();
    }

    fn set_voltage_source(&mut self, j: usize, vs: VsId) {
        if let Some(i) = self.output_pin_index(j) {
            self.core.pins[i].volt_source = Some(vs);
        }
    }
    fn voltage_source_nodes_at(&self, j: usize) -> Option<(NodeId, NodeId)> {
        if let Some(i) = self.logic.voltage_source_node(&self.core, j) {
            return self.core.base.nodes.get(i).map(|&n| (NodeId::GROUND, n));
        }
        let i = self.output_pin_index(j)?;
        self.core.base.nodes.get(i).map(|&n| (NodeId::GROUND, n))
    }

    fn stamp(&mut self, s: &mut Stamper) {
        if self.logic.stamp(&mut self.core, s) {
            return;
        }
        for (i, p) in self.core.pins.iter().enumerate() {
            if let (true, Some(vs)) = (p.output, p.volt_source) {
                s.stamp_voltage_source_var(NodeId::GROUND, self.core.base.nodes[i], vs);
            }
        }
    }

    fn start_iteration(&mut self, s: &Stamper) {
        let threshold = self.core.threshold();
        for (i, p) in self.core.pins.iter_mut().enumerate() {
            if !p.output {
                p.value = self.core.base.volts[i] > threshold;
            }
        }
        self.logic.execute(&mut self.core, s);
    }

    fn do_step(&mut self, s: &mut Stamper) {
        if self.logic.do_step(&mut self.core, s) {
            return;
        }
        let hv = self.core.high_voltage;
        for p in &self.core.pins {
            if let (true, Some(vs)) = (p.output, p.volt_source) {
                s.update_voltage_source(vs, if p.value { hv } else { 0.0 });
            }
        }
    }

    fn step_finished(&mut self, s: &mut Stamper) {
        self.logic.step_finished(&mut self.core, s);
    }

    fn reset(&mut self) {
        for (i, p) in self.core.pins.iter_mut().enumerate() {
            p.value = false;
            p.curcount = 0.0;
            self.core.base.volts[i] = 0.0;
        }
        self.core.last_clock = false;
        self.logic.on_reset(&mut self.core);
    }

    fn set_current(&mut self, vs: VsId, c: f64) {
        for p in &mut self.core.pins {
            if p.output && p.volt_source == Some(vs) {
                p.current = c;
            }
        }
    }

    fn validate(&mut self, idx: usize, v: &mut Validator<'_>) -> bool {
        if let Some(r) = self.logic.validate(&mut self.core, idx, v) {
            return r;
        }
        for i in 0..self.core.pins.len() {
            if self.core.pins[i].output && !validate_rail_node(v, idx, self.core.base.nodes[i]) {
                return false;
            }
        }
        true
    }

    fn get_connection(&self, n1: usize, n2: usize) -> bool {
        self.logic.connection(n1, n2)
    }
    fn get_matrix_connection(&self, n1: usize, n2: usize) -> bool {
        self.logic.matrix_connection(n1, n2)
    }
    fn has_ground_connection(&self, n1: usize) -> bool {
        self.logic.ground_connection(&self.core, n1)
    }
    fn calculate_current(&mut self) {
        self.logic.calculate_current(&mut self.core);
    }
    fn is_wire_equivalent(&self) -> bool {
        self.logic.wire_equivalent()
    }
    fn is_removable_wire(&self) -> bool {
        self.logic.wire_equivalent()
    }
    fn bus_width(&self) -> usize {
        self.logic.bus_width(&self.core)
    }
    fn connected_post(&self, bit: usize, _labels: &mut LabelRegistry) -> Option<Point> {
        let i = self.logic.connected_pin(bit)?;
        self.core.pins.get(i).map(Pin::post)
    }
    fn set_wire_current(&mut self, bit: usize, c: f64) {
        self.logic.set_wire_current(&mut self.core, bit, c);
    }
    fn get_current_into_node(&self, n: usize) -> f64 {
        if let Some(c) = self.logic.current_into_node(&self.core, n) {
            return c;
        }
        self.core.pins.get(n).map_or(0.0, |p| p.current)
    }

    fn draw(&mut self, g: &mut dyn Painter, c: &DrawCtx) {
        if !self.logic.custom_draw(g, &mut self.core, c) {
            self.draw_chip(g, c);
        }
        draw_posts(g, &self.core.base, self, c);
    }

    fn flip_x(&mut self, center2: i32, count: usize) {
        self.core.base.flags ^= FLAG_FLIP_X;
        if count != 1 {
            let xs = (self.core.flipped_size_x + 1) * self.core.cspc2;
            let b = &mut self.core.base;
            b.x = center2 - b.x - xs;
            b.x2 = center2 - b.x2;
        }
        self.set_points();
    }
    fn flip_y(&mut self, center2: i32, count: usize) {
        self.core.base.flags ^= FLAG_FLIP_Y;
        if count != 1 {
            let ys = (self.core.flipped_size_y - 1) * self.core.cspc2;
            let b = &mut self.core.base;
            b.y = center2 - b.y - ys;
            b.y2 = center2 - b.y2;
        }
        self.set_points();
    }
    fn flip_xy(&mut self, xmy: i32, count: usize) {
        self.core.base.flags ^= FLAG_FLIP_XY;
        if self.core.is_flipped_x() != self.core.is_flipped_y() {
            self.core.base.flags ^= FLAG_FLIP_X | FLAG_FLIP_Y;
        }
        if count != 1 {
            self.core.base.x += self.core.cspc2;
            self.core.base.flip_xy(xmy);
            self.core.base.x -= self.core.cspc2;
        }
        self.set_points();
    }
    fn num_handles(&self) -> usize {
        0
    }

    fn get_info(&self, arr: &mut Vec<String>) {
        arr.push(self.logic.chip_name().into());
        let threshold = self.core.threshold();
        let mut line = String::new();
        let mut shown = 0;
        let mut i = 0;
        while i < self.core.pins.len() {
            let p = &self.core.pins[i];
            if !line.is_empty() {
                line.push_str("; ");
            }
            let mut t = p.text.clone();
            if p.line_over {
                t.push('\'');
            }
            if p.clock {
                t = "Clk".into();
            }
            if p.bus_width > 1 {
                let mut value = 0_i64;
                for j in 0..p.bus_width as usize {
                    if self.core.base.volts[i + j] > threshold {
                        value |= 1 << self.core.pins[i + j].bus_z;
                    }
                }
                line += &format!("{t} = {value} / 0x{value:X}");
                i += p.bus_width as usize - 1;
            } else {
                line += &format!("{t} = {}", voltage_text(self.core.base.volts[i]));
            }
            shown += 1;
            if shown % 2 == 0 {
                arr.push(std::mem::take(&mut line));
            }
            i += 1;
        }
        if !line.is_empty() {
            arr.push(line);
        }
        self.logic.info_extra(&self.core, arr);
    }

    fn get_edit_info(&self, n: usize) -> Option<EditInfo> {
        let mut n = n;
        if self.logic.is_digital() {
            if n == 0 {
                return Some(
                    EditInfo::new("High Logic Voltage", self.core.high_voltage, 0.0, 0.0)
                        .set_unit_step(),
                );
            }
            n -= 1;
        }
        if self.logic.allow_bus() {
            if n == 0 {
                return Some(EditInfo::choice(
                    "Bit Order",
                    vec!["MSB First".into(), "LSB First".into(), "Bus".into()],
                    self.core.bit_order.max(0) as usize,
                ));
            }
            n -= 1;
        }
        self.logic.chip_edit_info(&self.core, n)
    }

    fn set_edit_value(&mut self, n: usize, ei: &mut EditInfo) {
        let mut n = n;
        if self.logic.is_digital() {
            if n == 0 {
                self.core.high_voltage = ei.value;
                return;
            }
            n -= 1;
        }
        if self.logic.allow_bus() {
            if n == 0 {
                self.core.bit_order = ei.choice_index as i32;
                self.rebuild_pins();
                self.set_points();
                return;
            }
            n -= 1;
        }
        self.logic.set_chip_edit_value(&mut self.core, n, ei);
        // the chip may have changed its flags or sizes; pins keep their logic state
        let old: Vec<(bool, f64, f64, Option<VsId>)> = self
            .core
            .pins
            .iter()
            .map(|p| (p.value, p.curcount, p.current, p.volt_source))
            .collect();
        self.rebuild_pins();
        if old.len() == self.core.pins.len() {
            for (p, (v, cc, cur, vs)) in self.core.pins.iter_mut().zip(old) {
                p.value = v;
                p.curcount = cc;
                p.current = cur;
                p.volt_source = vs;
            }
        }
        self.set_points();
    }

    fn shortcut(&self) -> i32 {
        self.logic.shortcut()
    }
}

/// Fixed-size text alignment helper used by chips that print a centred label.
pub fn draw_centered_label(
    g: &mut dyn Painter,
    ctx: &DrawCtx,
    text: &str,
    x: i32,
    y: i32,
    size: i32,
) {
    g.save();
    g.set_font_size(size);
    g.set_color(ctx.white);
    g.set_text_align(TextAlign::Center);
    g.set_text_baseline(TextBaseline::Middle);
    g.draw_string(text, f64::from(x), f64::from(y));
    g.restore();
}
