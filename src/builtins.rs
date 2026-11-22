//! Basic functions and routines: those of the Elementary Programming Manual
//! (sections 4.4, 25, 26.2), the transfer functions of 3.3.1, the array
//! functions of 11.2, and the list functions of the 1963 paper (section 23).

use crate::value::*;
use crate::vm::{err, Vm, R};
use std::rc::Rc;

pub type BFn = fn(&mut Vm, Vec<Value>) -> R<Value>;

pub enum BDef {
    F(BFn),
    C(fn() -> Value),
}

pub fn lookup(name: &str) -> Option<usize> {
    TABLE.iter().position(|(n, _)| *n == name)
}

fn lift<T>(r: Result<T, String>) -> R<T> {
    r.or_else(err)
}

/// The R-values of exactly `n` arguments.
fn args(vm: &mut Vm, a: Vec<Value>, n: usize, name: &str) -> R<Vec<Value>> {
    if a.len() != n {
        return err(format!("{} takes {} argument(s) but is given {}", name, n, a.len()));
    }
    vm.rvals(a)
}

fn real(v: &Value, name: &str) -> R<f64> {
    match lift(transfer(v.clone(), &Ty::Real, false)) {
        Ok(Value::Real(x)) => Ok(x),
        _ => err(format!(
            "{} needs a number, not a data item of type {}",
            name,
            v.ty().name()
        )),
    }
}

fn int(v: &Value, name: &str) -> R<i64> {
    match transfer(v.clone(), &Ty::Integer, false) {
        Ok(Value::Integer(n)) => Ok(n),
        Ok(_) => unreachable!(),
        Err(m) => err(format!("{}: {}", name, m)),
    }
}

/// The bits of a logical, how many there are, and whether it is long.
fn logical(v: &Value, name: &str) -> R<(u64, u32, bool)> {
    match v {
        Value::Logical(b) => Ok((*b, 24, false)),
        Value::LongLogical(b) => Ok((*b, 48, true)),
        v => err(format!(
            "{} needs a logical, not a data item of type {}",
            name,
            v.ty().name()
        )),
    }
}

fn mk_logical(bits: u64, long: bool) -> Value {
    logical_value(bits, long)
}

/// Two bit numbers, counted from 0 at the right hand end, in either order.
fn bit_range(j: &Value, k: &Value, width: u32, name: &str) -> R<(u32, u32)> {
    let (j, k) = (int(j, name)?, int(k, name)?);
    let (lo, hi) = (j.min(k), j.max(k));
    if lo < 0 || hi >= width as i64 {
        return err(format!(
            "{}: there is no bit {} in {} bits",
            name,
            if lo < 0 { lo } else { hi },
            width
        ));
    }
    Ok((lo as u32, hi as u32))
}

/// Bit[p, j] and Field[p, j, k]. "The functions Bit and Field may be used
/// on the left-hand side of assignment commands; their results, therefore,
/// are effectively the 'addresses' of the bit or area specified." So when
/// p is a variable the result is an L-value.
fn bit_field(vm: &mut Vm, mut a: Vec<Value>, name: &str, n: usize) -> R<Value> {
    if a.len() != n {
        return err(format!("{} takes {} argument(s) but is given {}", name, n, a.len()));
    }
    let target = a.remove(0);
    let rest = vm.rvals(a)?;
    let word = vm.rval(target.clone())?;
    let (bits, width, long) = logical(&word, name)?;
    let (lo, hi) = bit_range(&rest[0], rest.last().unwrap(), width, name)?;
    match target {
        Value::Ref(lv) => Ok(Value::Ref(LV::Field(Box::new(lv), lo, hi))),
        _ => Ok(mk_logical((bits & field_mask(lo, hi)) >> lo, long)),
    }
}

/// Lshift and Rshift: zeros move in, and "if j is negative the direction of
/// the shift is reversed" (Elementary Programming Manual, 25.3).
fn shift(vm: &mut Vm, a: Vec<Value>, name: &str, left: i64) -> R<Value> {
    let a = args(vm, a, 2, name)?;
    let (bits, _, long) = logical(&a[0], name)?;
    let n = int(&a[1], name)? * left;
    let by = n.unsigned_abs().min(63) as u32;
    Ok(mk_logical(if n >= 0 { bits << by } else { bits >> by }, long))
}

fn string(v: &Value, name: &str) -> R<Rc<str>> {
    match v {
        Value::Str(s) => Ok(s.clone()),
        v => err(format!(
            "{} needs a string, not a data item of type {}",
            name,
            v.ty().name()
        )),
    }
}

fn real1(vm: &mut Vm, a: Vec<Value>, name: &str, f: fn(f64) -> f64) -> R<Value> {
    let a = args(vm, a, 1, name)?;
    let y = f(real(&a[0], name)?);
    if y.is_nan() {
        return err(format!("{}[{}] is not defined", name, display(&a[0])));
    }
    Ok(Value::Real(y))
}

fn to_type(vm: &mut Vm, a: Vec<Value>, name: &str, ty: Ty) -> R<Value> {
    let mut a = args(vm, a, 1, name)?;
    lift(transfer(a.pop().unwrap(), &ty, true))
}

fn substring(vm: &mut Vm, a: Vec<Value>, name: &str, from_end: bool) -> R<Value> {
    let a = args(vm, a, 2, name)?;
    let n = int(&a[0], name)?.max(0) as usize;
    let cs: Vec<char> = string(&a[1], name)?.chars().collect();
    let n = n.min(cs.len());
    let part: String = if from_end {
        cs[cs.len() - n..].iter().collect()
    } else {
        cs[..n].iter().collect()
    };
    Ok(Value::str(&part))
}

fn new_array(ty: &Ty, bounds: &[(i64, i64)], fill: &mut dyn FnMut() -> Value) -> Rc<ArrayObj> {
    let (lo, hi) = bounds[0];
    let n = (hi - lo + 1).max(0) as usize;
    let dims = bounds.len() as u8;
    let cells = (0..n)
        .map(|_| {
            if dims == 1 {
                new_cell(fill(), ty.clone(), false)
            } else {
                let sub = new_array(ty, &bounds[1..], fill);
                new_cell(Value::Array(sub), Ty::Array(Box::new(ty.clone()), dims - 1), false)
            }
        })
        .collect();
    Rc::new(ArrayObj {
        lo,
        cells,
        elem: ty.clone(),
        dims,
        fixed: false,
    })
}

/// Copy (a new free array) and Fix (a fixed array, with unalterable
/// elements). Elements which are arrays are copied (11.2.1).
pub fn copy_array(a: &ArrayObj, fixed: bool) -> Rc<ArrayObj> {
    let cells = a
        .cells
        .iter()
        .map(|c| {
            let v = match &*c.val.borrow() {
                Value::Array(sub) => Value::Array(copy_array(sub, fixed)),
                v => v.clone(),
            };
            new_cell(v, c.ty.clone(), fixed)
        })
        .collect();
    Rc::new(ArrayObj {
        lo: a.lo,
        cells,
        elem: a.elem.clone(),
        dims: a.dims,
        fixed,
    })
}

/// The type and the subscript bounds given to Newarray and Formarray.
fn array_spec(vm: &mut Vm, a: Vec<Value>, name: &str) -> R<(Ty, Vec<(i64, i64)>)> {
    let a = vm.rvals(a)?;
    let Some(Value::Type(ty)) = a.first() else {
        return err(format!("the first argument of {} must be a type", name));
    };
    if a.len() < 3 || a.len() % 2 == 0 {
        return err(format!(
            "{} needs a type and then a lower and an upper bound for each dimension",
            name
        ));
    }
    let mut bounds = Vec::new();
    for pair in a[1..].chunks(2) {
        bounds.push((int(&pair[0], name)?, int(&pair[1], name)?));
    }
    Ok((ty.clone(), bounds))
}

fn array(v: &Value, name: &str) -> R<Rc<ArrayObj>> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        v => err(format!(
            "{} needs an array, not a data item of type {}",
            name,
            v.ty().name()
        )),
    }
}

fn b_formarray(vm: &mut Vm, a: Vec<Value>) -> R<Value> {
    let (ty, bounds) = array_spec(vm, a, "Formarray")?;
    let count: i64 = bounds.iter().map(|(lo, hi)| (hi - lo + 1).max(0)).product();
    let f = move |vm: &mut Vm, a: Vec<Value>| -> R<Value> {
        let a = vm.rvals(a)?;
        if a.len() as i64 != count {
            return err(format!(
                "the array has {} elements but {} values are given",
                count,
                a.len()
            ));
        }
        let mut vals = Vec::new();
        for v in a {
            vals.push(lift(transfer(v, &ty, false))?);
        }
        let mut it = vals.into_iter();
        Ok(Value::Array(new_array(&ty, &bounds, &mut || it.next().unwrap())))
    };
    Ok(Value::Native(Rc::new(NativeFn {
        name: "Formarray[...]".into(),
        f: Box::new(f),
    })))
}

fn b_newarray(vm: &mut Vm, a: Vec<Value>) -> R<Value> {
    let (ty, bounds) = array_spec(vm, a, "Newarray")?;
    Ok(Value::Array(new_array(&ty, &bounds, &mut || Value::Undef)))
}

fn list(v: &Value, name: &str) -> R<()> {
    match v {
        Value::Nil | Value::Cons(_) => Ok(()),
        v => err(format!(
            "{} needs a list, not a data item of type {}",
            name,
            v.ty().name()
        )),
    }
}

fn b_write(vm: &mut Vm, a: Vec<Value>) -> R<Value> {
    for v in vm.rvals(a)? {
        let s = display(&v);
        if vm.out.write_all(s.as_bytes()).is_err() {
            return err("the output stream cannot be written");
        }
    }
    Ok(Value::Undef)
}

fn put(vm: &mut Vm, s: &str) -> R<Value> {
    b_write(vm, vec![Value::str(s)])
}

/// Floating decimal style: a sign, i significant figures and a decimal
/// exponent, in i + 7 character positions.
fn floating(x: f64, i: usize) -> String {
    let i = i.max(1);
    let e = format!("{:.*e}", i - 1, x.abs());
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let body = format!(
        "{}{}@{}{:02}",
        if x < 0.0 { '-' } else { ' ' },
        mant,
        if exp < 0 { '-' } else { '+' },
        exp.abs()
    );
    format!("{:>1$}", body, i + 7)
}

/// PRINTF[x, i] of the pre-pilot system.
fn b_printf(vm: &mut Vm, a: Vec<Value>) -> R<Value> {
    let a = args(vm, a, 2, "Printf")?;
    let (x, i) = (real(&a[0], "Printf")?, int(&a[1], "Printf")?);
    if !(1..=15).contains(&i) {
        return err("Printf: the number of significant figures must be from 1 to 15");
    }
    put(vm, &floating(x, i as usize))
}

/// PRINTD[x, i, j]: i figures before the point and j after. "Non-significant
/// zeros are represented by spaces, and so is a plus sign. If j = 0 no point
/// is printed. If x >= 10^i, it is printed in floating decimal style."
fn b_printd(vm: &mut Vm, a: Vec<Value>) -> R<Value> {
    let a = args(vm, a, 3, "Printd")?;
    let (x, i, j) = (real(&a[0], "Printd")?, int(&a[1], "Printd")?, int(&a[2], "Printd")?);
    if i < 0 || j < 0 || i + j > 15 {
        return err("Printd: the numbers of figures must not be negative, nor more than 15 together");
    }
    let (i, j) = (i as usize, j as usize);
    let digits = format!("{:.*}", j, x.abs());
    let before = digits.split('.').next().unwrap().len();
    if before > i.max(1) {
        return put(vm, &floating(x, (i + j).clamp(1, 15)));
    }
    let signed = if x < 0.0 && digits.chars().any(|c| ('1'..='9').contains(&c)) {
        format!("-{}", digits)
    } else {
        digits
    };
    let width = i + 1 + if j > 0 { j + 1 } else { 0 };
    put(vm, &format!("{:>1$}", signed, width))
}

fn b_writechar(vm: &mut Vm, a: Vec<Value>) -> R<Value> {
    let a = args(vm, a, 1, "Writechar")?;
    match &a[0] {
        Value::Str(s) => put(vm, &s.clone()),
        v => {
            let n = int(v, "Writechar")?;
            match u32::try_from(n).ok().and_then(char::from_u32) {
                Some(c) => put(vm, &c.to_string()),
                None => err(format!("Writechar: {} is not the number of a character", n)),
            }
        }
    }
}

/// One character of the input stream, decoded from UTF-8.
fn read_char(vm: &mut Vm) -> Option<char> {
    if let Some(c) = vm.held.take() {
        return Some(c);
    }
    let mut bytes = [0u8; 4];
    let mut n = 0;
    loop {
        let buf = vm.inp.fill_buf().ok()?;
        let b = *buf.first()?;
        // A byte that cannot continue the character begins another.
        if n > 0 && b & 0xC0 != 0x80 {
            return Some(char::REPLACEMENT_CHARACTER);
        }
        vm.inp.consume(1);
        bytes[n] = b;
        n += 1;
        if let Ok(s) = std::str::from_utf8(&bytes[..n]) {
            return s.chars().next();
        }
        if n == 4 {
            return Some(char::REPLACEMENT_CHARACTER);
        }
    }
}

/// An underline or a stroke typed over a character makes one character of
/// it: "the set of CPL string characters contains all the permitted CPL
/// characters, possibly overpunched with a stroke or vertical bar or
/// both". Here the overstrikes are the Unicode combining marks.
pub fn is_overstrike(c: char) -> bool {
    ('\u{300}'..='\u{36f}').contains(&c)
}

/// The next CPL string character, with its overstrikes.
fn read_compound(vm: &mut Vm) -> Option<String> {
    let mut s = read_char(vm)?.to_string();
    while let Some(c) = read_char(vm) {
        if !is_overstrike(c) {
            vm.held = Some(c);
            break;
        }
        s.push(c);
    }
    Some(s)
}

fn b_read(vm: &mut Vm, a: Vec<Value>) -> R<Value> {
    let _ = vm.out.flush();
    for target in a {
        let Value::Ref(lv) = target else {
            return err("Read needs variables to read into");
        };
        let mut word = String::new();
        while let Some(c) = read_char(vm) {
            if c.is_whitespace() || c == ',' {
                if word.is_empty() {
                    continue;
                }
                break;
            }
            word.push(c);
        }
        if word.is_empty() {
            return err("Read: the input stream is exhausted");
        }
        let x: f64 = match word.replace('@', "e").parse() {
            Ok(x) => x,
            Err(_) => return err(format!("Read: '{}' is not a number", word)),
        };
        vm.update(&lv, number(x))?;
    }
    Ok(Value::Undef)
}

/// Readchar[x], or Readchar[stream, x] as the preprocessor of Appendix 1
/// writes it. There is one input stream, whatever it is called.
fn b_readchar(vm: &mut Vm, a: Vec<Value>) -> R<Value> {
    let _ = vm.out.flush();
    let lv = match &a[..] {
        [Value::Ref(lv)] => lv.clone(),
        [stream, Value::Ref(lv)] => {
            let lv = lv.clone();
            if !matches!(vm.rval(stream.clone())?, Value::Str(_)) {
                return err("Readchar: an input stream is named by a string");
            }
            lv
        }
        _ => return err("Readchar needs one string variable to read into"),
    };
    let s = read_compound(vm).unwrap_or_default();
    vm.update(&lv, Value::str(&s))?;
    Ok(Value::Undef)
}

fn quot_rem(vm: &mut Vm, a: Vec<Value>, name: &str, rem: bool) -> R<Value> {
    let a = args(vm, a, 2, name)?;
    let ints =
        matches!(a[0], Value::Index(_) | Value::Integer(_)) && matches!(a[1], Value::Index(_) | Value::Integer(_));
    let (x, y) = (real(&a[0], name)?, real(&a[1], name)?);
    if y == 0.0 {
        return err(format!("{}: division by zero", name));
    }
    let q = (x / y).trunc();
    let r = if rem { x - y * q } else { q };
    Ok(if ints { number(r) } else { Value::Real(r) })
}

pub static TABLE: &[(&str, BDef)] = &[
    // ---- arithmetic (Manual 4.4 and the 1966 supplement)
    ("Sqrt", BDef::F(|vm, a| real1(vm, a, "Sqrt", f64::sqrt))),
    ("Exp", BDef::F(|vm, a| real1(vm, a, "Exp", f64::exp))),
    ("Log", BDef::F(|vm, a| real1(vm, a, "Log", f64::ln))),
    ("Sin", BDef::F(|vm, a| real1(vm, a, "Sin", f64::sin))),
    ("Cos", BDef::F(|vm, a| real1(vm, a, "Cos", f64::cos))),
    ("Tan", BDef::F(|vm, a| real1(vm, a, "Tan", f64::tan))),
    ("Arctan", BDef::F(|vm, a| real1(vm, a, "Arctan", f64::atan))),
    ("Arcsin", BDef::F(|vm, a| real1(vm, a, "Arcsin", f64::asin))),
    ("Arccos", BDef::F(|vm, a| real1(vm, a, "Arccos", f64::acos))),
    ("Frpt", BDef::F(|vm, a| real1(vm, a, "Frpt", |x| x - x.floor()))),
    (
        "Intpt",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Intpt")?;
            Ok(number(real(&a[0], "Intpt")?.floor()))
        }),
    ),
    (
        "Round",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Round")?;
            Ok(number(real(&a[0], "Round")?.round()))
        }),
    ),
    (
        "Sign",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Sign")?;
            let x = real(&a[0], "Sign")?;
            Ok(Value::Index(if x > 0.0 {
                1
            } else if x < 0.0 {
                -1
            } else {
                0
            }))
        }),
    ),
    (
        "Mod",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Mod")?;
            match &a[0] {
                Value::Index(n) => Ok(int_value(n.abs())),
                Value::Integer(n) => Ok(Value::Integer(n.abs())),
                Value::Complex(x, y) | Value::DComplex(x, y) => Ok(Value::Real(x.hypot(*y))),
                v => Ok(Value::Real(real(v, "Mod")?.abs())),
            }
        }),
    ),
    ("Quot", BDef::F(|vm, a| quot_rem(vm, a, "Quot", false))),
    ("Rem", BDef::F(|vm, a| quot_rem(vm, a, "Rem", true))),
    (
        "Realpt",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Realpt")?;
            match &a[0] {
                Value::Complex(x, _) | Value::DComplex(x, _) => Ok(Value::Real(*x)),
                v => Ok(Value::Real(real(v, "Realpt")?)),
            }
        }),
    ),
    (
        "Imagpt",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Imagpt")?;
            match &a[0] {
                Value::Complex(_, y) | Value::DComplex(_, y) => Ok(Value::Real(*y)),
                v => real(v, "Imagpt").map(|_| Value::Real(0.0)),
            }
        }),
    ),
    // Not in the papers: stands in for the basic symbol i (3.4.3).
    (
        "Cpx",
        BDef::F(|vm, a| {
            let a = args(vm, a, 2, "Cpx")?;
            Ok(Value::Complex(real(&a[0], "Cpx")?, real(&a[1], "Cpx")?))
        }),
    ),
    // ---- transfer functions (3.3.1)
    ("Index", BDef::F(|vm, a| to_type(vm, a, "Index", Ty::Index))),
    ("Integer", BDef::F(|vm, a| to_type(vm, a, "Integer", Ty::Integer))),
    ("Real", BDef::F(|vm, a| to_type(vm, a, "Real", Ty::Real))),
    ("Double", BDef::F(|vm, a| to_type(vm, a, "Double", Ty::Double))),
    ("Complex", BDef::F(|vm, a| to_type(vm, a, "Complex", Ty::Complex))),
    (
        "DoubleComplex",
        BDef::F(|vm, a| to_type(vm, a, "DoubleComplex", Ty::DComplex)),
    ),
    ("Logical", BDef::F(|vm, a| to_type(vm, a, "Logical", Ty::Logical))),
    (
        "LongLogical",
        BDef::F(|vm, a| to_type(vm, a, "LongLogical", Ty::LongLogical)),
    ),
    ("String", BDef::F(|vm, a| to_type(vm, a, "String", Ty::String))),
    ("Boolean", BDef::F(|vm, a| to_type(vm, a, "Boolean", Ty::Boolean))),
    // ---- mode forcing functions (3.3.1 c)
    ("RValue", BDef::F(|vm, a| Ok(args(vm, a, 1, "RValue")?.pop().unwrap()))),
    (
        "LValue",
        BDef::F(|_, mut a| {
            if a.len() != 1 {
                return err("LValue takes one argument");
            }
            Ok(match a.pop().unwrap() {
                v @ Value::Ref(_) => v,
                v => {
                    let ty = v.ty();
                    Value::Ref(LV::Cell(new_cell(v, ty, true)))
                }
            })
        }),
    ),
    // Not in the papers: the dynamically current type of a data item, without
    // which nothing can be asked of a general variable.
    (
        "Type",
        BDef::F(|vm, a| Ok(Value::Type(args(vm, a, 1, "Type")?[0].ty()))),
    ),
    // ---- logical (Manual 25.3)
    ("Lshift", BDef::F(|vm, a| shift(vm, a, "Lshift", 1))),
    ("Rshift", BDef::F(|vm, a| shift(vm, a, "Rshift", -1))),
    (
        "Rotate",
        BDef::F(|vm, a| {
            let a = args(vm, a, 2, "Rotate")?;
            let (bits, width, long) = logical(&a[0], "Rotate")?;
            let n = int(&a[1], "Rotate")?.rem_euclid(width as i64) as u32;
            let mask = (1u64 << width) - 1;
            Ok(mk_logical(
                if n == 0 {
                    bits
                } else {
                    ((bits << n) | (bits >> (width - n))) & mask
                },
                long,
            ))
        }),
    ),
    (
        "Ones",
        BDef::F(|vm, a| {
            let a = args(vm, a, 2, "Ones")?;
            let (lo, hi) = bit_range(&a[0], &a[1], 24, "Ones")?;
            Ok(mk_logical(field_mask(lo, hi), false))
        }),
    ),
    (
        "Longones",
        BDef::F(|vm, a| {
            let a = args(vm, a, 2, "Longones")?;
            let (lo, hi) = bit_range(&a[0], &a[1], 48, "Longones")?;
            Ok(mk_logical(field_mask(lo, hi), true))
        }),
    ),
    (
        "Mask",
        BDef::F(|vm, a| {
            let a = args(vm, a, 3, "Mask")?;
            let (bits, width, long) = logical(&a[0], "Mask")?;
            let (lo, hi) = bit_range(&a[1], &a[2], width, "Mask")?;
            Ok(mk_logical(bits & field_mask(lo, hi), long))
        }),
    ),
    ("Field", BDef::F(|vm, a| bit_field(vm, a, "Field", 3))),
    ("Bit", BDef::F(|vm, a| bit_field(vm, a, "Bit", 2))),
    // ---- strings (Manual 26.2)
    (
        "Length",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Length")?;
            match &a[0] {
                Value::Str(s) => Ok(int_value(s.chars().count() as i64)),
                v => match list_members(v) {
                    Some(m) => Ok(int_value(m.len() as i64)),
                    None => err(format!(
                        "Length needs a string or a list, not a data item of type {}",
                        v.ty().name()
                    )),
                },
            }
        }),
    ),
    (
        "First",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "First")?;
            Ok(Value::str(&string(&a[0], "First")?.chars().take(1).collect::<String>()))
        }),
    ),
    (
        "Last",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Last")?;
            Ok(Value::str(
                &string(&a[0], "Last")?
                    .chars()
                    .last()
                    .map(String::from)
                    .unwrap_or_default(),
            ))
        }),
    ),
    (
        "Character",
        BDef::F(|vm, a| {
            let a = args(vm, a, 2, "Character")?;
            let i = int(&a[0], "Character")?;
            match string(&a[1], "Character")?.chars().nth((i - 1).max(0) as usize) {
                Some(c) if i >= 1 => Ok(Value::str(&c.to_string())),
                _ => err(format!("Character: the string has no character number {}", i)),
            }
        }),
    ),
    ("Initials", BDef::F(|vm, a| substring(vm, a, "Initials", false))),
    ("Finals", BDef::F(|vm, a| substring(vm, a, "Finals", true))),
    // ---- arrays (11.2); Array is the 1963 name, Vector and Matrix the
    // names accepted by the pre-pilot system of 1966.
    ("Newarray", BDef::F(b_newarray)),
    ("Array", BDef::F(b_newarray)),
    ("Formarray", BDef::F(b_formarray)),
    ("Vector", BDef::F(b_formarray)),
    ("Matrix", BDef::F(b_formarray)),
    (
        "Copy",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Copy")?;
            Ok(Value::Array(copy_array(&*array(&a[0], "Copy")?, false)))
        }),
    ),
    (
        "Fix",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Fix")?;
            Ok(Value::Array(copy_array(&*array(&a[0], "Fix")?, true)))
        }),
    ),
    (
        "Bounds",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Bounds")?;
            let arr = array(&a[0], "Bounds")?;
            let hi = arr.lo + arr.cells.len() as i64 - 1;
            Ok(Value::Multi(Rc::new(vec![int_value(arr.lo), int_value(hi)])))
        }),
    ),
    (
        "Equal",
        BDef::F(|vm, a| {
            let a = args(vm, a, 2, "Equal")?;
            Ok(Value::Bool(arrays_equal(
                &*array(&a[0], "Equal")?,
                &*array(&a[1], "Equal")?,
            )))
        }),
    ),
    // ---- lists (Main Features of CPL, 1963, section 23)
    ("NIL", BDef::C(|| Value::Nil)),
    ("Nil", BDef::C(|| Value::Nil)),
    (
        "Cons",
        BDef::F(|vm, a| {
            let mut a = args(vm, a, 2, "Cons")?;
            list(&a[1], "Cons")?;
            let tl = a.pop().unwrap();
            Ok(cons(a.pop().unwrap(), tl))
        }),
    ),
    ("List", BDef::F(|vm, a| Ok(make_list(vm.rvals(a)?)))),
    ("List1", BDef::F(|vm, a| Ok(make_list(args(vm, a, 1, "List1")?)))),
    (
        "Hd",
        BDef::F(|vm, a| match &args(vm, a, 1, "Hd")?[0] {
            Value::Cons(c) => Ok(Value::Ref(LV::Cell(c.hd.clone()))),
            Value::Nil => err("Hd is applied to the empty list"),
            v => list(v, "Hd").map(|_| Value::Undef),
        }),
    ),
    (
        "Tl",
        BDef::F(|vm, a| match &args(vm, a, 1, "Tl")?[0] {
            Value::Cons(c) => Ok(Value::Ref(LV::Cell(c.tl.clone()))),
            Value::Nil => err("Tl is applied to the empty list"),
            v => list(v, "Tl").map(|_| Value::Undef),
        }),
    ),
    (
        "Null",
        BDef::F(|vm, a| Ok(Value::Bool(matches!(args(vm, a, 1, "Null")?[0], Value::Nil)))),
    ),
    (
        "Atom",
        BDef::F(|vm, a| {
            Ok(Value::Bool(!matches!(
                args(vm, a, 1, "Atom")?[0],
                Value::Nil | Value::Cons(_)
            )))
        }),
    ),
    (
        "Members",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Members")?;
            list(&a[0], "Members")?;
            Ok(Value::Multi(Rc::new(list_members(&a[0]).unwrap_or_default())))
        }),
    ),
    (
        "Append",
        BDef::F(|vm, a| {
            let a = args(vm, a, 2, "Append")?;
            list(&a[0], "Append")?;
            list(&a[1], "Append")?;
            let mut l = a[1].clone();
            for v in list_members(&a[0]).unwrap_or_default().into_iter().rev() {
                l = cons(v, l);
            }
            Ok(l)
        }),
    ),
    // ---- input and output (pre-pilot system, 1966)
    ("Write", BDef::F(b_write)),
    (
        "Newline",
        BDef::F(|vm, a| {
            args(vm, a, 0, "Newline")?;
            b_write(vm, vec![Value::str("\n")])
        }),
    ),
    (
        "Space",
        BDef::F(|vm, a| {
            let a = args(vm, a, 1, "Space")?;
            let n = int(&a[0], "Space")?.max(0) as usize;
            b_write(vm, vec![Value::str(&" ".repeat(n))])
        }),
    ),
    ("Read", BDef::F(b_read)),
    ("Readchar", BDef::F(b_readchar)),
    ("Writechar", BDef::F(b_writechar)),
    ("Printf", BDef::F(b_printf)),
    ("Printd", BDef::F(b_printd)),
    // ---- as spelt in the supplement for the pre-pilot system
    ("WRITE", BDef::F(b_write)),
    ("READ", BDef::F(b_read)),
    ("READCHAR", BDef::F(b_readchar)),
    ("WRITECHAR", BDef::F(b_writechar)),
    ("PRINTF", BDef::F(b_printf)),
    ("PRINTD", BDef::F(b_printd)),
];
