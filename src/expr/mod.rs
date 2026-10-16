// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (Expr, ExprParser, ExprState).
//! Expression evaluator used by custom elements (controlled sources, custom logic, lookup
//! tables): arithmetic, comparisons, a ternary operator and a set of functions over the
//! variables `a` to `i`, the time `t` and the previous values `lasta` to `lasti`.

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
    }

    /// Forgets the previous values.
    pub fn reset(&mut self) {
        self.last_values = [0.0; VARIABLES];
        self.last_output = 0.0;
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
        let mut p = Parser::new(text);
        let e = p.parse_expression();
        match p.err {
            Some(err) => Err(err),
            None => Ok(e),
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

struct Parser {
    text: Vec<char>,
    token: String,
    pos: usize,
    err: Option<String>,
}

impl Parser {
    fn new(s: &str) -> Parser {
        let mut p = Parser {
            text: s.to_lowercase().chars().collect(),
            token: String::new(),
            pos: 0,
            err: None,
        };
        p.get_token();
        p
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

    fn parse(&mut self) -> Expr {
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

    #[test]
    fn errors_are_reported() {
        assert!(Expr::parse("1 +").is_err());
        assert!(Expr::parse("foo").is_err());
        assert!(Expr::parse("(1").is_err());
        assert!(Expr::parse("min(1)").is_err());
        assert_eq!(eval(""), 0.0);
    }
}
