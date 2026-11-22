//! A language server for CPL.
//!
//! It speaks the Language Server Protocol over standard input and output, so
//! any editor with an LSP client can use it. Everything it knows about a
//! program comes from `src/analysis.rs`, the same code behind `cpl --check`,
//! which in turn asks the system's own lexer, parser and compiler.
//!
//! The server reads one message at a time and keeps nothing but the open
//! documents, which lets it also be compiled for WASI and run inside an
//! editor or a web page with nothing to install (see `editors/vscode` and
//! `web/lsp-worker.js`).

use cpl::analysis::{self, analyse, Analysis, DefKind, Definition, Kind, Severity, Token};
use cpl::lexer;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};

const SEMANTIC_TYPES: &[&str] = &[
    "comment",
    "string",
    "number",
    "variable",
    "function",
    "parameter",
    "operator",
    "keyword",
    "type",
    "label",
];
const SEMANTIC_MODIFIERS: &[&str] = &["definition", "defaultLibrary", "readonly"];

struct Doc {
    text: String,
    analysis: Analysis,
    line_starts: Vec<usize>,
}

impl Doc {
    fn new(text: String) -> Self {
        let analysis = analyse(&text);
        let mut line_starts = vec![0];
        line_starts.extend(
            text.bytes()
                .enumerate()
                .filter(|&(_, b)| b == b'\n')
                .map(|(i, _)| i + 1),
        );
        Doc {
            text,
            analysis,
            line_starts,
        }
    }

    fn line_of(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|&start| start <= offset) - 1
    }

    /// A byte offset as an LSP position: a line and a count of UTF-16 units.
    fn position(&self, offset: usize) -> Value {
        let offset = offset.min(self.text.len());
        let line = self.line_of(offset);
        let character: usize = self.text[self.line_starts[line]..offset]
            .chars()
            .map(char::len_utf16)
            .sum();
        json!({ "line": line, "character": character })
    }

    fn range(&self, start: usize, end: usize) -> Value {
        json!({ "start": self.position(start), "end": self.position(end) })
    }

    fn token_range(&self, t: &Token) -> Value {
        self.range(t.start, t.end)
    }

    fn offset(&self, position: &Value) -> usize {
        let line = position["line"].as_u64().unwrap_or(0) as usize;
        let mut units = position["character"].as_u64().unwrap_or(0) as usize;
        let Some(&start) = self.line_starts.get(line) else {
            return self.text.len();
        };
        let end = self.line_starts.get(line + 1).map_or(self.text.len(), |&next| next - 1);
        let mut offset = start;
        for c in self.text[start..end].chars() {
            if units < c.len_utf16() {
                break;
            }
            units -= c.len_utf16();
            offset += c.len_utf8();
        }
        offset
    }

    fn line_text(&self, offset: usize) -> &str {
        let line = self.line_of(offset);
        let end = self.line_starts.get(line + 1).map_or(self.text.len(), |&next| next - 1);
        self.text[self.line_starts[line]..end].trim_end_matches('\r')
    }

    fn token_at(&self, position: &Value) -> Option<(usize, &Token)> {
        let i = self.analysis.token_at(self.offset(position))?;
        Some((i, &self.analysis.tokens[i]))
    }

    fn text_of(&self, t: &Token) -> &str {
        &self.text[t.range()]
    }

    /// The definition a name token refers to.
    fn definition(&self, t: &Token) -> Option<(usize, &Definition)> {
        t.def.map(|d| (d, &self.analysis.definitions[d]))
    }
}

fn times(n: usize) -> String {
    match n {
        0 => "nowhere".into(),
        1 => "once".into(),
        2 => "twice".into(),
        n => format!("{n} times"),
    }
}

fn kind_name(kind: DefKind) -> &'static str {
    match kind {
        DefKind::Variable => "Variable",
        DefKind::Constant => "Constant",
        DefKind::Function => "Function",
        DefKind::Routine => "Routine",
        DefKind::Parameter => "Parameter",
        DefKind::Label => "Label",
        DefKind::Loop => "Controlled variable",
    }
}

/// How a basic symbol is usually written, where that differs from the form
/// the syntax tables use.
fn spelling(symbol: &str) -> &str {
    match symbol {
        "valof" => "value of",
        "refof" => "reference of",
        "resultis" => "result is",
        "goto" => "go to",
        "allbe" => "all be",
        "switchon" => "switch on",
        "repeatwhile" => "repeat while",
        "repeatuntil" => "repeat until",
        "doublecomplex" => "double complex",
        "longlogical" => "long logical",
        "recursive" => "rec",
        "boolean" => "Boolean",
        s => s,
    }
}

/// What a basic symbol is for: a heading and a paragraph. The symbol is in
/// the reduced form the lexer gives it, so `then` arrives here as `do`.
fn word_help(symbol: &str) -> (&'static str, &'static str) {
    match symbol {
        "let" => ("definition", "Begins a definition: `let x = 1`, `let F[x] = 2x`, `let x be real`. The name is known from the end of the definition to the end of the block it is in."),
        "be" => ("definition", "`let x be real` gives x a type and no value. `routine R[x] be C` gives a routine its body. `is` means the same."),
        "allbe" => ("definition", "`let x, y, z all be real` gives several names the same type. `are` and `both be` mean the same."),
        "all" => ("one value for several names", "`a, b, c all := 0` assigns one value to several variables, and `let a, b all = E` defines several names by one expression. `both` means the same."),
        "and" => ("definitions in parallel", "Joins definitions that are made together: none is in the scope of the others, unless the group is marked `rec`."),
        "in" => ("private definition", "`let D1 in D2`: the names that D1 defines are known only inside D2."),
        "where" => ("where-clause", "`C where D` or `(E where D)`: the names that D defines are known only in the command or expression before it."),
        "recursive" => ("recursive definition", "`let rec F[n] = ...`: the function may use its own name in its definition. Without `rec` it may not."),
        "function" => ("function", "`let function F[P] be load B update B` defines an LH function, which may stand on the left of an assignment. After a type, as in `real function`, it is the type of a function. `fn` means the same."),
        "routine" => ("routine", "`routine R[P] be C` defines a routine: a command with parameters, called by writing `R[E, ...]`. It is also the type of such a value. `rt` means the same."),
        "value" => ("parameter by value", "The parameters after it are called by value: the function is given a copy. This is the mode when none is written. `val` means the same."),
        "reference" => ("parameter by reference", "The parameters after it are called by reference: the function is given the variable itself and may assign to it. `ref` means the same."),
        "subst" => ("parameter by substitution", "The parameters after it are called by substitution: the expression is passed as it stands and worked out each time the parameter is used. In CPL in 1963 and withdrawn by 1966."),
        "fixed" => ("fixed function", "Marks a function or routine as fixed: it takes the values of its free variables when it is defined, and may not assign to them."),
        "free" => ("free function", "Marks a function or routine as free: it shares its free variables with the program around it."),
        "variable" => ("variable", "Marks what is defined as a variable, which may be assigned to. Simple definitions are variables unless marked `constant`; functions and routines are constants unless marked `variable`. `var` means the same."),
        "constant" => ("constant", "Marks what is defined as a constant, which may not be assigned to. `const` means the same."),
        "load" => ("LH function", "The part of an LH function that gives its value: a block that ends with `result is`."),
        "update" => ("LH function", "The part of an LH function that is obeyed when the function is assigned to. Inside it, `rhs` is the value being assigned."),
        "fix" => ("LH function", "`fix D` in an LH function: definitions made when the function is applied, and known to both its `load` and its `update` part."),
        "rhs" => ("LH function", "In the `update` part of an LH function, the value being assigned."),
        "valof" => ("result expression", "`value of B` makes an expression of a block. The block gives its value with `result is`. `valof` means the same."),
        "refof" => ("result expression", "`reference of B` is like `value of B`, but the result is a variable (an L-value), not what the variable holds."),
        "resultis" => ("result", "`result is E` leaves the enclosing `value of` block, giving it the value of E."),
        "if" => ("conditional command", "`if B do C` obeys C when B is true. `then` may be written for `do`."),
        "unless" => ("conditional command", "`unless B do C` obeys C when B is false."),
        "do" => ("do", "Stands between a condition or a for-list and the command it governs. `then` means the same."),
        "test" => ("conditional command", "`test B then C1 or C2` obeys C1 when B is true and C2 otherwise."),
        "or" => ("conditional command", "Begins the second command of `test B then C1 or C2`. `else` and `otherwise` mean the same."),
        "while" => ("cycle", "`while B do C` obeys C again and again for as long as B is true."),
        "until" => ("cycle", "`until B do C` obeys C again and again for as long as B is false."),
        "repeat" => ("cycle", "`C repeat` obeys C again and again, until a `break`, a `return` or a jump leaves it."),
        "repeatwhile" => ("cycle", "`C repeat while B` obeys C once, and then again for as long as B is true."),
        "repeatuntil" => ("cycle", "`C repeat until B` obeys C once, and then again until B is true."),
        "for" => ("cycle", "`for v = list do C` obeys C once for each value in the list. The list may give values (`1, 5, 9`), a range (`1 to 10`), a start, a step and a limit (`step 1, 2, 9`), or a dot string (`1, 3, ..., 9`). The variable v is a new one, known only in C."),
        "ext" => ("cycle", "`for ext v = list do C` uses the existing variable v as the controlled variable, not a new one. `external` means the same."),
        "step" => ("cycle", "In a for-list, `step a, b, c` goes from a in steps of b as far as c."),
        "to" => ("cycle", "In a for-list, `a to b` goes from a to b in steps of one. `thru` means the same."),
        "goto" => ("jump", "`go to E` jumps to the label that E gives. `goto` means the same."),
        "break" => ("jump", "Leaves the smallest enclosing cycle."),
        "return" => ("jump", "Leaves the routine being obeyed."),
        "finish" => ("jump", "Ends the program."),
        "prefer" => ("preferred type", "`prefer T` sets the type that is assumed in this block wherever none is written."),
        "sic" => ("sic", "Asks that expressions be evaluated as written. It is accepted and ignored: nothing is ever rearranged here."),
        "note" => ("note", "`note 'text'` is a comment in the form of a command. It is accepted and ignored."),
        "switchon" => ("switch", "`switch on E into $( case K: ... default: ... $)` jumps to the case equal to E. Control then runs on through the cases after it, as it still does in C. `jump on` and `caseswitch` mean the same."),
        "into" => ("switch", "Stands between the expression of a `switch on` and its block of cases."),
        "case" => ("switch", "`case K:` marks the place that a `switch on` jumps to when its expression equals K."),
        "default" => ("switch", "`default:` marks the place that a `switch on` jumps to when no case matches. `the rest` and `others` mean the same."),
        "true" => ("Boolean constant", "The truth value true."),
        "false" => ("Boolean constant", "The truth value false."),
        "real" => ("type", "The type of numbers with a fractional part."),
        "integer" => ("type", "The type of whole numbers: 64 bits here."),
        "index" => ("type", "The type of small whole numbers, as used for subscripts: 32 bits here."),
        "complex" => ("type", "The type of complex numbers."),
        "double" => ("type", "The type of double length real numbers. Here it is no more precise than `real`."),
        "doublecomplex" => ("type", "The type of double length complex numbers."),
        "logical" => ("type", "The type of a string of 24 bits."),
        "longlogical" => ("type", "The type of a string of 48 bits."),
        "boolean" => ("type", "The type of `true` and `false`. Followed by `[`, it is the transfer function to that type."),
        "string" => ("type", "The type of a string of characters."),
        "label" => ("type", "The type of a label: a place in the program that `go to` can jump to."),
        "general" => ("type", "The type of a variable that can hold a data item of any type, which carries its type with it."),
        "type" => ("type", "The type of a type. `Type[x]` gives the type of x."),
        "array" => ("type", "`T n array` is the type of an array of n dimensions with elements of type T."),
        "vector" => ("type", "`T vector` is the type of an array of one dimension with elements of type T."),
        "matrix" => ("type", "`T matrix` is the type of an array of two dimensions with elements of type T."),
        "list" => ("type", "The type of a list, from the CPL of 1963."),
        "of" => ("half a symbol", "The second word of `value of` and `reference of`. On its own it means nothing."),
        "result" => ("half a symbol", "The first word of `result is`. On its own it means nothing."),
        "go" => ("half a symbol", "The first word of `go to`. On its own it means nothing."),
        "switch" | "jump" | "on" => ("half a symbol", "One word of `switch on` or `jump on`. On its own it means nothing."),
        "the" | "rest" => ("half a symbol", "One word of `the rest`, which means `default`. On its own it means nothing."),
        "long" => ("half a symbol", "The first word of `long logical`. On its own it means nothing."),
        _ => ("basic symbol", "One of the basic symbols of CPL."),
    }
}

/// What an operator or a piece of punctuation is for.
fn symbol_help(symbol: &str) -> (&'static str, &'static str) {
    match symbol {
        ":=" => ("assignment", "`L := E` gives the variable L the value of E. `a, b := b, a` assigns to several at once, all the values being worked out first."),
        "->" => ("conditional expression", "`B -> E1, E2` is E1 when B is true and E2 otherwise. Conditions may follow one another: `a -> x, b -> y, z`."),
        "=" => ("equals", "Between expressions, the relation `equals`. In a definition, `let x = E` defines x by value, and `let F[x] = E` defines a fixed function."),
        "~~" => ("by reference", "`let x ~~ y` defines x as another name for the variable y. In publication CPL it is printed \u{2243}."),
        "==" => ("equivalence, or a free function", "Between truth values, `equivalent to`. In a definition, `let F[x] == E` defines a free function, which shares its free variables, and `let x == E` defines x by substitution (1963). In publication CPL it is printed \u{2261}."),
        "~==" => ("not equivalent", "True when one of two truth values is true and the other false."),
        "<=>" => ("joining strings", "`s <=> t` is the string s followed by the string t."),
        "**" => ("to the power", "`x ** y` is x to the power y. `^` means the same, and in publication CPL it is printed \u{2191}."),
        "/\\" => ("and", "True when both are true. Between logicals, it works on every bit."),
        "\\/" => ("or", "True when either is true. Between logicals, it works on every bit."),
        "~" => ("not", "Turns true into false and false into true. On a logical, it changes every bit. It binds more tightly than any operator with two operands."),
        "~=" => ("not equal", "The relation `is not equal to`."),
        "<" => ("less than", "A relation. Relations may be chained: `a < b <= c`."),
        "<=" => ("less than or equal", "A relation. Relations may be chained: `a < b <= c`."),
        ">" => ("greater than", "A relation. Relations may be chained: `a > b >= c`."),
        ">=" => ("greater than or equal", "A relation. Relations may be chained: `a > b >= c`."),
        "<<" => ("much less than", "`x << y` asks whether x is so small beside y that adding it makes no difference."),
        ">>" => ("much greater than", "`x >> y` asks whether y is so small beside x that adding it makes no difference."),
        "+" => ("plus", "Addition, or a sign in front of a number."),
        "-" => ("minus", "Subtraction, or a sign in front of a number."),
        "*" => ("times", "Multiplication. It may be left out: `2x` and `a b` are products."),
        "/" => ("divided by", "Division. `a/bc` is `(a/b)c`."),
        "[" | "]" => ("application", "Square brackets hold the arguments of a function or routine, the subscripts of an array, and the parameters in a definition."),
        "(" | ")" => ("brackets", "Round brackets group part of an expression."),
        "," => ("comma", "Separates the members of a list: arguments, parameters, names, or the two arms of a conditional expression."),
        ";" => ("semicolon", "Ends a command or definition, as the end of a line does."),
        ":" => ("label", "`Name: C` sets the label Name on the command C. After `case K` and `default` it marks a case of a switch."),
        "..." => ("dot string", "In a for-list, `a, b, ..., c` goes from a in steps of b - a as far as c."),
        _ => ("symbol", "One of the basic symbols of CPL."),
    }
}

/// The basic symbols offered for completion, as they are usually written.
const WORDS: &[&str] = &[
    "let",
    "be",
    "and",
    "in",
    "where",
    "rec",
    "function",
    "routine",
    "value",
    "reference",
    "val",
    "ref",
    "fixed",
    "free",
    "constant",
    "variable",
    "load",
    "update",
    "fix",
    "rhs",
    "value of",
    "reference of",
    "result is",
    "if",
    "unless",
    "then",
    "do",
    "test",
    "or",
    "while",
    "until",
    "repeat",
    "repeat while",
    "repeat until",
    "for",
    "step",
    "to",
    "go to",
    "break",
    "return",
    "finish",
    "prefer",
    "switch on",
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
    "double complex",
    "logical",
    "long logical",
    "Boolean",
    "string",
    "label",
    "general",
    "type",
    "vector",
    "matrix",
    "array",
    "all",
];

/// Templates offered for completion: a label, what it is, and the text in
/// the snippet form editors use (`$(` is written `\$(` there).
const TEMPLATES: &[(&str, &str, &str)] = &[
    ("function", "let F[x] = E: define a function", "let ${1:F}[${2:x}] = $0"),
    (
        "recursive function",
        "let rec F[n] = E: a function that uses itself",
        "let rec ${1:F}[${2:n}] = $0",
    ),
    (
        "routine",
        "let routine R[x] be $( ... $): define a routine",
        "let routine ${1:R}[${2:x}] be\n\\$( $0 \\$)",
    ),
    ("if", "if B do C", "if ${1:x > 0} do $0"),
    ("test", "test B then C1 or C2", "test ${1:x > 0} then ${2:x := 0} or $0"),
    ("while", "while B do $( ... $)", "while ${1:x > 0} do\n\\$( $0 \\$)"),
    ("for", "for i = 1 to n do C", "for ${1:i} = ${2:1} to ${3:10} do $0"),
    (
        "value of",
        "let x = value of $( ... result is E $)",
        "let ${1:x} = value of \\$( $0\n   result is ${2:1} \\$)",
    ),
    (
        "switch on",
        "switch on E into $( case K: ... default: ... $)",
        "switch on ${1:x} into\n\\$( case ${2:1}: $0\n   default: \\$)",
    ),
    ("write", "Write[..., '*n']: write a line", "Write[$0, '*n']"),
];

struct Server {
    docs: BTreeMap<String, Doc>,
    shutdown: bool,
    exit: bool,
}

impl Server {
    fn new() -> Self {
        Server {
            docs: BTreeMap::new(),
            shutdown: false,
            exit: false,
        }
    }

    /// Handles one incoming message and returns the messages to send back.
    fn handle(&mut self, message: &Value) -> Vec<Value> {
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("").to_string();

        let Some(id) = message.get("id").filter(|_| !method.is_empty()) else {
            // A notification, or a reply to something we never ask.
            return match method {
                "textDocument/didOpen" => self.open(uri, params["textDocument"]["text"].as_str().unwrap_or("")),
                "textDocument/didChange" => {
                    // Full sync: the last change holds the whole text.
                    let changes = params["contentChanges"].as_array();
                    match changes.and_then(|c| c.last()).and_then(|c| c["text"].as_str()) {
                        Some(text) => self.open(uri, text),
                        None => vec![],
                    }
                }
                "textDocument/didClose" => {
                    self.docs.remove(&uri);
                    vec![json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                        "params": { "uri": uri, "diagnostics": [] } })]
                }
                "exit" => {
                    self.exit = true;
                    vec![]
                }
                _ => vec![],
            };
        };

        let reply = |result: Value| vec![json!({ "jsonrpc": "2.0", "id": id, "result": result })];
        let fail = |code: i32, text: &str| {
            vec![json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": text } })]
        };

        match method {
            "initialize" => return reply(self.capabilities()),
            "shutdown" => {
                self.shutdown = true;
                return reply(Value::Null);
            }
            _ => {}
        }
        if !method.starts_with("textDocument/") {
            return fail(-32601, &format!("method not supported: {method}"));
        }
        let Some(doc) = self.docs.get(&uri) else {
            return reply(Value::Null);
        };
        let position = &params["position"];

        match method {
            "textDocument/hover" => reply(hover(doc, position)),
            "textDocument/definition" => reply(definition(doc, &uri, position)),
            "textDocument/references" => {
                let with_declaration = params["context"]["includeDeclaration"].as_bool().unwrap_or(true);
                let found = uses(doc, position, with_declaration);
                reply(json!(found
                    .iter()
                    .map(|r| json!({ "uri": uri, "range": r }))
                    .collect::<Vec<_>>()))
            }
            "textDocument/documentHighlight" => {
                let found = uses(doc, position, true);
                reply(json!(found.iter().map(|r| json!({ "range": r })).collect::<Vec<_>>()))
            }
            "textDocument/prepareRename" => reply(prepare_rename(doc, position)),
            "textDocument/rename" => match rename(doc, &uri, position, params["newName"].as_str().unwrap_or("")) {
                Ok(edit) => reply(edit),
                Err(text) => fail(-32602, &text),
            },
            "textDocument/completion" => reply(completion(doc, position)),
            "textDocument/signatureHelp" => reply(signature_help(doc, position)),
            "textDocument/documentSymbol" => reply(symbols(doc)),
            "textDocument/foldingRange" => reply(folding(doc)),
            "textDocument/semanticTokens/full" => reply(json!({ "data": semantic_tokens(doc) })),
            _ => fail(-32601, &format!("method not supported: {method}")),
        }
    }

    fn capabilities(&self) -> Value {
        json!({
            "serverInfo": { "name": "cpl-lsp", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": {
                "positionEncoding": "utf-16",
                "textDocumentSync": { "openClose": true, "change": 1 },
                "hoverProvider": true,
                "definitionProvider": true,
                "referencesProvider": true,
                "documentHighlightProvider": true,
                "renameProvider": { "prepareProvider": true },
                "completionProvider": {},
                "signatureHelpProvider": { "triggerCharacters": ["[", ","], "retriggerCharacters": [","] },
                "documentSymbolProvider": true,
                "foldingRangeProvider": true,
                "semanticTokensProvider": {
                    "legend": { "tokenTypes": SEMANTIC_TYPES, "tokenModifiers": SEMANTIC_MODIFIERS },
                    "full": true,
                },
            },
        })
    }

    fn open(&mut self, uri: String, text: &str) -> Vec<Value> {
        let doc = Doc::new(text.to_string());
        let diagnostics: Vec<Value> = doc
            .analysis
            .diagnostics
            .iter()
            .map(|d| {
                let mut out = json!({
                    "range": doc.range(d.start, d.end),
                    "severity": match d.severity { Severity::Error => 1, Severity::Warning => 2, Severity::Hint => 4 },
                    "code": d.code,
                    "source": "cpl",
                    "message": d.message,
                });
                if d.unnecessary {
                    out["tags"] = json!([1]);
                }
                out
            })
            .collect();
        self.docs.insert(uri.clone(), doc);
        vec![json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
            "params": { "uri": uri, "diagnostics": diagnostics } })]
    }
}

/// The words of a basic symbol as written, without their underlining.
fn written(doc: &Doc, i: usize) -> (String, usize, usize) {
    let a = &doc.analysis;
    let t = &a.tokens[i];
    let plain = |t: &Token| doc.text_of(t).replace(lexer::UNDERLINE, "");
    match t.partner {
        Some(j) => {
            let (first, second) = (&a.tokens[i.min(j)], &a.tokens[i.max(j)]);
            (format!("{} {}", plain(first), plain(second)), first.start, second.end)
        }
        None => (plain(t), t.start, t.end),
    }
}

/// What to say about the definition of a name.
fn describe(doc: &Doc, d: usize) -> String {
    let a = &doc.analysis;
    let def = &a.definitions[d];
    let mut text = format!("**{} `{}`**", kind_name(def.kind), def.name);
    let heading = doc.line_text(a.tokens[def.token].start).trim();
    text.push_str(&format!("\n\n```cpl\n{heading}\n```"));
    if let Some(comment) = a.doc(&doc.text, d) {
        text.push_str(&format!("\n\n{comment}"));
    }
    let mut facts = Vec::new();
    match def.kind {
        DefKind::Function | DefKind::Routine => {
            let names: Vec<String> = def
                .params
                .iter()
                .map(|&p| format!("`{}`", a.definitions[p].name))
                .collect();
            facts.push(match names.len() {
                0 => "It has no parameters.".to_string(),
                1 => format!("Its parameter is {}.", names[0]),
                _ => format!("Its parameters are {}.", names.join(", ")),
            });
        }
        DefKind::Parameter => {
            if let Some(owner) = def.owner {
                facts.push(format!("A parameter of `{}`.", a.definitions[owner].name));
            }
        }
        DefKind::Label => {
            facts.push("A label is a value: `go to` jumps to it, and it can be passed to a routine.".to_string())
        }
        DefKind::Loop => facts.push("It is new for each cycle and known only in the body of the `for`.".to_string()),
        _ => {}
    }
    if def.by_where {
        facts.push("Defined by a where-clause.".to_string());
    }
    facts.push(format!("Used {}.", times(def.uses.len())));
    text.push_str(&format!("\n\n{}", facts.join(" ")));
    text
}

fn hover(doc: &Doc, position: &Value) -> Value {
    let Some((i, t)) = doc.token_at(position) else {
        return Value::Null;
    };
    let a = &doc.analysis;
    let (mut start, mut end) = (t.start, t.end);
    let text = match t.kind {
        Kind::Comment => return Value::Null,
        Kind::Word => {
            let (as_written, from, to) = written(doc, i);
            (start, end) = (from, to);
            let (heading, body) = word_help(t.symbol);
            let usual = spelling(t.symbol);
            let mut text = format!("**`{as_written}`** {heading}\n\n{body}");
            let lower = as_written.to_lowercase();
            if lower != usual.to_lowercase() && lower != t.symbol && !heading.starts_with("half") {
                text.push_str(&format!("\n\n`{as_written}` is another way of writing `{usual}`."));
            }
            text
        }
        Kind::Name => {
            let name = doc.text_of(t);
            match (doc.definition(t), analysis::builtin(name)) {
                (Some((d, _)), _) => describe(doc, d),
                (None, Some(b)) => {
                    let what = if b.signature.contains('[') {
                        "basic function"
                    } else {
                        "basic constant"
                    };
                    format!("**`{}`** {what}\n\n{}", b.signature, b.doc)
                }
                (None, None) if a.complete => {
                    format!("**Name `{name}`**\n\nNothing defines this name here.")
                }
                (None, None) => return Value::Null,
            }
        }
        Kind::Number => "**Number**\n\nA whole number, or one with a decimal point. `@` is times ten to the power: `2.5@-8`."
            .to_string(),
        Kind::Str => "**String constant**\n\nInside it a star begins an escape: `*n` new line, `*s` space, `*t` tab, `*b` backspace, `*'` a quote, `**` a star."
            .to_string(),
        Kind::Bits => "**Logical constant**\n\nA string of bits written in octal (`#o777`), binary (`#b1010`) or hexadecimal (`#x1FF`). A bar after the letter, as in `#o|273`, puts the digits at the left hand end of the 24 bits."
            .to_string(),
        Kind::Sym => {
            let (heading, body) = symbol_help(t.symbol);
            format!("**`{}`** {heading}\n\n{body}", doc.text_of(t))
        }
        Kind::Open | Kind::Close => {
            let tag = analysis::tag(doc.text_of(t));
            let mut text = match (t.kind, tag) {
                (Kind::Open, "") => "**Opening section bracket**".to_string(),
                (Kind::Open, tag) => format!("**Opening section bracket, tagged `{tag}`**"),
                (_, "") => "**Closing section bracket**".to_string(),
                (_, tag) => format!("**Closing section bracket, tagged `{tag}`**"),
            };
            text.push_str("\n\nSection brackets make one command, or one block, of the commands between them.");
            if t.kind == Kind::Close && !tag.is_empty() {
                let closed = a.tokens.iter().filter(|o| o.kind == Kind::Open && o.partner == Some(i)).count();
                text.push_str(&format!(
                    " A tagged closing bracket closes every section opened since the opening bracket with the same tag: here {}.",
                    match closed { 1 => "one section".to_string(), n => format!("{n} sections") }
                ));
            }
            match t.partner {
                Some(j) => text.push_str(&format!(
                    "\n\nIt matches the bracket on line {}.",
                    doc.line_of(a.tokens[j].start) + 1
                )),
                None => text.push_str("\n\nNothing matches it."),
            }
            text
        }
        Kind::Unknown => "This text means nothing in CPL.".to_string(),
    };
    json!({ "contents": { "kind": "markdown", "value": text }, "range": doc.range(start, end) })
}

fn definition(doc: &Doc, uri: &str, position: &Value) -> Value {
    match doc.token_at(position).and_then(|(_, t)| doc.definition(t)) {
        Some((_, def)) => json!({ "uri": uri, "range": doc.token_range(&doc.analysis.tokens[def.token]) }),
        None => Value::Null,
    }
}

/// The tokens that mean the same thing as the one under the cursor.
fn related(doc: &Doc, position: &Value) -> Vec<usize> {
    let a = &doc.analysis;
    let Some((i, t)) = doc.token_at(position) else {
        return vec![];
    };
    let mut found = match t.kind {
        Kind::Name => match doc.definition(t) {
            Some((_, def)) => {
                let mut all = vec![def.token];
                all.extend(&def.uses);
                all
            }
            // A basic function, or a name nothing defines: every other
            // mention of it that is in the same position.
            None => (0..a.tokens.len())
                .filter(|&j| {
                    let other = &a.tokens[j];
                    other.kind == Kind::Name && other.def.is_none() && doc.text_of(other) == doc.text_of(t)
                })
                .collect(),
        },
        Kind::Open => vec![Some(i), t.partner].into_iter().flatten().collect(),
        Kind::Close => {
            let mut all = vec![i];
            all.extend(
                (0..a.tokens.len()).filter(|&j| a.tokens[j].kind == Kind::Open && a.tokens[j].partner == Some(i)),
            );
            all
        }
        Kind::Word => vec![Some(i), t.partner].into_iter().flatten().collect(),
        _ => vec![],
    };
    found.sort();
    found
}

fn uses(doc: &Doc, position: &Value, with_declaration: bool) -> Vec<Value> {
    related(doc, position)
        .into_iter()
        .map(|i| &doc.analysis.tokens[i])
        .filter(|t| with_declaration || !t.defining)
        .map(|t| doc.token_range(t))
        .collect()
}

fn prepare_rename(doc: &Doc, position: &Value) -> Value {
    match doc.token_at(position) {
        Some((_, t)) if t.def.is_some() => json!({ "range": doc.token_range(t), "placeholder": doc.text_of(t) }),
        _ => Value::Null,
    }
}

fn rename(doc: &Doc, uri: &str, position: &Value, new_name: &str) -> Result<Value, String> {
    let a = &doc.analysis;
    let Some((_, def)) = doc.token_at(position).and_then(|(_, t)| doc.definition(t)) else {
        return Err("only a name that is defined in this program can be renamed".into());
    };
    // The lexer is the judge of what a name is.
    let scan = lexer::scan(new_name);
    let is_name = scan.errors.is_empty()
        && matches!(&scan.tokens[..], [name, _] if name.tok == lexer::Tok::Name(new_name.to_string()));
    if !is_name {
        return Err(format!(
            "{new_name} is not a CPL name: a name is one small letter, or a capital followed by letters and digits, and may end in primes"
        ));
    }
    if new_name != def.name {
        if analysis::builtin(new_name).is_some() {
            return Err(format!("{new_name} is the name of a basic function"));
        }
        let taken = |t: &Token| t.kind == Kind::Name && doc.text_of(t) == new_name;
        if a.tokens.iter().any(taken) {
            return Err(format!("the name {new_name} is already used in this program"));
        }
    }
    let src = doc.text.as_bytes();
    let small = new_name.as_bytes()[0].is_ascii_lowercase();
    let is_word = |run: &str| lexer::WORDS.contains(&run);
    let mut sites = vec![def.token];
    sites.extend(&def.uses);
    let edits: Vec<Value> = sites
        .into_iter()
        .map(|i| {
            let t = &a.tokens[i];
            // Small letters side by side are separate names, unless together
            // they spell a basic symbol; a large name runs on through any
            // letters and digits after it. A space keeps the new name apart.
            let mut text = new_name.to_string();
            if small {
                let before: String = {
                    let n = src[..t.start]
                        .iter()
                        .rev()
                        .take_while(|b| b.is_ascii_lowercase())
                        .count();
                    doc.text[t.start - n..t.start].to_string()
                };
                let after: String = if new_name.len() > 1 {
                    String::new()
                } else {
                    src[t.end..]
                        .iter()
                        .take_while(|b| b.is_ascii_lowercase())
                        .map(|&b| b as char)
                        .collect()
                };
                if !before.is_empty() && is_word(&format!("{before}{}{after}", &new_name[..1])) {
                    text.insert(0, ' ');
                    if !after.is_empty() && is_word(&format!("{new_name}{after}")) {
                        text.push(' ');
                    }
                } else if !after.is_empty() && before.is_empty() && is_word(&format!("{new_name}{after}")) {
                    text.push(' ');
                }
            } else if !new_name.ends_with('\'') {
                let runs_on = match (src.get(t.end), src.get(t.end + 1)) {
                    (Some(next), _) if next.is_ascii_alphanumeric() => true,
                    (Some(b'.'), Some(next)) => next.is_ascii_alphanumeric(),
                    _ => false,
                };
                if runs_on {
                    text.push(' ');
                }
            }
            json!({ "range": doc.token_range(t), "newText": text })
        })
        .collect();
    Ok(json!({ "changes": { uri: edits } }))
}

fn completion(doc: &Doc, position: &Value) -> Value {
    let a = &doc.analysis;
    let offset = doc.offset(position);
    let mut items = Vec::new();
    let mut defined: Vec<&str> = Vec::new();

    for d in a.visible_at(offset) {
        let def = &a.definitions[d];
        defined.push(&def.name);
        let kind = match def.kind {
            DefKind::Function | DefKind::Routine => 3,
            DefKind::Constant | DefKind::Label => 21,
            _ => 6,
        };
        items.push(json!({
            "label": def.name,
            "kind": kind,
            "detail": format!("{}: {}", kind_name(def.kind).to_lowercase(), doc.line_text(a.tokens[def.token].start).trim()),
            "documentation": a.doc(&doc.text, d).unwrap_or_default(),
            "sortText": format!("0{}", def.name),
        }));
    }
    for b in analysis::BUILTINS.iter().filter(|b| !defined.contains(&b.name)) {
        let function = b.signature.contains('[');
        items.push(json!({
            "label": b.name,
            "kind": if function { 3 } else { 21 },
            "detail": b.signature,
            "documentation": b.doc,
            "sortText": format!("1{}", b.name),
        }));
    }
    for word in WORDS {
        let reduced = lexer::scan(word).tokens.first().map(|t| t.tok.clone());
        let lexer::Tok::Kw(symbol) = reduced.unwrap_or(lexer::Tok::Eof) else {
            continue;
        };
        items.push(json!({
            "label": word,
            "kind": 14,
            "detail": word_help(symbol).0,
            "documentation": word_help(symbol).1,
            "sortText": format!("2{word}"),
        }));
    }
    for (label, detail, insert) in TEMPLATES {
        items.push(json!({
            "label": label,
            "kind": 15,
            "detail": detail,
            "insertText": insert,
            "insertTextFormat": 2,
            "sortText": format!("3{label}"),
        }));
    }
    json!(items)
}

fn signature_help(doc: &Doc, position: &Value) -> Value {
    let a = &doc.analysis;
    let offset = doc.offset(position);
    // The square brackets and sections still open at the cursor, innermost
    // last: the token before each, and the commas seen in it so far.
    let mut open: Vec<(Option<usize>, usize)> = Vec::new();
    for (i, t) in a.tokens.iter().enumerate().take_while(|(_, t)| t.end <= offset) {
        match (t.kind, t.symbol) {
            (Kind::Sym, "[") => open.push((i.checked_sub(1), 0)),
            (Kind::Open, _) => open.push((None, 0)),
            (Kind::Sym, "]") | (Kind::Close, _) => {
                open.pop();
            }
            (Kind::Sym, ",") => {
                if let Some(top) = open.last_mut() {
                    top.1 += 1;
                }
            }
            _ => {}
        }
    }
    let Some(&(Some(callee), commas)) = open.last() else {
        return Value::Null;
    };
    let t = &a.tokens[callee];
    if t.kind != Kind::Name || t.defining {
        return Value::Null;
    }
    let name = doc.text_of(t);
    let (label, documentation) = match (doc.definition(t), analysis::builtin(name)) {
        (Some((d, def)), _) if matches!(def.kind, DefKind::Function | DefKind::Routine) => {
            let params: Vec<&str> = def.params.iter().map(|&p| a.definitions[p].name.as_str()).collect();
            (format!("{}[{}]", def.name, params.join(", ")), a.doc(&doc.text, d))
        }
        (None, Some(b)) if b.signature.contains('[') => (b.signature.to_string(), Some(b.doc.to_string())),
        _ => return Value::Null,
    };
    // Where each parameter is in the label.
    let inside = label.find('[').map_or(label.len(), |at| at + 1);
    let mut parameters = Vec::new();
    let mut at = inside;
    for part in label[inside..label.len() - 1].split(", ").filter(|p| !p.is_empty()) {
        parameters.push(json!({ "label": [at, at + part.len()] }));
        at += part.len() + 2;
    }
    let active = commas.min(parameters.len().saturating_sub(1));
    let mut help = json!({ "label": label, "parameters": parameters });
    if let Some(text) = documentation {
        help["documentation"] = json!(text);
    }
    json!({ "signatures": [help], "activeSignature": 0, "activeParameter": active })
}

fn symbols(doc: &Doc) -> Value {
    let a = &doc.analysis;
    let mut listed: Vec<usize> = (0..a.definitions.len())
        .filter(|&d| !matches!(a.definitions[d].kind, DefKind::Parameter | DefKind::Loop))
        .collect();
    listed.sort_by_key(|&d| {
        (
            a.definitions[d].extent.start,
            std::cmp::Reverse(a.definitions[d].extent.end),
        )
    });

    // Definitions inside the body of a function are listed under it.
    fn place(out: &mut Vec<(usize, Value)>, end: usize, symbol: Value) {
        match out.last_mut() {
            Some((outer, parent)) if end <= *outer => {
                parent["children"].as_array_mut().unwrap().push(symbol);
            }
            _ => out.push((end, symbol)),
        }
    }
    let mut top: Vec<(usize, Value)> = Vec::new();
    for d in listed {
        let def = &a.definitions[d];
        let name = &a.tokens[def.token];
        let (start, end) = (def.extent.start.min(name.start), def.extent.end.max(name.end));
        let comment = a.doc(&doc.text, d).unwrap_or_default();
        let symbol = json!({
            "name": def.name,
            "detail": match comment.lines().next() {
                Some(line) => line.to_string(),
                None => kind_name(def.kind).to_lowercase(),
            },
            "kind": match def.kind { DefKind::Function | DefKind::Routine => 12, DefKind::Variable => 13, _ => 14 },
            "range": doc.range(start, end),
            "selectionRange": doc.token_range(name),
            "children": [],
        });
        place(&mut top, end, symbol);
    }
    json!(top.into_iter().map(|(_, symbol)| symbol).collect::<Vec<_>>())
}

fn folding(doc: &Doc) -> Value {
    let a = &doc.analysis;
    let mut out = Vec::new();
    let mut seen = Vec::new();
    let mut fold = |start: usize, end: usize, kind: Option<&str>| {
        let (first, last) = (doc.line_of(start), doc.line_of(end));
        if last > first && !seen.contains(&(first, last)) {
            seen.push((first, last));
            let mut range = json!({ "startLine": first, "endLine": last });
            if let Some(kind) = kind {
                range["kind"] = json!(kind);
            }
            out.push(range);
        }
    };
    for def in a
        .definitions
        .iter()
        .filter(|d| matches!(d.kind, DefKind::Function | DefKind::Routine))
    {
        fold(def.extent.start, def.extent.end, None);
    }
    for t in a.tokens.iter().filter(|t| t.kind == Kind::Open) {
        if let Some(partner) = t.partner {
            fold(t.start, a.tokens[partner].start, None);
        }
    }
    // Runs of comment lines.
    let comments: Vec<&Token> = a.tokens.iter().filter(|t| t.kind == Kind::Comment).collect();
    let mut i = 0;
    while i < comments.len() {
        let mut j = i;
        while j + 1 < comments.len() && doc.line_of(comments[j + 1].start) == doc.line_of(comments[j].start) + 1 {
            j += 1;
        }
        fold(comments[i].start, comments[j].start, Some("comment"));
        i = j + 1;
    }
    json!(out)
}

/// The semantic type and modifiers of a token, or nothing for text that
/// means nothing.
fn semantic(doc: &Doc, t: &Token) -> Option<(&'static str, u32)> {
    const DEFINITION: u32 = 1;
    const LIBRARY: u32 = 2;
    const READONLY: u32 = 4;
    Some(match t.kind {
        Kind::Comment => ("comment", 0),
        Kind::Str => ("string", 0),
        Kind::Number | Kind::Bits => ("number", 0),
        Kind::Sym => ("operator", 0),
        Kind::Open | Kind::Close => ("keyword", 0),
        Kind::Word if analysis::is_type_word(t.symbol) => ("type", 0),
        Kind::Word => ("keyword", 0),
        Kind::Name => {
            let (kind, mut modifiers) = match (doc.definition(t), analysis::builtin(doc.text_of(t))) {
                (Some((_, def)), _) => match def.kind {
                    DefKind::Function | DefKind::Routine => ("function", 0),
                    DefKind::Parameter => ("parameter", 0),
                    DefKind::Label => ("label", 0),
                    DefKind::Constant => ("variable", READONLY),
                    DefKind::Variable | DefKind::Loop => ("variable", 0),
                },
                (None, Some(b)) if b.signature.contains('[') => ("function", LIBRARY),
                (None, Some(_)) => ("variable", LIBRARY | READONLY),
                (None, None) => ("variable", 0),
            };
            if t.defining {
                modifiers |= DEFINITION;
            }
            (kind, modifiers)
        }
        Kind::Unknown => return None,
    })
}

/// Semantic tokens, in the protocol's five-numbers-per-token encoding.
fn semantic_tokens(doc: &Doc) -> Vec<u32> {
    let type_index = |name: &str| SEMANTIC_TYPES.iter().position(|&t| t == name).unwrap() as u32;
    let mut data = Vec::new();
    let (mut last_line, mut last_start) = (0u32, 0u32);
    for t in &doc.analysis.tokens {
        let Some((kind, modifiers)) = semantic(doc, t) else {
            continue;
        };
        // A token may not span lines, so one that did would be sent in pieces.
        let mut start = t.start;
        while start < t.end {
            let line = doc.line_of(start);
            let line_end = doc.line_starts.get(line + 1).map_or(doc.text.len(), |&next| next - 1);
            let end = t.end.min(line_end);
            let text = doc.text[start..end].trim_end_matches('\r');
            let length: usize = text.chars().map(char::len_utf16).sum();
            if length > 0 {
                let column = doc.position(start)["character"].as_u64().unwrap() as u32;
                let line = line as u32;
                let delta_line = line - last_line;
                let delta_start = if delta_line == 0 { column - last_start } else { column };
                data.extend([delta_line, delta_start, length as u32, type_index(kind), modifiers]);
                (last_line, last_start) = (line, column);
            }
            start = line_end + 1;
        }
    }
    data
}

fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            if length.is_some() {
                break;
            }
            continue;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length.unwrap()];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn main() -> io::Result<()> {
    if std::env::args().any(|arg| arg == "--version") {
        println!("cpl-lsp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout().lock();
    let mut server = Server::new();
    while let Some(message) = read_message(&mut input)? {
        for reply in server.handle(&message) {
            let body = reply.to_string();
            write!(output, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
        }
        output.flush()?;
        if server.exit {
            break;
        }
    }
    std::process::exit(if server.shutdown { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    const URI: &str = "file:///test.cpl";
    const PROGRAM: &str = "\
|| The highest common factor,
|| by Euclid.
let rec HCF[integer n, m] = (m > n -> HCF[m, n],
                             m = 0 -> n,
                             HCF[m, Rem[n, m]])
let a, b = 1071, 462
if a < b go to Swap
Next: Write['The answer is ', HCF[a, b], '*n']
finish
Swap: a, b := b, a
go to Next
";

    fn server_with(text: &str) -> (Server, Vec<Value>) {
        let mut server = Server::new();
        let out = server.handle(&json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen",
            "params": { "textDocument": { "uri": URI, "languageId": "cpl", "version": 1, "text": text } } }));
        (server, out)
    }

    fn ask(server: &mut Server, method: &str, line: u32, character: u32, extra: Value) -> Value {
        let mut params =
            json!({ "textDocument": { "uri": URI }, "position": { "line": line, "character": character } });
        for (key, value) in extra.as_object().into_iter().flatten() {
            params[key] = value.clone();
        }
        let mut out = server.handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }));
        assert_eq!(out.len(), 1);
        out.remove(0)
    }

    fn hover_text(server: &mut Server, line: u32, character: u32) -> String {
        let reply = ask(server, "textDocument/hover", line, character, json!({}));
        reply["result"]["contents"]["value"].as_str().unwrap_or("").to_string()
    }

    #[test]
    fn initialise_advertises_what_is_implemented() {
        let mut server = Server::new();
        let out = server.handle(&json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {} }));
        let capabilities = &out[0]["result"]["capabilities"];
        assert_eq!(capabilities["hoverProvider"], true);
        assert_eq!(capabilities["renameProvider"]["prepareProvider"], true);
        assert_eq!(
            capabilities["semanticTokensProvider"]["legend"]["tokenTypes"][4],
            "function"
        );
        assert_eq!(out[0]["result"]["serverInfo"]["name"], "cpl-lsp");
    }

    #[test]
    fn a_good_program_publishes_no_diagnostics_and_a_bad_one_does() {
        let (_, out) = server_with(PROGRAM);
        assert_eq!(out[0]["method"], "textDocument/publishDiagnostics");
        assert_eq!(out[0]["params"]["diagnostics"], json!([]));

        let (_, out) = server_with("let x, y = 1, 2\nWrite[x\n");
        let diagnostics = out[0]["params"]["diagnostics"].as_array().unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0]["severity"], 1);
        assert_eq!(diagnostics[0]["code"], "syntax");
        assert_eq!(diagnostics[0]["source"], "cpl");
        assert_eq!(diagnostics[0]["message"], "']' expected (found the end of the program)");
        assert_eq!(
            diagnostics[0]["range"],
            json!({ "start": { "line": 1, "character": 6 }, "end": { "line": 1, "character": 7 } })
        );

        let (_, out) = server_with("let x, y = 1, 2\nWrite[x]\n");
        let hint = &out[0]["params"]["diagnostics"][0];
        assert_eq!(
            (&hint["severity"], &hint["code"], &hint["tags"]),
            (&json!(4), &json!("unused-name"), &json!([1]))
        );
    }

    #[test]
    fn hover_explains_symbols_basic_functions_and_definitions() {
        let (mut server, _) = server_with(PROGRAM);
        let rec = hover_text(&mut server, 2, 5);
        assert!(rec.starts_with("**`rec`** recursive definition"), "{rec}");
        // Two words that make one symbol are explained together.
        let go = hover_text(&mut server, 6, 13);
        assert!(go.starts_with("**`go to`** jump"), "{go}");
        let reply = ask(&mut server, "textDocument/hover", 6, 13, json!({}));
        assert_eq!(reply["result"]["range"]["start"]["character"], 9);
        assert_eq!(reply["result"]["range"]["end"]["character"], 14);

        let rem = hover_text(&mut server, 4, 36);
        assert!(
            rem.starts_with("**`Rem[x, y]`** basic function") && rem.contains("remainder"),
            "{rem}"
        );

        let hcf = hover_text(&mut server, 7, 31);
        assert!(hcf.starts_with("**Function `HCF`**"), "{hcf}");
        assert!(hcf.contains("let rec HCF[integer n, m] = (m > n -> HCF[m, n],"));
        assert!(hcf.contains("The highest common factor,\nby Euclid."));
        assert!(hcf.contains("Its parameters are `n`, `m`. Used 3 times."), "{hcf}");

        let n = hover_text(&mut server, 3, 38);
        assert!(
            n.starts_with("**Parameter `n`**") && n.contains("A parameter of `HCF`."),
            "{n}"
        );
        let label = hover_text(&mut server, 6, 15);
        assert!(
            label.starts_with("**Label `Swap`**") && label.contains("Used once."),
            "{label}"
        );
        assert!(hover_text(&mut server, 2, 35).starts_with("**`->`** conditional expression"));
        assert!(hover_text(&mut server, 7, 13).starts_with("**String constant**"));
        assert_eq!(
            ask(&mut server, "textDocument/hover", 0, 5, json!({}))["result"],
            Value::Null
        );
    }

    #[test]
    fn hover_says_when_a_word_is_another_spelling() {
        let (mut server, _) = server_with("let x = 1\ntest x = 1 then Write[1] else Write[2]\n$(A Write[x] $)A\n");
        let then = hover_text(&mut server, 1, 12);
        assert!(then.contains("`then` is another way of writing `do`."), "{then}");
        assert!(hover_text(&mut server, 1, 26).contains("`else` is another way of writing `or`."));
        let close = hover_text(&mut server, 2, 13);
        assert!(close.starts_with("**Closing section bracket, tagged `A`**"), "{close}");
        assert!(close.contains("here one section") && close.contains("It matches the bracket on line 3."));
    }

    #[test]
    fn definition_and_references_follow_scope() {
        let (mut server, _) = server_with(PROGRAM);
        let definition = ask(&mut server, "textDocument/definition", 7, 31, json!({}));
        assert_eq!(
            definition["result"]["range"]["start"],
            json!({ "line": 2, "character": 8 })
        );
        // A label is found from a jump that comes before it.
        let label = ask(&mut server, "textDocument/definition", 6, 15, json!({}));
        assert_eq!(label["result"]["range"]["start"], json!({ "line": 9, "character": 0 }));
        // A basic function is defined nowhere in the program.
        assert_eq!(
            ask(&mut server, "textDocument/definition", 4, 36, json!({}))["result"],
            Value::Null
        );

        let all = json!({ "context": { "includeDeclaration": true } });
        let references = ask(&mut server, "textDocument/references", 2, 8, all);
        assert_eq!(references["result"].as_array().unwrap().len(), 4);
        let only = json!({ "context": { "includeDeclaration": false } });
        let calls = ask(&mut server, "textDocument/references", 2, 8, only);
        assert_eq!(calls["result"].as_array().unwrap().len(), 3);

        // The parameter n and nothing else called n.
        let n = ask(&mut server, "textDocument/documentHighlight", 2, 20, json!({}));
        let lines: Vec<u64> = n["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["range"]["start"]["line"].as_u64().unwrap())
            .collect();
        assert_eq!(lines, [2, 2, 2, 3, 4]);
    }

    #[test]
    fn section_brackets_are_highlighted_in_pairs() {
        let (mut server, _) = server_with("$(T Write[1]\n$( Write[2]\n$)T\n");
        let pair = ask(&mut server, "textDocument/documentHighlight", 2, 1, json!({}));
        let lines: Vec<u64> = pair["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["range"]["start"]["line"].as_u64().unwrap())
            .collect();
        assert_eq!(lines, [0, 1, 2]);
    }

    #[test]
    fn rename_changes_a_name_everywhere_and_refuses_a_clash() {
        let (mut server, _) = server_with(PROGRAM);
        let prepared = ask(&mut server, "textDocument/prepareRename", 7, 31, json!({}));
        assert_eq!(prepared["result"]["placeholder"], "HCF");
        assert_eq!(prepared["result"]["range"]["start"]["character"], 30);
        assert_eq!(
            ask(&mut server, "textDocument/prepareRename", 4, 36, json!({}))["result"],
            Value::Null
        );

        let renamed = ask(&mut server, "textDocument/rename", 7, 31, json!({ "newName": "Gcd" }));
        let edits = renamed["result"]["changes"][URI].as_array().unwrap();
        assert_eq!(edits.len(), 4);
        assert!(edits.iter().all(|e| e["newText"] == "Gcd"));

        let clash = ask(&mut server, "textDocument/rename", 7, 31, json!({ "newName": "Swap" }));
        assert!(clash["error"]["message"].as_str().unwrap().contains("already used"));
        let basic = ask(&mut server, "textDocument/rename", 7, 31, json!({ "newName": "Sqrt" }));
        assert!(basic["error"]["message"].as_str().unwrap().contains("basic function"));
        for bad in ["total", "let", "2x", "A B", ""] {
            let reply = ask(&mut server, "textDocument/rename", 7, 31, json!({ "newName": bad }));
            assert!(
                reply["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("is not a CPL name"),
                "{bad}"
            );
        }
    }

    /// Applies the edits of a rename to the text.
    fn renamed(text: &str, line: u32, character: u32, new_name: &str) -> String {
        let (mut server, _) = server_with(text);
        let reply = ask(
            &mut server,
            "textDocument/rename",
            line,
            character,
            json!({ "newName": new_name }),
        );
        let doc = Doc::new(text.to_string());
        let mut edits: Vec<(usize, usize, String)> = reply["result"]["changes"][URI]
            .as_array()
            .unwrap_or_else(|| panic!("{reply}"))
            .iter()
            .map(|e| {
                (
                    doc.offset(&e["range"]["start"]),
                    doc.offset(&e["range"]["end"]),
                    e["newText"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        edits.sort();
        let mut out = text.to_string();
        for (start, end, new) in edits.into_iter().rev() {
            out.replace_range(start..end, &new);
        }
        out
    }

    #[test]
    fn rename_keeps_names_apart_that_would_run_together() {
        // Small letters side by side are a product; a large name would swallow what follows.
        assert_eq!(
            renamed("let a, x = 1, 2\nWrite[xa + 2x]", 0, 7, "Total"),
            "let a, Total = 1, 2\nWrite[Total a + 2Total]"
        );
        // "i" beside "n" would spell the basic symbol "in".
        assert_eq!(
            renamed("let a, n = 1, 2\nWrite[an]", 0, 4, "i"),
            "let i, n = 1, 2\nWrite[i n]"
        );
        assert_eq!(
            renamed("let a, n = 1, 2\nWrite[na]", 0, 4, "i"),
            "let i, n = 1, 2\nWrite[ni]"
        );
        assert_eq!(
            renamed("let Total, n = 1, 2\nWrite[n Total]", 0, 4, "t'"),
            "let t', n = 1, 2\nWrite[n t']"
        );
        // Every result is still a program with the same meaning.
        for text in [
            "let a, Total = 1, 2\nWrite[Total a + 2Total]",
            "let i, n = 1, 2\nWrite[i n]",
        ] {
            assert!(analyse(text).diagnostics.is_empty(), "{text}");
        }
    }

    #[test]
    fn completion_offers_what_is_in_scope_then_basic_functions_words_and_templates() {
        let (mut server, _) = server_with("let Total = 0\nlet routine Add[x] be\n$( Total := Total + \n");
        let reply = ask(&mut server, "textDocument/completion", 2, 20, json!({}));
        let items = reply["result"].as_array().unwrap();
        let labels: Vec<&str> = items.iter().map(|i| i["label"].as_str().unwrap()).collect();
        assert_eq!(&labels[..3], ["Total", "Add", "x"]);
        assert_eq!(items[2]["detail"], "parameter: let routine Add[x] be");
        for wanted in [
            "Sqrt",
            "Write",
            "NIL",
            "let",
            "value of",
            "Boolean",
            "routine",
            "switch on",
        ] {
            assert!(labels.contains(&wanted), "{wanted}");
        }
        let sqrt = items.iter().find(|i| i["label"] == "Sqrt").unwrap();
        assert_eq!((&sqrt["detail"], &sqrt["kind"]), (&json!("Sqrt[x]"), &json!(3)));
    }

    #[test]
    fn every_word_and_template_offered_is_cpl() {
        for word in WORDS {
            let scan = lexer::scan(word);
            assert!(scan.errors.is_empty() && scan.tokens.len() == 2, "{word}");
            let lexer::Tok::Kw(symbol) = scan.tokens[0].tok else {
                panic!("{word} is not a basic symbol")
            };
            assert_ne!(word_help(symbol).0, "basic symbol", "{word} has no description");
            assert_ne!(word_help(symbol).0, "half a symbol", "{word}");
        }
        // Every basic symbol the lexer knows has a description.
        for word in lexer::WORDS {
            let lexer::Tok::Kw(symbol) = lexer::scan(word).tokens[0].tok else {
                panic!("{word}")
            };
            assert_ne!(word_help(symbol).0, "basic symbol", "{word} has no description");
        }
        // A template with its fields filled in as offered is a program.
        for (label, _, insert) in TEMPLATES {
            let mut text = insert.replace("\\$", "$").replace("$0", "Write[1]");
            while let Some(at) = text.find("${") {
                let end = at + text[at..].find('}').unwrap();
                let default = text[at..end].split_once(':').unwrap().1.to_string();
                text.replace_range(at..=end, &default);
            }
            let program = format!("let x = 1\n{text}\n");
            let problems: Vec<String> = analyse(&program)
                .diagnostics
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .map(|d| d.message.clone())
                .collect();
            assert!(problems.is_empty(), "{label}: {program}{problems:?}");
        }
    }

    #[test]
    fn signature_help_tracks_the_argument_under_the_cursor() {
        let (mut server, _) = server_with(PROGRAM);
        let help = ask(&mut server, "textDocument/signatureHelp", 7, 37, json!({}));
        assert_eq!(help["result"]["signatures"][0]["label"], "HCF[n, m]");
        assert_eq!(help["result"]["signatures"][0]["parameters"][1]["label"], json!([7, 8]));
        assert_eq!(
            help["result"]["signatures"][0]["documentation"],
            "The highest common factor,\nby Euclid."
        );
        assert_eq!(help["result"]["activeParameter"], 1);
        let first = ask(&mut server, "textDocument/signatureHelp", 7, 34, json!({}));
        assert_eq!(first["result"]["activeParameter"], 0);

        let rem = ask(&mut server, "textDocument/signatureHelp", 4, 40, json!({}));
        assert_eq!(rem["result"]["signatures"][0]["label"], "Rem[x, y]");
        assert_eq!(
            rem["result"]["signatures"][0]["parameters"],
            json!([{ "label": [4, 5] }, { "label": [7, 8] }])
        );
        // Write takes any number of arguments: the last parameter stands for the rest.
        let write = ask(&mut server, "textDocument/signatureHelp", 7, 43, json!({}));
        assert_eq!(write["result"]["signatures"][0]["label"], "Write[x, y]");
        assert_eq!(write["result"]["activeParameter"], 1);
        assert_eq!(
            ask(&mut server, "textDocument/signatureHelp", 5, 4, json!({}))["result"],
            Value::Null
        );
    }

    #[test]
    fn symbols_folding_and_semantic_tokens() {
        let text = "\
|| one
|| two
let constant Limit = 3
let routine Show[x] be
$(S let Twice = 2x
   Write[Twice, '*n']
$)S
Show[Limit]
Done: finish
";
        let (mut server, out) = server_with(text);
        assert_eq!(out[0]["params"]["diagnostics"], json!([]));
        let symbols = ask(&mut server, "textDocument/documentSymbol", 0, 0, json!({}));
        let names: Vec<(&str, u64)> = symbols["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| (s["name"].as_str().unwrap(), s["kind"].as_u64().unwrap()))
            .collect();
        assert_eq!(names, [("Limit", 14), ("Show", 12), ("Done", 14)]);
        assert_eq!(symbols["result"][0]["detail"], "one");
        assert_eq!(symbols["result"][1]["children"][0]["name"], "Twice");
        assert_eq!(symbols["result"][1]["range"]["end"]["line"], 6);

        let folds = ask(&mut server, "textDocument/foldingRange", 0, 0, json!({}));
        let folds: Vec<(u64, u64)> = folds["result"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (f["startLine"].as_u64().unwrap(), f["endLine"].as_u64().unwrap()))
            .collect();
        assert_eq!(folds, [(3, 6), (4, 6), (0, 1)]);

        let tokens = ask(&mut server, "textDocument/semanticTokens/full", 0, 0, json!({}));
        let data: Vec<u64> = tokens["result"]["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap())
            .collect();
        assert_eq!(data.len() % 5, 0);
        assert_eq!(&data[..5], [0, 0, 6, 0, 0]);
        let index = |name: &str| SEMANTIC_TYPES.iter().position(|&t| t == name).unwrap() as u64;
        let has = |length: u64, kind: &str, modifiers: u64| {
            data.chunks(5)
                .any(|c| c[2] == length && c[3] == index(kind) && c[4] == modifiers)
        };
        // "Limit" is a constant being defined, "Show" a function, "x" a parameter.
        assert!(has(5, "variable", 1 | 4) && has(5, "variable", 4));
        assert!(has(4, "function", 1) && has(4, "function", 0));
        assert!(has(1, "parameter", 1) && has(1, "parameter", 0));
        // "Write" is a basic function, "Done" a label, "$(S" a keyword.
        assert!(has(5, "function", 2) && has(4, "label", 1) && has(3, "keyword", 0));
        assert!(has(4, "string", 0) && has(1, "number", 0) && has(1, "operator", 0));
    }

    #[test]
    fn publication_cpl_is_served_like_any_other() {
        let text = "l\u{332}e\u{332}t\u{332} x = 1\n\u{a7}1 i\u{332}f\u{332} x \u{2260} 2 g\u{332}o\u{332} t\u{332}o\u{332} L\n   Write[x] \u{a7}\u{338}1\nL: f\u{332}i\u{332}n\u{332}i\u{332}s\u{332}h\u{332}\n";
        let (mut server, out) = server_with(text);
        assert_eq!(out[0]["params"]["diagnostics"], json!([]));
        assert!(hover_text(&mut server, 0, 2).starts_with("**`let`** definition"));
        assert!(hover_text(&mut server, 1, 17).starts_with("**`go to`** jump"));
        // An underlined letter is two UTF-16 units, and so is the struck-through section sign.
        let label = ask(&mut server, "textDocument/definition", 1, 24, json!({}));
        assert_eq!(label["result"]["range"]["start"], json!({ "line": 3, "character": 0 }));
        let close = hover_text(&mut server, 2, 13);
        assert!(close.starts_with("**Closing section bracket, tagged `1`**"), "{close}");
        let tokens = ask(&mut server, "textDocument/semanticTokens/full", 0, 0, json!({}));
        assert_eq!(&tokens["result"]["data"].as_array().unwrap()[..5], [0, 0, 6, 7, 0]);
    }

    #[test]
    fn positions_count_utf16_units() {
        let (mut server, out) = server_with("Write['\u{1F600} \u{e9}'] ? 1\n");
        let range = &out[0]["params"]["diagnostics"][0]["range"];
        assert_eq!(range["start"]["character"], 14);
        let hover = hover_text(&mut server, 0, 14);
        assert!(hover.contains("means nothing in CPL"), "{hover}");
    }

    #[test]
    fn messages_are_framed_with_a_content_length() {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#;
        let framed = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let mut input = io::BufReader::new(framed.as_bytes());
        let message = read_message(&mut input).unwrap().unwrap();
        assert_eq!(message["method"], "shutdown");
        assert!(read_message(&mut input).unwrap().is_none());
    }
}
