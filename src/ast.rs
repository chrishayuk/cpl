//! Syntax tree for canonical CPL (Reference Manual draft, Working Papers 1966).

use crate::value::Ty;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    And,
    Or,
    Equiv,
    NotEquiv,
    Concat,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    MuchLt,
    MuchGt,
}

/// Mode of calling a formal parameter (10.3.3). `Subst` is the call by
/// substitution of the 1963 paper, withdrawn from the language in 1966.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PMode {
    Value,
    Ref,
    Subst,
}

/// A written type. A missing component type means "the preferred type" (7.4.1).
#[derive(Clone, Debug)]
pub enum TyExpr {
    Named(Ty),
    Function(Option<Box<TyExpr>>),
    Array(Option<Box<TyExpr>>, u8),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Num(String),
    Str(String),
    Bool(bool),
    Bits(u64),
    TypeC(TyExpr),
    Name(String, u32),
    Rhs(u32),
    Call(Box<Expr>, Vec<Expr>, u32),
    Un(UnOp, Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    /// A compound relation: n+1 operands, n relational operators (6.1.4).
    Rel(Vec<Expr>, Vec<RelOp>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `value of` block (false) or `reference of` block (true).
    Valof(Vec<Item>, bool, u32),
    /// A bracketed expression list. Bracketing has no significance (4.5).
    List(Vec<Expr>),
    Where(Box<Expr>, Box<Def>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefOp {
    /// `=`  definition by value
    Value,
    /// `~~` definition by reference
    Ref,
    /// `==` definition by substitution (1963)
    Subst,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Option<TyExpr>,
    pub mode: PMode,
}

#[derive(Clone, Debug)]
pub enum FuncBody {
    /// `f[..] = E` (fixed) or `f[..] == E` (free, when the flag is true).
    Expr(Box<Expr>, bool),
    Routine(Box<Cmd>),
    /// An LH function: `function F[..] be fix D load B update B`.
    LH {
        fix: Option<Box<Def>>,
        load: Vec<Item>,
        update: Vec<Item>,
    },
}

#[derive(Clone, Debug)]
pub enum Def {
    Simple {
        names: Vec<String>,
        op: DefOp,
        all: bool,
        exprs: Vec<Expr>,
        constant: bool,
        line: u32,
    },
    Type {
        names: Vec<String>,
        types: Vec<TyExpr>,
        line: u32,
    },
    Func {
        name: String,
        params: Vec<Param>,
        body: FuncBody,
        fixed: Option<bool>,
        constant: bool,
        line: u32,
    },
    And(Vec<Def>),
    In(Box<Def>, Box<Def>),
    Rec(Box<Def>),
}

#[derive(Clone, Debug)]
pub enum ForElem {
    Single(Expr),
    Step(Expr, Expr, Expr),
    To(Expr, Expr),
    Dots(Expr, Expr, Expr),
}

#[derive(Clone, Debug)]
pub struct Cmd {
    pub kind: CmdKind,
    pub line: u32,
}

#[derive(Clone, Debug)]
pub enum CmdKind {
    Block(Vec<Item>),
    Assign(Vec<Expr>, Vec<Expr>),
    AssignAll(Vec<Expr>, Expr),
    Call(Expr),
    Goto(Expr),
    ResultIs(Vec<Expr>),
    Break,
    Return,
    Finish,
    /// `if` (false) or `unless` (true)
    If(Expr, bool, Box<Cmd>),
    Test(Expr, Box<Cmd>, Box<Cmd>),
    /// `while` (false) or `until` (true)
    While(Expr, bool, Box<Cmd>),
    /// `C repeat`, `C repeat while E` (false), `C repeat until E` (true)
    Repeat(Box<Cmd>, Option<(Expr, bool)>),
    For(String, Vec<ForElem>, Box<Cmd>),
    Labelled(String, Box<Cmd>),
    Switch(Expr, Vec<Item>),
    Case(Expr, Box<Cmd>),
    Default(Box<Cmd>),
    Where(Box<Cmd>, Def),
    Empty,
}

#[derive(Clone, Debug)]
pub enum Item {
    Let(Def),
    Prefer(TyExpr),
    Cmd(Cmd),
}
