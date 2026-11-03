// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Expr, ExprParser, ExprState).
//! Expression evaluator used by custom elements (controlled sources, custom logic, lookup
//! tables): arithmetic, comparisons, a ternary operator and a set of functions over the
//! variables `a` to `i`, the time `t` and the previous values `lasta` to `lasti`.
//!
//! Expressions may also refer to the rest of the circuit by name: `v(name)` is the voltage of
//! a labeled node or across a labeled voltmeter, `v(a, b)` the difference of two such
//! voltages, `i(name)` the current through a labeled ammeter, and `dvdt(name)` / `didt(name)`
//! their rates of change. Each distinct quantity gets one slot ([`ExprRef`]); the element that
//! owns the expression fills [`ExprState::ref_values`] before evaluating (see [`refs`]).

pub mod refs;

/// Number of variables (`a` to `i`).
pub const VARIABLES: usize = 9;

/// Values an expression is evaluated with.
#[derive(Clone, Debug)]
pub struct ExprState {
    /// Current values of `a` to `i`.
    pub values: [f64; VARIABLES],
    /// Values at the previous time step (`lasta` to `lasti`, and the derivative `dadt`).
    pub last_values: [f64; VARIABLES],
    /// The previous output of the element (`lastoutput`).
    pub last_output: f64,
    /// Simulation time (`t`).
    pub t: f64,
    /// Simulation time step (`timestep`).
    pub time_step: f64,
    /// Current value of each referenced quantity, in the order of [`Expr::parse_with_refs`].
    pub ref_values: Vec<f64>,
    /// The same quantities one time step ago, for `dvdt()` and `didt()`.
    pub last_ref_values: Vec<f64>,
}

impl Default for ExprState {
    fn default() -> Self {
        let mut values = [0.0; VARIABLES];
        values[4] = std::f64::consts::E;
        ExprState {
            values,
            last_values: [0.0; VARIABLES],
            last_output: 0.0,
            t: 0.0,
            time_step: 0.0,
            ref_values: Vec::new(),
            last_ref_values: Vec::new(),
        }
    }
}

impl ExprState {
    /// A state with `e` in variable `e`, like the original.
    pub fn new() -> Self {
        Self::default()
    }

    /// Remembers the current values as the previous ones.
    pub fn update_last_values(&mut self, last_output: f64) {
        self.last_output = last_output;
        self.last_values = self.values;
        self.last_ref_values.clone_from(&self.ref_values);
    }

    /// Forgets the previous values.
    pub fn reset(&mut self) {
        self.last_values = [0.0; VARIABLES];
        self.last_ref_values.iter_mut().for_each(|v| *v = 0.0);
        self.last_output = 0.0;
    }

    /// Sizes the reference slots for `n` referenced quantities, all zero.
    pub fn alloc_refs(&mut self, n: usize) {
        self.ref_values = vec![0.0; n];
        self.last_ref_values = vec![0.0; n];
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Or,
    And,
    Equals,
    Neq,
    Leq,
    Geq,
    Less,
    Greater,
    Ternary,
    Uminus,
    Not,
    Val,
    T,
    Sin,
    Cos,
    Abs,
    Exp,
    Log,
    Sqrt,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    BitAnd,
    BitOr,
    Rshift,
    Floor,
    Ceil,
    Min,
    Max,
    Clamp,
    Step,
    Select,
    Triangle,
    Sawtooth,
    Mod,
    Pwl,
    Pwr,
    Pwrs,
    LastOutput,
    TimeStep,
    Var(usize),
    LastVar(usize),
    Dadt(usize),
    /// The value of reference slot `k` (`v(name)` or `i(name)`).
    Ref(usize),
    /// The rate of change of reference slot `k` (`dvdt(name)` or `didt(name)`).
    RefDt(usize),
}

/// A quantity an expression refers to by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprRef {
    /// The name inside the parentheses, case preserved (labels are case-sensitive).
    pub name: String,
    /// True for `i()` and `didt()` (a current), false for `v()` and `dvdt()` (a voltage).
    pub current: bool,
}

/// A parsed expression.
#[derive(Clone, Debug)]
pub struct Expr {
    kind: Kind,
    children: Vec<Expr>,
    value: f64,
}

fn posmod(x: f64, y: f64) -> f64 {
    let x = x % y;
    if x >= 0.0 { x } else { x + y }
}

impl Expr {
    fn node(kind: Kind, children: Vec<Expr>) -> Expr {
        Expr { kind, children, value: 0.0 }
    }

    fn leaf(kind: Kind) -> Expr {
        Expr::node(kind, Vec::new())
    }

    fn val(v: f64) -> Expr {
        Expr { kind: Kind::Val, children: Vec::new(), value: v }
    }

    /// Parses `text`; the error is the first problem found.
    pub fn parse(text: &str) -> Result<Expr, String> {
        Self::parse_with_refs(text).map(|(e, _)| e)
    }

    /// Parses `text` and lists the quantities it refers to by name, one entry per distinct
    /// quantity (`v(x)` and `dvdt(x)` share one, `v(x)` and `i(x)` do not), in order of first
    /// use. The error is the first problem found.
    pub fn parse_with_refs(text: &str) -> Result<(Expr, Vec<ExprRef>), String> {
        let mut p = Parser::new(text);
        let e = p.parse_expression();
        match p.err {
            Some(err) => Err(err),
            None => Ok((e, p.refs)),
        }
    }

    /// Evaluates the expression.
    pub fn eval(&self, es: &ExprState) -> f64 {
        let child = |i: usize| self.children.get(i).map_or(0.0, |c| c.eval(es));
        let (l, r) = (|| child(0), || child(1));
        let b = |v: bool| if v { 1.0 } else { 0.0 };
        match self.kind {
            Kind::Add => l() + r(),
            Kind::Sub => l() - r(),
            Kind::Mul => l() * r(),
            Kind::Div => l() / r(),
            Kind::Pow => l().powf(r()),
            Kind::Or => b(l() != 0.0 || r() != 0.0),
            Kind::And => b(l() != 0.0 && r() != 0.0),
            Kind::Equals => b(l() == r()),
            Kind::Neq => b(l() != r()),
            Kind::Leq => b(l() <= r()),
            Kind::Geq => b(l() >= r()),
            Kind::Less => b(l() < r()),
            Kind::Greater => b(l() > r()),
            Kind::Ternary => child(if l() != 0.0 { 1 } else { 2 }),
            Kind::Uminus => -l(),
            Kind::Not => b(l() == 0.0),
            Kind::Val => self.value,
            Kind::T => es.t,
            Kind::Sin => l().sin(),
            Kind::Cos => l().cos(),
            Kind::Abs => l().abs(),
            Kind::Exp => l().exp(),
            Kind::Log => l().ln(),
            Kind::Sqrt => l().sqrt(),
            Kind::Tan => l().tan(),
            Kind::Asin => l().asin(),
            Kind::Acos => l().acos(),
            Kind::Atan => l().atan(),
            Kind::Sinh => l().sinh(),
            Kind::Cosh => l().cosh(),
            Kind::Tanh => l().tanh(),
            Kind::BitAnd => f64::from((l() as i32) & (r() as i32)),
            Kind::BitOr => f64::from((l() as i32) | (r() as i32)),
            Kind::Rshift => f64::from((l() as i32).wrapping_shr(r() as i32 as u32)),
            Kind::Floor => l().floor(),
            Kind::Ceil => l().ceil(),
            Kind::Min => (1..self.children.len()).fold(l(), |x, i| x.min(child(i))),
            Kind::Max => (1..self.children.len()).fold(l(), |x, i| x.max(child(i))),
            Kind::Clamp => l().max(child(1)).min(child(2)),
            Kind::Step => {
                let x = l();
                if self.children.len() < 2 {
                    if x < 0.0 { 0.0 } else { 1.0 }
                } else if x > r() || x < 0.0 {
                    0.0
                } else {
                    1.0
                }
            }
            Kind::Select => child(if l() > 0.0 { 2 } else { 1 }),
            Kind::Triangle => {
                let x = posmod(l(), std::f64::consts::PI * 2.0) / std::f64::consts::PI;
                if x < 1.0 { -1.0 + x * 2.0 } else { 3.0 - x * 2.0 }
            }
            Kind::Sawtooth => posmod(l(), std::f64::consts::PI * 2.0) / std::f64::consts::PI - 1.0,
            Kind::Mod => l() % r(),
            Kind::Pwl => self.pwl(es),
            Kind::Pwr => l().abs().powf(r()),
            Kind::Pwrs => {
                let x = l();
                if x < 0.0 { -(-x).powf(r()) } else { x.powf(r()) }
            }
            Kind::LastOutput => es.last_output,
            Kind::TimeStep => es.time_step,
            Kind::Var(i) => es.values[i],
            Kind::LastVar(i) => es.last_values[i],
            Kind::Dadt(i) => (es.values[i] - es.last_values[i]) / es.time_step,
            Kind::Ref(k) => es.ref_values.get(k).copied().unwrap_or(0.0),
            Kind::RefDt(k) => {
                let now = es.ref_values.get(k).copied().unwrap_or(0.0);
                let last = es.last_ref_values.get(k).copied().unwrap_or(0.0);
                (now - last) / es.time_step
            }
        }
    }

    fn pwl(&self, es: &ExprState) -> f64 {
        let args = &self.children;
        let arg = |i: usize| args.get(i).map_or(0.0, |c| c.eval(es));
        let x = arg(0);
        let (mut x0, mut y0) = (arg(1), arg(2));
        if x < x0 {
            return y0;
        }
        let (mut x1, mut y1) = (arg(3), arg(4));
        let mut i = 5;
        loop {
            if x < x1 {
                return y0 + (x - x0) * (y1 - y0) / (x1 - x0);
            }
            if i + 1 >= args.len() {
                break;
            }
            x0 = x1;
            y0 = y1;
            x1 = arg(i);
            y1 = arg(i + 1);
            i += 2;
        }
        y1
    }
}

/// How deep parentheses, function calls and prefix operators may nest. The parser calls itself
/// for each, so a long run of them (typed or in a damaged file) would overflow the stack.
const MAX_DEPTH: usize = 128;

struct Parser {
    /// The text in lower case, one entry per character of `orig`.
    text: Vec<char>,
    /// The text as typed: names in `v(name)` keep their case.
    orig: Vec<char>,
    token: String,
    pos: usize,
    err: Option<String>,
    depth: usize,
    refs: Vec<ExprRef>,
}

impl Parser {
    fn new(s: &str) -> Parser {
        let orig: Vec<char> = s.chars().collect();
        // lower-case character by character so `text` and `orig` stay aligned
        let text = orig.iter().map(|c| c.to_lowercase().next().unwrap_or(*c)).collect();
        let mut p = Parser {
            text,
            orig,
            token: String::new(),
            pos: 0,
            err: None,
            depth: 0,
            refs: Vec::new(),
        };
        p.get_token();
        p
    }

    /// True when the next non-space character of the text is `(`; tells the input letter `i`
    /// (and the pin derivative `didt`) apart from the functions `i(` and `didt(`.
    fn next_char_is_open_paren(&self) -> bool {
        let mut i = self.pos;
        while i < self.orig.len() && self.orig[i] == ' ' {
            i += 1;
        }
        i < self.orig.len() && self.orig[i] == '('
    }

    /// Reads a name out of the original text, up to `,` or `)`; a double-quoted name may
    /// contain those characters.
    fn scan_name(&mut self) -> String {
        let n = self.orig.len();
        while self.pos < n && self.orig[self.pos] == ' ' {
            self.pos += 1;
        }
        let name: String = if self.pos < n && self.orig[self.pos] == '"' {
            self.pos += 1;
            let start = self.pos;
            while self.pos < n && self.orig[self.pos] != '"' {
                self.pos += 1;
            }
            let s = self.orig[start..self.pos].iter().collect();
            if self.pos < n {
                self.pos += 1; // the closing quote
            }
            while self.pos < n && self.orig[self.pos] == ' ' {
                self.pos += 1;
            }
            s
        } else {
            let start = self.pos;
            while self.pos < n && self.orig[self.pos] != ',' && self.orig[self.pos] != ')' {
                self.pos += 1;
            }
            self.orig[start..self.pos].iter().collect::<String>().trim().to_string()
        };
        if name.is_empty() {
            self.set_error("missing name in a v()/i() reference".into());
        }
        name
    }

    /// The node reading reference slot `name`/`current`, allocating the slot on first use.
    fn make_ref(&mut self, name: String, current: bool, deriv: bool) -> Expr {
        let k = match self.refs.iter().position(|r| r.name == name && r.current == current) {
            Some(k) => k,
            None => {
                self.refs.push(ExprRef { name, current });
                self.refs.len() - 1
            }
        };
        Expr::leaf(if deriv { Kind::RefDt(k) } else { Kind::Ref(k) })
    }

    /// Parses `v(name)`, `v(name1, name2)` or `i(name)` (and the `dvdt`/`didt` forms). Called
    /// with the function name skipped: the token is `(` and `pos` is just after it.
    fn parse_ref(&mut self, current: bool, deriv: bool) -> Expr {
        let func = format!(
            "{}{}{}",
            if deriv { "d" } else { "" },
            if current { "i" } else { "v" },
            if deriv { "dt" } else { "" }
        );
        if self.token != "(" {
            let got = self.token.clone();
            self.set_error(format!("expected ( after {func}, got {got}"));
            return Expr::val(0.0);
        }
        let first = self.scan_name();
        let mut e = self.make_ref(first, current, deriv);
        if self.pos < self.orig.len() && self.orig[self.pos] == ',' {
            self.pos += 1;
            let second = self.scan_name();
            let e2 = self.make_ref(second, current, deriv);
            if current {
                self.set_error(format!("{func}() takes one name"));
            } else {
                e = Expr::node(Kind::Sub, vec![e, e2]);
            }
        }
        if self.pos < self.orig.len() && self.orig[self.pos] == ')' {
            self.pos += 1;
        } else {
            self.set_error(format!("expected ) in {func}()"));
        }
        self.get_token();
        e
    }

    fn get_token(&mut self) {
        let n = self.text.len();
        while self.pos < n && self.text[self.pos] == ' ' {
            self.pos += 1;
        }
        if self.pos == n {
            self.token.clear();
            return;
        }
        let c = self.text[self.pos];
        let mut i = self.pos;
        if c.is_ascii_digit() || c == '.' {
            while i != n {
                if self.text[i] == 'e' {
                    i += 1;
                    if i < n && (self.text[i] == '+' || self.text[i] == '-') {
                        i += 1;
                    }
                }
                if i >= n || !(self.text[i].is_ascii_digit() || self.text[i] == '.') {
                    break;
                }
                i += 1;
            }
        } else if c.is_ascii_lowercase() {
            while i != n && self.text[i].is_ascii_lowercase() {
                i += 1;
            }
        } else {
            i += 1;
            if i < n {
                let d = self.text[i];
                if d == c && matches!(c, '|' | '&' | '<' | '>' | '=')
                    || (matches!(c, '<' | '>' | '!') && d == '=')
                {
                    i += 1;
                }
            }
        }
        self.token = self.text[self.pos..i].iter().collect();
        self.pos = i;
    }

    fn skip(&mut self, s: &str) -> bool {
        if self.token != s {
            return false;
        }
        self.get_token();
        true
    }

    fn set_error(&mut self, s: String) {
        if self.err.is_none() {
            self.err = Some(s);
        }
    }

    fn skip_or_error(&mut self, s: &str) {
        if !self.skip(s) {
            let got = self.token.clone();
            self.set_error(format!("expected {s}, got {got}"));
        }
    }

    fn parse_expression(&mut self) -> Expr {
        if self.token.is_empty() {
            return Expr::val(0.0);
        }
        let e = self.parse();
        if !self.token.is_empty() {
            let t = self.token.clone();
            self.set_error(format!("unexpected token: {t}"));
        }
        e
    }

    /// Enters one more level of nesting; false (with the error set) when that is too many.
    fn enter(&mut self) -> bool {
        if self.depth >= MAX_DEPTH {
            self.set_error("expression nested too deeply".to_string());
            return false;
        }
        self.depth += 1;
        true
    }

    fn parse(&mut self) -> Expr {
        if !self.enter() {
            return Expr::val(0.0);
        }
        let e = self.parse_ternary();
        self.depth -= 1;
        e
    }

    fn parse_ternary(&mut self) -> Expr {
        let e = self.parse_or();
        if self.skip("?") {
            let e2 = self.parse_or();
            self.skip_or_error(":");
            let e3 = self.parse();
            return Expr::node(Kind::Ternary, vec![e, e2, e3]);
        }
        e
    }

    fn binary(&mut self, left: Expr, kind: Kind, right: Expr) -> Expr {
        Expr::node(kind, vec![left, right])
    }

    fn parse_or(&mut self) -> Expr {
        let mut e = self.parse_and();
        while self.skip("||") {
            let r = self.parse_and();
            e = self.binary(e, Kind::Or, r);
        }
        e
    }

    fn parse_and(&mut self) -> Expr {
        let mut e = self.parse_bit_or();
        while self.skip("&&") {
            let r = self.parse_bit_or();
            e = self.binary(e, Kind::And, r);
        }
        e
    }

    fn parse_bit_or(&mut self) -> Expr {
        let mut e = self.parse_bit_and();
        while self.skip("|") {
            let r = self.parse_bit_and();
            e = self.binary(e, Kind::BitOr, r);
        }
        e
    }

    fn parse_bit_and(&mut self) -> Expr {
        let mut e = self.parse_equals();
        while self.skip("&") {
            let r = self.parse_equals();
            e = self.binary(e, Kind::BitAnd, r);
        }
        e
    }

    fn parse_equals(&mut self) -> Expr {
        let e = self.parse_compare();
        if self.skip("==") {
            let r = self.parse_compare();
            return self.binary(e, Kind::Equals, r);
        }
        e
    }

    fn parse_compare(&mut self) -> Expr {
        let e = self.parse_shift();
        for (tok, kind) in [
            ("<=", Kind::Leq),
            (">=", Kind::Geq),
            ("!=", Kind::Neq),
            ("<", Kind::Less),
            (">", Kind::Greater),
        ] {
            if self.skip(tok) {
                let r = self.parse_shift();
                return self.binary(e, kind, r);
            }
        }
        e
    }

    fn parse_shift(&mut self) -> Expr {
        let mut e = self.parse_add();
        while self.skip(">>") {
            let r = self.parse_add();
            e = self.binary(e, Kind::Rshift, r);
        }
        e
    }

    fn parse_add(&mut self) -> Expr {
        let mut e = self.parse_mult();
        loop {
            if self.skip("+") {
                let r = self.parse_mult();
                e = self.binary(e, Kind::Add, r);
            } else if self.skip("-") {
                let r = self.parse_mult();
                e = self.binary(e, Kind::Sub, r);
            } else {
                return e;
            }
        }
    }

    fn parse_mult(&mut self) -> Expr {
        let mut e = self.parse_uminus();
        loop {
            if self.skip("*") {
                let r = self.parse_uminus();
                e = self.binary(e, Kind::Mul, r);
            } else if self.skip("/") {
                let r = self.parse_uminus();
                e = self.binary(e, Kind::Div, r);
            } else {
                return e;
            }
        }
    }

    fn parse_uminus(&mut self) -> Expr {
        if !self.enter() {
            return Expr::val(0.0);
        }
        let e = self.parse_prefixed();
        self.depth -= 1;
        e
    }

    fn parse_prefixed(&mut self) -> Expr {
        self.skip("+");
        if self.skip("!") {
            let e = self.parse_uminus();
            return Expr::node(Kind::Not, vec![e]);
        }
        if self.skip("-") {
            let e = self.parse_uminus();
            return Expr::node(Kind::Uminus, vec![e]);
        }
        self.parse_pow()
    }

    fn parse_pow(&mut self) -> Expr {
        let mut e = self.parse_term();
        while self.skip("^") {
            let r = self.parse_term();
            e = self.binary(e, Kind::Pow, r);
        }
        e
    }

    fn parse_func(&mut self, kind: Kind) -> Expr {
        self.skip_or_error("(");
        let e = self.parse();
        self.skip_or_error(")");
        Expr::node(kind, vec![e])
    }

    fn parse_func_multi(&mut self, kind: Kind, min_args: usize, max_args: usize) -> Expr {
        self.skip_or_error("(");
        let mut args = vec![self.parse()];
        while self.skip(",") {
            args.push(self.parse());
        }
        self.skip_or_error(")");
        if args.len() < min_args || args.len() > max_args {
            self.set_error(format!("bad number of function args: {}", args.len()));
        }
        Expr::node(kind, args)
    }

    fn parse_term(&mut self) -> Expr {
        if self.skip("(") {
            let e = self.parse();
            self.skip_or_error(")");
            return e;
        }
        if self.skip("t") {
            return Expr::leaf(Kind::T);
        }
        // i(name) before the input letters: only an `i` directly followed by `(` is a
        // reference, any other `i` is still the 9th input
        if self.token == "i" && self.next_char_is_open_paren() {
            self.get_token();
            return self.parse_ref(true, false);
        }
        // dvdt(name)/didt(name); `didt` without `(` keeps its old meaning (d/dt of input i)
        if (self.token == "dvdt" || self.token == "didt") && self.next_char_is_open_paren() {
            let current = self.token == "didt";
            self.get_token();
            return self.parse_ref(current, true);
        }
        let tok: Vec<char> = self.token.chars().collect();
        let letter = |c: char| ('a'..='i').contains(&c).then(|| c as usize - 'a' as usize);
        if tok.len() == 1 {
            if let Some(i) = letter(tok[0]) {
                self.get_token();
                return Expr::leaf(Kind::Var(i));
            }
        }
        if self.token.starts_with("last") && tok.len() == 5 {
            if let Some(i) = letter(tok[4]) {
                self.get_token();
                return Expr::leaf(Kind::LastVar(i));
            }
        }
        if self.token.starts_with('d') && self.token.ends_with("dt") && tok.len() == 4 {
            if let Some(i) = letter(tok[1]) {
                self.get_token();
                return Expr::leaf(Kind::Dadt(i));
            }
        }
        if self.skip("lastoutput") {
            return Expr::leaf(Kind::LastOutput);
        }
        if self.skip("timestep") {
            return Expr::leaf(Kind::TimeStep);
        }
        if self.skip("pi") {
            return Expr::val(std::f64::consts::PI);
        }
        if self.skip("v") {
            return self.parse_ref(false, false);
        }
        const FUNCS: [(&str, Kind); 17] = [
            ("sin", Kind::Sin),
            ("cos", Kind::Cos),
            ("asin", Kind::Asin),
            ("acos", Kind::Acos),
            ("atan", Kind::Atan),
            ("sinh", Kind::Sinh),
            ("cosh", Kind::Cosh),
            ("tanh", Kind::Tanh),
            ("abs", Kind::Abs),
            ("exp", Kind::Exp),
            ("log", Kind::Log),
            ("sqrt", Kind::Sqrt),
            ("tan", Kind::Tan),
            ("tri", Kind::Triangle),
            ("saw", Kind::Sawtooth),
            ("floor", Kind::Floor),
            ("ceil", Kind::Ceil),
        ];
        for (name, kind) in FUNCS {
            if self.skip(name) {
                return self.parse_func(kind);
            }
        }
        const MULTI: [(&str, Kind, usize, usize); 8] = [
            ("min", Kind::Min, 2, 1000),
            ("max", Kind::Max, 2, 1000),
            ("pwl", Kind::Pwl, 2, 1000),
            ("mod", Kind::Mod, 2, 2),
            ("step", Kind::Step, 1, 2),
            ("select", Kind::Select, 3, 3),
            ("clamp", Kind::Clamp, 3, 3),
            ("pwr", Kind::Pwr, 2, 2),
        ];
        for (name, kind, lo, hi) in MULTI {
            if self.skip(name) {
                return self.parse_func_multi(kind, lo, hi);
            }
        }
        if self.skip("pwrs") {
            return self.parse_func_multi(Kind::Pwrs, 2, 2);
        }
        match self.token.parse::<f64>() {
            Ok(v) if !self.token.is_empty() => {
                self.get_token();
                Expr::val(v)
            }
            _ => {
                if self.token.is_empty() {
                    self.set_error("unexpected end of input".into());
                } else {
                    let t = self.token.clone();
                    self.set_error(format!("unrecognized token: {t}"));
                }
                Expr::val(0.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(text: &str) -> f64 {
        Expr::parse(text).unwrap().eval(&ExprState::new())
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("1+2*3"), 7.0);
        assert_eq!(eval("(1+2)*3"), 9.0);
        assert_eq!(eval("2^3^2"), 64.0); // left to right, as in the original
        assert_eq!(eval("-2^2"), -4.0); // the power binds tighter than the unary minus
        assert_eq!(eval("mod(7, 4)"), 3.0);
    }

    #[test]
    fn variables_and_functions() {
        let mut es = ExprState::new();
        es.values[0] = 3.0;
        assert_eq!(Expr::parse("a*2").unwrap().eval(&es), 6.0);
        assert_eq!(Expr::parse("max(a, 5, 1)").unwrap().eval(&es), 5.0);
        assert_eq!(Expr::parse("clamp(a, 0, 2)").unwrap().eval(&es), 2.0);
        assert!((Expr::parse("e").unwrap().eval(&es) - std::f64::consts::E).abs() < 1e-12);
        assert_eq!(Expr::parse("pwl(1, 0, 0, 2, 10)").unwrap().eval(&es), 5.0);
    }

    #[test]
    fn comparisons_and_ternary() {
        assert_eq!(eval("3 > 2 ? 10 : 20"), 10.0);
        assert_eq!(eval("1 == 2 || 2 == 2"), 1.0);
        assert_eq!(eval("!0"), 1.0);
        assert_eq!(eval("6 & 3"), 2.0);
        assert_eq!(eval("16 >> 2"), 4.0);
    }

    fn refs(text: &str) -> Vec<(String, bool)> {
        let (_, r) = Expr::parse_with_refs(text).unwrap();
        r.into_iter().map(|r| (r.name, r.current)).collect()
    }

    #[test]
    fn reference_functions_allocate_slots() {
        assert_eq!(refs("v(a)"), vec![("a".into(), false)]);
        assert_eq!(refs("i(a)"), vec![("a".into(), true)]);
        assert_eq!(refs("dvdt(a)"), vec![("a".into(), false)]);
        assert_eq!(refs("didt(a)"), vec![("a".into(), true)]);
        // the same quantity shares a slot, a voltage and a current of one name do not
        assert_eq!(refs("v(x)+dvdt(x)+v(x)"), vec![("x".into(), false)]);
        assert_eq!(refs("v(x)+i(x)"), vec![("x".into(), false), ("x".into(), true)]);
        // names keep their case and may be quoted
        assert_eq!(refs("v(Vout)"), vec![("Vout".into(), false)]);
        assert_eq!(refs("v(\"a,b\")"), vec![("a,b".into(), false)]);
        assert_eq!(refs("v( in )"), vec![("in".into(), false)]);
    }

    #[test]
    fn reference_values_come_from_the_state() {
        let mut es = ExprState::new();
        let (e, r) = Expr::parse_with_refs("2*v(in)").unwrap();
        es.alloc_refs(r.len());
        es.ref_values[0] = 1.5;
        assert_eq!(e.eval(&es), 3.0);
        // v(a, b) is the difference of the two voltages
        let (e, r) = Expr::parse_with_refs("v(p, n)").unwrap();
        es.alloc_refs(r.len());
        es.ref_values.copy_from_slice(&[5.0, 2.0]);
        assert_eq!(e.eval(&es), 3.0);
        // dvdt reads (now - one step ago) / timestep
        let (e, r) = Expr::parse_with_refs("dvdt(x)").unwrap();
        es.alloc_refs(r.len());
        es.time_step = 0.5;
        es.ref_values[0] = 1.0;
        es.update_last_values(0.0);
        es.ref_values[0] = 2.0;
        assert_eq!(e.eval(&es), 2.0);
        es.reset();
        assert_eq!(es.last_ref_values, vec![0.0]);
    }

    #[test]
    fn old_meanings_of_i_and_didt_are_kept() {
        let mut es = ExprState::new();
        es.values[8] = 4.0;
        // `i` alone is the 9th input
        let (e, r) = Expr::parse_with_refs("i*2").unwrap();
        assert!(r.is_empty());
        assert_eq!(e.eval(&es), 8.0);
        // `didt` alone is the derivative of input i
        let (_, r) = Expr::parse_with_refs("didt").unwrap();
        assert!(r.is_empty());
        // `dvdt` alone was never valid
        assert!(Expr::parse("dvdt").is_err());
        assert!(Expr::parse("dvdt + 1").is_err());
    }

    #[test]
    fn bad_references_are_reported() {
        assert!(Expr::parse("v()").is_err());
        assert!(Expr::parse("v(a").is_err());
        assert!(Expr::parse("i(a, b)").is_err());
        assert!(Expr::parse("v 1").is_err());
    }

    #[test]
    fn errors_are_reported() {
        assert!(Expr::parse("1 +").is_err());
        assert!(Expr::parse("foo").is_err());
        assert!(Expr::parse("(1").is_err());
        assert!(Expr::parse("min(1)").is_err());
        assert_eq!(eval(""), 0.0);
    }
}
