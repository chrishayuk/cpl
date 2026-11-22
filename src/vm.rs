//! The abstract machine: activations, L-values, jumps.

use crate::ast::{BinOp, PMode};
use crate::builtins::{self, BDef};
use crate::code::{CapSrc, Kind, Op, Proto};
use crate::value::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::rc::Rc;

pub struct RtError {
    pub msg: String,
    pub line: u32,
    pub place: String,
}

/// Ways in which an activation ends other than normally (9.3.2).
pub enum Unwind {
    Error(RtError),
    /// A jump to a label of some enclosing activation.
    Goto(Rc<LabelVal>),
    Finish,
}

pub type R<T> = Result<T, Unwind>;

pub fn err<T>(msg: impl Into<String>) -> R<T> {
    Err(Unwind::Error(RtError {
        msg: msg.into(),
        line: 0,
        place: String::new(),
    }))
}

fn lift<T>(r: Result<T, String>) -> R<T> {
    r.or_else(err)
}

/// The most activations that may be live at once before the program is
/// taken to be in a runaway recursion. Activations are kept on the heap.
pub const MAX_DEPTH: usize = 100_000;

/// A few things re-enter the machine on the host's own stack: the load and
/// update parts of an LH function, a substituted expression, and a function
/// called by a basic function. This is how deeply they may nest. A
/// WebAssembly engine has a small stack which cannot be asked for.
#[cfg(not(target_family = "wasm"))]
pub const MAX_NESTING: usize = 10_000;
#[cfg(target_family = "wasm")]
pub const MAX_NESTING: usize = 120;

struct Frame {
    id: u64,
    clo: Rc<Closure>,
    locals: Vec<Option<LV>>,
    stack: Vec<Value>,
}

/// What becomes of the result when an activation ends.
enum After {
    /// Push it on the caller's stack, as an R-value unless in L-mode.
    Push {
        lmode: bool,
    },
    Discard,
}

struct Act {
    f: Frame,
    pc: usize,
    after: After,
}

/// Why the current activation stopped obeying its own code.
enum Event {
    Call(Rc<Closure>, Vec<Value>, After),
    Ret(Value),
}

enum Flow {
    Next,
    Jump(usize),
    Ret(Value),
}

pub struct Vm {
    pub out: Box<dyn Write>,
    pub inp: Box<dyn BufRead>,
    pub max_steps: Option<u64>,
    steps: u64,
    next_frame: u64,
    depth: usize,
    nesting: usize,
    /// A character read ahead while looking for overstrikes.
    pub held: Option<char>,
}

impl Vm {
    pub fn new(out: Box<dyn Write>, inp: Box<dyn BufRead>) -> Vm {
        Vm {
            out,
            inp,
            max_steps: None,
            steps: 0,
            next_frame: 0,
            depth: 0,
            nesting: 0,
            held: None,
        }
    }

    pub fn execute(&mut self, main: Rc<Proto>) -> Result<(), String> {
        let clo = Rc::new(Closure {
            proto: main,
            caps: Vec::new(),
        });
        let r = self.run(&clo, Vec::new());
        let _ = self.out.flush();
        match r {
            Ok(_) | Err(Unwind::Finish) => Ok(()),
            Err(Unwind::Goto(l)) => Err(format!(
                "jump to the label {}, whose activation is no longer live",
                l.name
            )),
            Err(Unwind::Error(e)) => Err(format!("line {}: {} (in {})", e.line, e.msg, e.place)),
        }
    }

    // ---------------------------------------------------------- L-values

    /// L-value to R-value.
    pub fn load(&mut self, lv: &LV) -> R<Value> {
        match lv {
            LV::Cell(c) => {
                let v = c.val.borrow().clone();
                match v {
                    Value::Undef => err("a variable is used before it has been given a value"),
                    v => Ok(v),
                }
            }
            LV::LoadUpdate(l, _) => {
                let v = self.run(l, Vec::new())?;
                self.rval(v)
            }
            LV::Subst(t) => {
                let v = self.run(t, Vec::new())?;
                self.rval(v)
            }
            LV::Field(inner, lo, hi) => {
                let (bits, long) = self.bits_of(inner)?;
                Ok(logical_value((bits & field_mask(*lo, *hi)) >> lo, long))
            }
        }
    }

    /// The bits of the logical variable that a bit or field belongs to.
    fn bits_of(&mut self, lv: &LV) -> R<(u64, bool)> {
        match self.load(lv)? {
            Value::Logical(b) => Ok((b, false)),
            Value::LongLogical(b) => Ok((b, true)),
            v => err(format!("a bit field of a data item of type {}", v.ty().name())),
        }
    }

    /// Updating (9.2.2): the R-value is first transformed to the type of
    /// the L-value.
    pub fn update(&mut self, lv: &LV, v: Value) -> R<()> {
        match lv {
            LV::Cell(c) => {
                if c.constant.get() {
                    return err("assignment to a constant");
                }
                let v = lift(transfer(v, &c.ty, false))?;
                *c.val.borrow_mut() = v;
                Ok(())
            }
            LV::LoadUpdate(_, u) => self.run(u, vec![v]).map(|_| ()),
            LV::Field(inner, lo, hi) => {
                let (old, long) = self.bits_of(inner)?;
                let new = match lift(transfer(v, &Ty::LongLogical, false))? {
                    Value::LongLogical(b) => b,
                    _ => unreachable!(),
                };
                let mask = field_mask(*lo, *hi);
                self.update(inner, logical_value((old & !mask) | ((new << lo) & mask), long))
            }
            LV::Subst(t) => match self.run(t, Vec::new())? {
                Value::Ref(lv) => self.update(&lv, v),
                _ => err("assignment to a substituted expression that has no L-value"),
            },
        }
    }

    pub fn rval(&mut self, v: Value) -> R<Value> {
        match v {
            Value::Ref(lv) => self.load(&lv),
            v => Ok(v),
        }
    }

    pub fn rvals(&mut self, vs: Vec<Value>) -> R<Vec<Value>> {
        vs.into_iter().map(|v| self.rval(v)).collect()
    }

    // ------------------------------------------------------------- calls

    pub fn call_value(&mut self, callee: &Value, args: Vec<Value>) -> R<Value> {
        match callee {
            Value::Func(c) => self.run(c, args),
            Value::Builtin(i) => match &builtins::TABLE[*i].1 {
                BDef::F(f) => f(self, args),
                BDef::C(f) => Ok(f()),
            },
            Value::Native(n) => (n.f)(self, args),
            Value::Array(a) => self.subscript(a.clone(), args),
            v => err(format!(
                "a data item of type {} cannot be applied to arguments",
                v.ty().name()
            )),
        }
    }

    /// An array is "a particular sort of function, of one index parameter":
    /// applying it gives the L-value of an element (5.3.3, 11.1.2).
    fn subscript(&mut self, mut cur: Rc<ArrayObj>, args: Vec<Value>) -> R<Value> {
        let n = args.len();
        if n == 0 {
            return err("an array is applied to an empty subscript list");
        }
        for (k, a) in args.into_iter().enumerate() {
            let v = self.rval(a)?;
            let Value::Index(i) = lift(transfer(v, &Ty::Index, false))? else {
                unreachable!()
            };
            let off = i - cur.lo;
            if off < 0 || off >= cur.cells.len() as i64 {
                return err(format!(
                    "subscript {} is outside the bounds ({}, {})",
                    i,
                    cur.lo,
                    cur.lo + cur.cells.len() as i64 - 1
                ));
            }
            let cell = cur.cells[off as usize].clone();
            if k == n - 1 {
                return Ok(Value::Ref(LV::Cell(cell)));
            }
            let inner = cell.val.borrow().clone();
            match inner {
                Value::Array(b) => cur = b,
                _ => return err("too many subscripts for this array"),
            }
        }
        unreachable!()
    }

    /// Activate a function, routine or block-expression body and obey it to
    /// its end. Calls made from CPL code do not come back through here: the
    /// activations of one `run` are a list, so recursion in a CPL program
    /// does not use the host's stack.
    pub fn run(&mut self, clo: &Rc<Closure>, args: Vec<Value>) -> R<Value> {
        if self.nesting >= MAX_NESTING {
            return err("load, update and substitution are nested too deeply");
        }
        self.nesting += 1;
        let mut acts: Vec<Act> = Vec::new();
        let r = self.run_acts(&mut acts, clo, args);
        self.depth -= acts.len();
        self.nesting -= 1;
        r
    }

    fn run_acts(&mut self, acts: &mut Vec<Act>, clo: &Rc<Closure>, args: Vec<Value>) -> R<Value> {
        let first = self.enter(clo, args, After::Discard)?;
        acts.push(first);
        self.depth += 1;
        loop {
            let proto = acts.last().unwrap().f.clo.proto.clone();
            let top = acts.last_mut().unwrap();
            let step = match self.obey(top, &proto) {
                Ok(Event::Call(c, args, after)) => {
                    if self.depth >= MAX_DEPTH {
                        err("too many nested activations (runaway recursion?)")
                    } else {
                        self.enter(&c, args, after).map(|act| {
                            acts.push(act);
                            self.depth += 1;
                        })
                    }
                }
                Ok(Event::Ret(v)) => {
                    let done = acts.pop().unwrap();
                    self.depth -= 1;
                    let Some(caller) = acts.last_mut() else { return Ok(v) };
                    match done.after {
                        After::Discard => Ok(()),
                        After::Push { lmode: true } => {
                            caller.f.stack.push(v);
                            Ok(())
                        }
                        After::Push { lmode: false } => self.rval(v).map(|v| caller.f.stack.push(v)),
                    }
                }
                Err(u) => Err(u),
            };
            match step {
                Ok(()) => {}
                // A jump ends every activation that is not live to its
                // destination, and carries on from the label in the one
                // that is.
                Err(Unwind::Goto(l)) => loop {
                    match acts.last_mut() {
                        Some(a) if a.f.id == l.frame => {
                            a.f.stack.clear();
                            a.pc = l.pc;
                            break;
                        }
                        Some(_) => {
                            acts.pop();
                            self.depth -= 1;
                        }
                        None => return Err(Unwind::Goto(l)),
                    }
                },
                Err(Unwind::Error(mut e)) => {
                    if e.line == 0 {
                        if let Some(a) = acts.last() {
                            let p = &a.f.clo.proto;
                            e.line = p.lines[a.pc.saturating_sub(1)];
                            e.place = p.name.clone();
                        }
                    }
                    return Err(Unwind::Error(e));
                }
                Err(u) => return Err(u),
            }
        }
    }

    /// A new activation, with its formal parameters bound.
    fn enter(&mut self, clo: &Rc<Closure>, args: Vec<Value>, after: After) -> R<Act> {
        let proto = &clo.proto;
        if args.len() != proto.params.len() {
            return err(format!(
                "{} is called with {} argument(s) but has {} formal parameter(s)",
                proto.name,
                args.len(),
                proto.params.len()
            ));
        }
        let id = self.next_frame;
        self.next_frame += 1;
        let mut f = Frame {
            id,
            clo: clo.clone(),
            locals: vec![None; proto.nslots],
            stack: Vec::new(),
        };
        for (i, (p, a)) in proto.params.iter().zip(args).enumerate() {
            // Called by value: a fresh L-value holding the (transformed)
            // R-value of the actual parameter. Called by reference: the
            // L-value of the actual parameter itself (10.3.3).
            let lv = match p.mode {
                PMode::Value => {
                    let v = self.rval(a)?;
                    let v = transfer(v, &p.ty, false)
                        .or_else(|m| err(format!("parameter {} of {}: {}", p.name, proto.name, m)))?;
                    LV::Cell(new_cell(v, p.ty.clone(), false))
                }
                PMode::Ref | PMode::Subst => match a {
                    Value::Func(t) if p.mode == PMode::Subst && t.proto.kind == Kind::Thunk => LV::Subst(t),
                    Value::Ref(lv) => lv,
                    v => {
                        let ty = v.ty();
                        LV::Cell(new_cell(v, ty, true))
                    }
                },
            };
            f.locals[i] = Some(lv);
        }
        Ok(Act { f, pc: 0, after })
    }

    /// Obey the code of one activation until it calls a function or ends.
    fn obey(&mut self, act: &mut Act, proto: &Proto) -> R<Event> {
        loop {
            let op = &proto.code[act.pc];
            act.pc += 1;
            self.steps += 1;
            if self.max_steps.is_some_and(|m| self.steps > m) {
                return err("the step limit has been exceeded");
            }
            let f = &mut act.f;
            match op {
                Op::Call { n, lmode } => {
                    let args = Self::pop_args(f, *n);
                    match f.stack.pop().unwrap() {
                        Value::Func(c) => return Ok(Event::Call(c, args, After::Push { lmode: *lmode })),
                        callee => {
                            let r = self.call_value(&callee, args)?;
                            let r = if *lmode { r } else { self.rval(r)? };
                            f.stack.push(r);
                        }
                    }
                }
                Op::CallCmd(n) => {
                    let args = Self::pop_args(f, *n);
                    match f.stack.pop().unwrap() {
                        Value::Func(c) => return Ok(Event::Call(c, args, After::Discard)),
                        callee => {
                            self.call_value(&callee, args)?;
                        }
                    }
                }
                op => match self.exec(f, op)? {
                    Flow::Next => {}
                    Flow::Jump(t) => act.pc = t,
                    Flow::Ret(v) => return Ok(Event::Ret(v)),
                },
            }
        }
    }

    fn local(&self, f: &Frame, slot: usize) -> R<LV> {
        match &f.locals[slot] {
            Some(lv) => Ok(lv.clone()),
            None => err(format!(
                "{} is used before its definition has been obeyed",
                f.clo.proto.slot_names[slot]
            )),
        }
    }

    fn named<T>(r: R<T>, name: &str) -> R<T> {
        r.map_err(|u| match u {
            Unwind::Error(mut e) if e.line == 0 && e.msg.starts_with("a variable is used") => {
                e.msg = format!("the variable {} is used before it has been given a value", name);
                Unwind::Error(e)
            }
            u => u,
        })
    }

    fn pop_args(f: &mut Frame, n: usize) -> Vec<Value> {
        let at = f.stack.len() - n;
        let raw = f.stack.split_off(at);
        if !raw.iter().any(|v| matches!(v, Value::Multi(_))) {
            return raw;
        }
        let mut out = Vec::with_capacity(raw.len());
        for v in raw {
            match v {
                Value::Multi(vs) => out.extend(vs.iter().cloned()),
                v => out.push(v),
            }
        }
        out
    }

    /// Form a function R-value: the code and its free variable list. A free
    /// function takes the L-values of its free variables; a fixed function
    /// takes constant copies of their R-values (10.3.4).
    fn closure(&mut self, f: &Frame, proto: &Rc<Proto>) -> R<Rc<Closure>> {
        let mut caps = Vec::with_capacity(proto.caps.len());
        for c in &proto.caps {
            let lv = match c.src {
                CapSrc::Local(s) => f.locals[s].clone(),
                CapSrc::Cap(i) => Some(f.clo.caps[i].clone()),
            };
            caps.push(if c.copy {
                let v = match &lv {
                    Some(LV::Cell(cell)) => cell.val.borrow().clone(),
                    Some(other) => self.load(other)?,
                    None => Value::Undef,
                };
                let v = match v {
                    Value::Array(a) => Value::Array(builtins::copy_array(&a, true)),
                    v => v,
                };
                let ty = v.ty();
                LV::Cell(new_cell(v, ty, true))
            } else {
                lv.unwrap_or_else(|| LV::Cell(new_cell(Value::Undef, Ty::General, true)))
            });
        }
        Ok(Rc::new(Closure {
            proto: proto.clone(),
            caps,
        }))
    }

    fn truth(v: Value) -> R<bool> {
        match v {
            Value::Bool(b) => Ok(b),
            v => err(format!(
                "a Boolean value is needed here, not one of type {}",
                v.ty().name()
            )),
        }
    }

    fn exec(&mut self, f: &mut Frame, op: &Op) -> R<Flow> {
        match op {
            Op::Const(v) => f.stack.push(v.clone()),
            Op::LoadLocal(s) => {
                let lv = self.local(f, *s)?;
                let v = Self::named(self.load(&lv), &f.clo.proto.slot_names[*s])?;
                f.stack.push(v);
            }
            Op::LvalLocal(s) => {
                let lv = self.local(f, *s)?;
                f.stack.push(Value::Ref(lv));
            }
            Op::LoadCap(i) => {
                let lv = f.clo.caps[*i].clone();
                let v = Self::named(self.load(&lv), &f.clo.proto.caps[*i].name)?;
                f.stack.push(v);
            }
            Op::LvalCap(i) => f.stack.push(Value::Ref(f.clo.caps[*i].clone())),
            Op::Builtin(i) => f.stack.push(Value::Builtin(*i)),
            Op::Deref => {
                let v = f.stack.pop().unwrap();
                let v = self.rval(v)?;
                f.stack.push(v);
            }
            Op::MkLval => {
                let v = match f.stack.pop().unwrap() {
                    v @ Value::Ref(_) => v,
                    v => {
                        let ty = v.ty();
                        Value::Ref(LV::Cell(new_cell(v, ty, true)))
                    }
                };
                f.stack.push(v);
            }
            Op::Dup => {
                let v = f.stack.last().unwrap().clone();
                f.stack.push(v);
            }
            Op::Pop => {
                f.stack.pop();
            }
            Op::DefVal { slot, ty, constant } => {
                let v = f.stack.pop().unwrap();
                let v = self.rval(v)?;
                let ty = ty.clone().unwrap_or_else(|| v.ty());
                let v = lift(transfer(v, &ty, false))?;
                f.locals[*slot] = Some(LV::Cell(new_cell(v, ty, *constant)));
            }
            Op::DefRef { slot } => {
                // If the definiens has no L-value a new, constant one is made.
                let lv = match f.stack.pop().unwrap() {
                    Value::Ref(lv) => lv,
                    v => {
                        let ty = v.ty();
                        LV::Cell(new_cell(v, ty, true))
                    }
                };
                f.locals[*slot] = Some(lv);
            }
            Op::DefType { slot, ty } => {
                f.locals[*slot] = Some(LV::Cell(new_cell(Value::Undef, ty.clone(), false)));
            }
            Op::DefSubst { slot } => {
                let Value::Func(t) = f.stack.pop().unwrap() else {
                    unreachable!()
                };
                f.locals[*slot] = Some(LV::Subst(t));
            }
            Op::NewCell { slot } => {
                f.locals[*slot] = Some(LV::Cell(new_cell(
                    Value::Undef,
                    Ty::Function(Box::new(Ty::General)),
                    false,
                )));
            }
            Op::InitCell { slot, constant } => {
                let v = f.stack.pop().unwrap();
                if let Some(LV::Cell(c)) = &f.locals[*slot] {
                    *c.val.borrow_mut() = v;
                    c.constant.set(*constant);
                }
            }
            Op::Unpack { have, want } => {
                let mut vals = Self::pop_args(f, *have);
                // An explicit list of names matched against a single list
                // value: the transfer function Members is invoked (1963, s.24).
                if vals.len() != *want && vals.len() == 1 {
                    if let Some(m) = list_members(&vals[0]) {
                        if m.len() == *want {
                            vals = m;
                        } else {
                            return err(format!(
                                "a list of {} member(s) is matched against {} name(s)",
                                m.len(),
                                want
                            ));
                        }
                    }
                }
                if vals.len() != *want {
                    return err(format!(
                        "{} value(s) are supplied where {} are needed",
                        vals.len(),
                        want
                    ));
                }
                f.stack.extend(vals);
            }
            Op::Assign(n) => {
                // Both sides are fully evaluated before any updating (9.2.2).
                let at = f.stack.len() - n;
                let vals = f.stack.split_off(at);
                let at = f.stack.len() - n;
                let targets = f.stack.split_off(at);
                for (t, v) in targets.into_iter().zip(vals) {
                    match t {
                        Value::Ref(lv) => self.update(&lv, v)?,
                        _ => return err("the left hand side of the assignment has no L-value"),
                    }
                }
            }
            Op::AssignAll(n) => {
                let v = f.stack.pop().unwrap();
                let at = f.stack.len() - n;
                let targets = f.stack.split_off(at);
                for t in targets {
                    match t {
                        Value::Ref(lv) => self.update(&lv, v.clone())?,
                        _ => return err("the left hand side of the assignment has no L-value"),
                    }
                }
            }
            Op::MkMulti(n) => {
                let vals = Self::pop_args(f, *n);
                f.stack.push(Value::Multi(Rc::new(vals)));
            }
            Op::Transfer(ty) => {
                if *ty != Ty::General && f.stack.last().is_some_and(|v| v.ty().is_numeric()) {
                    let v = f.stack.pop().unwrap();
                    f.stack.push(lift(transfer(v, ty, false))?);
                }
            }
            Op::Jump(t) => return Ok(Flow::Jump(*t)),
            Op::JumpIfFalse(t) => {
                if !Self::truth(f.stack.pop().unwrap())? {
                    return Ok(Flow::Jump(*t));
                }
            }
            Op::JumpIfTrue(t) => {
                if Self::truth(f.stack.pop().unwrap())? {
                    return Ok(Flow::Jump(*t));
                }
            }
            Op::AndJump(t) => {
                if let Some(Value::Bool(false)) = f.stack.last() {
                    return Ok(Flow::Jump(*t));
                }
            }
            Op::OrJump(t) => {
                if let Some(Value::Bool(true)) = f.stack.last() {
                    return Ok(Flow::Jump(*t));
                }
            }
            Op::Call { .. } | Op::CallCmd(_) => unreachable!("calls are made by obey"),
            Op::Closure(p) => {
                let c = self.closure(f, p)?;
                f.stack.push(Value::Func(c));
            }
            Op::MakeLU(l, u) => {
                let l = self.closure(f, l)?;
                let u = self.closure(f, u)?;
                f.stack.push(Value::Ref(LV::LoadUpdate(l, u)));
            }
            Op::Return => return Ok(Flow::Ret(Value::Undef)),
            Op::ReturnVal => return Ok(Flow::Ret(f.stack.pop().unwrap())),
            Op::Bin(op) => {
                let b = f.stack.pop().unwrap();
                let a = f.stack.pop().unwrap();
                f.stack.push(lift(binary(*op, &a, &b))?);
            }
            Op::Un(op) => {
                let a = f.stack.pop().unwrap();
                f.stack.push(lift(unary(*op, &a))?);
            }
            Op::Rel(ops) => {
                let at = f.stack.len() - (ops.len() + 1);
                let vals = f.stack.split_off(at);
                let mut all = true;
                for (i, op) in ops.iter().enumerate() {
                    all &= lift(relate(*op, &vals[i], &vals[i + 1]))?;
                }
                f.stack.push(Value::Bool(all));
            }
            Op::Goto => match f.stack.pop().unwrap() {
                Value::Label(l) => return Err(Unwind::Goto(l)),
                v => {
                    return err(format!(
                        "'go to' needs a label, not a data item of type {}",
                        v.ty().name()
                    ))
                }
            },
            Op::MakeLabel { slot, idx, name } => {
                let l = LabelVal {
                    pc: f.clo.proto.label_pcs[*idx],
                    frame: f.id,
                    name: name.clone(),
                };
                f.locals[*slot] = Some(LV::Cell(new_cell(Value::Label(Rc::new(l)), Ty::Label, true)));
            }
            Op::ForInit { slot, steps } => {
                let mut segs = VecDeque::new();
                for is_step in steps.iter().rev() {
                    if *is_step {
                        let limit = f.stack.pop().unwrap();
                        let step = f.stack.pop().unwrap();
                        let start = f.stack.pop().unwrap();
                        let span = lift(binary(BinOp::Sub, &limit, &start))?;
                        let q = lift(binary(BinOp::Div, &span, &step))?;
                        let Value::Real(q) = lift(transfer(q, &Ty::Real, false))? else {
                            unreachable!()
                        };
                        let n = q.round() as i64;
                        if n >= 0 {
                            segs.push_front(Seg::Step { start, step, r: 0, n });
                        }
                    } else {
                        segs.push_front(Seg::One(f.stack.pop().unwrap()));
                    }
                }
                let it = Value::Iter(Rc::new(RefCell::new(ForIter { segs })));
                f.locals[*slot] = Some(LV::Cell(new_cell(it, Ty::General, true)));
            }
            Op::ForNext { iter, var, exit } => {
                let Some(LV::Cell(c)) = &f.locals[*iter] else {
                    unreachable!()
                };
                let Value::Iter(it) = c.val.borrow().clone() else {
                    unreachable!()
                };
                let mut it = it.borrow_mut();
                let v = match it.segs.front_mut() {
                    None => return Ok(Flow::Jump(*exit)),
                    Some(Seg::One(_)) => {
                        let Some(Seg::One(v)) = it.segs.pop_front() else {
                            unreachable!()
                        };
                        v
                    }
                    Some(Seg::Step { start, step, r, n }) => {
                        let v =
                            lift(binary(BinOp::Mul, &int_value(*r), step).and_then(|d| binary(BinOp::Add, start, &d)))?;
                        *r += 1;
                        if *r > *n {
                            it.segs.pop_front();
                        }
                        v
                    }
                };
                let ty = v.ty();
                f.locals[*var] = Some(LV::Cell(new_cell(v, ty, false)));
            }
            Op::Fail(msg) => return err(msg.clone()),
            Op::Finish => return Err(Unwind::Finish),
        }
        Ok(Flow::Next)
    }
}
