//! Syntax analysis. The grammar is that of the Reference Manual draft
//! (Working Papers, July 1966), Sections 4 to 10, with the grouping of
//! infixed operators of 6.1.4.
//!
//! Layout follows 2.1.2: the end of a line ends a command unless the command
//! could not end there. In practice that means an expression does not carry
//! on to a following line that begins with a name, a number, `(`, `+` or `-`
//! (all of which can start a command); it does carry on over a line that
//! begins with any other operator, or `[`.

use crate::ast::*;
use crate::lexer::{Tok, Token};
use crate::value::Ty;

pub type PR<T> = Result<T, String>;

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    /// The definitions met so far.
    notes: Vec<Note>,
    /// True within a definition marked `rec`, whose functions know themselves.
    rec: bool,
    /// The bodies that labels belong to, innermost last: where each starts
    /// and the labels set in it so far.
    frames: Vec<(usize, Vec<usize>)>,
}

/// What kind of thing a name is defined to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteKind {
    Variable,
    Constant,
    Function,
    Routine,
    Parameter,
    Label,
    /// The controlled variable of a for-command.
    Loop,
}

/// A definition the parser met, kept for `analysis.rs`, which tells an
/// editor where each name is defined. Places are indexes into the tokens.
#[derive(Clone, Debug)]
pub struct Note {
    pub kind: NoteKind,
    /// The token that is the name.
    pub token: usize,
    /// The tokens in which the name has this meaning: from, and up to.
    pub scope: (usize, usize),
    /// The tokens of the whole definition.
    pub extent: (usize, usize),
    /// For a parameter, the note of its function or routine.
    pub owner: Option<usize>,
    /// Defined by a where-clause.
    pub by_where: bool,
    /// A function in a recursive definition: its scope includes itself.
    rec: bool,
    /// The scope is not yet known: the construct the definition is part of
    /// (a block, `in`, `where`) will say.
    waiting: bool,
}

/// What `parse_noting` gives back.
pub struct Parsed {
    pub items: PR<Vec<Item>>,
    pub notes: Vec<Note>,
    /// The token the parser had reached: if it failed, the one it could not take.
    pub at: usize,
}

pub fn parse(toks: Vec<Token>) -> PR<Vec<Item>> {
    parse_noting(toks).items
}

/// Parses, and also reports every definition met on the way, even when the
/// program turns out to be wrong further on.
pub fn parse_noting(toks: Vec<Token>) -> Parsed {
    let end = toks.len();
    let mut p = Parser {
        toks,
        pos: 0,
        notes: Vec::new(),
        rec: false,
        frames: vec![(0, Vec::new())],
    };
    let mut items = p.items(false);
    if items.is_ok() && p.peek().tok != Tok::Eof {
        items = p.err("a closing section bracket with no matching opening bracket");
    }
    // A label is known throughout the body it is in.
    for (from, labels) in std::mem::take(&mut p.frames) {
        for k in labels {
            p.notes[k].scope = (from, end);
        }
    }
    Parsed {
        items,
        at: p.pos.min(end - 1),
        notes: p.notes,
    }
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Name(n) => format!("the name {}", n),
        Tok::Num(n) => format!("the number {}", n),
        Tok::Str(s) => format!("the string '{}'", s),
        Tok::Bits(_) => "a logical constant".into(),
        Tok::Kw(k) => format!("'{}'", k),
        Tok::Sym(s) => format!("'{}'", s),
        Tok::Open => "an opening section bracket".into(),
        Tok::Close => "a closing section bracket".into(),
        Tok::Eof => "the end of the program".into(),
    }
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.pos + n).min(self.toks.len() - 1)].tok
    }

    fn line(&self) -> u32 {
        self.peek().line
    }

    fn next(&mut self) -> Tok {
        let t = self.peek().tok.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn err<T>(&self, msg: &str) -> PR<T> {
        Err(format!(
            "line {}: {} (found {})",
            self.line(),
            msg,
            describe(&self.peek().tok)
        ))
    }

    fn is_sym(&self, s: &str) -> bool {
        matches!(&self.peek().tok, Tok::Sym(x) if *x == s)
    }

    fn is_kw(&self, k: &str) -> bool {
        matches!(&self.peek().tok, Tok::Kw(x) if *x == k)
    }

    fn eat_sym(&mut self, s: &str) -> bool {
        let hit = self.is_sym(s);
        if hit {
            self.next();
        }
        hit
    }

    fn eat_kw(&mut self, k: &str) -> bool {
        let hit = self.is_kw(k);
        if hit {
            self.next();
        }
        hit
    }

    fn expect_sym(&mut self, s: &str) -> PR<()> {
        if self.eat_sym(s) {
            Ok(())
        } else {
            self.err(&format!("'{}' expected", s))
        }
    }

    fn expect_kw(&mut self, k: &str) -> PR<()> {
        if self.eat_kw(k) {
            Ok(())
        } else {
            self.err(&format!("'{}' expected", k))
        }
    }

    // ------------------------------------------------- notes for an editor

    fn note(&mut self, kind: NoteKind, token: usize) -> usize {
        self.notes.push(Note {
            kind,
            token,
            scope: (token, usize::MAX),
            extent: (token, token + 1),
            owner: None,
            by_where: false,
            rec: self.rec && matches!(kind, NoteKind::Function | NoteKind::Routine),
            waiting: false,
        });
        self.notes.len() - 1
    }

    /// Gives a scope to the names defined since note `first` that are still
    /// waiting for one.
    fn settle(&mut self, first: usize, scope: (usize, usize), by_where: bool) -> Vec<usize> {
        let mut settled = Vec::new();
        for (k, n) in self.notes.iter_mut().enumerate().skip(first) {
            if n.waiting {
                n.waiting = false;
                n.scope = scope;
                n.by_where = by_where;
                settled.push(k);
            }
        }
        settled
    }

    /// The names of a simple definition, once the whole of it has been read.
    fn note_names(&mut self, kind: NoteKind, at: &[usize], from: usize) {
        for &token in at {
            let k = self.note(kind, token);
            self.notes[k].extent = (from, self.pos);
            self.notes[k].waiting = true;
        }
    }

    /// A function or routine whose body has just been read: its parameters
    /// are known up to here.
    fn note_function(&mut self, f: usize, params: (usize, usize), from: usize) {
        let to = self.pos;
        self.notes[f].extent = (from, to);
        self.notes[f].waiting = true;
        for n in &mut self.notes[params.0..params.1] {
            n.owner = Some(f);
            n.scope = (n.token, to);
        }
    }

    fn open_frame(&mut self) {
        self.frames.push((self.pos, Vec::new()));
    }

    fn close_frame(&mut self) {
        if let Some((from, labels)) = self.frames.pop() {
            for k in labels {
                self.notes[k].scope = (from, self.pos);
            }
        }
    }

    fn name(&mut self) -> PR<String> {
        match self.peek().tok.clone() {
            Tok::Name(n) => {
                self.next();
                Ok(n)
            }
            _ => self.err("a name expected"),
        }
    }

    // ------------------------------------------------------------ blocks

    /// The declarations and commands of a block, up to its closing bracket.
    fn items(&mut self, in_block: bool) -> PR<Vec<Item>> {
        let mut out = Vec::new();
        // The names defined in this block, which are known up to its end.
        let mut mine: Vec<usize> = Vec::new();
        let rec = std::mem::take(&mut self.rec);
        loop {
            while self.eat_sym(";") {}
            match &self.peek().tok {
                Tok::Eof | Tok::Close => {
                    let at_end = self.peek().tok == Tok::Eof;
                    if in_block && at_end {
                        return self.err("closing section bracket expected");
                    }
                    for k in mine {
                        self.notes[k].scope.1 = self.pos;
                    }
                    self.rec = rec;
                    if in_block {
                        self.next();
                    }
                    return Ok(out);
                }
                _ => {}
            }
            // The Elementary Programming Manual writes routine definitions
            // without let: "routine LINEQ [...] be".
            let heading = (self.is_kw("routine") || self.is_kw("function")) && matches!(self.peek_at(1), Tok::Name(_));
            if self.eat_kw("let") || heading {
                let (from, first) = (self.pos, self.notes.len());
                out.push(Item::Let(self.def()?));
                // A definition is known from its end, or from its start if
                // it is recursive (8.3).
                let to = self.pos;
                for (k, n) in self.notes.iter_mut().enumerate().skip(first) {
                    if n.waiting {
                        n.waiting = false;
                        n.scope.0 = if n.rec { from } else { to };
                        mine.push(k);
                    }
                }
            } else if self.eat_kw("prefer") {
                out.push(Item::Prefer(self.ty()?));
            } else if self.eat_kw("sic") {
                // Rearrangement (1.1.5) is never performed, so sic is implied.
            } else if self.eat_kw("note") {
                if !matches!(self.next(), Tok::Str(_)) {
                    return self.err("a string constant expected after note");
                }
            } else {
                out.push(Item::Cmd(self.command()?));
            }
        }
    }

    fn block(&mut self) -> PR<Vec<Item>> {
        if self.peek().tok != Tok::Open {
            return self.err("opening section bracket expected");
        }
        self.next();
        self.items(true)
    }

    // ------------------------------------------------------------- types

    fn data_type(&self) -> Option<Ty> {
        let Tok::Kw(k) = &self.peek().tok else { return None };
        Some(match *k {
            "real" => Ty::Real,
            "integer" => Ty::Integer,
            "index" => Ty::Index,
            "complex" => Ty::Complex,
            "double" => Ty::Double,
            "doublecomplex" => Ty::DComplex,
            "logical" => Ty::Logical,
            "longlogical" => Ty::LongLogical,
            "boolean" => Ty::Boolean,
            "string" => Ty::String,
            "label" => Ty::Label,
            "routine" => Ty::Routine,
            "general" => Ty::General,
            "type" => Ty::Type,
            "list" => Ty::List,
            _ => return None,
        })
    }

    fn at_type_suffix(&self) -> bool {
        self.is_kw("function")
            || self.is_kw("vector")
            || self.is_kw("matrix")
            || self.is_kw("array")
            || (matches!(self.peek().tok, Tok::Num(_)) && matches!(self.peek_at(1), Tok::Kw("array")))
    }

    fn at_type(&self) -> bool {
        self.data_type().is_some() || self.at_type_suffix()
    }

    fn ty(&mut self) -> PR<TyExpr> {
        let mut t = None;
        if let Some(d) = self.data_type() {
            self.next();
            t = Some(TyExpr::Named(d));
        } else if self.is_sym("(") {
            // A list of result types: such a function is treated as general.
            self.next();
            loop {
                self.ty()?;
                if !self.eat_sym(",") {
                    break;
                }
            }
            self.expect_sym(")")?;
            t = Some(TyExpr::Named(Ty::General));
        }
        loop {
            if self.eat_kw("function") {
                t = Some(TyExpr::Function(t.map(Box::new)));
            } else if self.eat_kw("vector") || self.eat_kw("array") {
                t = Some(TyExpr::Array(t.map(Box::new), 1));
            } else if self.eat_kw("matrix") {
                t = Some(TyExpr::Array(t.map(Box::new), 2));
            } else if matches!(self.peek().tok, Tok::Num(_)) && matches!(self.peek_at(1), Tok::Kw("array")) {
                let Tok::Num(n) = self.next() else { unreachable!() };
                self.next();
                let dims: u8 = n
                    .parse()
                    .map_err(|_| format!("line {}: bad array dimension {}", self.line(), n))?;
                t = Some(TyExpr::Array(t.map(Box::new), dims.max(1)));
            } else {
                break;
            }
        }
        match t {
            Some(t) => Ok(t),
            None => self.err("a type expected"),
        }
    }

    // ------------------------------------------------------- definitions

    /// <definition> ::= <and definition> | <and definition> in <definition>
    fn def(&mut self) -> PR<Def> {
        let first = self.notes.len();
        let d = self.and_def()?;
        if self.eat_kw("in") {
            let (last, from) = (self.notes.len(), self.pos);
            let rest = self.def()?;
            // D1 in D2: the scope of D1 is D2 alone (8.3.4).
            let to = self.pos;
            for n in &mut self.notes[first..last] {
                if n.waiting {
                    n.waiting = false;
                    n.scope = (from, to);
                }
            }
            return Ok(Def::In(Box::new(d), Box::new(rest)));
        }
        Ok(d)
    }

    fn and_def(&mut self) -> PR<Def> {
        // `rec` before a group joined by `and` makes the whole group
        // mutually recursive, as `rec $( ... and ... $)` does in 8.3.2.
        let rec = self.eat_kw("recursive");
        let outer = self.rec;
        self.rec |= rec;
        let mut v = vec![self.basic_def()?];
        while self.eat_kw("and") {
            self.eat_kw("recursive");
            v.push(self.basic_def()?);
        }
        self.rec = outer;
        let d = if v.len() == 1 { v.pop().unwrap() } else { Def::And(v) };
        Ok(if rec { Def::Rec(Box::new(d)) } else { d })
    }

    fn params(&mut self) -> PR<Vec<Param>> {
        let mut out = Vec::new();
        self.expect_sym("[")?;
        if self.eat_sym("]") {
            return Ok(out);
        }
        // The type and the mode of a parameter are the nearest ones
        // specified to its left (10.3.3).
        let (mut ty, mut mode) = (None, PMode::Value);
        loop {
            loop {
                if self.eat_kw("value") {
                    mode = PMode::Value;
                } else if self.eat_kw("reference") {
                    mode = PMode::Ref;
                } else if self.eat_kw("subst") {
                    mode = PMode::Subst;
                } else if self.at_type() || self.is_sym("(") {
                    ty = Some(self.ty()?);
                } else {
                    break;
                }
            }
            let at = self.pos;
            out.push(Param {
                name: self.name()?,
                ty: ty.clone(),
                mode,
            });
            self.note(NoteKind::Parameter, at);
            if !self.eat_sym(",") {
                break;
            }
        }
        self.expect_sym("]")?;
        Ok(out)
    }

    /// A definiens: an expression list, possibly qualified by a where-clause
    /// whose scope is this definiens (and so lies inside the scope of the
    /// formal parameters of the definition).
    fn definiens(&mut self) -> PR<Expr> {
        let (from, rec) = (self.pos, std::mem::take(&mut self.rec));
        let mut v = self.expr_list()?;
        let mut e = if v.len() == 1 { v.pop().unwrap() } else { Expr::List(v) };
        while self.eat_kw("where") {
            let first = self.notes.len();
            let d = self.def()?;
            self.settle(first, (from, self.pos), true);
            e = Expr::Where(Box::new(e), Box::new(d));
        }
        self.rec = rec;
        Ok(e)
    }

    fn basic_def(&mut self) -> PR<Def> {
        let line = self.line();
        let from = self.pos;
        if self.peek().tok == Tok::Open {
            self.next();
            let d = self.def()?;
            while self.eat_sym(";") {}
            if self.next() != Tok::Close {
                return self.err("closing section bracket expected after a bracketed definition");
            }
            return Ok(d);
        }
        if self.eat_kw("recursive") {
            let outer = std::mem::replace(&mut self.rec, true);
            let d = self.basic_def()?;
            self.rec = outer;
            return Ok(Def::Rec(Box::new(d)));
        }
        let mut constant = None;
        let mut fixed = None;
        loop {
            if self.eat_kw("variable") {
                constant = Some(false);
            } else if self.eat_kw("constant") {
                constant = Some(true);
            } else if self.eat_kw("fixed") {
                fixed = Some(true);
            } else if self.eat_kw("free") {
                fixed = Some(false);
            } else {
                break;
            }
        }
        if self.is_kw("function") || self.is_kw("routine") {
            let is_fn = self.is_kw("function");
            self.next();
            let at = self.pos;
            let name = self.name()?;
            let f = self.note(if is_fn { NoteKind::Function } else { NoteKind::Routine }, at);
            let p0 = self.notes.len();
            let params = if self.is_sym("[") { self.params()? } else { Vec::new() };
            let p1 = self.notes.len();
            self.expect_kw("be")?;
            let rec = std::mem::take(&mut self.rec);
            let body = if is_fn {
                let bracketed = self.peek().tok == Tok::Open;
                if bracketed {
                    self.next();
                }
                let mut fixed_names = Vec::new();
                let fix = if self.eat_kw("fix") {
                    let first = self.notes.len();
                    let d = self.def()?;
                    fixed_names = self.settle(first, (self.pos, usize::MAX), false);
                    Some(Box::new(d))
                } else {
                    None
                };
                while self.eat_sym(";") {}
                self.expect_kw("load")?;
                self.open_frame();
                let load = self.block()?;
                self.close_frame();
                self.expect_kw("update")?;
                self.open_frame();
                let update = self.block()?;
                self.close_frame();
                for k in fixed_names {
                    self.notes[k].scope.1 = self.pos;
                }
                if bracketed && self.next() != Tok::Close {
                    return self.err("closing section bracket expected after the body of an LH function");
                }
                FuncBody::LH { fix, load, update }
            } else {
                self.open_frame();
                let c = self.basic()?;
                self.close_frame();
                FuncBody::Routine(Box::new(c))
            };
            self.rec = rec;
            self.note_function(f, (p0, p1), from);
            return Ok(Def::Func {
                name,
                params,
                body,
                fixed,
                constant: constant.unwrap_or(true),
                line,
            });
        }
        let mut name_at = vec![self.pos];
        let mut names = vec![self.name()?];
        if self.is_sym("[") {
            let f = self.note(NoteKind::Function, name_at[0]);
            let p0 = self.notes.len();
            let params = self.params()?;
            let p1 = self.notes.len();
            let free = if self.eat_sym("=") {
                false
            } else if self.eat_sym("==") {
                true
            } else {
                return self.err("'=' or '==' expected in a function definition");
            };
            let body = FuncBody::Expr(Box::new(self.definiens()?), free);
            self.note_function(f, (p0, p1), from);
            let name = names.pop().unwrap();
            return Ok(Def::Func {
                name,
                params,
                body,
                fixed,
                constant: constant.unwrap_or(true),
                line,
            });
        }
        while self.eat_sym(",") {
            name_at.push(self.pos);
            names.push(self.name()?);
        }
        let kind = if constant == Some(true) {
            NoteKind::Constant
        } else {
            NoteKind::Variable
        };
        if self.eat_kw("be") || self.is_kw("allbe") {
            let all = self.eat_kw("allbe");
            let mut types = vec![self.ty()?];
            while self.eat_sym(",") {
                types.push(self.ty()?);
            }
            if all || (types.len() == 1 && names.len() > 1) {
                types = vec![types[0].clone(); names.len()];
            }
            if types.len() != names.len() {
                return Err(format!(
                    "line {}: {} names are defined but {} types are given",
                    line,
                    names.len(),
                    types.len()
                ));
            }
            self.note_names(kind, &name_at, from);
            return Ok(Def::Type { names, types, line });
        }
        let all = self.eat_kw("all");
        let op = if self.eat_sym("=") {
            DefOp::Value
        } else if self.eat_sym("~~") {
            DefOp::Ref
        } else if self.eat_sym("==") {
            DefOp::Subst
        } else {
            return self.err("'=', '~~', '==' or 'be' expected in a definition");
        };
        let e = self.definiens()?;
        let exprs = match e {
            Expr::List(v) => v,
            e => vec![e],
        };
        self.note_names(kind, &name_at, from);
        Ok(Def::Simple {
            names,
            op,
            all,
            exprs,
            constant: constant.unwrap_or(false),
            line,
        })
    }

    // ---------------------------------------------------------- commands

    /// <command> ::= <basic command> | <command> where <in definition>
    fn command(&mut self) -> PR<Cmd> {
        let from = self.pos;
        let mut c = self.basic()?;
        while self.is_kw("where") {
            let line = self.line();
            self.next();
            let (first, rec) = (self.notes.len(), std::mem::take(&mut self.rec));
            let d = self.def()?;
            self.rec = rec;
            self.settle(first, (from, self.pos), true);
            c = Cmd {
                kind: CmdKind::Where(Box::new(c), d),
                line,
            };
        }
        Ok(c)
    }

    fn at_end_of_block(&self) -> bool {
        matches!(self.peek().tok, Tok::Close | Tok::Eof)
    }

    fn labelled_body(&mut self) -> PR<Box<Cmd>> {
        if self.at_end_of_block() || self.is_kw("let") {
            return Ok(Box::new(Cmd {
                kind: CmdKind::Empty,
                line: self.line(),
            }));
        }
        Ok(Box::new(self.basic()?))
    }

    fn basic(&mut self) -> PR<Cmd> {
        let line = self.line();
        let mk = |kind| Cmd { kind, line };
        if matches!(self.peek().tok, Tok::Name(_)) && matches!(self.peek_at(1), Tok::Sym(":")) {
            let at = self.pos;
            let n = self.name()?;
            self.next();
            let k = self.note(NoteKind::Label, at);
            if let Some(frame) = self.frames.last_mut() {
                frame.1.push(k);
            }
            return Ok(mk(CmdKind::Labelled(n, self.labelled_body()?)));
        }
        if self.eat_kw("case") {
            let k = self.basic_expr()?;
            self.expect_sym(":")?;
            return Ok(mk(CmdKind::Case(k, self.labelled_body()?)));
        }
        if self.eat_kw("default") {
            self.expect_sym(":")?;
            return Ok(mk(CmdKind::Default(self.labelled_body()?)));
        }
        let kind = if self.peek().tok == Tok::Open {
            CmdKind::Block(self.block()?)
        } else if self.is_kw("if") || self.is_kw("unless") {
            let unless = self.is_kw("unless");
            self.next();
            let e = self.basic_expr()?;
            self.eat_kw("do");
            CmdKind::If(e, unless, Box::new(self.basic()?))
        } else if self.eat_kw("test") {
            let e = self.basic_expr()?;
            self.eat_kw("do");
            let a = self.basic()?;
            while self.eat_sym(";") {}
            self.expect_kw("or")?;
            CmdKind::Test(e, Box::new(a), Box::new(self.basic()?))
        } else if self.is_kw("while") || self.is_kw("until") {
            let until = self.is_kw("until");
            self.next();
            let e = self.basic_expr()?;
            self.eat_kw("do");
            CmdKind::While(e, until, Box::new(self.basic()?))
        } else if self.eat_kw("for") {
            // "If the programmer wishes an external variable to be used,
            // the for symbol is followed by ext" (Elementary Manual, 13.2).
            // The variable is then assigned to at the start of each cycle.
            let ext = self.eat_kw("ext");
            let at = self.pos;
            let n = self.name()?;
            self.expect_sym("=")?;
            let list = self.for_list()?;
            self.eat_kw("do");
            // The controlled variable is known in the body of the cycle.
            let noted = (!ext).then(|| self.note(NoteKind::Loop, at));
            let from = self.pos;
            let body = self.basic()?;
            if let Some(k) = noted {
                self.notes[k].scope = (from, self.pos);
            }
            if ext {
                let hidden = format!("{} (for ext)", n);
                let set = CmdKind::Assign(vec![Expr::Name(n, line)], vec![Expr::Name(hidden.clone(), line)]);
                let items = vec![Item::Cmd(Cmd { kind: set, line }), Item::Cmd(body)];
                CmdKind::For(
                    hidden,
                    list,
                    Box::new(Cmd {
                        kind: CmdKind::Block(items),
                        line,
                    }),
                )
            } else {
                CmdKind::For(n, list, Box::new(body))
            }
        } else if self.eat_kw("goto") {
            CmdKind::Goto(self.basic_expr()?)
        } else if self.eat_kw("resultis") {
            CmdKind::ResultIs(self.expr_list()?)
        } else if self.eat_kw("break") {
            CmdKind::Break
        } else if self.eat_kw("return") {
            CmdKind::Return
        } else if self.eat_kw("finish") {
            CmdKind::Finish
        } else if self.eat_kw("switchon") {
            let e = self.basic_expr()?;
            self.expect_kw("into")?;
            CmdKind::Switch(e, self.block()?)
        } else {
            let lhs = self.expr_list()?;
            if self.eat_sym(":=") {
                CmdKind::Assign(lhs, self.expr_list()?)
            } else if self.is_kw("all") && matches!(self.peek_at(1), Tok::Sym(":=")) {
                self.next();
                self.next();
                CmdKind::AssignAll(lhs, self.basic_expr()?)
            } else if lhs.len() == 1 && matches!(lhs[0], Expr::Call(..) | Expr::Name(..)) {
                CmdKind::Call(lhs.into_iter().next().unwrap())
            } else {
                return self.err("this expression is not a command; ':=' expected");
            }
        };
        let mut c = mk(kind);
        loop {
            let line = self.line();
            if self.eat_kw("repeat") {
                c = Cmd {
                    kind: CmdKind::Repeat(Box::new(c), None),
                    line,
                };
            } else if self.is_kw("repeatwhile") || self.is_kw("repeatuntil") {
                let until = self.is_kw("repeatuntil");
                self.next();
                let e = self.basic_expr()?;
                c = Cmd {
                    kind: CmdKind::Repeat(Box::new(c), Some((e, until))),
                    line,
                };
            } else {
                return Ok(c);
            }
        }
    }

    fn for_list(&mut self) -> PR<Vec<ForElem>> {
        let mut v = Vec::new();
        loop {
            if self.eat_kw("step") {
                let a = self.basic_expr()?;
                self.expect_sym(",")?;
                let b = self.basic_expr()?;
                self.expect_sym(",")?;
                v.push(ForElem::Step(a, b, self.basic_expr()?));
            } else {
                let a = self.basic_expr()?;
                if self.eat_kw("to") {
                    v.push(ForElem::To(a, self.basic_expr()?));
                } else if self.is_sym(",") && !matches!(self.peek_at(1), Tok::Kw("step")) {
                    // Perhaps the form  E1, E2, ..., E3
                    let save = self.pos;
                    self.next();
                    let b = self.basic_expr()?;
                    if self.is_sym(",") && matches!(self.peek_at(1), Tok::Sym("...")) {
                        self.next();
                        self.next();
                        self.eat_sym(",");
                        v.push(ForElem::Dots(a, b, self.basic_expr()?));
                    } else {
                        self.pos = save;
                        v.push(ForElem::Single(a));
                    }
                } else {
                    v.push(ForElem::Single(a));
                }
            }
            if !self.eat_sym(",") {
                return Ok(v);
            }
        }
    }

    // ------------------------------------------------------- expressions

    fn expr_list(&mut self) -> PR<Vec<Expr>> {
        let mut v = vec![self.basic_expr()?];
        while self.eat_sym(",") {
            v.push(self.basic_expr()?);
        }
        Ok(v)
    }

    /// <basic expression>: an infixed expression or a conditional expression
    ///   E1 -> E2, E3
    fn basic_expr(&mut self) -> PR<Expr> {
        let c = self.equiv()?;
        if self.eat_sym("->") {
            let a = self.basic_expr()?;
            self.expect_sym(",")?;
            let b = self.basic_expr()?;
            return Ok(Expr::Cond(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn equiv(&mut self) -> PR<Expr> {
        let mut e = self.or()?;
        loop {
            let op = if self.eat_sym("==") {
                BinOp::Equiv
            } else if self.eat_sym("~==") {
                BinOp::NotEquiv
            } else {
                return Ok(e);
            };
            e = Expr::Bin(op, Box::new(e), Box::new(self.or()?));
        }
    }

    fn or(&mut self) -> PR<Expr> {
        let mut e = self.and()?;
        while self.eat_sym("\\/") {
            e = Expr::Bin(BinOp::Or, Box::new(e), Box::new(self.and()?));
        }
        Ok(e)
    }

    fn and(&mut self) -> PR<Expr> {
        let mut e = self.relation()?;
        while self.eat_sym("/\\") {
            e = Expr::Bin(BinOp::And, Box::new(e), Box::new(self.relation()?));
        }
        Ok(e)
    }

    fn rel_op(&self) -> Option<RelOp> {
        let Tok::Sym(s) = &self.peek().tok else { return None };
        Some(match *s {
            "=" => RelOp::Eq,
            "~=" => RelOp::Ne,
            "<" => RelOp::Lt,
            "<=" => RelOp::Le,
            ">" => RelOp::Gt,
            ">=" => RelOp::Ge,
            "<<" => RelOp::MuchLt,
            ">>" => RelOp::MuchGt,
            _ => return None,
        })
    }

    /// Relations combine into one compound relation: a < b <= c (6.1.4).
    fn relation(&mut self) -> PR<Expr> {
        let first = self.concat()?;
        let mut operands = vec![first];
        let mut ops = Vec::new();
        while let Some(op) = self.rel_op() {
            self.next();
            ops.push(op);
            operands.push(self.concat()?);
        }
        if ops.is_empty() {
            return Ok(operands.pop().unwrap());
        }
        Ok(Expr::Rel(operands, ops))
    }

    fn concat(&mut self) -> PR<Expr> {
        let mut e = self.additive()?;
        while self.eat_sym("<=>") {
            e = Expr::Bin(BinOp::Concat, Box::new(e), Box::new(self.additive()?));
        }
        Ok(e)
    }

    fn additive(&mut self) -> PR<Expr> {
        let mut e = self.term()?;
        loop {
            if self.peek().nl {
                return Ok(e);
            }
            let op = if self.eat_sym("+") {
                BinOp::Add
            } else if self.eat_sym("-") {
                BinOp::Sub
            } else {
                return Ok(e);
            };
            e = Expr::Bin(op, Box::new(e), Box::new(self.term()?));
        }
    }

    /// Multiplication, division, and multiplication written by juxtaposition
    /// ("the operator is inserted before any further analysis", 6.1.3).
    ///
    /// Grouping follows the table of precedences of 6.1.4: x has PL 9 and
    /// PR 9, / has PL 9 and PR 8. So a group of multiplications may be
    /// followed by a single / , and the / takes everything after it at
    /// this level as its right operand: a/bc is a/(bc), a/b/c is a/(b/c),
    /// pq/r is (pq)/r. The 1963 paper gives a/bc the same meaning.
    fn term(&mut self) -> PR<Expr> {
        let e = self.product()?;
        if self.eat_sym("/") {
            return Ok(Expr::Bin(BinOp::Div, Box::new(e), Box::new(self.term()?)));
        }
        Ok(e)
    }

    fn product(&mut self) -> PR<Expr> {
        let mut e = self.signed()?;
        loop {
            let juxtaposed = !self.peek().nl
                && (matches!(self.peek().tok, Tok::Name(_) | Tok::Num(_)) || self.is_sym("("))
                && !matches!(self.peek_at(1), Tok::Sym(":"));
            if !self.eat_sym("*") && !juxtaposed {
                return Ok(e);
            }
            e = Expr::Bin(BinOp::Mul, Box::new(e), Box::new(self.signed()?));
        }
    }

    /// Monadic + and - stand for multiplication by (+1) and (-1) (5.1.2 b).
    fn signed(&mut self) -> PR<Expr> {
        if self.eat_sym("-") {
            return Ok(Expr::Un(UnOp::Neg, Box::new(self.signed()?)));
        }
        if self.eat_sym("+") {
            return self.signed();
        }
        self.power()
    }

    /// The power operator has PL 10 and PR 8 (6.1.4): it binds more tightly
    /// than x or / on its left, but its right operand is the whole of the
    /// multiplications and division that follow it, so a**bc is a**(bc)
    /// and a**b**c is a**(b**c).
    fn power(&mut self) -> PR<Expr> {
        let base = self.not()?;
        if self.eat_sym("**") {
            let exp = self.term()?;
            return Ok(Expr::Bin(BinOp::Pow, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    /// ~ binds more tightly than any infixed operator (5.1.2 a).
    fn not(&mut self) -> PR<Expr> {
        if self.eat_sym("~") {
            return Ok(Expr::Un(UnOp::Not, Box::new(self.not()?)));
        }
        if self.eat_sym("~~") {
            let e = Expr::Un(UnOp::Not, Box::new(self.not()?));
            return Ok(Expr::Un(UnOp::Not, Box::new(e)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> PR<Expr> {
        let mut e = self.primary()?;
        while self.is_sym("[") {
            let line = self.line();
            self.next();
            let args = if self.is_sym("]") {
                Vec::new()
            } else {
                self.expr_list()?
            };
            self.expect_sym("]")?;
            e = Expr::Call(Box::new(e), args, line);
        }
        Ok(e)
    }

    fn primary(&mut self) -> PR<Expr> {
        let line = self.line();
        // Boolean is both a type and the name of the transfer function to it.
        if self.is_kw("boolean") && matches!(self.peek_at(1), Tok::Sym("[")) {
            self.next();
            return Ok(Expr::Name("Boolean".into(), line));
        }
        if self.data_type().is_some() || self.is_kw("vector") || self.is_kw("matrix") || self.is_kw("function") {
            return Ok(Expr::TypeC(self.ty()?));
        }
        match self.peek().tok.clone() {
            Tok::Num(n) => {
                self.next();
                Ok(Expr::Num(n))
            }
            Tok::Str(s) => {
                self.next();
                Ok(Expr::Str(s))
            }
            Tok::Bits(b) => {
                self.next();
                Ok(Expr::Bits(b))
            }
            Tok::Name(n) => {
                self.next();
                Ok(Expr::Name(n, line))
            }
            Tok::Kw("true") => {
                self.next();
                Ok(Expr::Bool(true))
            }
            Tok::Kw("false") => {
                self.next();
                Ok(Expr::Bool(false))
            }
            Tok::Kw("rhs") => {
                self.next();
                Ok(Expr::Rhs(line))
            }
            Tok::Kw(k) if k == "valof" || k == "refof" => {
                self.next();
                self.open_frame();
                let items = self.block()?;
                self.close_frame();
                Ok(Expr::Valof(items, k == "refof", line))
            }
            Tok::Sym("(") => {
                self.next();
                let e = self.definiens()?;
                self.expect_sym(")")?;
                Ok(e)
            }
            _ => self.err("an expression expected"),
        }
    }
}
