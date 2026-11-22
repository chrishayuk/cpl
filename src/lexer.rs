//! The hardware representation: publication CPL transliterated to ASCII.
//!
//! "Any practical implementation of CPL will probably accept an input which
//! differs to some extent from both publication and canonical CPL" (1.2.1).
//! This one differs as follows. Underlining is not available, so the basic
//! symbols are reserved lower case words; a run of lower case letters that
//! is not a reserved word is a sequence of one-letter names (`ax` is a times
//! x). Section brackets are `$(` and `$)` as in BCPL, optionally tagged, or
//! `{` and `}`.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Name(String),
    Num(String),
    Str(String),
    Bits(u64),
    Kw(&'static str),
    Sym(&'static str),
    Open,
    Close,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub line: u32,
    /// True when this token is the first on its line.
    pub nl: bool,
    /// Where the token is in the text, as byte offsets.
    pub start: usize,
    pub end: usize,
}

/// A mistake in the text of a program, and where it is.
#[derive(Clone, Debug)]
pub struct LexError {
    /// A short stable name for the kind of mistake.
    pub code: &'static str,
    /// As the system reports it, beginning with the line number.
    pub message: String,
    pub start: usize,
    pub end: usize,
}

/// Everything the lexer can say about a text, right or wrong.
pub struct Scan {
    /// The basic symbols, as the parser takes them.
    pub tokens: Vec<Token>,
    /// The same before two-word symbols are joined and synonyms reduced:
    /// one entry for each word as it was written.
    pub words: Vec<Token>,
    /// Where the comments are.
    pub comments: Vec<(usize, usize)>,
    /// The mistakes, in the order they are met.
    pub errors: Vec<LexError>,
    /// The opening section brackets that are never closed.
    pub unclosed: Vec<(usize, usize)>,
}

pub const WORDS: &[&str] = &[
    "let",
    "be",
    "is",
    "and",
    "in",
    "where",
    "rec",
    "recursive",
    "function",
    "fn",
    "routine",
    "rt",
    "value",
    "val",
    "reference",
    "ref",
    "subst",
    "fixed",
    "free",
    "variable",
    "var",
    "constant",
    "const",
    "load",
    "update",
    "fix",
    "rhs",
    "valof",
    "refof",
    "of",
    "resultis",
    "result",
    "if",
    "unless",
    "then",
    "do",
    "test",
    "or",
    "else",
    "otherwise",
    "while",
    "until",
    "repeat",
    "repeatwhile",
    "repeatuntil",
    "for",
    "ext",
    "external",
    "step",
    "to",
    "goto",
    "go",
    "break",
    "return",
    "finish",
    "prefer",
    "sic",
    "note",
    "switchon",
    "switch",
    "on",
    "caseswitch",
    "jumpon",
    "jump",
    "the",
    "rest",
    "others",
    "thendo",
    "ordo",
    "thru",
    "resultbe",
    "into",
    "case",
    "default",
    "true",
    "false",
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
    "all",
    "both",
    "allbe",
    "bothbe",
    "are",
];

/// Pairs of words that make one basic symbol ("spaces are ignored").
pub const COMBOS: &[(&str, &str, &str)] = &[
    ("go", "to", "goto"),
    ("value", "of", "valof"),
    ("val", "of", "valof"),
    ("reference", "of", "refof"),
    ("ref", "of", "refof"),
    ("result", "is", "resultis"),
    ("result", "be", "resultis"),
    ("repeat", "while", "repeatwhile"),
    ("repeat", "until", "repeatuntil"),
    ("all", "be", "allbe"),
    ("both", "be", "allbe"),
    ("then", "do", "do"),
    ("or", "do", "or"),
    ("long", "logical", "longlogical"),
    ("double", "complex", "doublecomplex"),
    ("switch", "on", "switchon"),
    ("jump", "on", "switchon"),
    ("the", "rest", "default"),
];

/// Synonyms, reduced to the form used in the syntax tables (3.1.3 d).
pub const SYNONYMS: &[(&str, &str)] = &[
    ("is", "be"),
    ("are", "allbe"),
    ("bothbe", "allbe"),
    ("both", "all"),
    ("fn", "function"),
    ("rt", "routine"),
    ("external", "ext"),
    ("val", "value"),
    ("ref", "reference"),
    ("rec", "recursive"),
    ("var", "variable"),
    ("const", "constant"),
    ("then", "do"),
    ("else", "or"),
    ("otherwise", "or"),
    ("caseswitch", "switchon"),
    ("jumpon", "switchon"),
    ("others", "default"),
    ("thendo", "do"),
    ("ordo", "or"),
    ("thru", "to"),
    ("resultbe", "resultis"),
];

const SYMS: &[&str] = &[
    "<=>", "~==", ":=", "->", "<=", ">=", "<<", ">>", "~=", "~~", "==", "**", "/\\", "\\/", "\\=", "+", "-", "*", "/",
    "=", "<", ">", "~", "(", ")", "[", "]", ",", ";", ":", "^",
];

const UNICODE: &[(char, &str)] = &[
    ('→', "->"),
    ('≡', "=="),
    ('≢', "~=="),
    ('≃', "~~"),
    ('∧', "/\\"),
    ('∨', "\\/"),
    ('¬', "~"),
    ('∼', "~"),
    ('≠', "~="),
    ('≤', "<="),
    ('≥', ">="),
    ('×', "*"),
    ('↑', "**"),
];

/// The marks of publication CPL: basic symbols are underlined, and the
/// closing section bracket is the section sign struck through.
pub const UNDERLINE: char = '\u{332}';
pub const STROKE: char = '\u{338}';
pub const SECTION: char = '\u{a7}';

fn word(w: &str) -> Option<&'static str> {
    WORDS.iter().find(|k| **k == w).copied()
}

/// The basic symbols of a program, or the first mistake in its text.
pub fn lex(src: &str) -> Result<Vec<Token>, String> {
    let scan = scan(src);
    match scan.errors.into_iter().next() {
        Some(e) => Err(e.message),
        None => Ok(scan.tokens),
    }
}

/// Reads the whole text whether or not it is right. After a mistake the
/// lexer notes it and carries on, so that an editor can still be told about
/// the rest of the program (see `analysis.rs`).
pub fn scan(src: &str) -> Scan {
    let cs: Vec<char> = src.chars().collect();
    // The byte offset of each character, and of the end of the text.
    let mut offs: Vec<usize> = src.char_indices().map(|(at, _)| at).collect();
    offs.push(src.len());
    let mut out: Vec<Token> = Vec::new();
    let mut comments: Vec<(usize, usize)> = Vec::new();
    let mut errors: Vec<LexError> = Vec::new();
    // The tag of each open section, and where its bracket is.
    let mut tags: Vec<(String, usize, usize)> = Vec::new();
    let (mut i, mut line, mut nl) = (0usize, 1u32, true);
    let tagch = |c: char| c.is_ascii_alphanumeric() || c == '.';
    while i < cs.len() {
        let c = cs[i];
        if c == '\n' {
            line += 1;
            nl = true;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '|' && cs.get(i + 1) == Some(&'|') {
            let st = i;
            while i < cs.len() && cs[i] != '\n' {
                i += 1;
            }
            comments.push((offs[st], offs[i]));
            continue;
        }
        // Where this symbol starts, and how many came before it.
        let (st0, n0) = (i, out.len());
        let mut push = |tok: Tok, nl: &mut bool| {
            out.push(Token {
                tok,
                line,
                nl: *nl,
                start: 0,
                end: 0,
            });
            *nl = false;
        };
        let mut fail = |code: &'static str, message: String, from: usize, to: usize| {
            errors.push(LexError {
                code,
                message: format!("line {}: {}", line, message),
                start: offs[from],
                end: offs[to.min(cs.len())],
            });
        };
        let primes = |i: &mut usize, s: &mut String| {
            while *i < cs.len() && cs[*i] == '\'' {
                s.push('\'');
                *i += 1;
            }
        };
        if c.is_ascii_alphabetic() && cs.get(i + 1) == Some(&UNDERLINE) {
            // Publication CPL: an underlined word is a basic symbol, and
            // "may be written in upper or lower case, or a mixture".
            let mut w = String::new();
            while i + 1 < cs.len() && cs[i].is_ascii_alphabetic() && cs[i + 1] == UNDERLINE {
                w.push(cs[i].to_ascii_lowercase());
                i += 2;
            }
            match word(&w) {
                Some(k) => push(Tok::Kw(k), &mut nl),
                None => fail(
                    "not-a-basic-symbol",
                    format!("the underlined word {} is not a basic symbol", w),
                    st0,
                    i,
                ),
            }
        } else if c.is_ascii_lowercase() {
            let st = i;
            while i < cs.len() && cs[i].is_ascii_lowercase() {
                i += 1;
            }
            let w: String = cs[st..i].iter().collect();
            if let Some(k) = word(&w) {
                push(Tok::Kw(k), &mut nl);
            } else {
                let n = w.len();
                for (k, ch) in w.chars().enumerate() {
                    let mut s = ch.to_string();
                    if k == n - 1 {
                        primes(&mut i, &mut s);
                    }
                    push(Tok::Name(s), &mut nl);
                }
                // Each letter is a name of its own, with its own place.
                for (k, t) in out[n0..].iter_mut().enumerate() {
                    t.start = offs[st + k];
                    t.end = if k == n - 1 { offs[i] } else { offs[st + k + 1] };
                }
            }
        } else if c.is_ascii_uppercase() {
            let st = i;
            while i < cs.len()
                && (cs[i].is_ascii_alphanumeric()
                    || (cs[i] == '.' && cs.get(i + 1).is_some_and(|d| d.is_ascii_alphanumeric())))
            {
                i += 1;
            }
            let mut s: String = cs[st..i].iter().collect();
            if s == "Boolean" {
                push(Tok::Kw("boolean"), &mut nl);
            } else {
                primes(&mut i, &mut s);
                push(Tok::Name(s), &mut nl);
            }
        } else if c.is_ascii_digit() || (c == '.' && cs.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let st = i;
            while i < cs.len() && cs[i].is_ascii_digit() {
                i += 1;
            }
            if i < cs.len() && cs[i] == '.' && cs.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                i += 1;
                while i < cs.len() && cs[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < cs.len() && cs[i] == '@' {
                let mut j = i + 1;
                if j < cs.len() && (cs[j] == '+' || cs[j] == '-') {
                    j += 1;
                }
                if j < cs.len() && cs[j].is_ascii_digit() {
                    while j < cs.len() && cs[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            push(Tok::Num(cs[st..i].iter().collect()), &mut nl);
        } else if c == '.' {
            let st = i;
            while i < cs.len() && (cs[i] == '.' || cs[i] == ' ') {
                i += 1;
            }
            if cs[st..i].iter().filter(|d| **d == '.').count() < 3 {
                fail(
                    "stray-dot",
                    "a stray dot (a dot string needs at least three dots)".into(),
                    st,
                    st + 1,
                );
            } else {
                push(Tok::Sym("..."), &mut nl);
            }
        } else if c == '\'' {
            let mut s = String::new();
            i += 1;
            loop {
                match cs.get(i) {
                    None | Some('\n') => {
                        fail("unclosed-string", "string constant has no closing quote".into(), st0, i);
                        break;
                    }
                    Some('\'') => {
                        i += 1;
                        break;
                    }
                    Some('*') => {
                        i += 1;
                        match cs.get(i).map(|e| e.to_ascii_lowercase()) {
                            Some('n') => s.push('\n'),
                            Some('s') => s.push(' '),
                            Some('t') => s.push('\t'),
                            Some('b') => s.push('\u{8}'),
                            Some('\'') => s.push('\''),
                            Some('*') => s.push('*'),
                            Some('|') => s.push('|'),
                            e => {
                                fail(
                                    "bad-escape",
                                    format!("unknown string escape *{}", e.unwrap_or(' ')),
                                    i - 1,
                                    i + 1,
                                );
                                if matches!(e, None | Some('\n')) {
                                    // The string ends here too, unclosed.
                                    continue;
                                }
                            }
                        }
                    }
                    Some(ch) => s.push(*ch),
                }
                i += 1;
            }
            push(Tok::Str(s), &mut nl);
        } else if c == '#' {
            let radix = match cs.get(i + 1) {
                Some('b') => 2,
                Some('o') => 8,
                Some('x') => 16,
                _ => {
                    fail(
                        "bad-logical",
                        "a logical constant is written #b..., #o... or #x...".into(),
                        i,
                        i + 1,
                    );
                    i += 1;
                    continue;
                }
            };
            i += 2;
            // "Positioning at the more significant end can be indicated by
            // a bar": #o|273 is #o27300000 in a logical of 24 bits.
            let barred = cs.get(i) == Some(&'|');
            if barred {
                i += 1;
            }
            let st = i;
            while i < cs.len() && cs[i].is_digit(radix) {
                i += 1;
            }
            let digits: String = cs[st..i].iter().collect();
            let mut bits = match u64::from_str_radix(&digits, radix) {
                Ok(bits) => bits,
                Err(_) => {
                    fail("bad-logical", "badly formed logical constant".into(), st0, i);
                    0
                }
            };
            if barred {
                let used = digits.len() as u32 * radix.trailing_zeros();
                if used > 24 {
                    fail(
                        "bad-logical",
                        "a barred logical constant has at most 24 bits".into(),
                        st0,
                        i,
                    );
                } else {
                    bits <<= 24 - used;
                }
            }
            push(Tok::Bits(bits), &mut nl);
        } else if c == '{'
            || (c == '$' && cs.get(i + 1) == Some(&'('))
            || (c == SECTION && cs.get(i + 1) != Some(&STROKE))
        {
            i += if c == '$' { 2 } else { 1 };
            let st = i;
            if c != '{' {
                while i < cs.len() && tagch(cs[i]) {
                    i += 1;
                }
            }
            tags.push((cs[st..i].iter().collect(), offs[st0], offs[i]));
            push(Tok::Open, &mut nl);
        } else if c == '}' || (c == '$' && cs.get(i + 1) == Some(&')')) || c == SECTION {
            i += if c == '}' { 1 } else { 2 };
            let st = i;
            if c != '}' {
                while i < cs.len() && tagch(cs[i]) {
                    i += 1;
                }
            }
            let tag: String = cs[st..i].iter().collect();
            if tag.is_empty() {
                if tags.pop().is_none() {
                    fail(
                        "unmatched-bracket",
                        "closing section bracket with nothing to close".into(),
                        st0,
                        i,
                    );
                } else {
                    push(Tok::Close, &mut nl);
                }
            } else {
                // A tagged closing bracket also closes every section opened
                // since its matching opening bracket (2.1.3).
                match tags.iter().rposition(|t| t.0 == tag) {
                    Some(at) => {
                        for _ in at..tags.len() {
                            push(Tok::Close, &mut nl);
                        }
                        tags.truncate(at);
                    }
                    None => fail(
                        "unknown-tag",
                        format!("no open section bracket is tagged {}", tag),
                        st0,
                        i,
                    ),
                }
            }
        } else if let Some((_, s)) = UNICODE.iter().find(|(u, _)| *u == c) {
            i += 1;
            push(Tok::Sym(SYMS.iter().find(|k| *k == s).unwrap()), &mut nl);
        } else {
            let rest: String = cs[i..(i + 3).min(cs.len())].iter().collect();
            match SYMS.iter().find(|s| rest.starts_with(**s)) {
                Some(s) => {
                    i += s.len();
                    let s = match *s {
                        "^" => "**",
                        "\\=" => "~=",
                        s => s,
                    };
                    push(Tok::Sym(s), &mut nl);
                }
                None => {
                    fail(
                        "bad-character",
                        format!("the character '{}' has no meaning here", c),
                        i,
                        i + 1,
                    );
                    i += 1;
                }
            }
        }
        for t in &mut out[n0..] {
            if t.end == 0 {
                (t.start, t.end) = (offs[st0], offs[i]);
            }
        }
    }
    let unclosed: Vec<(usize, usize)> = tags.iter().map(|t| (t.1, t.2)).collect();
    if let Some(&(start, end)) = unclosed.last() {
        errors.push(LexError {
            code: "unclosed-bracket",
            message: format!(
                "line {}: {} section bracket(s) left open at the end of the program",
                line,
                tags.len()
            ),
            start,
            end,
        });
    }
    // Carrying on after that mistake means closing them here.
    for _ in &unclosed {
        out.push(Token {
            tok: Tok::Close,
            line,
            nl: true,
            start: src.len(),
            end: src.len(),
        });
    }
    out.push(Token {
        tok: Tok::Eof,
        line,
        nl: true,
        start: src.len(),
        end: src.len(),
    });

    // Join two-word basic symbols, then reduce synonyms.
    let mut joined: Vec<Token> = Vec::with_capacity(out.len());
    let mut k = 0;
    while k < out.len() {
        let mut t = out[k].clone();
        if let (Tok::Kw(a), Some(Tok::Kw(b))) = (&t.tok, out.get(k + 1).map(|n| &n.tok)) {
            let same_line = !out[k + 1].nl;
            if let Some((_, _, to)) = COMBOS.iter().find(|(x, y, _)| x == a && y == b) {
                if same_line || !a.starts_with("repeat") {
                    t.tok = Tok::Kw(to);
                    t.end = out[k + 1].end;
                    k += 1;
                }
            }
        }
        if let Tok::Kw(a) = &t.tok {
            if let Some((_, to)) = SYNONYMS.iter().find(|(x, _)| x == a) {
                t.tok = Tok::Kw(to);
            }
        }
        joined.push(t);
        k += 1;
    }
    Scan {
        tokens: joined,
        words: out,
        comments,
        errors,
        unclosed,
    }
}

/// The program as it would be printed: basic symbols underlined, section
/// signs for the brackets, and the mathematical signs for the ASCII ones.
/// Strings, comments and names are left alone.
pub fn publication(src: &str) -> String {
    const SIGNS: &[(&str, &str)] = &[
        ("<=>", "<=>"),
        ("~==", "\u{2262}"),
        (":=", ":="),
        ("->", "\u{2192}"),
        ("<<", "<<"),
        (">>", ">>"),
        ("<=", "\u{2264}"),
        (">=", "\u{2265}"),
        ("~=", "\u{2260}"),
        ("~~", "\u{2243}"),
        ("==", "\u{2261}"),
        ("**", "\u{2191}"),
        ("/\\", "\u{2227}"),
        ("\\/", "\u{2228}"),
        ("$(", "\u{a7}"),
        ("$)", "\u{a7}\u{338}"),
        ("{", "\u{a7}"),
        ("}", "\u{a7}\u{338}"),
        ("*", "\u{d7}"),
        ("^", "\u{2191}"),
        ("~", "\u{223c}"),
    ];
    let cs: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        let rest: String = cs[i..(i + 3).min(cs.len())].iter().collect();
        if c == '|' && cs.get(i + 1) == Some(&'|') {
            while i < cs.len() && cs[i] != '\n' {
                out.push(cs[i]);
                i += 1;
            }
        } else if c == '\'' {
            // A string constant; a star takes the next character with it.
            out.push(c);
            i += 1;
            while i < cs.len() && cs[i] != '\'' && cs[i] != '\n' {
                if cs[i] == '*' && i + 1 < cs.len() {
                    out.push(cs[i]);
                    i += 1;
                }
                out.push(cs[i]);
                i += 1;
            }
            if i < cs.len() && cs[i] == '\'' {
                out.push('\'');
                i += 1;
            }
        } else if c.is_ascii_lowercase() {
            let st = i;
            while i < cs.len() && cs[i].is_ascii_lowercase() {
                i += 1;
            }
            let w: String = cs[st..i].iter().collect();
            for ch in w.chars() {
                out.push(ch);
                if word(&w).is_some() {
                    out.push(UNDERLINE);
                }
            }
            // Primes belong to the name, and are not string quotes.
            while i < cs.len() && cs[i] == '\'' && word(&w).is_none() {
                out.push('\'');
                i += 1;
            }
        } else if c.is_ascii_alphanumeric() || c == '#' {
            // A large name, a number or a logical constant.
            if c == '#' {
                out.push(c);
                i += 1;
            }
            while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '.' || cs[i] == '@' || cs[i] == '|') {
                if cs[i] == '|' && cs.get(i + 1) == Some(&'|') {
                    break;
                }
                out.push(cs[i]);
                i += 1;
            }
            if out.ends_with("Boolean") {
                // The one basic symbol written with a capital here.
                let at = out.len() - 7;
                let w: String = out[at..].chars().flat_map(|ch| [ch, UNDERLINE]).collect();
                out.truncate(at);
                out.push_str(&w);
            }
            while i < cs.len() && cs[i] == '\'' && c.is_ascii_uppercase() {
                out.push('\'');
                i += 1;
            }
        } else if let Some((from, to)) = SIGNS.iter().find(|(from, _)| rest.starts_with(from)) {
            out.push_str(to);
            i += from.chars().count();
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}
