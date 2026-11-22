//! Data items, types, L-values, and the transfer functions and polymorphic
//! operators of Sections 3 and 6 of the Reference Manual.

use crate::ast::{BinOp, RelOp, UnOp};
use crate::code::{Kind, Proto};
use crate::vm::{Vm, R};
use std::cell::{Cell as Flag, RefCell};
use std::collections::VecDeque;
use std::fmt;
use std::rc::Rc;

/// Width of `logical` and `long logical` on Atlas (half word and whole word).
pub const M24: u64 = (1 << 24) - 1;
pub const M48: u64 = (1 << 48) - 1;

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Index,
    Integer,
    Real,
    Double,
    Complex,
    DComplex,
    Boolean,
    Logical,
    LongLogical,
    String,
    Label,
    Routine,
    Type,
    General,
    List,
    Function(Box<Ty>),
    Array(Box<Ty>, u8),
}

impl Ty {
    pub fn name(&self) -> String {
        match self {
            Ty::Index => "index".into(),
            Ty::Integer => "integer".into(),
            Ty::Real => "real".into(),
            Ty::Double => "double".into(),
            Ty::Complex => "complex".into(),
            Ty::DComplex => "double complex".into(),
            Ty::Boolean => "Boolean".into(),
            Ty::Logical => "logical".into(),
            Ty::LongLogical => "long logical".into(),
            Ty::String => "string".into(),
            Ty::Label => "label".into(),
            Ty::Routine => "routine".into(),
            Ty::Type => "type".into(),
            Ty::General => "general".into(),
            Ty::List => "list".into(),
            Ty::Function(r) if **r == Ty::General => "function".into(),
            Ty::Function(r) => format!("{} function", r.name()),
            Ty::Array(e, 1) => format!("{} vector", e.name()),
            Ty::Array(e, 2) => format!("{} matrix", e.name()),
            Ty::Array(e, n) => format!("{} {} array", e.name(), n),
        }
    }

    /// Position in the hierarchy of numerical types (3.3.3).
    pub fn rank(&self) -> Option<u8> {
        Some(match self {
            Ty::Index => 0,
            Ty::Integer => 1,
            Ty::Real => 2,
            Ty::Double => 3,
            Ty::Complex => 4,
            Ty::DComplex => 5,
            _ => return None,
        })
    }

    pub fn is_numeric(&self) -> bool {
        self.rank().is_some()
    }
}

/// A storage location: the simplest kind of L-value. Its type and its
/// constancy attribute are fixed when it is created (7.3).
pub struct CellData {
    pub val: RefCell<Value>,
    pub ty: Ty,
    pub constant: Flag<bool>,
}

pub type CellRef = Rc<CellData>;

pub fn new_cell(v: Value, ty: Ty, constant: bool) -> CellRef {
    Rc::new(CellData {
        val: RefCell::new(v),
        ty,
        constant: Flag::new(constant),
    })
}

/// An L-value (1.3.4, 10.3.5). Either a location, or a load-update pair
/// produced by an LH function, or an unevaluated expression (substitution).
#[derive(Clone)]
pub enum LV {
    Cell(CellRef),
    LoadUpdate(Rc<Closure>, Rc<Closure>),
    Subst(Rc<Closure>),
    /// Bits lo to hi of a logical variable: what Bit and Field give when
    /// their first argument has an L-value (Elementary Manual 25.3).
    Field(Box<LV>, u32, u32),
}

/// A function or routine R-value: code plus its free variable list.
pub struct Closure {
    pub proto: Rc<Proto>,
    pub caps: Vec<LV>,
}

pub struct LabelVal {
    pub pc: usize,
    pub frame: u64,
    pub name: String,
}

pub struct ArrayObj {
    pub lo: i64,
    pub cells: Vec<CellRef>,
    pub elem: Ty,
    pub dims: u8,
    pub fixed: bool,
}

pub struct ConsCell {
    pub hd: CellRef,
    pub tl: CellRef,
}

/// A long list is a long chain of cells; freed the obvious way, each cell
/// would free the next from inside its own destructor and a list of a few
/// thousand members would overflow the stack. So the chain is undone in a
/// loop, as far as this cell is the only owner of it.
impl Drop for ConsCell {
    fn drop(&mut self) {
        let sole = |c: &CellRef| Rc::strong_count(c) == 1;
        if !sole(&self.tl) {
            return;
        }
        let mut next = self.tl.val.replace(Value::Nil);
        while let Value::Cons(rc) = next {
            match Rc::try_unwrap(rc) {
                Ok(cell) if sole(&cell.tl) => next = cell.tl.val.replace(Value::Nil),
                _ => break,
            }
        }
    }
}

pub struct NativeFn {
    pub name: String,
    #[allow(clippy::type_complexity)]
    pub f: Box<dyn Fn(&mut Vm, Vec<Value>) -> R<Value>>,
}

pub enum Seg {
    One(Value),
    Step { start: Value, step: Value, r: i64, n: i64 },
}

pub struct ForIter {
    pub segs: VecDeque<Seg>,
}

#[derive(Clone)]
pub enum Value {
    Undef,
    Index(i64),
    Integer(i64),
    Real(f64),
    Double(f64),
    Complex(f64, f64),
    DComplex(f64, f64),
    Bool(bool),
    Logical(u64),
    LongLogical(u64),
    Str(Rc<str>),
    Label(Rc<LabelVal>),
    Func(Rc<Closure>),
    Builtin(usize),
    Native(Rc<NativeFn>),
    Array(Rc<ArrayObj>),
    Type(Ty),
    Nil,
    Cons(Rc<ConsCell>),
    /// Several results travelling together (an expression list, 4.5).
    Multi(Rc<Vec<Value>>),
    /// An L-value in transit: an argument or result evaluated in L-mode.
    Ref(LV),
    Iter(Rc<RefCell<ForIter>>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Str(s) => write!(f, "'{}'", s.replace('\n', "*n")),
            v => write!(f, "{}", display(v)),
        }
    }
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(Rc::from(s))
    }

    pub fn ty(&self) -> Ty {
        match self {
            Value::Index(_) => Ty::Index,
            Value::Integer(_) => Ty::Integer,
            Value::Real(_) => Ty::Real,
            Value::Double(_) => Ty::Double,
            Value::Complex(..) => Ty::Complex,
            Value::DComplex(..) => Ty::DComplex,
            Value::Bool(_) => Ty::Boolean,
            Value::Logical(_) => Ty::Logical,
            Value::LongLogical(_) => Ty::LongLogical,
            Value::Str(_) => Ty::String,
            Value::Label(_) => Ty::Label,
            Value::Func(c) if c.proto.kind == Kind::Routine => Ty::Routine,
            Value::Func(_) | Value::Builtin(_) | Value::Native(_) => Ty::Function(Box::new(Ty::General)),
            Value::Array(a) => Ty::Array(Box::new(a.elem.clone()), a.dims),
            Value::Type(_) => Ty::Type,
            Value::Nil | Value::Cons(_) => Ty::List,
            Value::Undef | Value::Multi(_) | Value::Ref(_) | Value::Iter(_) => Ty::General,
        }
    }

    pub fn is_callable(&self) -> bool {
        matches!(self, Value::Func(_) | Value::Builtin(_) | Value::Native(_))
    }
}

/// A numerical constant takes its minimum type from its value (3.4.3).
pub fn number(x: f64) -> Value {
    if x == x.trunc() && x.abs() < 9.0e15 {
        int_value(x as i64)
    } else {
        Value::Real(x)
    }
}

pub fn int_value(n: i64) -> Value {
    if fits_index(n) {
        Value::Index(n)
    } else {
        Value::Integer(n)
    }
}

fn fits_index(n: i64) -> bool {
    n >= i32::MIN as i64 && n <= i32::MAX as i64
}

// ---------------------------------------------------------------- printing

pub fn fmt_real(x: f64) -> String {
    if x.is_nan() || x.is_infinite() {
        return format!("{}", x);
    }
    if x == x.trunc() && x.abs() < 1e15 {
        return format!("{:.1}", x);
    }
    let e = format!("{:.11e}", x);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let trim = |s: &str| {
        let t = s.trim_end_matches('0');
        if t.ends_with('.') {
            format!("{}0", t)
        } else {
            t.to_string()
        }
    };
    if (-5..15).contains(&exp) {
        let dec = (11 - exp).max(1) as usize;
        trim(&format!("{:.*}", dec, x))
    } else {
        format!("{}@{}", trim(mant), exp)
    }
}

pub fn display(v: &Value) -> String {
    match v {
        Value::Undef => "<undefined>".into(),
        Value::Index(n) | Value::Integer(n) => n.to_string(),
        Value::Real(x) | Value::Double(x) => fmt_real(*x),
        Value::Complex(a, b) | Value::DComplex(a, b) => format!("({}, {})", fmt_real(*a), fmt_real(*b)),
        Value::Bool(b) => b.to_string(),
        Value::Logical(b) | Value::LongLogical(b) => format!("#o{:o}", b),
        Value::Str(s) => s.to_string(),
        Value::Label(l) => format!("<label {}>", l.name),
        Value::Func(c) => {
            let k = if c.proto.kind == Kind::Routine {
                "routine"
            } else {
                "function"
            };
            format!("<{} {}>", k, c.proto.name)
        }
        Value::Builtin(i) => format!("<basic function {}>", crate::builtins::TABLE[*i].0),
        Value::Native(n) => format!("<function {}>", n.name),
        Value::Array(a) => format!("<{}>", Ty::Array(Box::new(a.elem.clone()), a.dims).name()),
        Value::Type(t) => t.name(),
        Value::Nil => "NIL".into(),
        Value::Cons(_) => {
            let items: Vec<String> = list_members(v)
                .unwrap_or_default()
                .iter()
                .map(|m| match m {
                    Value::Str(s) => format!("'{}'", s),
                    m => display(m),
                })
                .collect();
            format!("({})", items.join(", "))
        }
        Value::Multi(vs) => vs.iter().map(display).collect::<Vec<_>>().join(", "),
        Value::Ref(_) => "<L-value>".into(),
        Value::Iter(_) => "<for-list>".into(),
    }
}

/// The members of a list as R-values; `None` if the value is not a proper list.
pub fn list_members(v: &Value) -> Option<Vec<Value>> {
    let mut out = Vec::new();
    let mut cur = v.clone();
    loop {
        match cur {
            Value::Nil => return Some(out),
            Value::Cons(c) => {
                out.push(c.hd.val.borrow().clone());
                let next = c.tl.val.borrow().clone();
                cur = next;
            }
            _ => return None,
        }
    }
}

pub fn make_list(items: Vec<Value>) -> Value {
    let mut l = Value::Nil;
    for v in items.into_iter().rev() {
        l = cons(v, l);
    }
    l
}

pub fn cons(hd: Value, tl: Value) -> Value {
    Value::Cons(Rc::new(ConsCell {
        hd: new_cell(hd, Ty::General, false),
        tl: new_cell(tl, Ty::General, false),
    }))
}

pub fn field_mask(lo: u32, hi: u32) -> u64 {
    ((1u64 << (hi - lo + 1)) - 1) << lo
}

pub fn logical_value(bits: u64, long: bool) -> Value {
    if long {
        Value::LongLogical(bits & M48)
    } else {
        Value::Logical(bits & M24)
    }
}

// ------------------------------------------------------- transfer functions

fn to_f(v: &Value) -> f64 {
    match v {
        Value::Index(n) | Value::Integer(n) => *n as f64,
        Value::Real(x) | Value::Double(x) => *x,
        Value::Complex(a, _) | Value::DComplex(a, _) => *a,
        _ => f64::NAN,
    }
}

fn to_c(v: &Value) -> (f64, f64) {
    match v {
        Value::Complex(a, b) | Value::DComplex(a, b) => (*a, *b),
        v => (to_f(v), 0.0),
    }
}

fn to_real(v: &Value) -> Result<f64, String> {
    if let Value::Complex(a, b) | Value::DComplex(a, b) = v {
        if b.abs() > 1e-12 * a.abs().max(1.0) {
            return Err(format!(
                "alarm: the complex number {} has an imaginary part that is not negligible",
                display(v)
            ));
        }
    }
    Ok(to_f(v))
}

fn to_int(v: &Value) -> Result<i64, String> {
    match v {
        Value::Index(n) | Value::Integer(n) => Ok(*n),
        v => {
            let x = to_real(v)?;
            if (x - x.round()).abs() > 1e-9 * x.abs().max(1.0) {
                return Err(format!(
                    "alarm: the real number {} is not integral (use Round or Intpt to make an integer of it)",
                    fmt_real(x)
                ));
            }
            if x.abs() >= 9.2e18 {
                return Err("alarm: number out of the integer range".into());
            }
            Ok(x.round() as i64)
        }
    }
}

/// Transfer a data item to the representation of another type (3.3). The
/// transfers between Boolean and index are only made when written explicitly.
pub fn transfer(v: Value, to: &Ty, explicit: bool) -> Result<Value, String> {
    if let Value::Multi(_) = v {
        return Err("an expression list with several values stands where a single value is needed".into());
    }
    if *to == Ty::General {
        return Ok(v);
    }
    let from = v.ty();
    if from == *to {
        return Ok(v);
    }
    let fail = |v: &Value| {
        Err(format!(
            "there is no transfer function from {} to {}",
            v.ty().name(),
            to.name()
        ))
    };
    if from.is_numeric() && to.is_numeric() {
        return Ok(match to {
            Ty::Index => {
                let n = to_int(&v)?;
                if !fits_index(n) {
                    return Err(format!("alarm: {} is outside the index range", n));
                }
                Value::Index(n)
            }
            Ty::Integer => Value::Integer(to_int(&v)?),
            Ty::Real => Value::Real(to_real(&v)?),
            Ty::Double => Value::Double(to_real(&v)?),
            Ty::Complex => {
                let (a, b) = to_c(&v);
                Value::Complex(a, b)
            }
            _ => {
                let (a, b) = to_c(&v);
                Value::DComplex(a, b)
            }
        });
    }
    match (&v, to) {
        (Value::Index(n) | Value::Integer(n), Ty::Logical | Ty::LongLogical) => {
            let m = if *to == Ty::Logical { M24 } else { M48 };
            if *n < 0 || *n as u64 > m {
                return Err(format!("alarm: {} does not fit in a {}", n, to.name()));
            }
            Ok(if *to == Ty::Logical {
                Value::Logical(*n as u64)
            } else {
                Value::LongLogical(*n as u64)
            })
        }
        (Value::Logical(b) | Value::LongLogical(b), Ty::Index | Ty::Integer) => {
            transfer(Value::Integer(*b as i64), to, explicit)
        }
        (Value::Logical(b), Ty::LongLogical) => Ok(Value::LongLogical(*b)),
        (Value::LongLogical(b), Ty::Logical) => {
            if *b > M24 {
                return Err("alarm: long logical with non-zero extra bits transferred to logical".into());
            }
            Ok(Value::Logical(*b))
        }
        (Value::Str(s), Ty::Index | Ty::Integer | Ty::Logical) => {
            let mut cs = s.chars();
            match (cs.next(), cs.next()) {
                (Some(c), None) => transfer(Value::Index(c as i64), to, explicit),
                _ => Err("only a string of length one (a character) transfers to a number".into()),
            }
        }
        (Value::Index(n) | Value::Integer(n), Ty::String) => {
            let code = *n;
            match u32::try_from(code).ok().and_then(char::from_u32) {
                Some(c) => Ok(Value::str(&c.to_string())),
                None => Err(format!("alarm: no character has the number {}", code)),
            }
        }
        (Value::Bool(b), Ty::Index | Ty::Integer | Ty::Real | Ty::Logical) if explicit => {
            transfer(Value::Index(*b as i64), to, true)
        }
        (Value::Index(n) | Value::Integer(n), Ty::Boolean) if explicit => match n {
            0 => Ok(Value::Bool(false)),
            1 => Ok(Value::Bool(true)),
            _ => Err(format!("alarm: {} is neither 0 nor 1 and has no Boolean equivalent", n)),
        },
        (v2, Ty::Function(_) | Ty::Routine) if v2.is_callable() => Ok(v),
        (Value::Array(_), Ty::Array(..)) => Ok(v),
        _ => fail(&v),
    }
}

// ------------------------------------------------- polymorphic operators

fn op_sym(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Pow => "**",
        BinOp::And => "/\\",
        BinOp::Or => "\\/",
        BinOp::Equiv => "==",
        BinOp::NotEquiv => "~==",
        BinOp::Concat => "<=>",
    }
}

fn cannot(op: BinOp, a: &Value, b: &Value) -> String {
    format!(
        "the operator {} is not defined for {} and {}",
        op_sym(op),
        a.ty().name(),
        b.ty().name()
    )
}

/// Least upper bound in the hierarchy of numerical types (3.3.3): index,
/// integer, real, double on one side, complex above real, double complex
/// above both.
pub fn rank_lub(ra: u8, rb: u8, min: u8) -> u8 {
    let cx = ra >= 4 || rb >= 4;
    let dbl = ra == 3 || ra == 5 || rb == 3 || rb == 5;
    if cx {
        if dbl {
            5
        } else {
            4
        }
    } else {
        ra.max(rb).max(min)
    }
}

pub fn rank_type(r: u8) -> Ty {
    [Ty::Index, Ty::Integer, Ty::Real, Ty::Double, Ty::Complex, Ty::DComplex][r as usize].clone()
}

/// Rule A of 3.3.4: the version of the operator is the least upper bound of
/// the operand types and of the lowest type for which the operator exists.
pub fn arith(op: BinOp, a: &Value, b: &Value) -> Result<Value, String> {
    let (ra, rb) = match (a.ty().rank(), b.ty().rank()) {
        (Some(x), Some(y)) => (x, y),
        _ => return Err(cannot(op, a, b)),
    };
    let min = if matches!(op, BinOp::Div | BinOp::Pow) { 2 } else { 0 };
    let r = rank_lub(ra, rb, min);
    match r {
        0 | 1 => {
            let (x, y) = (to_int(a)?, to_int(b)?);
            let z = match op {
                BinOp::Add => x.checked_add(y),
                BinOp::Sub => x.checked_sub(y),
                _ => x.checked_mul(y),
            }
            .ok_or("alarm: integer arithmetic out of range")?;
            if r == 0 {
                if !fits_index(z) {
                    return Err(format!(
                        "alarm: index arithmetic out of range ({} {} {}); use type integer",
                        x,
                        op_sym(op),
                        y
                    ));
                }
                Ok(Value::Index(z))
            } else {
                Ok(Value::Integer(z))
            }
        }
        2 | 3 => {
            let (x, y) = (to_f(a), to_f(b));
            let z = match op {
                BinOp::Add => x + y,
                BinOp::Sub => x - y,
                BinOp::Mul => x * y,
                BinOp::Div => {
                    if y == 0.0 {
                        return Err("alarm: division by zero".into());
                    }
                    x / y
                }
                _ => {
                    let z = x.powf(y);
                    if z.is_nan() {
                        return Err(format!("{} ** {} has no real value", fmt_real(x), fmt_real(y)));
                    }
                    z
                }
            };
            Ok(if r == 2 { Value::Real(z) } else { Value::Double(z) })
        }
        _ => {
            let ((a1, b1), (a2, b2)) = (to_c(a), to_c(b));
            let (x, y) = match op {
                BinOp::Add => (a1 + a2, b1 + b2),
                BinOp::Sub => (a1 - a2, b1 - b2),
                BinOp::Mul => (a1 * a2 - b1 * b2, a1 * b2 + b1 * a2),
                BinOp::Div => {
                    let d = a2 * a2 + b2 * b2;
                    if d == 0.0 {
                        return Err("alarm: division by zero".into());
                    }
                    ((a1 * a2 + b1 * b2) / d, (b1 * a2 - a1 * b2) / d)
                }
                _ if b2 == 0.0 && a2 == a2.trunc() && a2.abs() <= 4096.0 => {
                    // A whole number power: by repeated multiplication.
                    let (mut x, mut y) = (1.0, 0.0);
                    for _ in 0..a2.abs() as u32 {
                        (x, y) = (x * a1 - y * b1, x * b1 + y * a1);
                    }
                    if a2 < 0.0 {
                        let d = x * x + y * y;
                        if d == 0.0 {
                            return Err("alarm: division by zero".into());
                        }
                        (x, y) = (x / d, -y / d);
                    }
                    (x, y)
                }
                _ => {
                    if a1 == 0.0 && b1 == 0.0 {
                        if a2 == 0.0 && b2 == 0.0 {
                            (1.0, 0.0)
                        } else {
                            (0.0, 0.0)
                        }
                    } else {
                        let (lr, li) = ((a1 * a1 + b1 * b1).sqrt().ln(), b1.atan2(a1));
                        let (wr, wi) = (a2 * lr - b2 * li, a2 * li + b2 * lr);
                        (wr.exp() * wi.cos(), wr.exp() * wi.sin())
                    }
                }
            };
            Ok(if r == 4 {
                Value::Complex(x, y)
            } else {
                Value::DComplex(x, y)
            })
        }
    }
}

pub fn binary(op: BinOp, a: &Value, b: &Value) -> Result<Value, String> {
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow => arith(op, a, b),
        BinOp::Concat => match (a, b) {
            (Value::Str(x), Value::Str(y)) => Ok(Value::str(&format!("{}{}", x, y))),
            _ => Err(cannot(op, a, b)),
        },
        _ => {
            let bits = |x: u64, y: u64, m: u64| {
                (match op {
                    BinOp::And => x & y,
                    BinOp::Or => x | y,
                    BinOp::Equiv => !(x ^ y),
                    _ => x ^ y,
                }) & m
            };
            match (a, b) {
                (Value::Bool(x), Value::Bool(y)) => Ok(Value::Bool(match op {
                    BinOp::And => *x && *y,
                    BinOp::Or => *x || *y,
                    BinOp::Equiv => x == y,
                    _ => x != y,
                })),
                (Value::Logical(x), Value::Logical(y)) => Ok(Value::Logical(bits(*x, *y, M24))),
                (Value::Logical(x) | Value::LongLogical(x), Value::Logical(y) | Value::LongLogical(y)) => {
                    Ok(Value::LongLogical(bits(*x, *y, M48)))
                }
                _ => Err(cannot(op, a, b)),
            }
        }
    }
}

pub fn unary(op: UnOp, a: &Value) -> Result<Value, String> {
    match (op, a) {
        (UnOp::Neg, Value::Index(_) | Value::Integer(_)) => arith(BinOp::Sub, &Value::Index(0), a),
        (UnOp::Neg, Value::Real(x)) => Ok(Value::Real(-x)),
        (UnOp::Neg, Value::Double(x)) => Ok(Value::Double(-x)),
        (UnOp::Neg, Value::Complex(x, y)) => Ok(Value::Complex(-x, -y)),
        (UnOp::Neg, Value::DComplex(x, y)) => Ok(Value::DComplex(-x, -y)),
        (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
        (UnOp::Not, Value::Logical(b)) => Ok(Value::Logical(!b & M24)),
        (UnOp::Not, Value::LongLogical(b)) => Ok(Value::LongLogical(!b & M48)),
        (UnOp::Neg, _) => Err(format!("monadic - is not defined for {}", a.ty().name())),
        (UnOp::Not, _) => Err(format!("the operator ~ is not defined for {}", a.ty().name())),
    }
}

/// Equality of R-values (6.4, 10.2.4, 11.1.2). Functions, routines and free
/// arrays are equal only when they are the very same item.
pub fn values_equal(a: &Value, b: &Value) -> bool {
    if a.ty().is_numeric() && b.ty().is_numeric() {
        return to_c(a) == to_c(b);
    }
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Logical(x) | Value::LongLogical(x), Value::Logical(y) | Value::LongLogical(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        // Function types are not told apart by the types of their results.
        (Value::Type(Ty::Function(_)), Value::Type(Ty::Function(_))) => true,
        (Value::Type(x), Value::Type(y)) => x == y,
        (Value::Label(x), Value::Label(y)) => x.pc == y.pc && x.frame == y.frame,
        (Value::Func(x), Value::Func(y)) => Rc::ptr_eq(x, y),
        (Value::Builtin(x), Value::Builtin(y)) => x == y,
        (Value::Native(x), Value::Native(y)) => Rc::ptr_eq(x, y),
        (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y) || (x.fixed && y.fixed && arrays_equal(x, y)),
        (Value::Nil, Value::Nil) => true,
        (Value::Cons(_), Value::Cons(_)) => {
            // Down the tails in a loop: lists can be long.
            let (mut p, mut q) = (a.clone(), b.clone());
            loop {
                match (p, q) {
                    (Value::Cons(x), Value::Cons(y)) => {
                        if Rc::ptr_eq(&x, &y) {
                            return true;
                        }
                        if !values_equal(&x.hd.val.borrow(), &y.hd.val.borrow()) {
                            return false;
                        }
                        let (xt, yt) = (x.tl.val.borrow().clone(), y.tl.val.borrow().clone());
                        p = xt;
                        q = yt;
                    }
                    (x, y) => return values_equal(&x, &y),
                }
            }
        }
        _ => false,
    }
}

/// The basic function Equal: equality of arrays "in the mathematical sense".
pub fn arrays_equal(x: &ArrayObj, y: &ArrayObj) -> bool {
    x.lo == y.lo
        && x.cells.len() == y.cells.len()
        && x.cells
            .iter()
            .zip(&y.cells)
            .all(|(p, q)| match (&*p.val.borrow(), &*q.val.borrow()) {
                (Value::Array(a), Value::Array(b)) => arrays_equal(a, b),
                (Value::Undef, Value::Undef) => true,
                (a, b) => values_equal(a, b),
            })
}

pub fn relate(op: RelOp, a: &Value, b: &Value) -> Result<bool, String> {
    use std::cmp::Ordering::*;
    match op {
        RelOp::Eq => return Ok(values_equal(a, b)),
        RelOp::Ne => return Ok(!values_equal(a, b)),
        _ => {}
    }
    let numeric = |v: &Value| matches!(v.ty().rank(), Some(r) if r < 4);
    let ord = match (a, b) {
        (Value::Str(x), Value::Str(y)) if !matches!(op, RelOp::MuchLt | RelOp::MuchGt) => x.cmp(y),
        (Value::Index(x) | Value::Integer(x), Value::Index(y) | Value::Integer(y))
            if !matches!(op, RelOp::MuchLt | RelOp::MuchGt) =>
        {
            x.cmp(y)
        }
        _ if numeric(a) && numeric(b) => {
            let (x, y) = (to_f(a), to_f(b));
            match op {
                RelOp::MuchLt => return Ok(x + y == y && !(x == 0.0 && y == 0.0)),
                RelOp::MuchGt => return Ok(x + y == x && !(x == 0.0 && y == 0.0)),
                _ => x.partial_cmp(&y).ok_or("comparison of an undefined number")?,
            }
        }
        _ => {
            return Err(format!(
                "the relation is not defined between {} and {}",
                a.ty().name(),
                b.ty().name()
            ))
        }
    };
    Ok(match op {
        RelOp::Lt => ord == Less,
        RelOp::Le => ord != Greater,
        RelOp::Gt => ord == Greater,
        _ => ord != Less,
    })
}
