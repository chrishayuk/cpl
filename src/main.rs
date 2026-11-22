use std::io::{BufReader, BufWriter};
use std::process::exit;

const USAGE: &str = "Usage: cpl [OPTIONS] <FILE.cpl>
       cpl [OPTIONS] -e <PROGRAM>
       cpl [OPTIONS] -            (the program is read from standard input)

Options:
  -e, --eval <PROGRAM> Run the program text given on the command line
      --tokens         Print the basic symbols instead of running
      --ast            Print the syntax tree instead of running
      --code           Print the abstract machine code instead of running
      --max-steps <N>  Stop with an error after N machine steps
  -h, --help           Show this help
  -V, --version        Show the version";

fn main() {
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
