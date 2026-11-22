//! Static analysis of CPL source text.
//!
//! The system finds a mistake by running into it. This module reads a
//! program without running it: it classifies every piece of the text, finds
//! where each name is defined and where it is used, and collects what the
//! lexer, the parser and the compiler have to say. `cpl --check` prints the
//! result, and the language server in `lsp/` builds its editor features on
//! the same tokens.
//!
//! Nothing here decides what CPL is. The tokens are the lexer's own
//! (`lexer::scan`), the definitions are noted by the parser as it goes
//! (`parser::parse_noting`), and the messages are the ones the system
//! gives when asked to run the program.
//!
//! All offsets are byte offsets into the source.

use crate::lexer::{self, Tok};
pub use crate::parser::NoteKind as DefKind;
use crate::{builtins, compile, parser};
use std::collections::BTreeMap;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `||` to the end of the line.
    Comment,
    /// A basic symbol written as a word: `let`, or underlined in publication CPL.
    Word,
    /// A small name (one letter, perhaps primed) or a large one (`Total`).
    Name,
    Number,
    /// A string constant, `'...'`.
    Str,
    /// A logical constant: `#o777`, `#b1010`, `#x1FF`, `#o|273`.
    Bits,
    /// An operator or a piece of punctuation.
    Sym,
    /// An opening section bracket, with its tag if it has one.
    Open,
    /// A closing section bracket, with its tag if it has one.
    Close,
    /// Text that means nothing in CPL.
    Unknown,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    /// For a word, the basic symbol it stands for once two-word symbols are
    /// joined and synonyms reduced: `then` is "do", and each half of
    /// `value of` is "valof". For an operator, its usual spelling: `^` is
    /// "**". Otherwise empty.
    pub symbol: &'static str,
    /// The other word of a two-word symbol, or the section bracket that
    /// matches this one.
    pub partner: Option<usize>,
    /// For a name, the definition it refers to.
    pub def: Option<usize>,
    /// True where the name is being defined.
    pub defining: bool,
}

impl Token {
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Hint => "hint",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub start: usize,
    pub end: usize,
    pub severity: Severity,
    /// A short stable name for the kind of problem.
    pub code: &'static str,
    pub message: String,
    /// The text is unused, and an editor may fade it.
    pub unnecessary: bool,
}

/// One place where a name is given a meaning.
#[derive(Clone, Debug)]
pub struct Definition {
    pub name: String,
    pub kind: DefKind,
    /// The token that is the name being defined.
    pub token: usize,
    /// The part of the text in which the name has this meaning.
    pub scope: Range<usize>,
    /// The whole definition: for a function, its heading and body.
    pub extent: Range<usize>,
    /// For a parameter, the function or routine it belongs to.
    pub owner: Option<usize>,
    /// For a function or routine, its parameters.
    pub params: Vec<usize>,
    /// Defined by a where-clause.
    pub by_where: bool,
    /// The tokens that refer to it, not counting the definition itself.
    pub uses: Vec<usize>,
}

pub struct Analysis {
    pub tokens: Vec<Token>,
    pub definitions: Vec<Definition>,
    pub diagnostics: Vec<Diagnostic>,
    /// True when the whole program could be parsed, so that every
    /// definition in it is known. When false, the definitions are those
    /// met before the first mistake of syntax.
    pub complete: bool,
}

/// Brackets nested more deeply than this are not followed: the parser
/// calls itself for each one, and an editor must not be brought down by a
/// thousand opening brackets.
const MAX_NESTING: usize = 200;

/// The basic symbols that name types.
const TYPE_WORDS: &[&str] = &[
    "real",
    "integer",
    "index",
    "complex",
    "double",
    "doublecomplex",
    "logical",
    "longlogical",
    "long",
    "boolean",
    "string",
    "label",
    "general",
    "type",
    "array",
    "vector",
    "matrix",
    "list",
];

/// Whether a basic symbol (in its reduced form) names a type.
pub fn is_type_word(symbol: &str) -> bool {
    TYPE_WORDS.contains(&symbol)
}

fn token(kind: Kind, start: usize, end: usize, symbol: &'static str) -> Token {
    Token {
        kind,
        start,
        end,
        symbol,
        partner: None,
        def: None,
        defining: false,
    }
}

/// The tag of a section bracket: `1.2` in `$(1.2` and in `§̸1.2`.
pub fn tag(text: &str) -> &str {
    let tagch = |c: char| c.is_ascii_alphanumeric() || c == '.';
    let cut = text.char_indices().rev().take_while(|(_, c)| tagch(*c)).last();
    cut.map_or("", |(at, _)| &text[at..])
}

/// Every piece of the text in order, comments and mistakes included.
fn classify(source: &str, scan: &lexer::Scan) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    for &(start, end) in &scan.comments {
        tokens.push(token(Kind::Comment, start, end, ""));
    }
    for w in &scan.words {
        // Not the end of the program, nor the brackets the lexer closes there.
        if w.start == w.end {
            continue;
        }
        let (kind, symbol) = match &w.tok {
            Tok::Name(_) => (Kind::Name, ""),
            Tok::Num(_) => (Kind::Number, ""),
            Tok::Str(_) => (Kind::Str, ""),
            Tok::Bits(_) => (Kind::Bits, ""),
            Tok::Kw(k) => (Kind::Word, *k),
            Tok::Sym(s) => (Kind::Sym, *s),
            Tok::Open => (Kind::Open, ""),
            Tok::Close => (Kind::Close, ""),
            Tok::Eof => continue,
        };
        // A tagged closing bracket may close several sections at once.
        if tokens.last().is_some_and(|t| t.start == w.start && t.kind == kind) {
            continue;
        }
        tokens.push(token(kind, w.start, w.end, symbol));
    }
    tokens.sort_by_key(|t| t.start);
    // Text the lexer could make nothing of.
    let mut unknown: Vec<Token> = Vec::new();
    for e in &scan.errors {
        let next = tokens.partition_point(|t| t.end <= e.start);
        let covered = tokens.get(next).is_some_and(|t| t.start < e.end);
        if !covered && e.end > e.start && unknown.last().is_none_or(|u| u.end <= e.start) {
            unknown.push(token(Kind::Unknown, e.start, e.end, ""));
        }
    }
    tokens.extend(unknown);
    tokens.sort_by_key(|t| t.start);

    // Words: what each stands for, and the two halves of two-word symbols.
    for joined in &scan.tokens {
        let Tok::Kw(k) = &joined.tok else { continue };
        let first = tokens.partition_point(|t| t.start < joined.start);
        let mut halves = Vec::new();
        for (i, t) in tokens.iter_mut().enumerate().skip(first) {
            if t.start >= joined.end {
                break;
            }
            if t.kind == Kind::Word {
                t.symbol = k;
                halves.push(i);
            }
        }
        if let [a, b] = halves[..] {
            tokens[a].partner = Some(b);
            tokens[b].partner = Some(a);
        }
    }

    // Section brackets: a tagged closing bracket matches the nearest open
    // one with the same tag, and closes those opened since (2.1.3).
    let mut open: Vec<usize> = Vec::new();
    for i in 0..tokens.len() {
        match tokens[i].kind {
            Kind::Open => open.push(i),
            Kind::Close => {
                let wanted = tag(&source[tokens[i].range()]);
                let at = if wanted.is_empty() {
                    open.len().checked_sub(1)
                } else {
                    open.iter().rposition(|&j| tag(&source[tokens[j].range()]) == wanted)
                };
                if let Some(at) = at {
                    for &j in &open[at..] {
                        tokens[j].partner = Some(i);
                    }
                    tokens[i].partner = Some(open[at]);
                    open.truncate(at);
                }
            }
            _ => {}
        }
    }
    tokens
}

/// A message without the line number the system puts in front of it.
fn split_line(message: &str) -> (Option<usize>, &str) {
    let Some(rest) = message.strip_prefix("line ") else {
        return (None, message);
    };
    match rest.split_once(": ") {
        Some((n, text)) => match n.parse() {
            Ok(n) => (Some(n), text),
            Err(_) => (None, message),
        },
        None => (None, message),
    }
}

struct Checked {
    notes: Vec<parser::Note>,
    /// What the parser objected to, and the token it had reached.
    syntax: Option<(String, usize)>,
    /// What the compiler objected to.
    meaning: Option<String>,
}

/// Parses and, if asked, compiles. Both follow the nesting of the program
/// by calling themselves, so this is done where the system itself does it.
fn check(tokens: Vec<lexer::Token>, compile_too: bool) -> Checked {
    crate::on_big_stack(move || {
        let parsed = parser::parse_noting(tokens);
        match parsed.items {
            Err(e) => Checked {
                notes: parsed.notes,
                syntax: Some((e, parsed.at)),
                meaning: None,
            },
            Ok(items) => Checked {
                notes: parsed.notes,
                syntax: None,
                meaning: if compile_too {
                    compile::compile(&items).err()
                } else {
                    None
                },
            },
        }
    })
}

/// Analyses a whole program.
pub fn analyse(source: &str) -> Analysis {
    let scan = lexer::scan(source);
    let mut tokens = classify(source, &scan);
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut report = |start: usize, end: usize, severity: Severity, code: &'static str, message: String| {
        diagnostics.push(Diagnostic {
            start,
            end,
            severity,
            code,
            message,
            unnecessary: false,
        });
    };

    // What the lexer found. The brackets left open are reported one by one,
    // each where it is, not in a single message at the end of the program.
    for e in scan.errors.iter().filter(|e| e.code != "unclosed-bracket") {
        report(
            e.start,
            e.end,
            Severity::Error,
            e.code,
            split_line(&e.message).1.to_string(),
        );
    }
    for &(start, end) in &scan.unclosed {
        let message = match tag(&source[start..end]) {
            "" => "this section bracket is never closed".to_string(),
            tag => format!("the section bracket tagged {} is never closed", tag),
        };
        report(start, end, Severity::Error, "unclosed-bracket", message);
    }
    let lexical = scan.errors.is_empty();

    // Where each of the parser's tokens is in the text.
    let spans: Vec<(usize, usize)> = scan.tokens.iter().map(|t| (t.start, t.end)).collect();
    let (mut depth, mut deepest) = (0usize, 0usize);
    for t in &scan.tokens {
        match &t.tok {
            Tok::Open | Tok::Sym("(") | Tok::Sym("[") => depth += 1,
            Tok::Close | Tok::Sym(")") | Tok::Sym("]") => depth = depth.saturating_sub(1),
            _ => {}
        }
        deepest = deepest.max(depth);
    }
    let mut complete = false;
    let mut notes = Vec::new();
    let mut meaning = None;
    if deepest > MAX_NESTING {
        let at = tokens.first().map_or(0..0, Token::range);
        let message = format!(
            "brackets are nested {} deep, which is too deep to check without running the program",
            deepest
        );
        report(at.start, at.end, Severity::Warning, "too-deep", message);
    } else {
        // After a mistake in the text the parser is still asked what is
        // defined, but what it then objects to would only be an echo.
        let checked = check(scan.tokens, lexical);
        complete = checked.syntax.is_none();
        notes = checked.notes;
        if let (Some((message, at)), true) = (&checked.syntax, lexical) {
            let (mut start, mut end) = spans[*at];
            if start == end {
                // The end of the program: point at the last thing in it.
                if let Some(last) = tokens.iter().rev().find(|t| t.kind != Kind::Comment) {
                    (start, end) = (last.start, last.end);
                }
            }
            report(start, end, Severity::Error, "syntax", split_line(message).1.to_string());
        }
        meaning = checked.meaning;
    }

    // Definitions, as the parser noted them.
    let index_of = |tokens: &[Token], at: usize| {
        let start = spans.get(at)?.0;
        let i = tokens.partition_point(|t| t.start < start);
        tokens
            .get(i)
            .filter(|t| t.start == start && t.kind == Kind::Name)
            .map(|_| i)
    };
    let offset = |at: usize| spans.get(at).map_or(source.len(), |s| s.0);
    let mut definitions: Vec<Definition> = Vec::new();
    let mut placed: Vec<Option<usize>> = Vec::new();
    for n in &notes {
        let Some(t) = index_of(&tokens, n.token) else {
            placed.push(None);
            continue;
        };
        let extent_end = n
            .extent
            .1
            .checked_sub(1)
            .and_then(|at| spans.get(at))
            .map_or(source.len(), |s| s.1);
        placed.push(Some(definitions.len()));
        definitions.push(Definition {
            name: source[tokens[t].range()].to_string(),
            kind: n.kind,
            token: t,
            scope: offset(n.scope.0)..offset(n.scope.1).max(offset(n.scope.0)),
            extent: offset(n.extent.0)..extent_end.max(offset(n.extent.0)),
            owner: None,
            params: Vec::new(),
            by_where: n.by_where,
            uses: Vec::new(),
        });
    }
    for (n, d) in notes.iter().zip(&placed) {
        if let (Some(owner), Some(d)) = (n.owner.and_then(|o| placed[o]), *d) {
            definitions[d].owner = Some(owner);
            definitions[owner].params.push(d);
        }
    }

    // Uses: each name means the innermost definition whose scope it is in.
    let mut by_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (d, def) in definitions.iter().enumerate() {
        by_name.entry(def.name.as_str()).or_default().push(d);
        tokens[def.token].def = Some(d);
        tokens[def.token].defining = true;
    }
    let mut uses: Vec<(usize, usize)> = Vec::new();
    for (i, t) in tokens.iter_mut().enumerate() {
        if t.kind != Kind::Name || t.defining {
            continue;
        }
        let found = by_name.get(&source[t.range()]).and_then(|candidates| {
            candidates
                .iter()
                .copied()
                .filter(|&d| definitions[d].scope.contains(&t.start))
                .max_by_key(|&d| (definitions[d].scope.start, d))
        });
        if let Some(d) = found {
            t.def = Some(d);
            uses.push((d, i));
        }
    }
    drop(by_name);
    for (d, i) in uses {
        definitions[d].uses.push(i);
    }

    // What the compiler objected to. It knows the line; the names on that
    // line, now that they are resolved, say where on it.
    if let Some(message) = &meaning {
        let (line, text) = split_line(message);
        let (start, end) = place(source, &tokens, line.unwrap_or(1), text);
        diagnostics.push(Diagnostic {
            start,
            end,
            severity: Severity::Error,
            code: if text.ends_with("is not defined here") {
                "undefined-name"
            } else {
                "meaning"
            },
            message: text.to_string(),
            unnecessary: false,
        });
    }

    // A definition nothing refers to. Only said of a program that is
    // otherwise in order, since a mistake hides the uses that follow it.
    if complete && lexical {
        for def in &definitions {
            let named = matches!(
                def.kind,
                DefKind::Variable | DefKind::Constant | DefKind::Function | DefKind::Routine
            );
            if named && def.uses.is_empty() {
                let t = &tokens[def.token];
                diagnostics.push(Diagnostic {
                    start: t.start,
                    end: t.end,
                    severity: Severity::Hint,
                    code: "unused-name",
                    message: format!("{} is defined but never used", def.name),
                    unnecessary: true,
                });
            }
        }
    }

    diagnostics.sort_by_key(|d| (d.start, d.end));
    Analysis {
        tokens,
        definitions,
        diagnostics,
        complete,
    }
}

/// Where to point for a message from the compiler, which knows only the
/// line: at the name the message is about if it can be found there, and
/// otherwise at the whole line.
fn place(source: &str, tokens: &[Token], line: usize, message: &str) -> (usize, usize) {
    let mut starts = vec![0];
    starts.extend(
        source
            .bytes()
            .enumerate()
            .filter(|&(_, b)| b == b'\n')
            .map(|(i, _)| i + 1),
    );
    let from = starts.get(line.saturating_sub(1)).copied().unwrap_or(source.len());
    let to = starts.get(line).map_or(source.len(), |&next| next - 1);
    let on_line = |t: &&Token| t.start >= from && t.end <= to && t.kind != Kind::Comment;

    let named = |t: &&Token| {
        let name = &source[t.range()];
        // "the name x is ..." for any name; a large name wherever it is mentioned.
        let quoted = message.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '\''));
        t.kind == Kind::Name
            && (message.starts_with(&format!("the name {} ", name))
                || (name.len() > 1 && quoted.into_iter().any(|w| w == name)))
    };
    let undefined = message.ends_with("is not defined here");
    let mut candidates = tokens.iter().filter(on_line).filter(named);
    let found = if undefined {
        candidates.find(|t| t.def.is_none())
    } else {
        // The name the message mentions first is the one it is about.
        let mut best: Option<(usize, &Token)> = None;
        for t in candidates {
            let at = message.find(&source[t.range()]).unwrap_or(usize::MAX);
            if best.is_none_or(|(least, _)| at < least) {
                best = Some((at, t));
            }
        }
        best.map(|(_, t)| t)
    };
    if let Some(t) = found {
        return (t.start, t.end);
    }
    let first = tokens.iter().find(on_line);
    let last = tokens.iter().rev().find(on_line);
    match (first, last) {
        (Some(first), Some(last)) => (first.start, last.end),
        _ => (from, to.max(from)),
    }
}

impl Analysis {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    /// The token under a cursor at `offset`. A cursor just after a token
    /// counts as on it when nothing else starts there.
    pub fn token_at(&self, offset: usize) -> Option<usize> {
        let i = self.tokens.partition_point(|t| t.end <= offset);
        match self.tokens.get(i) {
            Some(t) if t.start <= offset => Some(i),
            _ if i > 0 && self.tokens[i - 1].end == offset => Some(i - 1),
            _ => None,
        }
    }

    /// The definition a name token refers to, or is.
    pub fn definition_of(&self, token: usize) -> Option<usize> {
        self.tokens.get(token).and_then(|t| t.def)
    }

    /// The definitions a name written at `offset` could refer to: for each
    /// name the innermost one, in the order they are defined.
    pub fn visible_at(&self, offset: usize) -> Vec<usize> {
        let mut best: BTreeMap<&str, usize> = BTreeMap::new();
        for (d, def) in self.definitions.iter().enumerate() {
            if !def.scope.contains(&offset) && def.scope.end != offset {
                continue;
            }
            let entry = best.entry(def.name.as_str()).or_insert(d);
            if (def.scope.start, d) > (self.definitions[*entry].scope.start, *entry) {
                *entry = d;
            }
        }
        let mut out: Vec<usize> = best.into_values().collect();
        out.sort();
        out
    }

    /// The comment that describes a definition: the comment lines directly
    /// above the line it starts on.
    pub fn doc(&self, source: &str, definition: usize) -> Option<String> {
        let def = self.definitions.get(definition)?;
        let src = source.as_bytes();
        let line_of = |offset: usize| src[..offset].iter().filter(|&&b| b == b'\n').count();
        let line_start = src[..def.extent.start]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |p| p + 1);
        let first = self.tokens.partition_point(|t| t.start < line_start);
        let mut expect = line_of(def.extent.start);
        let mut lines = Vec::new();
        for t in self.tokens[..first].iter().rev() {
            if t.kind != Kind::Comment || expect == 0 || line_of(t.start) != expect - 1 {
                break;
            }
            // A comment after code on its line describes that code.
            let line_start = src[..t.start].iter().rposition(|&b| b == b'\n').map_or(0, |p| p + 1);
            if !source[line_start..t.start].trim().is_empty() {
                break;
            }
            lines.push(source[t.start + 2..t.end].trim().to_string());
            expect -= 1;
        }
        lines.reverse();
        Some(lines.join("\n")).filter(|s| !s.trim().is_empty())
    }
}

/// Converts a byte offset to a 1-based line and column, counting characters.
pub fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.bytes().filter(|&b| b == b'\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |p| p + 1);
    (line, before[start..].chars().count() + 1)
}

/// A basic function or routine: one of those in `builtins.rs`.
pub struct Builtin {
    pub name: &'static str,
    /// How it is called: `Rem[x, y]`. A constant has no brackets.
    pub signature: &'static str,
    pub doc: &'static str,
}

const fn b(name: &'static str, signature: &'static str, doc: &'static str) -> Builtin {
    Builtin { name, signature, doc }
}

/// The basic functions and routines, in the order of `builtins::TABLE`.
pub const BUILTINS: &[Builtin] = &[
    b("Sqrt", "Sqrt[x]", "The square root of x."),
    b("Exp", "Exp[x]", "e to the power x."),
    b("Log", "Log[x]", "The natural logarithm of x."),
    b("Sin", "Sin[x]", "The sine of x, with x in radians."),
    b("Cos", "Cos[x]", "The cosine of x, with x in radians."),
    b("Tan", "Tan[x]", "The tangent of x, with x in radians."),
    b("Arctan", "Arctan[x]", "The angle, in radians, whose tangent is x."),
    b("Arcsin", "Arcsin[x]", "The angle, in radians, whose sine is x."),
    b("Arccos", "Arccos[x]", "The angle, in radians, whose cosine is x."),
    b("Frpt", "Frpt[x]", "The fractional part of x: x less `Intpt[x]`."),
    b("Intpt", "Intpt[x]", "The integral part of x: the largest whole number not greater than x."),
    b("Round", "Round[x]", "The whole number nearest to x. A half goes away from zero."),
    b("Sign", "Sign[x]", "1, 0 or -1 as x is positive, zero or negative."),
    b("Mod", "Mod[x]", "The modulus of x: its size without its sign, or the modulus of a complex number."),
    b("Quot", "Quot[x, y]", "The quotient of x by y, with any fraction dropped."),
    b("Rem", "Rem[x, y]", "The remainder when x is divided by y: x less y times `Quot[x, y]`."),
    b("Realpt", "Realpt[z]", "The real part of z."),
    b("Imagpt", "Imagpt[z]", "The imaginary part of z, which is zero for a number that is not complex."),
    b("Cpx", "Cpx[x, y]", "The complex number with real part x and imaginary part y. Not in the papers: it stands in for the basic symbol i."),
    b("Index", "Index[x]", "The transfer function to type index: x as a small whole number."),
    b("Integer", "Integer[x]", "The transfer function to type integer."),
    b("Real", "Real[x]", "The transfer function to type real."),
    b("Double", "Double[x]", "The transfer function to type double."),
    b("Complex", "Complex[x]", "The transfer function to type complex."),
    b("DoubleComplex", "DoubleComplex[x]", "The transfer function to type double complex."),
    b("Logical", "Logical[x]", "The transfer function to type logical: 24 bits."),
    b("LongLogical", "LongLogical[x]", "The transfer function to type long logical: 48 bits."),
    b("String", "String[x]", "The transfer function to type string. A number gives the character with that number."),
    b("Boolean", "Boolean[x]", "The transfer function to type Boolean."),
    b("RValue", "RValue[x]", "The R-value of x: what the variable holds, not the variable."),
    b("LValue", "LValue[x]", "The L-value of x: the variable itself. If x is not a variable, a new one holding its value."),
    b("Type", "Type[x]", "The type that the data item x has at this moment. Not in the papers."),
    b("Lshift", "Lshift[p, j]", "The logical p shifted j places to the left. Zeros move in, and a negative j shifts to the right."),
    b("Rshift", "Rshift[p, j]", "The logical p shifted j places to the right. Zeros move in, and a negative j shifts to the left."),
    b("Rotate", "Rotate[p, j]", "The logical p moved j places to the left, the bits that leave at one end coming back at the other."),
    b("Ones", "Ones[j, k]", "A logical with ones in bits j to k and zeros elsewhere. Bits are numbered from 0 at the right hand end."),
    b("Longones", "Longones[j, k]", "A long logical with ones in bits j to k and zeros elsewhere."),
    b("Mask", "Mask[p, j, k]", "The logical p with every bit outside bits j to k made zero."),
    b("Field", "Field[p, j, k]", "Bits j to k of the logical p, moved to the right hand end. With a variable p it may stand on the left of an assignment."),
    b("Bit", "Bit[p, j]", "Bit j of the logical p. With a variable p it may stand on the left of an assignment."),
    b("Length", "Length[s]", "The number of characters in a string, or of members in a list."),
    b("First", "First[s]", "The first character of the string s, as a string."),
    b("Last", "Last[s]", "The last character of the string s, as a string."),
    b("Character", "Character[i, s]", "Character number i of the string s, counting from 1."),
    b("Initials", "Initials[n, s]", "The first n characters of the string s."),
    b("Finals", "Finals[n, s]", "The last n characters of the string s."),
    b("Newarray", "Newarray[T, l, u]", "A new array of type T with subscripts from l to u. A further pair of bounds gives a further dimension. Its elements have no values yet."),
    b("Array", "Array[T, l, u]", "As `Newarray`: the name used in 1963."),
    b("Formarray", "Formarray[T, l, u]", "A function that makes an array with these bounds from a list of values: `Formarray[real, 1, 3][1.5, 2.5, 3.5]`."),
    b("Vector", "Vector[T, l, u]", "As `Formarray`: a name accepted by the pre-pilot system of 1966."),
    b("Matrix", "Matrix[T, l, u, l', u']", "As `Formarray`: a name accepted by the pre-pilot system of 1966."),
    b("Copy", "Copy[a]", "A new array with the same elements as a."),
    b("Fix", "Fix[a]", "A copy of the array a whose elements cannot be altered."),
    b("Bounds", "Bounds[a]", "The lower and upper subscript bounds of the array a, as two results."),
    b("Equal", "Equal[a, b]", "True when the arrays a and b are equal."),
    b("NIL", "NIL", "The empty list."),
    b("Nil", "Nil", "The empty list."),
    b("Cons", "Cons[x, l]", "The list whose head is x and whose tail is the list l."),
    b("List", "List[x, y]", "The list of its arguments, however many there are."),
    b("List1", "List1[x]", "The list with the one member x."),
    b("Hd", "Hd[l]", "The head of the list l: its first member. It may stand on the left of an assignment."),
    b("Tl", "Tl[l]", "The tail of the list l: the list of all but its first member. It may stand on the left of an assignment."),
    b("Null", "Null[l]", "True when l is the empty list."),
    b("Atom", "Atom[x]", "True when x is not a list."),
    b("Members", "Members[l]", "The members of the list l as separate results, to be matched against a list of names."),
    b("Append", "Append[l, m]", "The list of the members of l followed by those of m."),
    b("Write", "Write[x, y]", "Writes each argument on the output stream. `'*n'` is a new line."),
    b("Newline", "Newline[]", "Writes a new line."),
    b("Space", "Space[n]", "Writes n spaces."),
    b("Read", "Read[x, y]", "Reads a number from the input stream into each variable. Numbers are separated by spaces, commas or new lines."),
    b("Readchar", "Readchar[x]", "Reads the next character of the input into the string variable x, which is left empty at the end of the input. `Readchar[stream, x]` is accepted too; there is one stream."),
    b("Writechar", "Writechar[x]", "Writes the string x, or the character whose number is x."),
    b("Printf", "Printf[x, i]", "Writes x in floating decimal style with i significant figures, in i + 7 character positions."),
    b("Printd", "Printd[x, i, j]", "Writes x with i figures before the decimal point and j after it."),
    b("WRITE", "WRITE[x, y]", "As `Write`, spelt as in the supplement for the pre-pilot system."),
    b("READ", "READ[x, y]", "As `Read`, spelt as in the supplement for the pre-pilot system."),
    b("READCHAR", "READCHAR[x]", "As `Readchar`, spelt as in the supplement for the pre-pilot system."),
    b("WRITECHAR", "WRITECHAR[x]", "As `Writechar`, spelt as in the supplement for the pre-pilot system."),
    b("PRINTF", "PRINTF[x, i]", "As `Printf`, spelt as in the supplement for the pre-pilot system."),
    b("PRINTD", "PRINTD[x, i, j]", "As `Printd`, spelt as in the supplement for the pre-pilot system."),
];

/// The description of a basic function or routine, if `name` is one.
pub fn builtin(name: &str) -> Option<&'static Builtin> {
    builtins::lookup(name).and_then(|_| BUILTINS.iter().find(|b| b.name == name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(src: &str) -> Vec<&'static str> {
        analyse(src).diagnostics.iter().map(|d| d.code).collect()
    }

    fn kinds(src: &str) -> Vec<Kind> {
        analyse(src).tokens.iter().map(|t| t.kind).collect()
    }

    #[test]
    fn every_basic_function_is_described() {
        let described: Vec<&str> = BUILTINS.iter().map(|b| b.name).collect();
        let defined: Vec<&str> = builtins::TABLE.iter().map(|(name, _)| *name).collect();
        assert_eq!(described, defined);
        for b in BUILTINS {
            assert!(b.signature.starts_with(b.name) && b.doc.ends_with('.'), "{}", b.name);
        }
        assert!(builtin("Sqrt").is_some() && builtin("Total").is_none());
    }

    #[test]
    fn small_names_large_names_and_words() {
        // "ax" is a times x; "let" is a basic symbol; "Total" is one name.
        let a = analyse("let a, x, Total = 1, 2, 3\nWrite[ax + Total']");
        let names: Vec<&str> = a.tokens.iter().filter(|t| t.kind == Kind::Name).map(|_| "n").collect();
        assert_eq!(names.len(), 7);
        assert_eq!(a.tokens[0].kind, Kind::Word);
        assert_eq!(a.tokens[0].symbol, "let");
        // A prime belongs to the name before it.
        assert_eq!(kinds("a' := x''"), [Kind::Name, Kind::Sym, Kind::Name]);
    }

    #[test]
    fn two_word_symbols_and_synonyms_are_reduced() {
        let src = "test x then go to L or result is 1";
        let a = analyse(src);
        let words: Vec<(&str, &str)> = a
            .tokens
            .iter()
            .filter(|t| t.kind == Kind::Word)
            .map(|t| (&src[t.range()], t.symbol))
            .collect();
        assert_eq!(
            words,
            [
                ("test", "test"),
                ("then", "do"),
                ("go", "goto"),
                ("to", "goto"),
                ("or", "or"),
                ("result", "resultis"),
                ("is", "resultis")
            ]
        );
        let go = a.tokens.iter().position(|t| &src[t.range()] == "go").unwrap();
        assert_eq!(a.tokens[go].partner, Some(go + 1));
        assert_eq!(a.tokens[go + 1].partner, Some(go));
    }

    #[test]
    fn comments_strings_and_constants() {
        let src = "Write['it*'s || not a comment', #o|273, 2.5@-8] || a comment";
        assert_eq!(
            kinds(src),
            [
                Kind::Name,
                Kind::Sym,
                Kind::Str,
                Kind::Sym,
                Kind::Bits,
                Kind::Sym,
                Kind::Number,
                Kind::Sym,
                Kind::Comment
            ]
        );
        assert!(codes(src).is_empty());
    }

    #[test]
    fn publication_cpl_is_read_as_it_is_printed() {
        let src = "l\u{332}e\u{332}t\u{332} x = 1\n\u{a7}1 Write[x] \u{a7}\u{338}1";
        let a = analyse(src);
        assert!(a.diagnostics.is_empty(), "{:?}", a.diagnostics);
        assert_eq!(a.tokens[0].kind, Kind::Word);
        assert_eq!(&src[a.tokens[0].range()], "l\u{332}e\u{332}t\u{332}");
        let open = a.tokens.iter().position(|t| t.kind == Kind::Open).unwrap();
        let close = a.tokens.iter().position(|t| t.kind == Kind::Close).unwrap();
        assert_eq!(a.tokens[open].partner, Some(close));
        assert_eq!(tag(&src[a.tokens[close].range()]), "1");
    }

    #[test]
    fn a_tagged_bracket_closes_the_sections_inside_it() {
        let src = "$(T $( $(Row Write[1] $)T";
        let a = analyse(src);
        assert!(a.diagnostics.is_empty(), "{:?}", a.diagnostics);
        let close = a.tokens.len() - 1;
        let opens: Vec<Option<usize>> = a
            .tokens
            .iter()
            .filter(|t| t.kind == Kind::Open)
            .map(|t| t.partner)
            .collect();
        assert_eq!(opens, [Some(close), Some(close), Some(close)]);
        assert_eq!(a.tokens[close].partner, Some(0));
    }

    #[test]
    fn mistakes_in_the_text_are_each_reported_where_they_are() {
        assert_eq!(codes("Write['abc]"), ["unclosed-string"]);
        assert_eq!(codes("Write['a*q']"), ["bad-escape"]);
        assert_eq!(codes("x := #z"), ["bad-logical"]);
        assert_eq!(codes("x := 1 ? 2"), ["bad-character"]);
        assert_eq!(codes("x := 1 $)"), ["unmatched-bracket"]);
        assert_eq!(codes("$(A x := 1 $)B"), ["unclosed-bracket", "unknown-tag"]);
        assert_eq!(codes("x := 1 . 2"), ["stray-dot"]);
        assert_eq!(codes("x\u{332}y\u{332} := 1"), ["not-a-basic-symbol"]);

        let src = "let x = 1\n$(A if x = 1 do\n   $( Write[x]\n";
        let a = analyse(src);
        let at: Vec<(usize, usize)> = a.diagnostics.iter().map(|d| line_column(src, d.start)).collect();
        assert_eq!(at, [(2, 1), (3, 4)]);
        assert_eq!(a.diagnostics[0].message, "the section bracket tagged A is never closed");
        assert_eq!(a.diagnostics[1].message, "this section bracket is never closed");
    }

    #[test]
    fn a_mistake_of_syntax_points_at_the_symbol_that_is_wrong() {
        let src = "let count = 0";
        let a = analyse(src);
        assert_eq!(a.diagnostics.len(), 1);
        let d = &a.diagnostics[0];
        assert_eq!(d.code, "syntax");
        assert_eq!(
            d.message,
            "'=', '~~', '==' or 'be' expected in a definition (found the name o)"
        );
        assert_eq!(&src[d.start..d.end], "o");
        assert_eq!(line_column(src, d.start), (1, 6));
        assert!(!a.complete);

        // At the end of the program there is nothing to point at but the last symbol.
        let src = "let x = ";
        let a = analyse(src);
        assert_eq!(&src[a.diagnostics[0].start..a.diagnostics[0].end], "=");
    }

    #[test]
    fn the_compiler_is_asked_too() {
        let src = "let f[x] = x = 0 -> 1, x f[x - 1]\nWrite[f[3]]";
        let a = analyse(src);
        assert_eq!(a.diagnostics.len(), 1);
        let d = &a.diagnostics[0];
        assert_eq!(
            (d.code, d.message.as_str()),
            ("undefined-name", "the name f is not defined here")
        );
        // The f inside its own definition, which is not recursive.
        assert_eq!(line_column(src, d.start), (1, 26));

        let src = "let Total = 0\nlet Add[x] = value of $( Total := Total + x; result is Total $)\nWrite[Add[1]]";
        let a = analyse(src);
        assert_eq!(a.diagnostics[0].code, "meaning");
        assert_eq!(&src[a.diagnostics[0].start..a.diagnostics[0].end], "Total");
        assert_eq!(line_column(src, a.diagnostics[0].start), (2, 26));
    }

    #[test]
    fn definitions_and_their_scopes() {
        let src = "\
let a = 1
let rec Fact[integer n] = n = 0 -> 1, n Fact[n - 1]
let routine Show[x] be $( let a = 2; Write[a, x] $)
for i = 1 to 3 do Show[Fact[i] + a]
Write[d] where d = a
L: finish";
        let a = analyse(src);
        assert!(a.diagnostics.is_empty(), "{:?}", a.diagnostics);
        assert!(a.complete);
        let described: Vec<(&str, DefKind, usize)> = a
            .definitions
            .iter()
            .map(|d| (d.name.as_str(), d.kind, d.uses.len()))
            .collect();
        assert_eq!(
            described,
            [
                ("a", DefKind::Variable, 2),
                ("Fact", DefKind::Function, 2),
                ("n", DefKind::Parameter, 3),
                ("Show", DefKind::Routine, 1),
                ("x", DefKind::Parameter, 1),
                ("a", DefKind::Variable, 1),
                ("i", DefKind::Loop, 1),
                ("d", DefKind::Variable, 1),
                ("L", DefKind::Label, 0),
            ]
        );
        let fact = &a.definitions[1];
        assert_eq!(fact.params, [2]);
        assert_eq!(a.definitions[2].owner, Some(1));
        assert_eq!(&src[fact.extent.clone()], "Fact[integer n] = n = 0 -> 1, n Fact[n - 1]");
        assert!(a.definitions[7].by_where);
        // The a in "Write[a, x]" is the one defined inside Show.
        let inner = src.find("Write[a, x]").unwrap() + 6;
        assert_eq!(a.definition_of(a.token_at(inner).unwrap()), Some(5));
        // Inside Show both are in scope, and the inner one is the one seen. Show
        // itself is not: it is not recursive.
        let seen: Vec<&str> = a
            .visible_at(inner)
            .iter()
            .map(|&d| a.definitions[d].name.as_str())
            .collect();
        assert_eq!(seen, ["Fact", "x", "a", "L"]);
        assert!(a.visible_at(inner).contains(&5));
    }

    #[test]
    fn the_scope_rules_for_in_and_for_labels() {
        // "D1 in D2": the names of D1 are known only in D2.
        let src = "let c = 0 in Next[] == value of $( c := c + 1; result is c $)\nWrite[Next[], c]";
        let a = analyse(src);
        assert_eq!(codes(src), ["undefined-name"]);
        assert_eq!(a.definitions[0].uses.len(), 3);
        // A label is known before it is set.
        let a = analyse("go to End\nWrite['skipped']\nEnd: finish");
        assert!(a.diagnostics.is_empty());
        assert_eq!(a.definitions[0].uses.len(), 1);
    }

    #[test]
    fn what_is_defined_before_a_mistake_is_still_known() {
        let src = "let Total = 0\nlet Add[x] be\nTotal := ";
        let a = analyse(src);
        assert!(!a.complete);
        let names: Vec<&str> = a.definitions.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["Total", "Add", "x"]);
        // And with a bracket still open, as it is while a block is being typed.
        let src = "let Total = 0\nlet routine Add[x] be\n$( Total := Total + x\n";
        let a = analyse(src);
        assert_eq!(codes(src), ["unclosed-bracket"]);
        assert_eq!(a.definitions[0].uses.len(), 2);
        assert_eq!(a.definitions[2].uses.len(), 1);
    }

    #[test]
    fn a_name_never_used_is_a_hint() {
        let a = analyse("let x, y = 1, 2\nWrite[x]");
        assert_eq!(a.diagnostics.len(), 1);
        let d = &a.diagnostics[0];
        assert_eq!(
            (d.severity, d.code, d.unnecessary),
            (Severity::Hint, "unused-name", true)
        );
        assert_eq!(d.message, "y is defined but never used");
        assert!(!a.has_errors());
    }

    #[test]
    fn a_definition_is_described_by_the_comment_above_it() {
        let src = "let a = 1 || not this\n|| The highest common factor,\n|| by Euclid.\nlet rec HCF[n, m] = m = 0 -> n, HCF[m, Rem[n, m]]\nWrite[HCF[a, 2]]";
        let a = analyse(src);
        let hcf = a.definitions.iter().position(|d| d.name == "HCF").unwrap();
        assert_eq!(
            a.doc(src, hcf).as_deref(),
            Some("The highest common factor,\nby Euclid.")
        );
        assert_eq!(a.doc(src, 0), None);
    }

    #[test]
    fn token_at_finds_the_token_under_or_just_before_the_cursor() {
        let a = analyse("Write[x]  || done");
        assert_eq!(a.tokens[a.token_at(0).unwrap()].kind, Kind::Name);
        assert_eq!(a.token_at(5), Some(1));
        assert_eq!(a.token_at(8), Some(3));
        assert_eq!(a.token_at(9), None);
        assert_eq!(line_column("ab\ncd", 4), (2, 2));
    }

    #[test]
    fn positions_survive_text_outside_ascii() {
        let src = "Write['caf\u{e9} \u{2014} ok'] ? 1";
        let a = analyse(src);
        assert_eq!(a.diagnostics.len(), 1);
        assert_eq!(&src[a.diagnostics[0].start..a.diagnostics[0].end], "?");
        assert_eq!(line_column(src, a.diagnostics[0].start), (1, 20));
    }

    #[test]
    fn very_deep_nesting_is_not_followed() {
        let src = format!("x := {}1{}", "(".repeat(300), ")".repeat(300));
        assert_eq!(codes(&src), ["too-deep"]);
    }
}
