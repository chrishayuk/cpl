//! `cpl --check` and the analysis behind it (src/analysis.rs), tried on the
//! programs in examples/, appendix1/ and the tutorial.
//!
//! The check must never disagree with the system: a program has an error
//! here exactly when the system refuses to compile it, and the message is
//! the same one.

use cpl::analysis::{analyse, line_column, DefKind, Kind, Severity};
use std::path::{Path, PathBuf};

/// The package directory. Inside a WASI runtime only the current directory
/// can be reached, and cargo runs tests from the package directory.
fn root() -> PathBuf {
    if cfg!(target_family = "wasm") {
        PathBuf::from(".")
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
    }
}

fn files(dir: &str, extension: &str) -> Vec<PathBuf> {
    let mut found: Vec<_> = std::fs::read_dir(root().join(dir))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == extension))
        .collect();
    found.sort();
    found
}

/// What `cpl --check` prints for a program.
fn report(name: &str, src: &str) -> Vec<String> {
    analyse(src)
        .diagnostics
        .iter()
        .map(|d| {
            let (line, column) = line_column(src, d.start);
            format!("{}:{}:{}: {}: {}", name, line, column, d.severity.name(), d.message)
        })
        .collect()
}

/// The programs written out in the tutorial, fragments included.
fn tutorial_programs() -> Vec<String> {
    let mut out = Vec::new();
    for path in files("tutorial", "md") {
        let text = std::fs::read_to_string(&path).unwrap();
        let mut program: Option<String> = None;
        for line in text.split_inclusive('\n') {
            match (&mut program, line.trim_end()) {
                (None, "```cpl") => program = Some(String::new()),
                (Some(_), "```") => out.extend(program.take()),
                (Some(text), _) => text.push_str(line),
                _ => {}
            }
        }
    }
    out
}

/// No example may have anything wrong with it but the one that is there to
/// show a mistake the compiler finds, and none defines a name it never uses.
#[test]
fn examples_pass_the_static_check() {
    let mut programs = files("examples", "cpl");
    assert!(programs.len() >= 60, "example programs are missing");
    programs.push(root().join("appendix1/preprocessor.cpl"));
    let mut lines = Vec::new();
    for path in programs {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        lines.extend(report(&name, &std::fs::read_to_string(&path).unwrap()));
    }
    assert_eq!(
        lines,
        [
            "fixed-and-free.cpl:23:25: error: Count is a free variable of the fixed function Bad, \
          which may not assign to it (a function defined with == is free and may)"
        ]
    );
}

/// Appendix 1 as it was printed has mismatched bracket tags. Each is
/// reported where it is, not as one message at the end of the program.
#[test]
fn the_slips_of_1966_are_each_found() {
    let src = std::fs::read_to_string(root().join("appendix1/preprocessor-as-printed.cpl")).unwrap();
    let lines = report("as-printed", &src);
    assert_eq!(lines.len(), 8, "{:#?}", lines);
    assert_eq!(
        lines[0],
        "as-printed:135:4: error: the section bracket tagged R.T is never closed"
    );
    assert_eq!(
        lines[2],
        "as-printed:141:45: error: no open section bracket is tagged R.T."
    );
    // The system itself stops at the first of them.
    let first = cpl::compile_source(&src).unwrap_err();
    assert_eq!(first, "line 141: no open section bracket is tagged R.T.");
}

#[test]
fn the_check_agrees_with_the_system() {
    let mut programs: Vec<String> = files("examples", "cpl")
        .into_iter()
        .chain(files("appendix1", "cpl"))
        .chain(files("appendix1", "pub"))
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    let written = programs.len();
    programs.extend(tutorial_programs());
    assert!(programs.len() >= written + 200, "tutorial programs are missing");
    for broken in [
        "let x = ",
        "let count = 0",
        "Write['abc]",
        "Write['a*q']",
        "x := #z",
        "x := #o|777777777",
        "x := 1 ? 2",
        "x := 1 . 2",
        "Write[1] $)",
        "$(A Write[1] $)B",
        "$( $( Write[1] $)",
        "x\u{332}y\u{332} := 1",
        "Write[y]",
        "L: L: finish",
        "let x, y be real, real, real",
        "rhs := 1",
        "",
    ] {
        programs.push(broken.to_string());
    }
    let mut wrong = 0;
    for src in &programs {
        let found = analyse(src);
        let errors: Vec<_> = found
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        match cpl::compile_source(src) {
            Ok(_) => assert!(errors.is_empty(), "{}\n{:?}", src, errors),
            Err(message) => {
                wrong += 1;
                assert!(!errors.is_empty(), "{}\nthe system says: {}", src, message);
                // Brackets left open are reported one by one, in other words.
                if message.contains("left open") {
                    assert!(
                        errors.iter().all(|d| d.code != "syntax" && d.code != "meaning"),
                        "{}",
                        src
                    );
                    assert!(errors.iter().any(|d| d.code == "unclosed-bracket"), "{}", src);
                    continue;
                }
                let text = message.split_once(": ").unwrap().1;
                let same = errors.iter().find(|d| d.message == text);
                let same = same.unwrap_or_else(|| panic!("{}\nthe system says: {}\n{:?}", src, message, errors));
                // And it is on the line the system names.
                let line: usize = message["line ".len()..].split(':').next().unwrap().parse().unwrap();
                let at = line_column(src, same.start).0;
                let last = src.lines().count().max(1);
                assert!(
                    at == line || (line > last && at == last),
                    "{}\n{} is put on line {}",
                    src,
                    message,
                    at
                );
            }
        }
    }
    assert!(wrong >= 25, "only {} programs with mistakes were tried", wrong);
}

/// The messages of chapter 19 of the tutorial, each put where it belongs.
#[test]
fn messages_you_will_meet() {
    for (src, wanted, text) in [
        (
            "let count = 0",
            "program:1:6: error: '=', '~~', '==' or 'be' expected in a definition (found the name o)",
            "o",
        ),
        (
            "let f[x] = x = 0 -> 1, x f[x - 1]\nWrite[f[3]]",
            "program:1:26: error: the name f is not defined here",
            "f",
        ),
        (
            "for i = 1 to 3 do\n   $( Write[i]\n",
            "program:2:4: error: this section bracket is never closed",
            "$(",
        ),
        (
            "Write['one', 'two]",
            "program:1:14: error: string constant has no closing quote",
            "'two]",
        ),
    ] {
        let found = analyse(src);
        assert_eq!(report("program", src), [wanted]);
        assert_eq!(&src[found.diagnostics[0].start..found.diagnostics[0].end], text);
    }
}

/// The definitions the parser notes, on programs from the documents.
#[test]
fn definitions_in_the_programs_of_the_manuals() {
    let src = std::fs::read_to_string(root().join("examples/manual-lineq.cpl")).unwrap();
    let found = analyse(&src);
    let lineq = found.definitions.iter().find(|d| d.name == "LINEQ").unwrap();
    assert_eq!(lineq.kind, DefKind::Routine);
    let params: Vec<&str> = lineq
        .params
        .iter()
        .map(|&p| found.definitions[p].name.as_str())
        .collect();
    assert_eq!(params, ["a", "b", "c", "a'", "b'", "c'", "x", "y", "L"]);
    // DET is defined in the body, and used three times there.
    let det = found.definitions.iter().find(|d| d.name == "DET").unwrap();
    assert_eq!((det.kind, det.uses.len()), (DefKind::Variable, 3));
    assert!(lineq.extent.start < det.extent.start && det.extent.end <= lineq.extent.end);

    // Every name in every example means something: a definition or a basic function.
    for path in files("examples", "cpl") {
        let src = std::fs::read_to_string(&path).unwrap();
        let found = analyse(&src);
        if found.has_errors() {
            continue;
        }
        for t in found.tokens.iter().filter(|t| t.kind == Kind::Name) {
            let name = &src[t.start..t.end];
            assert!(
                t.def.is_some() || cpl::analysis::builtin(name).is_some(),
                "{}: {} at {:?} means nothing",
                path.display(),
                name,
                line_column(&src, t.start)
            );
        }
    }
}

/// Publication CPL: the examples as `cpl --publication` prints them have
/// the same definitions and the same uses as the examples themselves.
#[test]
fn publication_form_is_analysed_like_the_ascii_form() {
    for path in files("examples", "cpl") {
        let src = std::fs::read_to_string(&path).unwrap();
        let printed = cpl::lexer::publication(&src);
        let (plain, underlined) = (analyse(&src), analyse(&printed));
        let shape = |a: &cpl::analysis::Analysis| -> Vec<(String, DefKind, usize)> {
            a.definitions
                .iter()
                .map(|d| (d.name.clone(), d.kind, d.uses.len()))
                .collect()
        };
        assert_eq!(shape(&plain), shape(&underlined), "{}", path.display());
        assert_eq!(plain.has_errors(), underlined.has_errors(), "{}", path.display());
    }
}

#[cfg(not(target_family = "wasm"))]
mod command_line {
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn check(args: &[&str], input: &str) -> (i32, String, String) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cpl"))
            .arg("--check")
            .args(args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8(out.stdout).unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    }

    #[test]
    fn a_program_with_nothing_wrong_prints_nothing() {
        assert_eq!(
            check(&["examples/hello.cpl", "examples/queens.cpl"], ""),
            (0, String::new(), String::new())
        );
    }

    #[test]
    fn mistakes_are_printed_with_file_line_and_column_and_the_status_is_one() {
        let (status, out, err) = check(&["examples/hello.cpl", "examples/fixed-and-free.cpl"], "");
        assert_eq!(status, 1);
        assert_eq!(
            out,
            "examples/fixed-and-free.cpl:23:25: error: Count is a free variable of the fixed function Bad, \
             which may not assign to it (a function defined with == is free and may)\n"
        );
        assert_eq!(err, "");
    }

    #[test]
    fn the_program_may_come_from_standard_input() {
        let (status, out, _) = check(&["-"], "let count = 0\n");
        assert_eq!(status, 1);
        assert_eq!(
            out,
            "program:1:6: error: '=', '~~', '==' or 'be' expected in a definition (found the name o)\n"
        );
    }

    #[test]
    fn a_hint_is_printed_but_is_not_a_failure() {
        let (status, out, _) = check(&["-"], "let x, y = 1, 2\nWrite[x]\n");
        assert_eq!(
            (status, out.as_str()),
            (0, "program:1:8: hint: y is defined but never used\n")
        );
    }

    #[test]
    fn nothing_is_run() {
        // This program would never end, and would print for ever.
        let (status, out, _) = check(&["-"], "Write['again*n'] repeat\n");
        assert_eq!((status, out.as_str()), (0, ""));
    }

    #[test]
    fn a_file_that_cannot_be_read_is_status_two() {
        let (status, out, err) = check(&["no-such-file.cpl"], "");
        assert_eq!((status, out.as_str()), (2, ""));
        assert!(err.starts_with("cpl: cannot read no-such-file.cpl"), "{}", err);
        let (status, _, err) = check(&[], "");
        assert_eq!(status, 2);
        assert!(err.starts_with("cpl: --check needs a file"), "{}", err);
    }
}
