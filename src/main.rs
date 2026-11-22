use std::io::{BufReader, BufWriter};
use std::process::exit;

const USAGE: &str = "Usage: cpl [OPTIONS] <FILE.cpl>
       cpl [OPTIONS] -e <PROGRAM>
       cpl [OPTIONS] -            (the program is read from standard input)
       cpl --check <FILE.cpl>...  (- for standard input)

Options:
  -e, --eval <PROGRAM> Run the program text given on the command line
      --tokens         Print the basic symbols instead of running
      --ast            Print the syntax tree instead of running
      --code           Print the abstract machine code instead of running
      --publication    Print the program as it would be printed: underlined
                       basic symbols, section signs, mathematical signs
      --max-steps <N>  Stop with an error after N machine steps
      --check          Report what is wrong with each program, with line and
                       column, without running it
  -h, --help           Show this help
  -V, --version        Show the version";

/// `cpl --check FILE...`: report mistakes without running anything. The
/// status is 1 if any program has an error, and 2 if one cannot be read.
fn check(files: &[String]) -> i32 {
    if files.is_empty() {
        eprintln!("cpl: --check needs a file\n\n{}", USAGE);
        return 2;
    }
    let mut status = 0;
    for file in files {
        let (name, read) = if file == "-" {
            ("program", std::io::read_to_string(std::io::stdin()))
        } else {
            (file.as_str(), std::fs::read_to_string(file))
        };
        let src = match read {
            Ok(src) => src,
            Err(e) => {
                eprintln!("cpl: cannot read {}: {}", file, e);
                status = 2;
                continue;
            }
        };
        let result = cpl::analysis::analyse(&src);
        for d in &result.diagnostics {
            let (line, column) = cpl::analysis::line_column(&src, d.start);
            println!("{}:{}:{}: {}: {}", name, line, column, d.severity.name(), d.message);
        }
        if result.has_errors() && status == 0 {
            status = 1;
        }
    }
    status
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--check") {
        exit(check(&args[1..]));
    }
    let mut file = None;
    let mut text = None;
    let (mut tokens, mut ast, mut code, mut max_steps) = (false, false, false, None);
    let mut publication = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--tokens" => tokens = true,
            "--ast" => ast = true,
            "--code" => code = true,
            "--publication" => publication = true,
            "--max-steps" => match it.next().and_then(|n| n.parse().ok()) {
                Some(n) => max_steps = Some(n),
                None => {
                    eprintln!("cpl: --max-steps needs a number");
                    exit(2);
                }
            },
            "-e" | "--eval" => match it.next() {
                Some(p) => text = Some(p),
                None => {
                    eprintln!("cpl: -e needs a program");
                    exit(2);
                }
            },
            "-h" | "--help" => {
                println!("{}", USAGE);
                return;
            }
            "-V" | "--version" => {
                println!("cpl {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            f if (f == "-" || !f.starts_with('-')) && file.is_none() && text.is_none() => file = Some(f.to_string()),
            other => {
                eprintln!("cpl: unexpected argument {}\n\n{}", other, USAGE);
                exit(2);
            }
        }
    }
    let (file, src) = match (text, file) {
        (Some(text), None) => ("program".to_string(), text),
        (None, Some(file)) => {
            let read = if file == "-" {
                std::io::read_to_string(std::io::stdin())
            } else {
                std::fs::read_to_string(&file)
            };
            match read {
                Ok(s) => (if file == "-" { "program".to_string() } else { file }, s),
                Err(e) => {
                    eprintln!("cpl: cannot read {}: {}", file, e);
                    exit(2);
                }
            }
        }
        _ => {
            eprintln!("{}", USAGE);
            exit(2);
        }
    };
    if publication {
        print!("{}", cpl::lexer::publication(&src));
        return;
    }
    let status = cpl::on_big_stack(move || {
        let fail = |e: String| {
            eprintln!("{}: {}", file, e);
            1
        };
        let toks = match cpl::lexer::lex(&src) {
            Ok(t) => t,
            Err(e) => return fail(e),
        };
        if tokens {
            for t in &toks {
                println!("{:4}  {:?}", t.line, t.tok);
            }
            return 0;
        }
        let items = match cpl::parser::parse(toks) {
            Ok(i) => i,
            Err(e) => return fail(e),
        };
        if ast {
            println!("{:#?}", items);
            return 0;
        }
        let proto = match cpl::compile::compile(&items) {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        if code {
            let mut s = String::new();
            cpl::code::listing(&proto, &mut s);
            print!("{}", s);
            return 0;
        }
        let mut vm = cpl::vm::Vm::new(
            Box::new(BufWriter::new(std::io::stdout())),
            Box::new(BufReader::new(std::io::stdin())),
        );
        vm.max_steps = max_steps;
        match vm.execute(proto) {
            Ok(()) => 0,
            Err(e) => fail(e),
        }
    });
    exit(status);
}
