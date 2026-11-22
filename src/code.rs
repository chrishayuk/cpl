//! The abstract machine code. The Cambridge CPL compiler of 1965-66 compiled
//! to calls of GPM macros (LRX, LLX, RV, Fn, ...) for a stack machine in which
//! a function was "a pointer to code plus a pointer to its free variable
//! list" (Richards). This is the same shape of machine.

use crate::ast::{BinOp, PMode, RelOp, UnOp};
use crate::value::{Ty, Value};
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Main,
    Function,
    Routine,
    /// The body of a `value of` or `reference of` block, or of a load part.
    Valof,
    /// An unevaluated actual parameter or definiens (substitution).
    Thunk,
    /// The update part of an LH function; its one parameter is `rhs`.
    Update,
}

#[derive(Clone, Debug)]
pub enum CapSrc {
    Local(usize),
    Cap(usize),
}

/// One entry of a free variable list. `copy` is set for the free variables
/// of fixed functions, which take R-values at definition time (10.3.4).
#[derive(Clone, Debug)]
pub struct Capture {
    pub name: String,
    pub src: CapSrc,
    pub copy: bool,
}

#[derive(Clone, Debug)]
pub struct ParamSpec {
    pub name: String,
    pub ty: Ty,
    pub mode: PMode,
}

pub struct Proto {
    pub name: String,
    pub kind: Kind,
    pub params: Vec<ParamSpec>,
    pub nslots: usize,
    pub slot_names: Vec<String>,
    pub code: Vec<Op>,
    pub lines: Vec<u32>,
    pub caps: Vec<Capture>,
    pub label_pcs: Vec<usize>,
    pub fixed: bool,
}

impl fmt::Debug for Proto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.name)
    }
}

#[derive(Clone, Debug)]
pub enum Op {
    Const(Value),
    /// Load the R-value of a local (cf. LRX).
    LoadLocal(usize),
    /// Load the L-value of a local (cf. LLX).
    LvalLocal(usize),
    LoadCap(usize),
    LvalCap(usize),
    Builtin(usize),
    /// L-value to R-value (cf. RV).
    Deref,
    /// R-value to L-value: a new constant L-value (4.2 b).
    MkLval,
    Dup,
    Pop,
    DefVal {
        slot: usize,
        ty: Option<Ty>,
        constant: bool,
    },
    DefRef {
        slot: usize,
    },
    DefType {
        slot: usize,
        ty: Ty,
    },
    DefSubst {
        slot: usize,
    },
    /// Create the location of a recursively defined function ...
    NewCell {
        slot: usize,
    },
    /// ... and give it its (normally constant) value.
    InitCell {
        slot: usize,
        constant: bool,
    },
    /// Match `have` values (flattening lists of results) against `want` names.
    Unpack {
        have: usize,
        want: usize,
    },
    Assign(usize),
    AssignAll(usize),
    MkMulti(usize),
    /// An automatically inserted transfer function (3.3.3); general = none.
    Transfer(Ty),
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    AndJump(usize),
    OrJump(usize),
    Call {
        n: usize,
        lmode: bool,
    },
    CallCmd(usize),
    Closure(Rc<Proto>),
    /// Compound an L-value from a load function and an update routine.
    MakeLU(Rc<Proto>, Rc<Proto>),
    Return,
    ReturnVal,
    Bin(BinOp),
    Un(UnOp),
    Rel(Vec<RelOp>),
    Goto,
    MakeLabel {
        slot: usize,
        idx: usize,
        name: String,
    },
    ForInit {
        slot: usize,
        steps: Vec<bool>,
    },
    ForNext {
        iter: usize,
        var: usize,
        exit: usize,
    },
    Fail(String),
    Finish,
}

/// A readable listing of the compiled program (`cpl --code`).
pub fn listing(p: &Proto, out: &mut String) {
    let mut nested = Vec::new();
    out.push_str(&format!(
        "{:?} {}{}  [{} locals; free variables: {}]\n",
        p.kind,
        p.name,
        if p.fixed { " (fixed)" } else { "" },
        p.nslots,
        if p.caps.is_empty() {
            "none".to_string()
        } else {
            p.caps
                .iter()
                .map(|c| format!("{}{}", c.name, if c.copy { " by value" } else { " by reference" }))
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    for (i, op) in p.code.iter().enumerate() {
        let text = match op {
            Op::LoadLocal(s) => format!("LoadLocal {} ({})", s, p.slot_names[*s]),
            Op::LvalLocal(s) => format!("LvalLocal {} ({})", s, p.slot_names[*s]),
            Op::LoadCap(c) => format!("LoadFree {} ({})", c, p.caps[*c].name),
            Op::LvalCap(c) => format!("LvalFree {} ({})", c, p.caps[*c].name),
            Op::Builtin(b) => format!("Basic {}", crate::builtins::TABLE[*b].0),
            Op::Closure(q) => {
                nested.push(q.clone());
                format!("Closure {}", q.name)
            }
            Op::MakeLU(l, u) => {
                nested.push(l.clone());
                nested.push(u.clone());
                format!("MakeLU {} {}", l.name, u.name)
            }
            op => format!("{:?}", op),
        };
        out.push_str(&format!("  {:4}  {}\n", i, text));
    }
    out.push('\n');
    for q in nested {
        listing(&q, out);
    }
}
