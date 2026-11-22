//! Translation of the syntax tree into abstract machine code: scope rules
//! (Section 8), the modes of evaluation (4.2), definitions (7), commands (9)
//! and function and routine definitions (10.3).

use crate::ast::*;
use crate::builtins::{self, BDef};
use crate::code::*;
use crate::value::{number, rank_lub, rank_type, transfer, Ty, Value, M24};
use std::collections::{BTreeMap as HashMap, BTreeSet as HashSet};
use std::rc::Rc;

type CR<T> = Result<T, String>;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    R,
    L,
}

/// What is known of the type of an expression before the program runs.
/// `Bot` is the type of a call of the recursive function whose result type
/// is still being determined (10.3.6 c); `Any` is "not known until run time".
#[derive(Clone, PartialEq)]
enum St {
    T(Ty),
    Bot,
    Any,
}

fn st(ty: Ty) -> St {
    match ty {
        Ty::General => St::Any,
        t => St::T(t),
    }
}

/// The least upper bound of the types of alternative expressions (3.3.3 b).
/// `None` in second place: no transfer function is needed or possible.
fn st_lub(a: &St, b: &St) -> St {
    match (a, b) {
        (St::Bot, x) | (x, St::Bot) => x.clone(),
        (St::T(x), St::T(y)) if x == y => a.clone(),
        (St::T(x), St::T(y)) => match (x.rank(), y.rank()) {
            (Some(p), Some(q)) => St::T(rank_type(rank_lub(p, q, 0))),
            _ => St::Any,
        },
        _ => St::Any,
    }
}

fn st_arith(op: BinOp, a: &St, b: &St) -> St {
    let min = if matches!(op, BinOp::Div | BinOp::Pow) { 2 } else { 0 };
    let rank = |s: &St| match s {
        St::T(t) => t.rank(),
        _ => None,
    };
    match (a, b) {
        (St::Bot, St::Bot) => St::Bot,
        (St::Bot, x) | (x, St::Bot) => match rank(x) {
            Some(r) => St::T(rank_type(rank_lub(r, r, min))),
            None => St::Any,
        },
        _ => match (rank(a), rank(b)) {
            (Some(p), Some(q)) => St::T(rank_type(rank_lub(p, q, min))),
            _ => St::Any,
        },
    }
}

const REAL_FUNCTIONS: &[&str] = &[
    "Sqrt", "Exp", "Log", "Sin", "Cos", "Tan", "Arctan", "Arcsin", "Arccos", "Frpt", "Real", "Realpt", "Imagpt",
];

struct Var {
    slot: usize,
    ty: St,
    /// The result type, for a name introduced by a function definition.
    result: St,
    /// Parameter modes, known when the name was introduced by a function or
    /// routine definition; needed to pass substitution parameters.
    sig: Option<Rc<Vec<PMode>>>,
}

struct Scope {
    vars: HashMap<String, Var>,
    prefer: Ty,
}

struct Switch {
    cases: Vec<(Expr, usize)>,
    default: Option<usize>,
}

/// The function, routine or block-expression body being compiled.
struct Fs {
    name: String,
    kind: Kind,
    refof: bool,
    fixed: bool,
    /// Names of a recursive group: shared even by a fixed function.
    shared: HashSet<String>,
    params: Vec<ParamSpec>,
    code: Vec<Op>,
    lines: Vec<u32>,
    nslots: usize,
    slot_names: Vec<String>,
    scopes: Vec<Scope>,
    caps: Vec<Capture>,
    cap_idx: HashMap<String, usize>,
    loops: Vec<Vec<usize>>,
    label_pcs: Vec<usize>,
    label_idx: HashMap<String, usize>,
    switches: Vec<Switch>,
    /// Least upper bound of the types met in 'result is' commands, and the
    /// places where each result is to be transformed to it (4.4.1).
    result: St,
    result_fixups: Vec<usize>,
}

enum Loc {
    Local(usize),
    Cap(usize),
    Builtin(usize),
    Const(Value),
}

pub struct Compiler {
    fs: Vec<Fs>,
    line: u32,
}

pub fn compile(items: &[Item]) -> CR<Rc<Proto>> {
    let mut c = Compiler {
        fs: Vec::new(),
        line: 1,
    };
    c.push_fs("main program", Kind::Main, false, false, &[]);
    let mut labels = Vec::new();
    labels_in_items(items, &mut labels);
    c.label_prelude(labels)?;
    for it in items {
        c.item(it)?;
    }
    c.emit(Op::Finish);
    Ok(c.pop_fs())
}

fn labels_in_items(items: &[Item], out: &mut Vec<String>) {
    for it in items {
        if let Item::Cmd(c) = it {
            labels_in_cmd(c, out);
        }
    }
}

/// Command labels have as scope the smallest enclosing routine body or
/// block-expression body (9.3.3), so they are collected before it is compiled.
fn labels_in_cmd(c: &Cmd, out: &mut Vec<String>) {
    match &c.kind {
        CmdKind::Labelled(n, c) => {
            out.push(n.clone());
            labels_in_cmd(c, out);
        }
        CmdKind::Block(items) | CmdKind::Switch(_, items) => labels_in_items(items, out),
        CmdKind::If(_, _, c)
        | CmdKind::While(_, _, c)
        | CmdKind::Repeat(c, _)
        | CmdKind::For(_, _, c)
        | CmdKind::Case(_, c)
        | CmdKind::Default(c)
        | CmdKind::Where(c, _) => labels_in_cmd(c, out),
        CmdKind::Test(_, a, b) => {
            labels_in_cmd(a, out);
            labels_in_cmd(b, out);
        }
        _ => {}
    }
}

fn flatten<'a>(es: &'a [Expr], out: &mut Vec<&'a Expr>) {
    for e in es {
        match e {
            Expr::List(v) => flatten(v, out),
            e => out.push(e),
        }
    }
}

/// Whether evaluating the expression in L-mode can give an existing L-value.
fn has_lvalue(e: &Expr) -> bool {
    match e {
        Expr::Name(..) | Expr::Call(..) | Expr::Valof(..) => true,
        Expr::Cond(_, a, b) => has_lvalue(a) || has_lvalue(b),
        Expr::Where(inner, _) => has_lvalue(inner),
        _ => false,
    }
}

fn numeric_literal(e: &Expr) -> Option<f64> {
    match e {
        Expr::Num(t) => t.replace('@', "e").parse().ok(),
        Expr::Un(UnOp::Neg, inner) => numeric_literal(inner).map(|x| -x),
        _ => None,
    }
}

impl Compiler {
    fn cur(&mut self) -> &mut Fs {
        self.fs.last_mut().unwrap()
    }

    fn fail<T>(&self, msg: impl AsRef<str>) -> CR<T> {
        Err(format!("line {}: {}", self.line, msg.as_ref()))
    }

    fn emit(&mut self, op: Op) -> usize {
        let line = self.line;
        let f = self.cur();
        f.code.push(op);
        f.lines.push(line);
        f.code.len() - 1
    }

    fn here(&mut self) -> usize {
        self.cur().code.len()
    }

    fn patch(&mut self, at: usize, target: usize) {
        match &mut self.cur().code[at] {
            Op::Jump(t) | Op::JumpIfFalse(t) | Op::JumpIfTrue(t) => *t = target,
            Op::ForNext { exit, .. } => *exit = target,
            _ => unreachable!(),
        }
    }

    fn new_slot(&mut self, name: &str) -> usize {
        let f = self.cur();
        f.slot_names.push(name.to_string());
        f.nslots += 1;
        f.nslots - 1
    }

    fn declare(&mut self, scope: usize, name: &str, sig: Option<Rc<Vec<PMode>>>) -> usize {
        self.declare_typed(scope, name, sig, St::Any, St::Any)
    }

    fn declare_typed(&mut self, scope: usize, name: &str, sig: Option<Rc<Vec<PMode>>>, ty: St, result: St) -> usize {
        let slot = self.new_slot(name);
        self.cur().scopes[scope]
            .vars
            .insert(name.to_string(), Var { slot, sig, ty, result });
        slot
    }

    fn var(&self, name: &str) -> Option<&Var> {
        for f in self.fs.iter().rev() {
            for sc in f.scopes.iter().rev() {
                if let Some(v) = sc.vars.get(name) {
                    return Some(v);
                }
            }
        }
        None
    }

    fn top(&mut self) -> usize {
        self.cur().scopes.len() - 1
    }

    fn prefer(&self) -> Ty {
        self.fs.last().unwrap().scopes.last().unwrap().prefer.clone()
    }

    fn push_scope(&mut self) {
        let prefer = self.prefer();
        self.cur().scopes.push(Scope {
            vars: HashMap::new(),
            prefer,
        });
    }

    fn pop_scope(&mut self) {
        self.cur().scopes.pop();
    }

    fn push_fs(&mut self, name: &str, kind: Kind, refof: bool, fixed: bool, shared: &[String]) {
        let prefer = if self.fs.is_empty() { Ty::Real } else { self.prefer() };
        self.fs.push(Fs {
            name: name.to_string(),
            kind,
            refof,
            fixed,
            shared: shared.iter().cloned().collect(),
            params: Vec::new(),
            code: Vec::new(),
            lines: Vec::new(),
            nslots: 0,
            slot_names: Vec::new(),
            scopes: vec![Scope {
                vars: HashMap::new(),
                prefer,
            }],
            caps: Vec::new(),
            cap_idx: HashMap::new(),
            loops: Vec::new(),
            label_pcs: Vec::new(),
            label_idx: HashMap::new(),
            switches: Vec::new(),
            result: St::Bot,
            result_fixups: Vec::new(),
        });
    }

    fn pop_fs(&mut self) -> Rc<Proto> {
        let f = self.fs.pop().unwrap();
        Rc::new(Proto {
            name: f.name,
            kind: f.kind,
            params: f.params,
            nslots: f.nslots,
            slot_names: f.slot_names,
            code: f.code,
            lines: f.lines,
            caps: f.caps,
            label_pcs: f.label_pcs,
            fixed: f.fixed,
        })
    }

    fn resolve_ty(&self, t: &TyExpr) -> Ty {
        let inner = |o: &Option<Box<TyExpr>>| match o {
            Some(t) => self.resolve_ty(t),
            None => self.prefer(),
        };
        match t {
            TyExpr::Named(t) => t.clone(),
            TyExpr::Function(o) => Ty::Function(Box::new(inner(o))),
            TyExpr::Array(o, n) => Ty::Array(Box::new(inner(o)), *n),
        }
    }

    // ------------------------------------------------------------- names

    /// Find a name in the function at `level`, adding it to the free
    /// variable lists of the functions between its definition and its use.
    fn resolve_at(&mut self, level: usize, name: &str) -> Option<Loc> {
        for sc in self.fs[level].scopes.iter().rev() {
            if let Some(v) = sc.vars.get(name) {
                return Some(Loc::Local(v.slot));
            }
        }
        if let Some(&i) = self.fs[level].cap_idx.get(name) {
            return Some(Loc::Cap(i));
        }
        if level == 0 {
            return None;
        }
        let src = match self.resolve_at(level - 1, name)? {
            Loc::Local(s) => CapSrc::Local(s),
            Loc::Cap(i) => CapSrc::Cap(i),
            _ => unreachable!(),
        };
        let f = &mut self.fs[level];
        let copy = f.fixed && !f.shared.contains(name);
        f.caps.push(Capture {
            name: name.to_string(),
            src,
            copy,
        });
        f.cap_idx.insert(name.to_string(), f.caps.len() - 1);
        Some(Loc::Cap(f.caps.len() - 1))
    }

    fn resolve(&mut self, name: &str) -> CR<Loc> {
        let top = self.fs.len() - 1;
        if let Some(l) = self.resolve_at(top, name) {
            return Ok(l);
        }
        match builtins::lookup(name) {
            Some(i) => Ok(match &builtins::TABLE[i].1 {
                BDef::C(f) => Loc::Const(f()),
                BDef::F(_) => Loc::Builtin(i),
            }),
            None if name == "<rhs>" => self.fail("rhs may only be used in the update part of an LH function"),
            None => self.fail(format!("the name {} is not defined here", name)),
        }
    }

    fn sig_of(&self, name: &str) -> Option<Rc<Vec<PMode>>> {
        self.var(name).and_then(|v| v.sig.clone())
    }

    fn name(&mut self, name: &str, mode: Mode) -> CR<()> {
        let op = match (self.resolve(name)?, mode) {
            (Loc::Local(s), Mode::R) => Op::LoadLocal(s),
            (Loc::Local(s), Mode::L) => Op::LvalLocal(s),
            (Loc::Cap(i), Mode::R) => Op::LoadCap(i),
            (Loc::Cap(i), Mode::L) => Op::LvalCap(i),
            (Loc::Builtin(i), _) => Op::Builtin(i),
            (Loc::Const(v), _) => Op::Const(v),
        };
        self.emit(op);
        Ok(())
    }

    /// "Within the bodies of fixed functions and routines, no assignments
    /// may be made to free variables" (10.3.4).
    fn check_assignable(&self, target: &Expr) -> CR<()> {
        let Expr::Name(n, _) = target else { return Ok(()) };
        for f in self.fs.iter().rev() {
            if f.scopes.iter().any(|sc| sc.vars.contains_key(n)) {
                break;
            }
            if let Some(&i) = f.cap_idx.get(n) {
                if f.caps[i].copy {
                    return self.fail(format!(
                        "{} is a free variable of the fixed function {}, which may not assign to it \
                         (a function defined with == is free and may)",
                        n, f.name
                    ));
                }
            }
        }
        Ok(())
    }

    fn label_prelude(&mut self, names: Vec<String>) -> CR<()> {
        for n in names {
            if self.cur().label_idx.contains_key(&n) {
                return self.fail(format!("the label {} is set twice in the same scope", n));
            }
            let top = self.top();
            let slot = self.declare_typed(top, &n, None, St::T(Ty::Label), St::Any);
            let f = self.cur();
            let idx = f.label_pcs.len();
            f.label_pcs.push(usize::MAX);
            f.label_idx.insert(n.clone(), idx);
            self.emit(Op::MakeLabel { slot, idx, name: n });
        }
        Ok(())
    }

    // ------------------------------------------------------- definitions

    fn item(&mut self, it: &Item) -> CR<()> {
        match it {
            Item::Let(d) => {
                let top = self.top();
                self.def(d, top)
            }
            Item::Prefer(t) => {
                let ty = self.resolve_ty(t);
                self.cur().scopes.last_mut().unwrap().prefer = ty;
                Ok(())
            }
            Item::Cmd(c) => self.cmd(c),
        }
    }

    /// Compile a definition; the names it defines go into scope `target`.
    fn def(&mut self, d: &Def, target: usize) -> CR<()> {
        match d {
            // D1 in D2: the scope of D1 is D2 alone (8.3.4).
            Def::In(d1, d2) => {
                self.push_scope();
                let inner = self.top();
                self.def(d1, inner)?;
                self.def(d2, target)?;
                self.pop_scope();
                Ok(())
            }
            Def::Rec(inner) => {
                let mut parts = Vec::new();
                Self::and_parts(inner, &mut parts);
                self.group(&parts, true, target)
            }
            Def::And(_) => {
                let mut parts = Vec::new();
                Self::and_parts(d, &mut parts);
                self.group(&parts, false, target)
            }
            d => self.group(&[d], false, target),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn and_parts<'a>(d: &'a Def, out: &mut Vec<&'a Def>) {
        match d {
            Def::And(v) => v.iter().for_each(|p| Self::and_parts(p, out)),
            d => out.push(d),
        }
    }

    /// Definitions joined by `and` are activated in parallel: no definiens
    /// lies in the scope of another definition of the group, unless the
    /// group is recursive (8.3.1, 8.3.3).
    fn group(&mut self, parts: &[&Def], rec: bool, target: usize) -> CR<()> {
        if parts
            .iter()
            .any(|p| matches!(p, Def::In(..) | Def::Rec(_) | Def::And(_)))
        {
            for p in parts {
                self.def(p, target)?;
            }
            return Ok(());
        }
        if rec {
            let names: Vec<String> = parts
                .iter()
                .filter_map(|p| match p {
                    Def::Func { name, .. } => Some(name.clone()),
                    _ => None,
                })
                .collect();
            let mut slots = Vec::new();
            let alone = names.len() == 1;
            for p in parts {
                if let Def::Func { name, params, line, .. } = p {
                    self.line = *line;
                    let sig = Rc::new(params.iter().map(|p| p.mode).collect());
                    let result = if alone { St::Bot } else { St::Any };
                    let slot = self.declare_typed(target, name, Some(sig), St::Any, result);
                    self.emit(Op::NewCell { slot });
                    slots.push(slot);
                }
            }
            let mut slots = slots.into_iter();
            for p in parts {
                match p {
                    Def::Func {
                        name, constant, line, ..
                    } => {
                        // The result type of a recursive function is found by
                        // assuming a type and repeating until it is consistent.
                        let mut assumed = if alone { St::Bot } else { St::Any };
                        let mut tries = 0;
                        let proto = loop {
                            self.cur().scopes[target].vars.get_mut(name).unwrap().result = assumed.clone();
                            self.line = *line;
                            let (proto, found) = self.func(p, &names)?;
                            tries += 1;
                            if found == assumed || assumed == St::Any {
                                break proto;
                            }
                            assumed = if tries >= 3 { St::Any } else { found };
                        };
                        let settled = if assumed == St::Bot { St::Any } else { assumed };
                        self.cur().scopes[target].vars.get_mut(name).unwrap().result = settled;
                        self.line = *line;
                        self.emit(Op::Closure(proto));
                        self.emit(Op::InitCell {
                            slot: slots.next().unwrap(),
                            constant: *constant,
                        });
                    }
                    p => self.group(&[p], false, target)?,
                }
            }
            return Ok(());
        }
        // Phase 1: every definiens, in the scope outside the definition.
        let mut types: Vec<Vec<St>> = Vec::new();
        for p in parts {
            let mut tys = Vec::new();
            match p {
                Def::Simple {
                    names,
                    op,
                    all,
                    exprs,
                    line,
                    ..
                } => {
                    self.line = *line;
                    if *all {
                        if exprs.len() != 1 {
                            return self.fail("'all' needs a single expression on the right");
                        }
                        let t = self.definiens(&exprs[0], *op)?;
                        for _ in 1..names.len() {
                            self.emit(Op::Dup);
                        }
                        tys = vec![t; names.len()];
                    } else if *op == DefOp::Value {
                        let mut have = 0;
                        for e in exprs {
                            let (n, t) = self.multi(e, Mode::R, true)?;
                            have += n;
                            tys.push(t);
                        }
                        if have != names.len() || tys.len() != names.len() {
                            tys.clear();
                        }
                        self.emit(Op::Unpack {
                            have,
                            want: names.len(),
                        });
                    } else {
                        let mut flat = Vec::new();
                        flatten(exprs, &mut flat);
                        if flat.len() != names.len() {
                            return self.fail(format!(
                                "{} names are defined by {} expressions",
                                names.len(),
                                flat.len()
                            ));
                        }
                        for e in flat {
                            tys.push(self.definiens(e, *op)?);
                        }
                    }
                }
                Def::Func { line, .. } => {
                    self.line = *line;
                    let (proto, result) = self.func(p, &[])?;
                    self.emit(Op::Closure(proto));
                    tys.push(result);
                }
                _ => {}
            }
            types.push(tys);
        }
        // Phase 2: associate the names, last value first.
        for (p, tys) in parts.iter().zip(types).rev() {
            match p {
                Def::Simple {
                    names,
                    op,
                    constant,
                    line,
                    ..
                } => {
                    self.line = *line;
                    for (i, n) in names.iter().enumerate().rev() {
                        let ty = tys.get(i).cloned().unwrap_or(St::Any);
                        let slot = self.declare_typed(target, n, None, ty, St::Any);
                        self.emit(match op {
                            DefOp::Value => Op::DefVal {
                                slot,
                                ty: None,
                                constant: *constant,
                            },
                            DefOp::Ref => Op::DefRef { slot },
                            DefOp::Subst => Op::DefSubst { slot },
                        });
                    }
                }
                Def::Type { names, types, line } => {
                    self.line = *line;
                    for (n, t) in names.iter().zip(types) {
                        let ty = self.resolve_ty(t);
                        let slot = self.declare_typed(target, n, None, st(ty.clone()), St::Any);
                        self.emit(Op::DefType { slot, ty });
                    }
                }
                Def::Func {
                    name,
                    params,
                    body,
                    constant,
                    line,
                    ..
                } => {
                    self.line = *line;
                    let sig = Rc::new(params.iter().map(|p| p.mode).collect());
                    let result = tys.first().cloned().unwrap_or(St::Any);
                    let slot = self.declare_typed(target, name, Some(sig), St::Any, result);
                    let ty = match body {
                        FuncBody::Routine(_) => Ty::Routine,
                        _ => Ty::Function(Box::new(Ty::General)),
                    };
                    self.emit(Op::DefVal {
                        slot,
                        ty: Some(ty),
                        constant: *constant,
                    });
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    /// One defining expression of a simple definition.
    fn definiens(&mut self, e: &Expr, op: DefOp) -> CR<St> {
        match op {
            DefOp::Value => {
                let (n, t) = self.multi(e, Mode::R, true)?;
                if n != 1 {
                    self.emit(Op::Unpack { have: n, want: 1 });
                }
                Ok(t)
            }
            DefOp::Ref => self.expr(e, Mode::L),
            DefOp::Subst => self.thunk(e).map(|_| St::Any),
        }
    }

    /// An expression left unevaluated: a parameterless function yielding the
    /// L-value (or, failing that, the R-value) of the expression when called.
    fn thunk(&mut self, e: &Expr) -> CR<()> {
        self.push_fs("substituted expression", Kind::Thunk, false, false, &[]);
        self.expr(e, Mode::L)?;
        self.emit(Op::ReturnVal);
        let p = self.pop_fs();
        self.emit(Op::Closure(p));
        Ok(())
    }

    /// Compile a function or routine definition; also gives its result type.
    fn func(&mut self, d: &Def, rec_names: &[String]) -> CR<(Rc<Proto>, St)> {
        let Def::Func {
            name,
            params,
            body,
            fixed,
            ..
        } = d
        else {
            unreachable!()
        };
        // `=` defines a fixed function and `==` a free one. A routine or LH
        // function is free unless it is said to be fixed (10.3.2).
        let (kind, fixed) = match body {
            FuncBody::Expr(_, free) => (Kind::Function, fixed.unwrap_or(!*free)),
            FuncBody::Routine(_) => (Kind::Routine, fixed.unwrap_or(false)),
            FuncBody::LH { .. } => (Kind::Function, fixed.unwrap_or(false)),
        };
        // The preferred type is taken where a parameter has no specifier.
        let specs: Vec<ParamSpec> = params
            .iter()
            .map(|p| ParamSpec {
                name: p.name.clone(),
                ty: p
                    .ty
                    .as_ref()
                    .map(|t| self.resolve_ty(t))
                    .unwrap_or_else(|| self.prefer()),
                mode: p.mode,
            })
            .collect();
        self.push_fs(name, kind, false, fixed, rec_names);
        for p in &specs {
            let ty = if p.mode == PMode::Subst {
                St::Any
            } else {
                st(p.ty.clone())
            };
            self.declare_typed(0, &p.name, None, ty, St::Any);
        }
        self.cur().params = specs;
        let mut result = St::Any;
        match body {
            FuncBody::Expr(e, _) => {
                let (n, t) = self.multi(e, Mode::R, true)?;
                if n != 1 {
                    self.emit(Op::MkMulti(n));
                }
                result = t;
                self.emit(Op::ReturnVal);
            }
            FuncBody::Routine(c) => {
                let mut labels = Vec::new();
                labels_in_cmd(c, &mut labels);
                self.label_prelude(labels)?;
                self.cmd(c)?;
                self.emit(Op::Return);
            }
            FuncBody::LH { fix, load, update } => {
                if let Some(d) = fix {
                    self.def(d, 0)?;
                }
                let (lp, t) = self.valof_proto(&format!("load part of {}", name), load, false, Kind::Valof)?;
                let (up, _) = self.valof_proto(&format!("update part of {}", name), update, false, Kind::Update)?;
                result = t;
                self.emit(Op::MakeLU(lp, up));
                self.emit(Op::ReturnVal);
            }
        }
        Ok((self.pop_fs(), result))
    }

    fn valof_proto(&mut self, name: &str, items: &[Item], refof: bool, kind: Kind) -> CR<(Rc<Proto>, St)> {
        self.push_fs(name, kind, refof, false, &[]);
        if kind == Kind::Update {
            self.declare(0, "<rhs>", None);
            self.cur().params = vec![ParamSpec {
                name: "rhs".into(),
                ty: Ty::General,
                mode: PMode::Value,
            }];
        }
        let mut labels = Vec::new();
        labels_in_items(items, &mut labels);
        self.label_prelude(labels)?;
        for it in items {
            self.item(it)?;
        }
        if kind == Kind::Update {
            self.emit(Op::Return);
        } else {
            self.emit(Op::Fail(
                "the block of a result expression came to an end without obeying 'result is'".into(),
            ));
        }
        // Each result is transformed to the least upper bound of them all.
        let result = self.cur().result.clone();
        if let St::T(ty) = &result {
            if ty.is_numeric() {
                for at in std::mem::take(&mut self.cur().result_fixups) {
                    self.cur().code[at] = Op::Transfer(ty.clone());
                }
            }
        }
        let result = if result == St::Bot { St::Any } else { result };
        Ok((self.pop_fs(), result))
    }

    // ---------------------------------------------------------- commands

    fn cond_jump(&mut self, e: &Expr, jump_if: bool) -> CR<usize> {
        self.expr(e, Mode::R)?;
        Ok(self.emit(if jump_if { Op::JumpIfTrue(0) } else { Op::JumpIfFalse(0) }))
    }

    fn end_loop(&mut self) {
        let end = self.here();
        for b in self.cur().loops.pop().unwrap() {
            self.patch(b, end);
        }
    }

    fn cmd(&mut self, c: &Cmd) -> CR<()> {
        self.line = c.line;
        match &c.kind {
            CmdKind::Empty => {}
            CmdKind::Block(items) => {
                self.push_scope();
                for it in items {
                    self.item(it)?;
                }
                self.pop_scope();
            }
            CmdKind::Assign(lhs, rhs) => {
                let mut targets = Vec::new();
                flatten(lhs, &mut targets);
                for t in &targets {
                    self.expr(t, Mode::L)?;
                    self.check_assignable(t)?;
                }
                let mut have = 0;
                for e in rhs {
                    have += self.multi(e, Mode::R, false)?.0;
                }
                self.emit(Op::Unpack {
                    have,
                    want: targets.len(),
                });
                self.emit(Op::Assign(targets.len()));
            }
            CmdKind::AssignAll(lhs, e) => {
                let mut targets = Vec::new();
                flatten(lhs, &mut targets);
                for t in &targets {
                    self.expr(t, Mode::L)?;
                }
                self.expr(e, Mode::R)?;
                self.emit(Op::AssignAll(targets.len()));
            }
            CmdKind::Call(e) => match e {
                Expr::Call(f, args, line) => {
                    self.line = *line;
                    self.expr(f, Mode::R)?;
                    let n = self.args(f, args)?;
                    self.emit(Op::CallCmd(n));
                }
                // The command N has the same effect as the command N[] (9.4.2).
                e => {
                    self.expr(e, Mode::R)?;
                    self.emit(Op::CallCmd(0));
                }
            },
            CmdKind::Goto(e) => {
                self.expr(e, Mode::R)?;
                self.emit(Op::Goto);
            }
            CmdKind::ResultIs(es) => {
                if !matches!(self.cur().kind, Kind::Valof) {
                    return self.fail("'result is' may only appear in the block of a result expression or load part");
                }
                if self.cur().refof {
                    if es.len() != 1 {
                        return self.fail("a 'reference of' block yields a single L-value");
                    }
                    self.expr(&es[0], Mode::L)?;
                    self.emit(Op::MkLval);
                } else {
                    let (mut n, mut ty) = (0, St::Any);
                    for e in es {
                        let (k, t) = self.multi(e, Mode::R, false)?;
                        n += k;
                        ty = t;
                    }
                    if n != 1 {
                        self.emit(Op::MkMulti(n));
                        ty = St::Any;
                    } else if es.len() == 1 {
                        let at = self.emit(Op::Transfer(Ty::General));
                        self.cur().result_fixups.push(at);
                    }
                    let lub = st_lub(&self.cur().result, &ty);
                    self.cur().result = lub;
                }
                self.emit(Op::ReturnVal);
            }
            CmdKind::Break => {
                if self.cur().loops.is_empty() {
                    return self.fail("'break' is not inside a cycle command of this routine or result expression");
                }
                let j = self.emit(Op::Jump(0));
                self.cur().loops.last_mut().unwrap().push(j);
            }
            CmdKind::Return => match self.cur().kind {
                Kind::Routine | Kind::Update => {
                    self.emit(Op::Return);
                }
                Kind::Main => {
                    self.emit(Op::Finish);
                }
                _ => return self.fail("'return' may only appear in the body of a routine"),
            },
            CmdKind::Finish => {
                self.emit(Op::Finish);
            }
            CmdKind::If(e, unless, body) => {
                let j = self.cond_jump(e, *unless)?;
                self.cmd(body)?;
                let end = self.here();
                self.patch(j, end);
            }
            CmdKind::Test(e, a, b) => {
                let j = self.cond_jump(e, false)?;
                self.cmd(a)?;
                let over = self.emit(Op::Jump(0));
                let other = self.here();
                self.patch(j, other);
                self.cmd(b)?;
                let end = self.here();
                self.patch(over, end);
            }
            CmdKind::While(e, until, body) => {
                let top = self.here();
                let j = self.cond_jump(e, *until)?;
                self.cur().loops.push(vec![j]);
                self.cmd(body)?;
                self.emit(Op::Jump(top));
                self.end_loop();
            }
            CmdKind::Repeat(body, cond) => {
                let top = self.here();
                self.cur().loops.push(Vec::new());
                self.cmd(body)?;
                self.line = c.line;
                match cond {
                    None => {
                        self.emit(Op::Jump(top));
                    }
                    Some((e, until)) => {
                        let j = self.cond_jump(e, !*until)?;
                        self.patch(j, top);
                    }
                }
                self.end_loop();
            }
            CmdKind::For(name, elems, body) => {
                // The for-list is evaluated once and for all before the
                // cycle; the control variable is defined afresh for each
                // cycle (9.7.4).
                let iter = self.new_slot("<for-list>");
                let one = Expr::Num("1".into());
                let mut steps = Vec::new();
                for el in elems {
                    match el {
                        ForElem::Single(e) => {
                            self.expr(e, Mode::R)?;
                            steps.push(false);
                        }
                        ForElem::Step(a, b, c) => {
                            for e in [a, b, c] {
                                self.expr(e, Mode::R)?;
                            }
                            steps.push(true);
                        }
                        ForElem::To(a, b) => {
                            for e in [a, &one, b] {
                                self.expr(e, Mode::R)?;
                            }
                            steps.push(true);
                        }
                        ForElem::Dots(a, b, c) => {
                            self.expr(a, Mode::R)?;
                            self.expr(b, Mode::R)?;
                            self.expr(a, Mode::R)?;
                            self.emit(Op::Bin(BinOp::Sub));
                            self.expr(c, Mode::R)?;
                            steps.push(true);
                        }
                    }
                }
                self.emit(Op::ForInit { slot: iter, steps });
                self.push_scope();
                let top_scope = self.top();
                let var = self.declare(top_scope, name, None);
                let top = self.here();
                let j = self.emit(Op::ForNext { iter, var, exit: 0 });
                self.cur().loops.push(vec![j]);
                self.cmd(body)?;
                self.emit(Op::Jump(top));
                self.end_loop();
                self.pop_scope();
            }
            CmdKind::Labelled(name, body) => {
                let at = self.here();
                let f = self.cur();
                let idx = f.label_idx[name];
                f.label_pcs[idx] = at;
                self.cmd(body)?;
            }
            CmdKind::Switch(e, items) => {
                let tmp = self.new_slot("<switch>");
                self.expr(e, Mode::R)?;
                self.emit(Op::DefVal {
                    slot: tmp,
                    ty: None,
                    constant: true,
                });
                let dispatch = self.emit(Op::Jump(0));
                self.cur().switches.push(Switch {
                    cases: Vec::new(),
                    default: None,
                });
                self.push_scope();
                for it in items {
                    self.item(it)?;
                }
                self.pop_scope();
                let sw = self.cur().switches.pop().unwrap();
                let leave = self.emit(Op::Jump(0));
                let table = self.here();
                self.patch(dispatch, table);
                for (k, pc) in &sw.cases {
                    self.emit(Op::LoadLocal(tmp));
                    self.expr(k, Mode::R)?;
                    self.emit(Op::Rel(vec![RelOp::Eq]));
                    self.emit(Op::JumpIfTrue(*pc));
                }
                if let Some(pc) = sw.default {
                    self.emit(Op::Jump(pc));
                }
                let end = self.here();
                self.patch(leave, end);
            }
            CmdKind::Case(k, body) => {
                let at = self.here();
                match self.cur().switches.last_mut() {
                    Some(sw) => sw.cases.push((k.clone(), at)),
                    None => return self.fail("'case' does not belong to any switch-command"),
                }
                self.cmd(body)?;
            }
            CmdKind::Default(body) => {
                let at = self.here();
                match self.cur().switches.last_mut() {
                    Some(sw) => sw.default = Some(at),
                    None => return self.fail("'default' does not belong to any switch-command"),
                }
                self.cmd(body)?;
            }
            CmdKind::Where(body, d) => {
                self.push_scope();
                let top = self.top();
                self.def(d, top)?;
                self.cmd(body)?;
                self.pop_scope();
            }
        }
        Ok(())
    }

    // ------------------------------------------------------- expressions

    /// Compile an expression that may stand for several values; returns how
    /// many items it leaves on the stack. `defining` marks the right hand
    /// side of a definition, where a lone numerical constant takes the
    /// preferred type (3.4.3, 7.4.3).
    fn multi(&mut self, e: &Expr, mode: Mode, defining: bool) -> CR<(usize, St)> {
        match e {
            Expr::List(v) => {
                let (mut n, mut ty) = (0, St::Any);
                for e in v {
                    let (k, t) = self.multi(e, mode, defining)?;
                    n += k;
                    ty = t;
                }
                Ok((n, if v.len() == 1 && n == 1 { ty } else { St::Any }))
            }
            Expr::Where(inner, d) => {
                self.push_scope();
                let top = self.top();
                self.def(d, top)?;
                let r = self.multi(inner, mode, defining)?;
                self.pop_scope();
                Ok(r)
            }
            e => {
                if let (true, Some(x)) = (defining, numeric_literal(e)) {
                    let v = number(x);
                    let prefer = self.prefer();
                    let v = match (prefer.rank(), v.ty().rank()) {
                        (Some(p), Some(m)) if p > m => {
                            transfer(v, &prefer, false).map_err(|m| format!("line {}: {}", self.line, m))?
                        }
                        _ => v,
                    };
                    let ty = St::T(v.ty());
                    self.emit(Op::Const(v));
                    Ok((1, ty))
                } else {
                    Ok((1, self.expr(e, mode)?))
                }
            }
        }
    }

    /// The actual parameters of a call. They are evaluated in L-mode; the
    /// function called decides, parameter by parameter, whether it wants the
    /// L-value or the R-value (5.3.2, 10.3.3). An actual parameter called by
    /// substitution is handed over unevaluated.
    fn args(&mut self, callee: &Expr, args: &[Expr]) -> CR<usize> {
        let sig = match callee {
            Expr::Name(n, _) => self.sig_of(n),
            _ => None,
        };
        let mut flat = Vec::new();
        flatten(args, &mut flat);
        for (i, a) in flat.iter().enumerate() {
            let subst = sig.as_ref().is_some_and(|s| s.get(i) == Some(&PMode::Subst));
            if subst {
                self.thunk(a)?;
            } else {
                self.expr(a, Mode::L)?;
            }
        }
        Ok(flat.len())
    }

    /// Compile an expression; the result is what is known of its type.
    fn expr(&mut self, e: &Expr, mode: Mode) -> CR<St> {
        Ok(match e {
            Expr::Num(t) => {
                let x: f64 = t
                    .replace('@', "e")
                    .parse()
                    .map_err(|_| format!("line {}: badly formed number {}", self.line, t))?;
                let v = number(x);
                let ty = St::T(v.ty());
                self.emit(Op::Const(v));
                ty
            }
            Expr::Str(s) => {
                self.emit(Op::Const(Value::str(s)));
                St::T(Ty::String)
            }
            Expr::Bool(b) => {
                self.emit(Op::Const(Value::Bool(*b)));
                St::T(Ty::Boolean)
            }
            Expr::Bits(b) => {
                let v = if *b > M24 {
                    Value::LongLogical(*b)
                } else {
                    Value::Logical(*b)
                };
                let ty = St::T(v.ty());
                self.emit(Op::Const(v));
                ty
            }
            Expr::TypeC(t) => {
                let ty = self.resolve_ty(t);
                self.emit(Op::Const(Value::Type(ty)));
                St::T(Ty::Type)
            }
            Expr::Name(n, line) => {
                self.line = *line;
                self.name(n, mode)?;
                self.var(n).map(|v| v.ty.clone()).unwrap_or(St::Any)
            }
            Expr::Rhs(line) => {
                self.line = *line;
                self.name("<rhs>", Mode::R)?;
                St::Any
            }
            Expr::Call(f, args, line) => {
                self.line = *line;
                self.expr(f, Mode::R)?;
                let n = self.args(f, args)?;
                self.line = *line;
                self.emit(Op::Call {
                    n,
                    lmode: mode == Mode::L,
                });
                match &**f {
                    Expr::Name(name, _) => match self.var(name) {
                        Some(v) if v.sig.is_some() => v.result.clone(),
                        Some(Var {
                            ty: St::T(Ty::Array(elem, dims)),
                            ..
                        }) if *dims as usize == n => st((**elem).clone()),
                        Some(_) => St::Any,
                        None if REAL_FUNCTIONS.contains(&name.as_str()) => St::T(Ty::Real),
                        None => St::Any,
                    },
                    _ => St::Any,
                }
            }
            Expr::Un(op, a) => {
                let t = self.expr(a, Mode::R)?;
                self.emit(Op::Un(*op));
                t
            }
            Expr::Bin(op @ (BinOp::And | BinOp::Or), a, b) => {
                // Both operands are evaluated: "an operator may be applied
                // to its operands at any time after each of them has been
                // evaluated" (4.2). There is no short cut.
                let ta = self.expr(a, Mode::R)?;
                let tb = self.expr(b, Mode::R)?;
                self.emit(Op::Bin(*op));
                if ta == tb {
                    ta
                } else {
                    St::Any
                }
            }
            Expr::Bin(op, a, b) => {
                let ta = self.expr(a, Mode::R)?;
                let tb = self.expr(b, Mode::R)?;
                self.emit(Op::Bin(*op));
                match op {
                    BinOp::Concat => St::T(Ty::String),
                    BinOp::Equiv | BinOp::NotEquiv => {
                        if ta == tb {
                            ta
                        } else {
                            St::Any
                        }
                    }
                    op => st_arith(*op, &ta, &tb),
                }
            }
            Expr::Rel(operands, ops) => {
                for o in operands {
                    self.expr(o, Mode::R)?;
                }
                self.emit(Op::Rel(ops.clone()));
                St::T(Ty::Boolean)
            }
            // One only of the arms is evaluated, in the mode of the whole
            // expression: the natural mode of a conditional is L-mode
            // (4.3.2). In R-mode each arm is transformed to the least upper
            // bound of the types of the two (3.3.3 b).
            Expr::Cond(c, a, b) => {
                let j = self.cond_jump(c, false)?;
                let ta = self.expr(a, mode)?;
                let fix_a = self.emit(Op::Transfer(Ty::General));
                let over = self.emit(Op::Jump(0));
                let other = self.here();
                self.patch(j, other);
                let tb = self.expr(b, mode)?;
                let fix_b = self.emit(Op::Transfer(Ty::General));
                let end = self.here();
                self.patch(over, end);
                let lub = st_lub(&ta, &tb);
                // An arm that has an L-value of its own must keep it.
                let transform = mode == Mode::R || !(has_lvalue(a) || has_lvalue(b));
                if let (true, St::T(ty)) = (transform, &lub) {
                    if ty.is_numeric() {
                        self.cur().code[fix_a] = Op::Transfer(ty.clone());
                        self.cur().code[fix_b] = Op::Transfer(ty.clone());
                    }
                }
                lub
            }
            Expr::Valof(items, refof, line) => {
                self.line = *line;
                let name = if *refof { "reference of block" } else { "value of block" };
                let (p, t) = self.valof_proto(name, items, *refof, Kind::Valof)?;
                self.line = *line;
                self.emit(Op::Closure(p));
                self.emit(Op::Call {
                    n: 0,
                    lmode: mode == Mode::L,
                });
                t
            }
            Expr::List(v) => {
                let mut n = 0;
                for e in v {
                    n += self.multi(e, mode, false)?.0;
                }
                self.emit(Op::MkMulti(n));
                St::Any
            }
            Expr::Where(inner, d) => {
                self.push_scope();
                let top = self.top();
                self.def(d, top)?;
                let t = self.expr(inner, mode)?;
                self.pop_scope();
                t
            }
        })
    }
}
