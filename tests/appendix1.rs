//! Appendix 1 of the Reference Manual draft: the preprocessor, "a general
//! example of a CPL program", and the only large one in the documents.
//!
//! appendix1/preprocessor.cpl is the program with its slips put right, and
//! appendix1/preprocessor-as-printed.cpl has them as printed. The table
//! below is the whole difference between the two, and for each slip says
//! what goes wrong when it alone is put back.
//!
//! Run with CPL_BLESS=1 to rewrite the generated files in appendix1/.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    if cfg!(target_family = "wasm") {
        PathBuf::from(".")
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
    }
}

fn read(name: &str) -> String {
    std::fs::read_to_string(root().join("appendix1").join(name)).unwrap_or_default()
}

/// Whether the slip is found when the program is compiled or when it runs.
enum Seen {
    Compiling(&'static str),
    Running(&'static str),
}
use Seen::*;

/// (corrected text, printed text, what happens with the printed text).
const SLIPS: &[(&str, &str, Seen)] = &[
    (
        "$)R.T             || printed: R.T.",
        "$)R.T.",
        Compiling("no open section bracket is tagged R.T."),
    ),
    (
        "$)R.U.W        || printed: R.U.W.",
        "$)R.U.W.",
        Compiling("no open section bracket is tagged R.U.W."),
    ),
    ("$(R.S ", "$(R.S.", Compiling("no open section bracket is tagged R.S")),
    (
        "Preprocessor['§', s, Mismatch]",
        "Proprocessor['§', s, Mismatch]",
        Compiling("the name Proprocessor is not defined here"),
    ),
    ("case ')':", "case ')' ", Compiling("':' expected (found 'unless')")),
    (
        "goto Switch $)\n                  Output[':']",
        "goto switch $)\n                  Output[':']",
        Compiling("an expression expected (found 'switch')"),
    ),
    (
        "thendo s := Standard[s <=> s']",
        "thendo s := standard[s <=> s']",
        Compiling("the name t is not defined here"),
    ),
    // The first letter of every underlined word is taken twice.
    (
        "s := ''                       || printed: s := c",
        "s := c",
        Running("l\u{332}letøN\u{338}aø,øN\u{338}bø=øK\u{338}1ø,øK\u{338}2øi\u{332}ifø"),
    ),
    // The character after every comma is lost.
    (
        "$( Output[',']\n",
        "$( Output[',']; NextChar[c]\n",
        Running("letøN\u{338}aø,ø=øK\u{338}1ø,ødo øifø"),
    ),
    // Part 3 (c) lists what CAN precede a command; Part 2 needs the others.
    (
        "~Among[x, List[';', '§', 'do', ':', 'or', '']]    || printed: the list, without the 'not'",
        "Among[x, List[';', '§', 'do', ':', 'or']]",
        Running("K\u{338}2ø; ødo øiføN\u{338}aø>øN\u{338}bøgotoø"),
    ),
    // Part 3 (f) makes go of goto; (b) and (d) expect goto.
    (
        "|| printed: is go if x is goto, which (b) and (d) do not expect",
        "case 'goto': result is 'go'",
        Running("øiføN\u{338}aø>øN\u{338}bøgoøN\u{338}Lø"),
    ),
    ("'to', 'goto', 'break'", "'to', 'go', 'break'", Running("")),
];

const TRIAL: &str = "l\u{332}e\u{332}t\u{332} a, b = 1, 2; i\u{332}f\u{332} a > b g\u{332}o\u{332}t\u{332}o\u{332} L\n";
const TRIAL_OUT: &str =
    "letøN\u{338}aø,øN\u{338}bø=øK\u{338}1ø,øK\u{338}2ø; øiføN\u{338}aø>øN\u{338}bødo øgotoøN\u{338}Lø";

fn run(program: &str, input: &str) -> (String, Result<(), String>) {
    cpl::run_source(program, input, Some(200_000_000))
}

#[test]
fn the_corrected_preprocessor_runs() {
    let (out, r) = run(&read("preprocessor.cpl"), TRIAL);
    assert_eq!(r, Ok(()));
    assert_eq!(out, TRIAL_OUT);
}

#[test]
fn as_printed_is_the_corrected_text_with_the_slips_put_back() {
    let fixed = read("preprocessor.cpl");
    let mut printed = fixed.clone();
    for (good, bad, _) in SLIPS {
        assert_eq!(printed.matches(good).count(), 1, "{:?} should occur once", good);
        printed = printed.replace(good, bad);
    }
    // Only Parts 2 and 3 are compared: the two files have their own headings.
    let body = |t: &str| t[t.find("|| -----").unwrap()..].to_string();
    assert_eq!(body(&printed), body(&read("preprocessor-as-printed.cpl")));
    // As printed, it does not get as far as running.
    let (_, r) = run(&read("preprocessor-as-printed.cpl"), TRIAL);
    assert!(r.unwrap_err().contains("no open section bracket is tagged R.T."));
}

#[test]
fn each_slip_alone_does_what_the_table_says() {
    let fixed = read("preprocessor.cpl");
    for (good, bad, seen) in SLIPS {
        let (out, r) = run(&fixed.replace(good, bad), TRIAL);
        match seen {
            Compiling(message) => {
                let e = r.expect_err(bad);
                assert!(e.contains(message), "{:?} gave {}", bad, e);
            }
            Running(part) => {
                assert_eq!(r, Ok(()), "{:?}", bad);
                assert!(out.contains(part), "{:?} gave {}", bad, out);
                assert_eq!(part.is_empty(), out == TRIAL_OUT, "{:?} gave {}", bad, out);
            }
        }
    }
}

/// Part 2 of a file, as it would be printed.
fn part2(name: &str) -> String {
    let text = read(name);
    cpl::lexer::publication(&text[text.find("let c, s, s' all be string").unwrap()..])
}

fn check(name: &str, got: &str) {
    if std::env::var("CPL_BLESS").is_ok() {
        std::fs::write(root().join("appendix1").join(name), got).unwrap();
    }
    assert_eq!(read(name), got, "appendix1/{}", name);
}

#[test]
fn the_preprocessor_preprocesses_a_program() {
    let (out, r) = run(&read("preprocessor.cpl"), &read("sample.pub"));
    assert_eq!(r, Ok(()));
    check("sample.out", &out);
}

#[test]
fn the_preprocessor_preprocesses_itself() {
    let text = part2("preprocessor.cpl");
    check("itself.pub", &text);
    let (out, r) = run(&read("preprocessor.cpl"), &text);
    assert_eq!(r, Ok(()));
    assert!(
        out.ends_with("øN\u{338}Errorø:øfinishø"),
        "it should reach its own last symbol"
    );
    check("itself.out", &out);
}

#[test]
fn given_its_own_printed_text_it_stops_at_the_first_mismatched_tag() {
    let text = part2("preprocessor-as-printed.cpl");
    check("itself-as-printed.pub", &text);
    let (out, r) = run(&read("preprocessor.cpl"), &text);
    assert_eq!(r, Ok(()));
    // The closing bracket of ReadTag is tagged R.T. and its opening one R.T,
    // so every open section is closed and the mismatch reaches the label
    // Error of the main program, which is "finish".
    assert!(
        out.ends_with("øN\u{338}NextCharø[øN\u{338}cø]ø§\u{338}ø§\u{338}ø§\u{338}ø"),
        "{}",
        out
    );
    check("itself-as-printed.out", &out);
}

#[test]
fn publication_form_means_the_same_as_the_ascii_form() {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root().join("examples"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "cpl"))
        .collect();
    files.push(root().join("appendix1").join("preprocessor.cpl"));
    assert!(files.len() > 70);
    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let toks =
            |s: &str| -> Vec<cpl::lexer::Tok> { cpl::lexer::lex(s).unwrap().into_iter().map(|t| t.tok).collect() };
        assert_eq!(toks(&src), toks(&cpl::lexer::publication(&src)), "{}", path.display());
    }
}
