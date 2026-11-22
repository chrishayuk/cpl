#!/usr/bin/env python3
"""Build dataset/cpl.jsonl, a chat-format training set about CPL.

Every program in an answer is run through the real system, and the output
shown in the answer is what it actually printed, so the dataset cannot drift
from the language. A program that fails when it should not, or runs when it
is there to show a mistake, stops the build. Run after building the system:

    cargo build --release
    python3 tutorial/build_dataset.py            rebuild the file
    python3 tutorial/build_dataset.py --check    fail if a rebuild would change it

The system is taken from $CPL, or from ../target.
"""
import json
import os
import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent
EXAMPLES = ROOT / "examples"
OUT = HERE / "dataset" / "cpl.jsonl"
SYSTEM = ("You are a knowledgeable assistant that provides detailed information "
          "about the CPL programming language.")
rows = []


def interpreter():
    if os.environ.get("CPL"):
        return os.environ["CPL"]
    for build in ("release", "debug"):
        for name in ("cpl", "cpl.exe"):
            path = ROOT / "target" / build / name
            if path.exists():
                return str(path)
    sys.exit("no cpl found: run `cargo build --release` or set $CPL")


CPL = interpreter()


def cpl(args, stdin="", cwd=ROOT):
    """Run the system; gives the exit status, standard output and standard error."""
    result = subprocess.run([CPL, *args], input=stdin, capture_output=True,
                            text=True, encoding="utf-8", timeout=60, cwd=cwd)
    return result.returncode, result.stdout, result.stderr


def block(text, lang=""):
    return f"```{lang}\n" + text.strip("\n") + "\n```"


def given(stdin):
    """The words between a program and its output."""
    if not stdin:
        return "It prints:\n\n"
    if "\n" in stdin.strip("\n"):
        return "Given this input:\n\n" + block(stdin) + "\n\nit prints:\n\n"
    return f"With the input `{stdin.strip()}` it prints:\n\n"


def prog(src, stdin=""):
    """A program, its input if any, and its real output, as markdown."""
    src = src.strip("\n")
    status, out, err = cpl(["-e", src], stdin)
    if status != 0:
        sys.exit(f"program failed:\n{src}\n{err}")
    return block(src, "cpl") + "\n\n" + given(stdin) + block(out.rstrip("\n"))


def fails(src, stdin=""):
    """A program that is meant to stop with an error, and its real message."""
    src = src.strip("\n")
    status, out, err = cpl(["-e", src], stdin)
    if status != 1 or not err.strip():
        sys.exit(f"program was expected to fail and did not:\n{src}\n{out}")
    text = block(src, "cpl") + "\n\n"
    if out.strip():
        text += given(stdin) + block(out.rstrip("\n")) + "\n\nand then stops with this message:\n\n"
    else:
        text += "It stops with this message:\n\n"
    return text + block(err)


def tool(args, stdin="", status=0):
    """The real standard output of a command that is not a plain run."""
    got, out, err = cpl(args, stdin)
    if got != status or not out.strip():
        sys.exit(f"cpl {' '.join(args)}: exit status {got}, expected {status}\n{err}")
    return block(out.rstrip("\n"))


def check(name, src):
    """What `cpl --check` really says about a broken program."""
    src = src.strip("\n") + "\n"
    with tempfile.TemporaryDirectory() as work:
        (pathlib.Path(work) / name).write_text(src, encoding="utf-8")
        status, out, err = cpl(["--check", name], cwd=work)
    if status != 1 or not (out + err).strip():
        sys.exit(f"--check was expected to find something wrong:\n{src}")
    return block((out + err).rstrip("\n"))


def example(name):
    """An example program run from its file, checked against its .out file."""
    src = (EXAMPLES / f"{name}.cpl").read_text(encoding="utf-8")
    want = (EXAMPLES / f"{name}.out").read_text(encoding="utf-8")
    stdin_file = EXAMPLES / f"{name}.in"
    stdin = stdin_file.read_text(encoding="utf-8") if stdin_file.exists() else ""
    status, out, err = cpl([f"examples/{name}.cpl"], stdin)
    meant_to_fail = want.rstrip("\n").split("\n")[-1].startswith("! ")
    if (status != 0) != meant_to_fail:
        sys.exit(f"examples/{name}.cpl: exit status {status}\n{err}")
    if not want.startswith(out):
        sys.exit(f"examples/{name}.cpl does not print what examples/{name}.out says")
    text = block(src, "cpl") + "\n\n"
    if not meant_to_fail:
        return text + given(stdin) + block(out.rstrip("\n"))
    if not out.strip():
        return (text + "The mistake in it is found when the program is compiled, so it "
                "prints nothing and, on purpose, stops with this message:\n\n" + block(err))
    return (text + given(stdin) + block(out.rstrip("\n"))
            + "\n\nand then, on purpose, it stops with this message:\n\n" + block(err))


def qa(question, answer):
    rows.append({"messages": [
        {"role": "system", "content": SYSTEM},
        {"role": "user", "content": question},
        {"role": "assistant", "content": answer.strip()},
    ]})


HCF = r"""
let rec HCF[integer n, m] =
        (m > n -> HCF[m, n],
         m = 0 -> n,
         HCF[m, Rem[n, m]])

Write['HCF[1071, 462] = ', HCF[1071, 462], '*n']
"""

# ------------------------------------------------------ what CPL is

qa("What is the CPL programming language?",
   "CPL is the Combined Programming Language, designed at Cambridge and London "
   "between 1962 and 1966 by Christopher Strachey, David Barron, John Buxton, David "
   "Hartley, Eric Nixon and others. It is the ancestor of BCPL, and through BCPL of "
   "B and C. The whole language was never implemented at the time: only subsets "
   "were, and the reference manual was still a draft when the project ended. The "
   "`cpl` system described here was written from the documents, chiefly the "
   "unfinished Reference Manual in the CPL Working Papers of July 1966. Here is a "
   "complete program, the Euclidean algorithm from section 24 of the Elementary "
   "Programming Manual:\n\n" + prog(HCF))

qa("What does CPL stand for?",
   "Combined Programming Language. It was designed between 1962 and 1966 by a group "
   "at the University Mathematical Laboratory in Cambridge and the University of "
   "London Institute of Computer Science.")

qa("Who designed CPL, and when?",
   "CPL was designed between 1962 and 1966 by Christopher Strachey, David Barron, "
   "John Buxton, David Hartley, Eric Nixon and others, working at the University "
   "Mathematical Laboratory in Cambridge and the University of London Institute of "
   "Computer Science. The first published account is \"The main features of CPL\" in "
   "The Computer Journal, volume 6, 1963, by Barron, Buxton, Hartley, Nixon and "
   "Strachey. The Reference Manual, edited by Strachey, was circulated as a draft in "
   "the CPL Working Papers of July 1966. There was no later edition.")

qa("What was CPL designed for?",
   "It was to be the language for two new machines, Titan at Cambridge and Atlas in "
   "London, and it was meant to do everything: numerical work like ALGOL 60, on "
   "which it was based, and also the non-numerical work that ALGOL could not do.")

qa("Was CPL ever fully implemented?",
   "Not in the 1960s. The London compiler was for a subset, CPL1, with \"no types "
   "general, complex or double length\" and no lists. The Cambridge compiler was "
   "written in CPL itself and translated by hand into the macros of Strachey's "
   "General Purpose Macrogenerator; Martin Richards, who worked on it, wrote later "
   "that it \"did not result [in] a satisfactory implementation\". The language was "
   "too big for the machines and the compiler writers of its day, and the reference "
   "manual was still a draft when the project ended. The `cpl` system in this "
   "project is a modern implementation written from that manual. Programs that "
   "use the whole language could not have been run on any system of the time; "
   "whether any has been run since, on another implementation, has not been "
   "looked into.")

qa("How is CPL related to BCPL, B and C?",
   "CPL is the ancestor of all three. Martin Richards, who worked on the Cambridge "
   "CPL compiler, took what he had learnt and designed a small language for writing "
   "compilers in: \"Basic CPL\", or BCPL, which he described as \"CPL with all the "
   "difficult bits removed\". Ken Thompson cut BCPL down to make B, and Dennis "
   "Ritchie built B up again to make C. Much of what a C programmer takes for "
   "granted reached C by way of CPL. Some of it CPL had from ALGOL 60: assignment "
   "with `:=`, `if`, `for` and `while`. Some of it is not in ALGOL 60 and came to "
   "C from CPL: Strachey's L-value and R-value, `break`, `return` and the block as "
   "an expression. `switch` and `case` came by way of BCPL: Richards says "
   "`SWITCHON` was put into BCPL and retro-fitted into CPL in 1966. The curly "
   "brackets descend from CPL's section brackets.")

qa("What documents is this CPL implementation based on?",
   "Chiefly the unfinished Reference Manual in the CPL Working Papers (University "
   "of London Institute of Computer Science and the Mathematical Laboratory, "
   "Cambridge, July 1966), which is the authority. It also uses the CPL Elementary "
   "Programming Manual, Edition II (Cambridge, January 1966) with its supplement for "
   "the pre-pilot system; \"The main features of CPL\" by Barron, Buxton, Hartley, "
   "Nixon and Strachey (The Computer Journal 6, 1963); and Martin Richards' \"How "
   "BCPL evolved from CPL\" (2011). \"The London CPL1 compiler\" by Coulouris, "
   "Goodey, Hill, Keeling and Levin was not used for the language definition but "
   "confirms two readings. No CPL compiler survives to test against, so the "
   "implementation is faithful to the documents and to nothing else.")

qa("What kind of implementation is the cpl system?",
   "It is a compiler to an abstract stack machine, and an interpreter for that "
   "machine, written in Rust with no dependencies. The source goes through a lexer "
   "to basic symbols, a parser to a tree, and a compiler to machine code that a "
   "virtual machine runs. The Cambridge compiler of 1965 also translated CPL for a "
   "stack machine, in which a function was a pointer to its code combined with a "
   "pointer to its free variable list, and the machine here has the same shape. "
   "`cpl --code` prints the code a program is compiled to:\n\n"
   + block("cpl --code -e \"Write[2 + 3]\"", "bash") + "\n\n"
   + tool(["--code", "-e", "Write[2 + 3]"]))

qa("Which features of this CPL come from the 1963 paper and not the 1966 manual?",
   "Three. Call by substitution (ALGOL's call by name) and definition by "
   "substitution, which by Richards' account were removed by mid 1966. Lists, with "
   "`Cons`, `Hd`, `Tl`, `Null`, `List` and `Members`; the 1966 manual defers all "
   "compound data structures. And a list of names matched against a list value, "
   "where \"the transfer function Members is automatically invoked\". They were "
   "added because the manual leaves a hole where they were. Where the 1963 paper and "
   "the 1966 manual disagree, 1966 wins: for example brackets round part of an "
   "expression list have no significance, although in 1963 they made a sublist.")

qa("How was CPL originally written down, and how do I type it in ASCII?",
   "CPL was written on a Flexowriter, a typewriter that punched paper tape, with "
   "162 printing characters because it could underline and overstrike. The words of "
   "the language were underlined, and the section brackets were a section sign § "
   "and the same sign struck through. In this implementation: underlined words are "
   "reserved words in small letters (`let`, `value of`); the section brackets are "
   "`$(` and `$)`, or `{` and `}`; `≃` (by reference) is `~~`; `≡` is `==`; `→` is "
   "`->`; `×` is `*`; `↑` is `**` or `^`; `≠ ≤ ≥` are `~=`, `<=`, `>=`; `∧ ∨ ∼` are "
   "`/\\`, `\\/` and `~`; `≢` is `~==`; the subscript ten is `@`, as in `2.5@-8`; "
   "logical constants are `#o777`, `#b1010` and `#x1FF`; and a comment is `||` to "
   "the end of the line. The Unicode forms are accepted too, and so is publication "
   "CPL itself.")

# ------------------------------------------------------ running things

qa("How do I install CPL?",
   "You need Rust. To install the system and the language server:\n\n"
   + block("cargo install --git https://github.com/chrishayuk/cpl cpl cpl-lsp", "bash")
   + "\n\nThat puts `cpl` and `cpl-lsp` in `~/.cargo/bin`. In a clone of the "
   "repository, `cargo build --release` builds `target/release/cpl`, and "
   "`cargo install --path .` installs it. Then try:\n\n"
   + block("cpl -e \"Write['Hello, world!*n']\"", "bash") + "\n\n"
   + tool(["-e", "Write['Hello, world!*n']"]) +
   "\n\nTo install nothing at all, there is a playground that runs CPL in the "
   "browser at https://chrishayuk.com/demos/cpl.")

qa("How do I run a CPL program?",
   "A CPL program is a text file, usually ending in `.cpl`. Give the file name to "
   "`cpl`:\n\n" + block("cpl examples/hello.cpl", "bash") + "\n\n"
   + tool(["examples/hello.cpl"]) +
   "\n\nThere are two other ways to hand over a program. `-e` takes the program "
   "text itself, and `-` reads the program from standard input:\n\n"
   + block("cpl -e \"Write[6 * 7]\"\necho \"Write[6 * 7]\" | cpl -", "bash") +
   "\n\nEach of those prints:\n\n" + tool(["-e", "Write[6 * 7]"]))

qa("What command-line options does cpl have?",
   "`cpl --help` prints them:\n\n" + tool(["--help"]) +
   "\n\n`Read` and `Readchar` take the program's input from standard input.")

qa("What exit status does cpl return?",
   "0 when the program finishes, 1 for an error in the program, and 2 for a "
   "mistake on the command line. An error in the program is reported on standard "
   "error with its line number.")

qa("How does CPL report an error when a program goes wrong?",
   "The program stops, and a message giving the line is written to standard error; "
   "the exit status is 1. Anything printed before the error has already appeared. "
   "The message begins with the name of the file, or with `program` when the text "
   "was given with `-e`.\n\n" + fails("Write['one*n']\nWrite[1/0]") +
   "\n\nThe manual calls an error of this kind an alarm.")

qa("How do I check a CPL program without running it?",
   "`cpl --check program.cpl` reads a program without running it and reports what "
   "is wrong, one line per problem, as `file:line:column: severity: message`: a "
   "string or a section bracket that is never closed, a tag that matches nothing, "
   "a mistake of syntax, a name that is not defined. The exit status is 1 if any "
   "program has an error, and `-` in place of a file reads the program from "
   "standard input. Given this file, `sum.cpl`:\n\n"
   + block("let Total, n = 0, 3\nfor i = 1 to n do\n   $( Total := Total + i\n"
           "Write['Total is ', total, '*n]", "cpl")
   + "\n\n`cpl --check sum.cpl` prints:\n\n"
   + check("sum.cpl", "let Total, n = 0, 3\nfor i = 1 to n do\n   $( Total := Total + i\n"
                      "Write['Total is ', total, '*n]"))

qa("What does cpl --publication do?",
   "It prints a program the way the papers of the 1960s printed CPL: the basic "
   "symbols underlined, section signs for the section brackets, and the "
   "mathematical signs. The underlining is a Unicode combining mark after each "
   "letter. The result is itself accepted as a program. Given\n\n"
   + block("let rec Fact[integer n] = n = 0 -> 1, n Fact[n - 1]\n"
           "if Fact[3] ~= 6 do $( Write['odd*n'] $)", "cpl")
   + "\n\n`cpl --publication` prints:\n\n"
   + tool(["--publication", "-e", "let rec Fact[integer n] = n = 0 -> 1, n Fact[n - 1]\n"
           "if Fact[3] ~= 6 do $( Write['odd*n'] $)"]))

qa("How can I see what a CPL program is compiled to?",
   "Three options print something instead of running the program: `--tokens` "
   "prints the basic symbols, `--ast` the syntax tree, and `--code` the abstract "
   "machine code. For example `cpl --tokens -e \"let x = 2x\"` prints:\n\n"
   + tool(["--tokens", "-e", "let x = 2x"]) +
   "\n\nNotice that `2x` is two symbols, a number and a name: writing them side by "
   "side multiplies them.")

qa("How do I stop a CPL program that runs for ever?",
   "`--max-steps N` stops a program with an error after N machine steps. For "
   "example, an empty compound command repeated with no condition never ends:\n\n"
   + block("cpl --max-steps 100 -e '$( $) repeat'", "bash") +
   "\n\nprints this on standard error and exits with status 1:\n\n"
   + block(cpl(["--max-steps", "100", "-e", "$( $) repeat"])[2]) +
   "\n\nIn the browser playground there is a **Stop** button, which works because "
   "each run happens in a worker.")

qa("Is there editor support for CPL?",
   "Yes. `cpl-lsp` is a language server, in the `lsp/` directory, that works with "
   "any editor that speaks the Language Server Protocol. It talks on standard input "
   "and output, takes no arguments, and provides diagnostics, hover, go to "
   "definition, references, document highlights, rename, completion, signature "
   "help, document symbols, folding ranges and semantic tokens. `editors/vscode` is "
   "a Visual Studio Code extension that bundles the server and the system as "
   "WebAssembly and adds syntax highlighting, for publication CPL in `.pub` files "
   "too, and a run button. Both are built on the same analysis as `cpl --check`, "
   "which asks the system's own lexer, parser and compiler, so the editor and the "
   "system cannot disagree about what is a mistake.")

qa("How do I install the CPL extension for Visual Studio Code?",
   "Download `cpl-language-0.1.0.vsix` from the v0.1.0 release at "
   "https://github.com/chrishayuk/cpl/releases/tag/v0.1.0 and install it:\n\n"
   + block("code --install-extension cpl-language-0.1.0.vsix", "bash") +
   "\n\nOpen any `.cpl` file and you get highlighting, hover help, problems as you "
   "type, and a run button. The extension carries its own copy of the system and "
   "the language server, so it works without installing `cpl` separately. To build "
   "the extension yourself, see `editors/vscode`.")

qa("How do I use the CPL language server in Neovim or Helix?",
   "Install it with `cargo install --git https://github.com/chrishayuk/cpl cpl-lsp`, "
   "or build it in a clone with `cargo build --release -p cpl-lsp`. It talks on "
   "standard input and output and takes no arguments. For Neovim 0.11 or later:\n\n"
   + block('vim.filetype.add({ extension = { cpl = "cpl" } })\n'
           'vim.lsp.config("cpl", {\n  cmd = { "/path/to/cpl-lsp" },\n'
           '  filetypes = { "cpl" },\n})\nvim.lsp.enable("cpl")', "lua") +
   "\n\nFor Helix, in `languages.toml`:\n\n"
   + block('[language-server.cpl-lsp]\ncommand = "/path/to/cpl-lsp"\n\n'
           '[[language]]\nname = "cpl"\nscope = "source.cpl"\nfile-types = ["cpl"]\n'
           'comment-token = "||"\nlanguage-servers = ["cpl-lsp"]', "toml") +
   "\n\nThe project's notes say that these two configurations follow each editor's "
   "documented format but have not been run by the authors; the server itself is "
   "tested over standard input and output.")

qa("Can I run CPL in a web browser?",
   "Yes. There is a playground at https://chrishayuk.com/demos/cpl, and its source "
   "is in the `web/` directory. It has an editor, the examples in a menu, a box for "
   "the program's input, **Run**, **Stop**, **Code** to see the stack machine a "
   "program is compiled to, and **Printed** to see the program as the papers would "
   "have set it. The system is compiled from the same source for WASI (`wasm32-wasip1`), and "
   "each run happens in a worker, which is how an endless program can be stopped. "
   "The editor part loads CodeMirror from a CDN and gives colours, problems as you "
   "type, hover help and completion; without a network connection the plain box "
   "still works. To run it locally:\n\n"
   + block("node web/build.mjs\npython3 -m http.server -d web 8000", "bash") +
   "\n\nthen open http://localhost:8000. Add `?example=mandelbrot` to the address "
   "to open a particular example, and `&run` to run it straight away.")

qa("Can CPL be built for WebAssembly?",
   "Yes. The same crate builds for any WASI preview 1 runtime:\n\n"
   + block("rustup target add wasm32-wasip1\n"
           "cargo build --release --target wasm32-wasip1\n"
           "wasmtime run --dir=. target/wasm32-wasip1/release/cpl.wasm examples/queens.cpl\n"
           "wasmtime run target/wasm32-wasip1/release/cpl.wasm -e \"Write[2**10, '*n']\"",
           "bash") +
   "\n\n`--dir=.` lets the sandbox read the `.cpl` file; with `-e` it needs no "
   "access to anything. The project checks that the native and WASI builds print "
   "exactly the same thing for every example.")

qa("How is the CPL tutorial kept correct?",
   "The tutorial has 19 chapters and 205 complete programs, each followed by its "
   "output. The test `tests/tutorial.rs` runs every one and compares, so the "
   "tutorial cannot drift from the implementation; `CPL_BLESS=1 cargo test --test "
   "tutorial` rewrites the outputs after a deliberate change. In the same way every "
   "program in `examples/` has a `.out` file with its expected output, and a `.in` "
   "file if it reads input, and `cargo test` runs them all.")

# ------------------------------------------------------ hello world

qa("How do I print Hello World in CPL?",
   "Call the routine `Write` with a string. A string is written between single "
   "quotes, and `*n` inside it is a new line.\n\n" + prog("Write['Hello, world!*n']") +
   "\n\nThe parameters of a function or routine go in square brackets, always; "
   "round brackets are only for grouping.")

qa("How do I print a new line in CPL?",
   "A star inside a string means that the next character is special, and `*n` is a "
   "new line. `Write` does not start a new line unless it is told to.\n\n"
   + prog("Write['one ']\nWrite['two ']\nWrite['three*n*nfour*n']") +
   "\n\nThe routine `Newline[]` does the same job on its own.")

qa("What are the special characters in a CPL string?",
   "A star inside a string marks the next character as special: `*n` is a new "
   "line, `*s` a space, `*t` a tab, `*'` a quote and `**` a star.\n\n"
   + prog("Write['It*'s 2 ** 3*n']"))

qa("How do I print several things at once in CPL?",
   "`Write` takes any number of parameters, of any type, and writes them one after "
   "another with nothing in between.\n\n"
   + prog("Write['Two and two make ', 2 + 2, '.*n']") +
   "\n\n`Newline[]` and `Space[n]` do what they say:\n\n"
   + prog("Write['a']\nSpace[5]\nWrite['b']\nNewline[]"))

qa("How do I write a comment in CPL?",
   "Two vertical bars begin a comment, which runs to the end of the line.\n\n"
   + prog("|| This program prints a greeting.\n"
          "Write['Hello!*n']       || this is the greeting"))

qa("Do CPL commands need semicolons?",
   "No. The end of a line ends a command, and semicolons are only needed between "
   "commands that share a line.\n\n"
   + prog("Write['one*n']; Write['two*n']; Write['three*n']") +
   "\n\nA command may run over several lines if it is clear that it has not "
   "finished, because a bracket is still open or because the line ends with an "
   "operator:\n\n"
   + prog("Write['The sum is ',\n      1 + 2 +\n      3 + 4,\n      '*n']"))

# ------------------------------------------------------ numbers and names

qa("How does arithmetic work in CPL?",
   "The operators are `+`, `-`, `*`, `/` and `**`, which raises to a power (CPL "
   "printed an upward arrow). Multiplication and division are done before addition "
   "and subtraction, and round brackets group.\n\n"
   + prog("Write[2 + 3, ' ', 7 - 10, ' ', 6 * 7, ' ', 2 ** 10, '*n']\n"
          "Write[2 + 3 * 4, ' ', (2 + 3) * 4, '*n']") +
   "\n\nThe result of `**` is always a real number, which is why 1024 has a decimal "
   "point.")

qa("Why does 7 / 2 give 3.5 in CPL, and how do I divide whole numbers?",
   "`/` always gives a real result, and a real number is always printed with a "
   "decimal point. Adding, subtracting and multiplying whole numbers gives a whole "
   "number.\n\n"
   + prog("Write[7 + 2, ' ', 7 * 2, ' ', 7 / 2, ' ', 8 / 2, '*n']") +
   "\n\nFor division of whole numbers there are two functions: `Quot` gives the "
   "quotient and `Rem` the remainder.\n\n"
   + prog("Write[Quot[17, 5], ' remainder ', Rem[17, 5], '*n']"))

qa("Why are CPL variable names only one letter long?",
   "Because a small letter is a whole name. `x` is a name and so is `y`, but `xy` "
   "is not a name: it is `x` times `y`. Mathematicians write *ab* for the product "
   "of *a* and *b*, and CPL lets you do the same. A small name is one small letter, "
   "which may be followed by primes: `x`, `y`, `a'`, `a''`. A longer name, a large "
   "name, must begin with a capital letter and goes on with letters and digits: "
   "`Total`, `Fib`, `X2`, `ERROR`.\n\n"
   + prog("let a, a' = 1, 2\nlet Total = a + a'\nWrite[Total, '*n']"))

qa("How does multiplication without a sign work in CPL?",
   "Writing two things side by side multiplies them, as in mathematics, where *2x* "
   "means 2 times *x*.\n\n"
   + prog("let x, y = 3, 4\n"
          "Write[2x, ' ', x y, ' ', xy, ' ', 2x(y + 1), ' ', (x + y)(x - y), '*n']") +
   "\n\nThe variables defined by `let x, y = 3, 4` are real, which is why the "
   "answers have decimal points.")

qa("In CPL, is a/bc the same as a/(bc)?",
   "Yes. Writing side by side is replaced by the multiplication sign before any "
   "further analysis, and the table of precedences in section 6.1.4 of the 1966 "
   "manual then makes a `/` take everything after it that is multiplied or "
   "divided: `a/bc` is `a/(bc)`, and `a/b/c` is `a/(b/c)`. The 1963 paper says the "
   "same of `a/bc`.\n\n"
   + prog("let a, b, c = 12, 2, 3\nWrite[a/b c, ' ', (a/b) c, ' ', a/b/c, ' ', (a/b)/c, '*n']"))

qa("What does -2 ** 2 give in CPL?",
   "A minus sign in front of something is treated as multiplication by -1, so it "
   "is done after a power.\n\n" + prog("Write[-2 ** 2, ' ', (-2) ** 2, '*n']"))

qa("How do I write a very large or very small number in CPL?",
   "With a power of ten. The manual printed a small subscript 10; here it is `@`. "
   "`6.02@23` is 6.02 times ten to the 23.\n\n"
   + prog("Write[6.02@23, ' ', 1@-6, ' ', 1.5@3, '*n']") +
   "\n\nThe last of these is a whole number, however it is written, and so it is "
   "an index and is printed without a decimal point.")

qa("What mathematical functions does CPL have?",
   "`Sqrt`, `Exp`, `Log` (the natural logarithm), `Sin`, `Cos`, `Tan` (the angle is "
   "in radians), `Arctan`, `Arcsin`, `Arccos`, `Mod` (the absolute value), `Sign` "
   "(-1, 0 or 1), `Intpt` and `Frpt` (integral part and fractional part), `Round` "
   "(the nearest whole number), and `Quot` and `Rem` (quotient and remainder).\n\n"
   + prog("Write[Sqrt[2], '*n']\nWrite[Exp[1], ' ', Log[10], '*n']\n"
          "Write[Sin[0], ' ', Cos[0], ' ', 4 Arctan[1], '*n']\n"
          "Write[Mod[-3.5], ' ', Sign[-3.5], ' ', Intpt[3.7], ' ', Frpt[3.7], ' ', "
          "Round[3.7], '*n']") +
   "\n\n`Intpt` and `Round` give whole numbers; that is the way to get from a real "
   "number to an integer.")

# ------------------------------------------------------ definitions

qa("How do I define a variable in CPL?",
   "A variable is brought into existence by a definition, which begins with `let`, "
   "and its value is changed by an assignment, which uses `:=`.\n\n"
   + prog("let x = 5\nWrite[x, '*n']\nx := x + 1\nWrite[x, '*n']") +
   "\n\n`=` in a definition says what the variable is to start as. `:=` is a "
   "command: work out the right hand side, and put the result in the variable on "
   "the left. `=` on its own, in an expression, is a test for equality.")

qa("Why does let x = 5 print 5.0 in CPL?",
   "Every variable has a type, fixed when it is defined. `let x = 5` does not say "
   "what type `x` is to be, so the system uses the preferred type, which is real, "
   "and a real number is always printed with a decimal point.\n\n"
   + prog("let x = 5\nWrite[x, ' is of type ', Type[x], '*n']") +
   "\n\nTo have whole numbers, give the type with `be`, or change the preferred "
   "type with `prefer integer`:\n\n"
   + prog("prefer integer\nlet n = 5\nWrite[n, ' is of type ', Type[n], '*n']"))

qa("How do I give a CPL variable a type?",
   "A definition by `be` gives the type and no value. `both be` and `all be` give "
   "one type to several names, and a list of names can be matched with a list of "
   "types.\n\n"
   + prog("let i, j both be index\nlet x, y, z all be real\n"
          "let n, s be integer, string\n"
          "i, j, x, y, z, n, s := 1, 2, 3, 4, 5, 6, 'seven'\n"
          "Write[i, j, ' ', x, y, z, ' ', n, s, '*n']") +
   "\n\n(`Write` puts nothing between its parameters, so the values run together.) "
   "If the right hand side of a `let` is plainly not a number, the variable takes "
   "its type from it:\n\n"
   + prog("let s = 'a string'\nlet b = 2 > 1\nlet c = Cpx[1, 2]\n"
          "Write[Type[s], ', ', Type[b], ', ', Type[c], '*n']"))

qa("How do I swap two variables in CPL?",
   "With a simultaneous assignment. All the right hand sides are worked out first, "
   "and then all the assignments are made, so `x, y := y, x` exchanges two "
   "variables with no third one to help.\n\n"
   + prog("let x, y, z = 1, 2, 3\nWrite[x + y + z, '*n']\nx, y := y, x\n"
          "Write[x, ' ', y, '*n']"))

qa("What happens when I assign a value of the wrong type in CPL?",
   "When a value is assigned to a variable it is first converted to the variable's "
   "type, if that can be done without loss.\n\n"
   + prog("let n be integer\nlet x be real\nn := 6.0\nx := 6\nWrite[n, ' ', x, '*n']") +
   "\n\nIf it cannot, the program stops. The manual calls this an alarm.\n\n"
   + fails("let n be integer\nn := 6.5"))

qa("How do I define a constant in CPL?",
   "Put `constant` in the definition. A constant may not be assigned to.\n\n"
   + fails("let constant Pi = 3.14159265359\nWrite[2Pi, '*n']\nPi := 3"))

qa("What types does CPL have?",
   "index, integer, real, double, complex, double complex, Boolean, logical, long "
   "logical, string, label, function, routine, array, type and general. A constant "
   "is the smallest type that will hold it: the whole numbers are index (small "
   "ones, for counting and subscripts) and integer (large ones).\n\n"
   + prog("Write[Type[7], ' ', Type[7.5], ' ', Type[5000000000], '*n']") +
   "\n\nIn this implementation index is 32 bits and integer 64; real and double "
   "are both IEEE double precision; logical is 24 bits and long logical 48, as on "
   "Atlas.")

# ------------------------------------------------------ conditions

qa("How do I compare values in CPL?",
   "The relations are `=` and `~=` (equal, not equal), `<`, `<=`, `>`, `>=`, and "
   "`<<` and `>>` (much less, much greater). The result is of type Boolean, `true` "
   "or `false`.\n\n"
   + prog("Write[2 < 3, ' ', 2 > 3, ' ', 2 = 2, ' ', 2 ~= 2, '*n']") +
   "\n\nRelations can be strung together as they are in mathematics: `a < b <= c` "
   "means that both `a < b` and `b <= c`.\n\n"
   + prog("let x = 5\nWrite[0 < x < 10, ' ', 0 < x < 3, ' ', 1 < 2 = 2 < 3, '*n']"))

qa("What do << and >> mean in CPL?",
   "Much less and much greater. `x << y` is for real numbers, and asks whether `x` "
   "is so small beside `y` that adding it would make no difference. It is how a "
   "loop that is closing in on an answer knows when to stop. They are not shifts: "
   "shifting is done by the functions `Lshift` and `Rshift`.\n\n"
   + prog("Write[1 << 1000, ' ', 1 << 1@20, '*n']"))

qa("How do I write and, or and not in CPL?",
   "`~` is not, `/\\` is and, `\\/` is or, `==` is \"if and only if\" and `~==` is "
   "exclusive or. CPL printed them as ∼, ∧, ∨, ≡ and ≢.\n\n"
   + prog(r"""
let p, q = true, false
Write[p /\ q, ' ', p \/ q, ' ', ~p, ' ', p == q, ' ', p ~== q, '*n']
""") +
   "\n\nBoth operands are always evaluated: the manual lets an operator be "
   "applied \"at any time after each of them has been evaluated\" (4.2), so there "
   "is no short cut. To guard a test, use a conditional expression: "
   "`(i <= n -> A[i] > 0, false)`.")

qa("How do I write an if statement in CPL?",
   "`if b do C` obeys the command `C` if `b` is true, and `unless b do C` if it is "
   "false. `then` may be written for `do`.\n\n"
   + prog("let x = 5\nif x > 3 do Write['big*n']\n"
          "unless x > 3 do Write['small*n']\nif x > 9 then Write['very big*n']") +
   "\n\nTo make several commands into one, put them between the section brackets "
   "`$(` and `$)`:\n\n"
   + prog("let x, y = 7, 3\nif x > y do\n   $( Write['exchanging*n']\n"
          "      x, y := y, x $)\nWrite[x, ' ', y, '*n']"))

qa("Does CPL have an else?",
   "Not with `if`. To choose between two commands there is a different word, "
   "`test`, with `or` before the second command.\n\n"
   + prog("let x = 5\ntest x > 9 then Write['big*n'] or Write['small*n']") +
   "\n\nCPL keeps `if` and `test` apart so that there is never any doubt which `if` "
   "an `else` belongs to: a problem ALGOL had, and C still has. A chain of tests "
   "reads well set out like this:\n\n"
   + prog("let n = 15\ntest Rem[n, 15] = 0 then Write['FizzBuzz*n']\n"
          "or test Rem[n, 5] = 0 then Write['Buzz*n']\n"
          "or test Rem[n, 3] = 0 then Write['Fizz*n']\nor Write[n, '*n']"))

qa("What is a conditional expression in CPL?",
   "`b -> E1, E2` is an expression whose value is `E1` if `b` is true and `E2` if "
   "it is not. C still has this expression, spelt `b ? E1 : E2`.\n\n"
   + prog("let x = -7\nWrite[(x < 0 -> -x, x), '*n']") +
   "\n\nThe second arm can be another conditional, which gives a list of cases to "
   "be tried in order:\n\n"
   + prog("let Grade[n] = n >= 70 -> 'first',\n               n >= 50 -> 'second',\n"
          "               n >= 40 -> 'third',\n               'fail'\n"
          "Write[Grade[82], ' ', Grade[55], ' ', Grade[12], '*n']") +
   "\n\nInside a larger expression a conditional needs round brackets, as in "
   "`10 + (x > 2 -> 1, 0)`.")

# ------------------------------------------------------ loops

qa("How do I write a while loop in CPL?",
   "`while b do C` obeys `C` again and again for as long as `b` is true. If `b` is "
   "false to begin with, `C` is not obeyed at all. `until b do C` is the same with "
   "the condition the other way up.\n\n"
   + prog("prefer integer\nlet n = 1\nwhile n < 100 do n := 2n\nWrite[n, '*n']"))

qa("How do I write a loop that tests at the end in CPL?",
   "Write the command first and `repeat` after it. `C repeat while b` and "
   "`C repeat until b` are the two forms, and the command is obeyed at least "
   "once.\n\n"
   + prog("prefer integer\nlet n = 1000\n$( n := Quot[n, 2]\n"
          "   Write[n, ' '] $) repeat until n < 10\nWrite['*n']"))

qa("How do I break out of a loop in CPL?",
   "`break` leaves the smallest loop it is in. `C repeat` with no condition goes on "
   "for ever, or until something stops it from inside, and that something is "
   "usually `break`. After `if`, the `do` may be left out before `break`.\n\n"
   + prog("prefer integer\nlet n, s = 0, 0\n$( n := n + 1\n   s := s + n\n"
          "   if s > 100 break $) repeat\n"
          "Write['1 + 2 + ... + ', n, ' = ', s, '*n']"))

qa("How do I write a for loop in CPL?",
   "`for` gives a variable each of a list of values in turn. The variable belongs "
   "to the loop: it is defined afresh for each cycle, its type comes from the "
   "values it is given, and when the loop is over it no longer exists.\n\n"
   + prog("for i = 1 to 5 do Write[i, ' ']\nWrite['*n']") +
   "\n\nThere are four ways to say what the values are: a list of any expressions; "
   "`a to b`, in steps of one; `step a, b, c`, which starts at `a` and goes up by "
   "`b` towards `c`; and dots, as one would write by hand, where the first two "
   "values give the step. The forms can be mixed in one list.\n\n"
   + prog("for s = 'red', 'green', 'blue' do Write[s, ' ']\nWrite['*n']\n"
          "for i = step 10, -1, 1 do Write[i, ' ']\nWrite['*n']\n"
          "for i = 1, 3, ..., 11 do Write[i, ' ']\nWrite['*n']\n"
          "for i = 1, 2, 3, step 10, 10, 50, 99 do Write[i, ' ']\nWrite['*n']"))

qa("How do I count down in a CPL for loop?",
   "Use `step` with a negative increment: `step a, b, c` starts at `a` and goes by "
   "`b` towards `c`.\n\n"
   + prog("for i = step 10, -1, 1 do Write[i, ' ']\nWrite['*n']"))

qa("Can a CPL for loop change its own limit?",
   "No. In the manual's words, \"the expressions are evaluated once and for all "
   "before the cycle is started; it is not possible for the cycle to change the "
   "increment or the end condition.\"\n\n"
   + prog("let n = 3\nfor i = 1 to n do\n   $( n := 100\n      Write[i, ' '] $)\n"
          "Write['*n']"))

qa("Why does my CPL for loop with step go past the end value?",
   "With `step a, b, c` the number of cycles is worked out in advance as "
   "(c - a)/b, rounded to the nearest whole number, plus one. That is sensible for "
   "real numbers, where a step of 0.1 might not land exactly on the end, but it "
   "means that the last value can overshoot.\n\n"
   + prog("for i = step 0, 2, 7 do Write[i, ' ']\nWrite['*n']") +
   "\n\n(7 - 0)/2 is 3.5, which rounds to 4, so there are five cycles and the last "
   "is 8. When the step may not divide the distance, use `while`.")

qa("How do I use the loop variable after a CPL for loop has finished?",
   "Ordinarily you cannot, because the loop variable disappears with the loop. "
   "Write `for ext` and the loop will use a variable that already exists.\n\n"
   + prog("prefer integer\nlet k = 0\nfor ext k = 1 to 100 do\n   if k k > 200 break\n"
          "Write['the first square over 200 is that of ', k, '*n']"))

qa("How do I write nested loops in CPL?",
   "Put one loop inside the section brackets of another. `break` leaves only the "
   "loop it is in.\n\n"
   + prog("for i = 1 to 4 do\n   $( for j = 1 to i do Write['**']\n      Write['*n'] $)") +
   "\n\n" + prog("for i = 1 to 5 do\n   $( for j = 1 to 5 do Printd[i j, 3, 0]\n"
                 "      Write['*n'] $)"))

# ------------------------------------------------------ sections, scope

qa("What are section brackets in CPL?",
   "`$(` and `$)` make several commands into one. CPL printed them as the section "
   "sign § and the same sign struck through; C took them from BCPL as `{` and `}`, "
   "and those are accepted here too. If there are definitions among the commands, "
   "the section is called a block, and the names defined in it belong to it.\n\n"
   + prog("let x = 1\n$( let x = 2\n   let y = 3\n"
          "   Write['inside: ', x, ' ', y, '*n'] $)\nWrite['outside: ', x, '*n']") +
   "\n\nInside the block, `x` means the new variable. The old one is still there, "
   "untouched, and is seen again when the block ends.")

qa("What are tags on CPL section brackets?",
   "A section bracket can carry a tag, written straight after it, and the closing "
   "bracket must carry the same one. In a long program this shows which bracket "
   "closes which.\n\n"
   + prog("$(1 for i = 1 to 2 do\n    $(1.1 for j = 1 to 2 do\n"
          "          $(1.2 Write[i, j, ' '] $)1.2\n          Write['*n'] $)1.1\n$)1") +
   "\n\nA closing bracket with a tag also closes any untagged or inner sections "
   "that are still open, so that one bracket can end several:\n\n"
   + prog("$(Outer\n   for i = 1 to 3 do\n      $( if i = 2 do\n"
          "            $( Write['two*n']\n$)Outer\nWrite['done*n']"))

qa("What is the difference between successive definitions and definitions joined by and in CPL?",
   "Definitions are obeyed in order, and each can use the ones before it. "
   "Definitions joined by `and` are made together, and none of them can see the "
   "others. The Elementary Programming Manual shows the difference with this "
   "pair:\n\n"
   + prog("$( let a = 5\n   $( let a = 10\n      let b = a\n      Write[b, ' '] $)\n"
          "   $( let a = 10 and b = a\n      Write[b, '*n'] $) $)") +
   "\n\nIn the first inner block `b` is defined after the new `a` and gets 10. In "
   "the second the two definitions are simultaneous, so the `a` on the right is the "
   "outer one, and `b` gets 5.")

qa("What does where do in CPL?",
   "It puts a definition after the thing that uses it, which is how a "
   "mathematician writes, and keeps the main line of the thought first.\n\n"
   + prog("let a, b, c = 1, -3, 2\n"
          "Write[(-b + d)/(2a), ' ', (-b - d)/(2a), '*n'] where d = Sqrt[b b - 4a c]") +
   "\n\nThe name `d` exists only for that one command. A where-clause can also "
   "qualify an expression in round brackets, and may hold several definitions "
   "joined by `and`:\n\n"
   + prog("Write[(a a + b b where a = 3 and b = 4), '*n']"))

qa("How do I make a private variable in CPL?",
   "`D1 in D2` makes the names defined by `D1` visible in `D2` and nowhere else. "
   "Here `Count` is known only to the function `Next`:\n\n"
   + prog("let Count = 0 in Next[] == value of $( Count := Count + 1; result is Count $)\n\n"
          "Next[]; Next[]\nWrite[Next[], '*n']") +
   "\n\nThe variable keeps its value from one call to the next, because it was "
   "created once, when the definition was obeyed, and not inside the function. "
   "Nobody else can reach it:\n\n"
   + fails("let Count = 0 in Next[] == value of $( Count := Count + 1; result is Count $)\n"
           "Write[Count]"))

# ------------------------------------------------------ functions, routines

qa("How do I define a function in CPL?",
   "A function is defined like a variable, with its parameters in square brackets "
   "after its name. This is the example from section 18 of the Elementary "
   "Programming Manual:\n\n"
   + prog("let F[x] = 3x**2 + 4x + 1\nWrite[F[0], ' ', F[1], ' ', F[2.5], '*n']") +
   "\n\n`x` is the formal parameter; in the call `F[2.5]`, 2.5 is the actual "
   "parameter. A function may have any number of parameters, including none, but "
   "the brackets are always written:\n\n"
   + prog("let Mean[a, b] = (a + b)/2\nlet Seven[] = 7\n"
          "Write[Mean[3, 4], ' ', Seven[], '*n']"))

qa("How do I give types to the parameters of a CPL function?",
   "A parameter is of the preferred type, real, unless something else is said. A "
   "type written in the parameter list applies to the names after it, until "
   "another type is written.\n\n"
   + prog("let Q[index i, j] = i(i + 1) + j\n"
          "let Initial[string s, real x] = First[s] <=> '.'\n"
          "Write[Q[3, 4], ' ', Type[Q[3, 4]], ' ', Initial['Strachey', 0], '*n']") +
   "\n\nThe actual parameter is converted to the type of the formal one, just as "
   "in an assignment:\n\n"
   + fails("let Half[integer n] = Quot[n, 2]\nWrite[Half[10.0], '*n']\n"
           "Write[Half[10.5], '*n']"))

qa("What are value of and result is in CPL?",
   "The body of a function is an expression. When working it out needs commands, a "
   "loop for example, the expression can be a block, introduced by `value of`. The "
   "command `result is E` ends the block and gives its value.\n\n"
   + prog("prefer integer\n\nlet Sum[index n] = value of\n   $( let s = 0\n"
          "      for i = 1 to n do s := s + i\n      result is s $)\n\n"
          "Write[Sum[100], '*n']") +
   "\n\n\"Throughout CPL there is a sharp distinction between commands and "
   "expressions\", says the manual, and `value of` is the bridge from one to the "
   "other. It can be used anywhere, not only in function definitions. `valof` and "
   "`resultis`, each as one word, are how BCPL later spelt these, and they are "
   "accepted too.")

qa("How do I return early from a CPL function?",
   "The first `result is` to be obeyed ends the `value of` block, so it can be "
   "used to leave early.\n\n"
   + prog("let Prime[index n] = value of\n   $( if n < 2 result is false\n"
          "      for d = 2 to n - 1 do\n         $( if d d > n break\n"
          "            if Rem[n, d] = 0 result is false $)\n      result is true $)\n\n"
          "for n = 1 to 30 do if Prime[n] do Write[n, ' ']\nWrite['*n']"))

qa("How do I write a recursive function in CPL?",
   "A function that uses itself must say so, with `rec`.\n\n"
   + prog("let rec Fact[integer n] = n = 0 -> 1, n Fact[n - 1]\n"
          "Write[Fact[5], ' ', Fact[20], '*n']") +
   "\n\nFunctions that call one another are defined together, with `and`:\n\n"
   + prog("let rec Even[index n] = n = 0 -> true, Odd[n - 1]\n"
          "    and Odd[index n] = n = 0 -> false, Even[n - 1]\n"
          "Write[Even[10], ' ', Odd[10], '*n']"))

qa("Why does CPL need rec for a recursive function?",
   "Because a definition normally cannot see itself: the names on the right of "
   "`let f[x] = ...` are the names that existed before the definition was made. "
   "Without `rec`, an `f` on the right is some other, older `f`.\n\n"
   + prog("let f[n] = n + 1\n$( let f[n] = n = 0 -> 100, f[n - 1]\n"
          "   Write[f[5], '*n'] $)") +
   "\n\nThe inner `f` calls the outer one, which adds 1 to 4. With `rec` it would "
   "have called itself and given 100. If there is no older `f`, the name is "
   "simply not defined:\n\n"
   + fails("let f[x] = x = 0 -> 1, x f[x - 1]\nWrite[f[3]]"))

qa("Can a CPL function return more than one result?",
   "Yes. The results are received by a definition or an assignment with the same "
   "number of names on the left.\n\n"
   + prog("let Divide[integer a, b] = Quot[a, b], Rem[a, b]\n\n"
          "let q, r = Divide[17, 5]\nWrite[q, ' remainder ', r, '*n']"))

qa("Can I pass a function to a function in CPL?",
   "Yes. A function is a value like any other, and can be handed to another "
   "function.\n\n"
   + prog("let Twice[function f, real x] = f[f[x]]\nlet Inc[x] = x + 1\n\n"
          "Write[Twice[Inc, 5], ' ', Twice[Sqrt, 16], '*n']"))

qa("What is a routine in CPL?",
   "A function is an expression with a name: calling it gives a value. A routine "
   "is a command with a name: calling it does something.\n\n"
   + prog("let routine Rule[index n] be\n   $( for i = 1 to n do Write['-']\n"
          "      Write['*n'] $)\n\nRule[20]\nWrite['Combined Programming*n']\nRule[20]") +
   "\n\n`routine` may be shortened to `rt`, and the `let` may be left out, as the "
   "Elementary Programming Manual does. A routine is left when its command ends, "
   "or sooner by the command `return`. Routines may be recursive, with `rec`, like "
   "functions.")

qa("How do I return early from a CPL routine?",
   "With the command `return`.\n\n"
   + prog("routine Countdown[index n] be\n   $( if n > 5 do\n"
          "         $( Write['too long to wait*n']\n            return $)\n"
          "      for i = step n, -1, 1 do Write[i, ' ']\n      Write['go*n'] $)\n\n"
          "Countdown[3]\nCountdown[10]"))

qa("What is the difference between call by value and call by reference in CPL?",
   "A parameter is normally called by value: the routine gets a new variable of "
   "its own, which starts with the value of the actual parameter, and assigning to "
   "it changes that private variable and nothing else.\n\n"
   + prog("let rt Double[x] be x := 2x\n\nlet a = 5\nDouble[a]\nWrite[a, '*n']") +
   "\n\nWith `ref` (in full, `reference`) the routine is given not the value of "
   "`a` but `a` itself. Inside the routine `x` is another name for the same "
   "variable, and assigning to `x` assigns to `a`.\n\n"
   + prog("let rt Double[ref x] be x := 2x\n\nlet a = 5\nDouble[a]\nWrite[a, '*n']") +
   "\n\n`ref` and `val` apply, like types, to the names that follow them.")

qa("What are L-values and R-values in CPL?",
   "Look at `x := x + 1`. The `x` on the right stands for a number, the contents "
   "of the variable. The `x` on the left stands for the place where the number is "
   "kept. So an expression has two values. Its R-value is what it means on the "
   "right of an assignment: a number, a string, a truth value. Its L-value is what "
   "it means on the left: a location. Every expression has an R-value; only some "
   "have L-values. A variable has one, and so does an array element; `x + 1` does "
   "not. A parameter called by value receives an R-value, and one called by "
   "reference receives an L-value. If the actual parameter has no L-value to give, "
   "a constant location is made to hold its value, which may be read and not "
   "assigned to:\n\n"
   + fails("let rt Show[ref x] be Write['x is ', x, '*n']\n"
           "let rt Double[ref x] be x := 2x\n\nlet a = 5\nShow[a + 1]\nDouble[a + 1]") +
   "\n\nThis is Strachey's distinction, worked out for CPL, and it survives in the "
   "`lvalue` and `rvalue` of C compilers' error messages.")

qa("What does ~~ mean in a CPL definition?",
   "Definition by reference. `let b = a` defines `b` by value: a new variable, "
   "starting with the value of `a`. `let b ~~ a` defines `b` by reference: a new "
   "name for the variable `a`. CPL printed the sign as ≃.\n\n"
   + prog("let a = 1\nlet b ~~ a\nlet c = a\nb := 2\nWrite[a, ' ', b, ' ', c, '*n']") +
   "\n\nAssigning to `b` changed `a`, because they are the same variable; `c` is a "
   "different one and kept the old value.")

qa("Can a conditional expression be assigned to in CPL?",
   "Yes. A conditional expression has an L-value if its arms have, so it can be "
   "assigned to, and the assignment goes to whichever variable is chosen.\n\n"
   + prog("let p, q = 0, 0\nfor x = 3, -2, 5, -1 do\n"
          "   (x > 0 -> p, q) := (x > 0 -> p, q) + x\nWrite[p, ' ', q, '*n']") +
   "\n\n`value of` gives a value; its counterpart `reference of` gives a "
   "variable. This block picks out the largest of three variables, and the "
   "assignment sets it to zero:\n\n"
   + prog(r"""
let a, b, c = 5, 9, 2
reference of $( if a >= b /\ a >= c result is a
                if b >= c result is b
                result is c $) := 0
Write[a, ' ', b, ' ', c, '*n']
"""))

qa("What is call by substitution in CPL?",
   "A third way to pass a parameter, which ALGOL 60 called call by name: the "
   "actual parameter is not worked out when the function is called, but handed "
   "over as an expression, to be worked out afresh each time the function uses it. "
   "CPL had it in 1963, under the name substitution, and had dropped it by 1966. A "
   "parameter is marked `subst`.\n\n"
   + prog("let Sum[ref i, val a, b, subst t] = value of\n   $( let s = 0\n"
          "      for k = step a, 1, b do\n         $( i := k\n"
          "            s := s + t $)\n      result is s $)\n\n"
          "let i = 0\nWrite[Sum[i, 1, 10, i i], '*n']\nWrite[Sum[i, 1, 100, 1/i], '*n']") +
   "\n\nThis trick is called Jensen's device. `Sum` sets `i` and then asks for "
   "`t`; `t` is the expression `i i`, which is worked out with the new `i`. A "
   "definition can be by substitution too, with `==`:\n\n"
   + prog("let x, y = 1, 2\nlet w == x x + 2x y + y y\nWrite[w, ' ']\nx := 10\n"
          "Write[w, '*n']") +
   "\n\nThe designers came to prefer passing a function, which says the same thing "
   "more plainly.")

qa("What are the three ways of passing a parameter in CPL?",
   "By value, by reference and by substitution, marked `val`, `ref` and `subst`; "
   "each applies to the parameters after it, as a type does. By value, the default, "
   "the function gets a new variable holding the R-value of the actual parameter. "
   "By reference it gets the L-value, so assigning to the parameter assigns to the "
   "caller's variable. By substitution, which is from the CPL of 1963 and had been "
   "dropped by 1966, it gets the expression itself, worked out afresh at each "
   "use.\n\n"
   + prog("let rt Try[val a, ref b, subst c] be\n   $( a := a + 1\n      b := b + 1\n"
          "      Write[c, '*n'] $)\n\nlet x, y = 1, 1\nTry[x, y, x + y]\n"
          "Write[x, ' ', y, '*n']") +
   "\n\n`x` is unchanged, `y` has been increased, and `x + y` was worked out when "
   "the routine came to print it, after `y` had changed.")

# ------------------------------------------------------ fixed and free

qa("What is the difference between a fixed function and a free function in CPL?",
   "It is about free variables: names in the body that are not parameters, whose "
   "meaning comes from outside. A function defined with `=` is a fixed function. "
   "When the definition is obeyed, the values of its free variables are taken and "
   "kept. A function defined with `==` is a free function (CPL printed the sign "
   "with three bars, ≡). It holds on to the variables, and looks at them afresh "
   "every time it is called.\n\n"
   + prog("let a = 1\nlet f[x] = a x\nlet g[x] == a x\n\na := 100\n"
          "Write[f[5], ' ', g[5], '*n']") +
   "\n\nThe free variables of a fixed function are taken by R-value, and those of "
   "a free function by L-value. It is the same choice as between calling by value "
   "and calling by reference, and between `=` and `~~` in a definition. Routines "
   "are always free.")

qa("Why can't my CPL function assign to a variable outside itself?",
   "Because it is a fixed function. A function defined with `=` has only copies of "
   "its free variables, so it would make no sense for it to assign to them, and "
   "the system will not allow it.\n\n"
   + fails("let Count = 0\nlet Next[] = value of $( Count := Count + 1\n"
           "                         result is Count $)\nWrite[Next[]]") +
   "\n\nDefine it with `==`, which makes it a free function, and it is allowed:\n\n"
   + prog("let Count = 0\nlet Next[] == value of $( Count := Count + 1\n"
          "                          result is Count $)\nNext[]; Next[]\n"
          "Write[Next[], ' ', Count, '*n']") +
   "\n\nThis gives fixed functions a valuable property: a fixed function whose "
   "parameters are called by value cannot alter anything outside itself, and the "
   "value of a call depends on nothing but the parameters.")

qa("Does CPL have closures?",
   "Yes, though the word is later. In CPL a function is a piece of program "
   "together with its free variables, and so it can outlive the call that made "
   "it. Each call of `Adder` makes a new function, with its own copy of `n` fixed "
   "inside it:\n\n"
   + prog("let Adder[n] = f where f[x] = x + n\n\n"
          "let Add1, Add10 = Adder[1], Adder[10]\n"
          "Write[Add1[5], ' ', Add10[5], ' ', Adder[100][5], '*n']") +
   "\n\n`Adder[100][5]` calls `Adder` to get a function and then calls that.")

qa("How do I write a counter that remembers its count in CPL?",
   "Use a free function, made by another function, sharing a variable that belongs "
   "to nobody else.\n\n"
   + prog("prefer integer\n\nlet Counter[Start] = value of\n   $( let n = Start\n"
          "      let Next[] == value of $( n := n + 1\n"
          "                                result is n $)\n      result is Next $)\n\n"
          "let a, b = Counter[0], Counter[100]\nWrite[a[], ' ', a[], ' ', a[], '*n']\n"
          "Write[b[], ' ', b[], '*n']\nWrite[a[], '*n']") +
   "\n\nEach call of `Counter` makes a new variable `n` and a new function that "
   "shares it. The two counters do not disturb each other, and nothing else in the "
   "program can touch either `n`. This cannot be written in BCPL, in B, or in C: "
   "it needs a function to carry its free variables with it.")

qa("How do I compose two functions in CPL?",
   "Write a function that returns a function.\n\n"
   + prog("let Compose[function f, g] = h where h[x] = f[g[x]]\nlet Inc[x] = x + 1\n"
          "let Sq[x] = x x\n\n"
          "Write[Compose[Inc, Sq][3], ' ', Compose[Sq, Inc][3], '*n']"))

qa("Can a CPL variable hold a function?",
   "Yes. A function can be assigned to a variable of type function, and the "
   "variable called.\n\n"
   + prog("let f be function\nfor i = 1 to 3 do\n"
          "   $( f := (i = 1 -> Sin, i = 2 -> Cos, Sqrt)\n      Write[f[0], ' '] $)\n"
          "Write['*n']") +
   "\n\nA function defined by `let` is a constant, so that its meaning cannot be "
   "changed behind the back of the code that uses it:\n\n"
   + fails("let f[x] = x + 1\nf := Sqrt"))

# ------------------------------------------------------ arrays

qa("How do I make an array in CPL?",
   "By calling a function. `Newarray` is given the type of the elements and, for "
   "each dimension, the lowest and highest subscript. An element is written with "
   "its subscript in square brackets, and has an L-value.\n\n"
   + prog("let a = Newarray[real, (1, 5)]\nfor i = 1 to 5 do a[i] := i i\n"
          "for i = 1 to 5 do Write[a[i], ' ']\nWrite['*n']") +
   "\n\nThe bounds are ordinary expressions worked out when the program runs, so "
   "the size can depend on anything and the lower bound need not be 1 or 0. "
   "`Newarray` gives no values to the elements. `Formarray` makes an array and "
   "fills it from a list:\n\n"
   + prog("let p = Formarray[index, (1, 6)][2, 3, 5, 7, 11, 13]\nWrite[p[1] + p[6], '*n']") +
   "\n\n`Formarray[index, (1, 6)]` is a function, which is then applied to the six "
   "values: that is why there are two pairs of brackets.")

qa("Can a CPL array have a negative lower bound?",
   "Yes. The bounds are ordinary expressions, and the lower bound need not be 1 "
   "or 0.\n\n"
   + prog("let n = 3\nlet t = Newarray[index, (-n, n)]\n"
          "for i = -n to n do t[i] := i i\nfor i = -n to n do Write[t[i], ' ']\n"
          "Write['*n']"))

qa("Are array subscripts checked in CPL?",
   "Yes, in this implementation a subscript outside the bounds stops the "
   "program.\n\n" + fails("let a = Newarray[real, (1, 5)]\na[6] := 0"))

qa("How do I make a two-dimensional array in CPL?",
   "Give `Newarray` or `Formarray` a pair of bounds for each dimension, and write "
   "the subscripts with a comma between them.\n\n"
   + prog("let m = Formarray[real, (1, 2), (1, 3)][1, 2, 3,\n"
          "                                        4, 5, 6]\nfor i = 1 to 2 do\n"
          "   $( for j = 1 to 3 do Write[m[i, j], ' ']\n      Write['*n'] $)") +
   "\n\nThe types of arrays have names, used for parameters: `real vector` or "
   "`real 1 array` for one dimension, `real matrix` or `real 2 array` for two, "
   "`index 3 array` and so on. `a[i, j]` means the same as `a[i][j]`: a matrix is "
   "a vector of vectors, and `a[i]` is a row.")

qa("How do I find the bounds of an array in CPL?",
   "`Bounds` gives the lowest and highest subscript of an array, so a function "
   "need not be told them.\n\n"
   + prog("let Total[vector v] = value of\n   $( let l, h = Bounds[v]\n"
          "      let s = 0\n      for i = step l, 1, h do s := s + v[i]\n"
          "      result is s $)\n\n"
          "Write[Total[Formarray[real, (0, 3)][1, 2, 3, 4]], '*n']"))

qa("Does assigning a CPL array copy it?",
   "No. The value of an array variable is not the elements; it is the place where "
   "they are. So assignment and definition do not copy the elements:\n\n"
   + prog("let v = Formarray[index, (1, 3)][1, 2, 3]\nlet w = v\nw[1] := 99\n"
          "Write[v[1], '*n']") +
   "\n\n`w` and `v` are two variables holding the same array. To have a separate "
   "one, ask for it with `Copy`:\n\n"
   + prog("let v = Formarray[index, (1, 3)][1, 2, 3]\nlet w = Copy[v]\nw[1] := 99\n"
          "Write[v[1], ' ', w[1], '*n']\nWrite[v = w, ' ', Equal[v, w], '*n']") +
   "\n\n`=` between arrays asks whether they are the same array; `Equal` asks "
   "whether they have the same elements. For the same reason a routine can change "
   "the elements of an array that was passed to it by value. This is where C's "
   "arrays come from, by way of BCPL.")

qa("Can a CPL array have rows of different lengths?",
   "Yes. A matrix is a vector of vectors, and each row is an array in its own "
   "right, so the rows need not be the same length. Here is a triangle:\n\n"
   + prog("let t = Newarray[general, (1, 4)]\nfor i = 1 to 4 do\n"
          "   $( t[i] := Newarray[index, (1, i)]\n"
          "      for j = 1 to i do t[i][j] := i j $)\n\nfor i = 1 to 4 do\n"
          "   $( for j = 1 to i do Write[t[i][j], ' ']\n      Write['*n'] $)"))

qa("How do I multiply two matrices in CPL?",
   "This is the example that closes section 14 of the Elementary Programming "
   "Manual, which sets C equal to the matrix product AB. The capital letters `A`, "
   "`B` and `C` are large names, so the product of two elements needs no sign. "
   "The closing bracket tagged `1.1` also closes section `1.2`.\n\n"
   + prog("let n = 2\nlet A = Formarray[real, (1, n), (1, n)][1, 2, 3, 4]\n"
          "let B = Formarray[real, (1, n), (1, n)][0, 1, 1, 0]\n"
          "let C = Newarray[real, (1, n), (1, n)]\n\nfor i = 1 to n do\n"
          "$(1.1 for j = 1 to n do\n   $(1.2 let a = 0\n"
          "         for k = 1 to n do\n         $(1.3 a := a + A[i, k] B[k, j] $)1.3\n"
          "         C[i, j] := a $)1.1\n\n"
          "Write[C[1, 1], ' ', C[1, 2], '*n', C[2, 1], ' ', C[2, 2], '*n']"))

# ------------------------------------------------------ strings and bits

qa("How do strings work in CPL?",
   "A string is a data item like a number. It can be kept in a variable, passed "
   "to a function and given back as a result, and it can be as long as you like. "
   "There is one operator on strings: `<=>` joins two of them end to end.\n\n"
   + prog("let s = 'Combined'\ns := s <=> ' ' <=> 'Programming'\nWrite[s <=> '!', '*n']") +
   "\n\nEverything else is done with functions: `Length[s]`; `First[s]` and "
   "`Last[s]`; `Character[i, s]`, the i-th character; and `Initials[i, s]` and "
   "`Finals[i, s]`, the first i characters and the last i. A character is simply a "
   "string of length one.\n\n"
   + prog("let s = 'Strachey'\n"
          "Write[Length[s], ' ', First[s], ' ', Last[s], ' ', Character[4, s], ' ',\n"
          "      Initials[3, s], ' ', Finals[3, s], '*n']"))

qa("How do I reverse a string in CPL?",
   "With a loop over the characters:\n\n"
   + prog("let s = 'Atlas'\nfor i = step Length[s], -1, 1 do Write[Character[i, s]]\n"
          "Write['*n']") +
   "\n\nor by recursion:\n\n"
   + prog("let rec Reverse[string s] = s = '' -> '', "
          "Reverse[Finals[Length[s] - 1, s]] <=> First[s]\nWrite[Reverse['Titan'], '*n']"))

qa("How do I compare strings in CPL?",
   "The relations work on strings, in dictionary order. In the manual's words, \"a "
   "longer string is greater than a shorter string identical to its starting "
   "characters.\"\n\n"
   + prog("Write['cat' < 'dog', ' ', 'cat' < 'catalogue', ' ', 'cat' = 'cat', ' ', "
          "'Cat' < 'cat', '*n']"))

qa("How do I convert between characters and numbers in CPL?",
   "Every character has a number. `Index` of a character gives it, and `String` of "
   "a number gives the character back.\n\n"
   + prog("Write[Index['A'], ' ', Index['a'], ' ', Index['0'], ' ', "
          "String[67], String[80], String[76], '*n']") +
   "\n\nThat is enough to turn a capital into a small letter, or a number into "
   "its digits:\n\n"
   + prog("let Small[string c] = 'A' <= c <= 'Z' -> String[Index[c] + 32], c\n\n"
          "let rec Digits[integer n] = n < 10 -> String[48 + n],\n"
          "                            Digits[Quot[n, 10]] <=> String[48 + Rem[n, 10]]\n\n"
          "Write[Small['Q'], Small['q'], ' ', Digits[1966] <=> ' was the year', '*n']"))

qa("What is the logical type in CPL?",
   "A row of 24 bits, the length of half a word on the Atlas computer; long "
   "logical has 48. The manual says that logical variables \"provide the means "
   "whereby most non-numerical work is carried out in CPL\". A logical constant is "
   "written in octal or binary. CPL printed a bold 8 or 2 in front of the digits; "
   "here it is `#o` or `#b`, with `#x` for hexadecimal as an extra.\n\n"
   + prog("let p = #o2736\nWrite[p, ' ', #b010111011110, ' ', #x5DE, '*n']") +
   "\n\nThe Boolean operators work on logicals one bit at a time:\n\n"
   + prog(r"""
let a, b = #b1100, #b1010
Write[a /\ b, ' ', a \/ b, ' ', a ~== b, ' ', ~a /\ #b1111, '*n']
"""))

qa("What bit functions does CPL have?",
   "The functions number the bits from 0 at the right hand end. `Lshift[p, j]` and "
   "`Rshift[p, j]` shift j places, and zeros come in at the end; `Rotate[p, j]` "
   "shifts left j places, round and round; `Ones[j, k]` is ones in bits j to k and "
   "zeros elsewhere; `Field[p, j, k]` is bits j to k of p, moved to the right hand "
   "end; `Mask[p, j, k]` is bits j to k of p, left where they are; and `Bit[p, j]` "
   "is bit j of p.\n\n"
   + prog("let p = #o2736\n"
          "Write[Lshift[p, 3], ' ', Rshift[p, 3], ' ', Rotate[p, 22], '*n']\n"
          "Write[Ones[3, 0], ' ', Field[p, 6, 10], ' ', Mask[p, 6, 10], ' ', "
          "Bit[p, 1], '*n']") +
   "\n\n`Bit` and `Field` may be used on the left hand side of an assignment, "
   "where they name part of a logical variable:\n\n"
   + prog("let p = #o0\nBit[p, 0] := #o1\nField[p, 3, 5] := #o7\nWrite[p, '*n']"))

qa("How do I convert between a number and its bits in CPL?",
   "`Index` turns a logical into the number it represents in binary, and "
   "`Logical` turns a number into bits.\n\n"
   + prog("Write[Index[#b1010], ' ', Logical[10], ' ', Index[#o777], '*n']"))

qa("How do I count the one bits in a word in CPL?",
   "Shift the word right a place at a time, adding up the bottom bit.\n\n"
   + prog("|| Count the ones in a word.\nlet Ones'[logical w] = value of\n"
          "   $( let n be index\n      n := 0\n      until w = #o0 do\n"
          "         $( n := n + Index[Bit[w, 0]]\n            w := Rshift[w, 1] $)\n"
          "      result is n $)\n\nWrite[Ones'[#o2736], ' ', Ones'[#o77777777], '*n']"))

# ------------------------------------------------------ types and general

qa("What happens when I mix types of number in CPL?",
   "There are six types of number: index, integer, real, double, complex and "
   "double complex. Each can do everything the one before it can. The arithmetic "
   "operators are the same for all of them, and when the two operands are of "
   "different types the result is of the larger one.\n\n"
   + prog("let i, n, x be index, integer, real\ni, n, x := 2, 3, 4\n"
          "Write[Type[i + i], ', ', Type[i + n], ', ', Type[n + x], ', ', Type[i x], '*n']") +
   "\n\nTwo operators always give at least a real result: `/` and `**`. The same "
   "rule settles the type of a conditional expression, whose two arms are brought "
   "up to the larger type:\n\n"
   + prog("let b = true\nWrite[(b -> 1, 2.5), ' ', Type[(b -> 1, 2.5)], '*n']"))

qa("Does CPL have complex numbers?",
   "Yes: complex and double complex are types. In this implementation `Cpx[a, b]` "
   "makes the complex number *a + ib* (the function `Cpx` is not in the papers), "
   "and it is printed as a pair.\n\n"
   + prog("let z = Cpx[3, 4]\nlet i = Cpx[0, 1]\n"
          "Write[z, ' ', z + 1, ' ', 2z, ' ', z z, ' ', i i, '*n']\n"
          "Write[Mod[z], ' ', Realpt[z], ' ', Imagpt[z], ' ', z/z, '*n']"))

qa("What are transfer functions in CPL?",
   "A value is changed from one type to another by a transfer function. The system "
   "puts them in for itself wherever a particular type is needed: on assignment, "
   "for a parameter called by value, and for the result of a function. They can "
   "also be written out, and are named after the type they produce.\n\n"
   + prog("Write[Real[3], ' ', Integer[3.0], ' ', Complex[3], ' ', Index['A'], ' ', "
          "String[65], '*n']\n"
          "Write[Logical[5], ' ', Index[#b101], ' ', Boolean[1], ' ', Index[true], '*n']") +
   "\n\nA transfer only happens automatically when nothing is lost. To go from a "
   "real number that is not whole to an integer you must say how, with `Round` or "
   "`Intpt`:\n\n"
   + fails("let n be integer\nn := Round[2.7]\nWrite[n, ' ']\nn := Intpt[2.7]\n"
           "Write[n, '*n']\nn := 2.7"))

qa("What is the type general in CPL?",
   "A variable of type general can hold a value of any type, and the value "
   "carries its type with it. The reference manual calls it \"a data item whose "
   "type may vary dynamically\", and admits that \"the introduction of the type "
   "general carries with it the need for type checking at run time\".\n\n"
   + prog("let g be general\n"
          "g := 3;        Write[g, ' is of type ', Type[g], '*n']\n"
          "g := 2.5;      Write[g, ' is of type ', Type[g], '*n']\n"
          "g := 'three';  Write[g, ' is of type ', Type[g], '*n']\n"
          "g := true;     Write[g, ' is of type ', Type[g], '*n']\n"
          "g := Sqrt;     Write[g, ' is of type ', Type[g], '*n']") +
   "\n\nAn operator given general operands decides what to do when it sees what "
   "they are:\n\n"
   + fails("let Add[general a, b] = a + b\n"
           "Write[Add[1, 2], ' ', Add[1, 2.5], ' ', Add[Cpx[1, 1], 1], '*n']\n"
           "Write[Add['one', 2]]"))

qa("How do I find out the type of a value in CPL?",
   "`Type[x]` gives the current type of a data item. (It is an addition of this "
   "implementation and is not in the papers.) A type is itself a data item, so the "
   "type of a value can be looked at and acted on.\n\n"
   + prog("let Describe[general x] = Type[x] = string -> 'the string ' <=> x,\n"
          "                          Type[x] = Boolean -> (x -> 'yes', 'no'),\n"
          "                          Type[x] = index -> 'a small whole number',\n"
          "                          Type[x] = real -> 'a real number',\n"
          "                          'something else'\n\n"
          "for x = 'abc', 2 > 1, 42, 4.2, Cpx[4, 2] do Write[Describe[x], '*n']") +
   "\n\nThe conditional expression in `Describe` has arms of unlike types that "
   "have no larger type in common, and so is general itself. A `for` list of "
   "unlike values makes a general loop variable in the same way.")

qa("What does prefer do in CPL?",
   "`prefer` changes the type that is assumed when nothing is said, which is "
   "otherwise real. A program that only counts is tidier with `prefer integer`. "
   "With `prefer general` at its head, a program need hardly mention types at all: "
   "every variable and parameter can hold anything.\n\n"
   + prog("prefer general\n\nlet Twice[f, x] = f[f[x]]\nlet Exclaim[s] = s <=> '!'\n"
          "Write[Twice[Sqrt, 16], ' ', Twice[Exclaim, 'CPL'], '*n']"))

qa("Are types checked at compile time in this CPL?",
   "Mostly not. CPL was to be compiled with every type known in advance. In this "
   "implementation every L-value has a type fixed when it is created and every "
   "assignment and parameter passes through the transfer functions, but this is "
   "checked when the program runs. Types are deduced at compile time only as far "
   "as needed to insert the transfers: the arms of a conditional, the results of a "
   "`value of` block, and the result of a recursive function. A type error is "
   "therefore reported when the offending command is obeyed, not before:\n\n"
   + fails("Write['before*n']\nlet n be integer\nn := 'six'"))

# ------------------------------------------------------ lists

qa("Does CPL have lists?",
   "The CPL of 1963 does. \"The main features of CPL\" promised lists \"similar in "
   "many respects to those of the LISP system\"; the 1966 reference manual left "
   "all such structures for a later edition that was never written. This "
   "implementation includes them. `List` makes a list of its parameters, which can "
   "be of any type and can be lists; `NIL` is the empty list.\n\n"
   + prog("let l = List[1, 2, 3]\nlet m = List['one', 2, 3.5, true, List['a', 'b']]\n"
          "Write[l, '*n', m, '*n', NIL, '*n']") +
   "\n\nA list is taken apart with `Hd` (the first member) and `Tl` (the list of "
   "all the others), and built up with `Cons`, which puts one more member on the "
   "front. `Null[l]` is true if `l` is empty, `Atom[x]` is true if `x` is not a "
   "list at all, and `Length` counts the members.\n\n"
   + prog("let l = List[1, 2, 3]\n"
          "Write[Hd[l], ' ', Tl[l], ' ', Hd[Tl[l]], ' ', Cons[0, l], '*n']\n"
          "Write[Null[l], ' ', Null[NIL], ' ', Null[Tl[Tl[Tl[l]]]], '*n']"))

qa("How do I write a function on lists in CPL?",
   "Nearly every function on lists has the same shape: say what the answer is for "
   "the empty list, and say how to get the answer for a list from its head and the "
   "answer for its tail. Begin with `prefer general`, so that parameters can be "
   "lists, or the members of lists, without saying so.\n\n"
   + prog("prefer general\n\n"
          "let rec Sum[l] = Null[l] -> 0, Hd[l] + Sum[Tl[l]]\n"
          "let rec Count[l] = Null[l] -> 0, 1 + Count[Tl[l]]\n"
          "let rec Join[a, b] = Null[a] -> b, Cons[Hd[a], Join[Tl[a], b]]\n"
          "let rec Reverse[l] = Null[l] -> NIL, Join[Reverse[Tl[l]], List[Hd[l]]]\n\n"
          "let l = List[3, 1, 4, 1, 5]\nWrite[Sum[l], ' ', Count[l], '*n']\n"
          "Write[Join[l, List[9, 2, 6]], '*n']\nWrite[Reverse[l], '*n']"))

qa("How do I write map, filter and fold in CPL?",
   "`Map` applies a function to every member; `Filter` keeps the members that "
   "pass a test; `Fold` combines all the members with a function of two "
   "parameters.\n\n"
   + prog("prefer general\n\n"
          "let rec Map[f, l] = Null[l] -> NIL, Cons[f[Hd[l]], Map[f, Tl[l]]]\n"
          "let rec Filter[p, l] = Null[l] -> NIL,\n"
          "                       p[Hd[l]] -> Cons[Hd[l], Filter[p, Tl[l]]],\n"
          "                       Filter[p, Tl[l]]\n"
          "let rec Fold[f, z, l] = Null[l] -> z, f[Hd[l], Fold[f, z, Tl[l]]]\n"
          "let rec Upto[a, b] = a > b -> NIL, Cons[a, Upto[a + 1, b]]\n\n"
          "let l = Upto[1, 10]\n"
          "Write[Map[Sq, l], '*n']             where Sq[x] = x x\n"
          "Write[Filter[Even, l], '*n']        where Even[x] = Rem[x, 2] = 0\n"
          "Write[Fold[Plus, 0, l], '*n']       where Plus[a, b] = a + b\n"
          "Write[Fold[Times, 1, l], '*n']      where Times[a, b] = a b\n"
          "Write[Map[Sqrt, List[1, 4, 9]], '*n']") +
   "\n\nThe `where` at the end of each line defines the little function that the "
   "line needs, on the spot.")

qa("What is Strachey's Cartesian product program in CPL?",
   "A program often quoted as an example of what CPL could say. Given a list of "
   "lists, it produces every way of choosing one member from each. It is the "
   "function `Lit` (a fold) used three times over, with three nested "
   "where-clauses, each of which can see the parameters of the function it "
   "qualifies. `List1[x]` is a list of one member.\n\n"
   + prog("prefer general\n\n"
          "let rec Lit[F, z, L] = Null[L] -> z, F[Hd[L], Lit[F, z, Tl[L]]]\n\n"
          "let Product[L] = Lit[f, List1[NIL], L]\n"
          "    where f[k, z] = Lit[g, NIL, k]\n"
          "          where g[x, y] = Lit[h, y, z]\n"
          "                where h[p, q] = Cons[Cons[x, p], q]\n\n"
          "Write[Product[List[List[1, 2], List['a', 'b', 'c']]], '*n']\n"
          "Write[Length[Product[List[List[1, 2, 3], List[4, 5], List[6, 7, 8, 9]]]], '*n']") +
   "\n\nThe project's README notes that the program is attributed to Strachey in "
   "the form in which it is usually quoted, and that the text has not been checked "
   "against a primary source. To run it, this implementation lets a where-clause "
   "after a definiens qualify that definiens alone, which departs from the 1966 "
   "manual's rule that a where-clause qualifies the longest command, definition or "
   "expression before it.")

qa("How do I spread a list into separate variables in CPL?",
   "In a definition or an assignment with several names on the left and one list "
   "on the right, the members are matched to the names.\n\n"
   + prog("let p = List['Strachey', 1916, true]\nlet a, b, c = p\n"
          "Write[a, ' was born in ', b, '*n']\n\nlet x, y = 0, 0\n"
          "x, y := List[10, 20]\nWrite[x + y, '*n']") +
   "\n\nThe numbers must agree:\n\n" + fails("let a, b = List[1, 2, 3]") +
   "\n\n`Members` does the same thing inside a list of parameters, where it turns "
   "a list into so many separate parameters:\n\n"
   + prog("let Add3[a, b, c] = a + b + c\nlet l = List[1, 2, 3]\n"
          "Write[Add3[Members[l]], ' ', List[0, Members[l], 4], '*n']"))

qa("Can I change a list in place in CPL?",
   "Yes. `Hd[l]` and `Tl[l]` have L-values, so the cells of a list can be "
   "assigned to.\n\n"
   + prog("let l = List[1, 2, 3]\nHd[l] := 'one'\nHd[Tl[l]] := 'two'\n"
          "Tl[Tl[Tl[l]]] := List[4, 5]\nWrite[l, '*n']") +
   "\n\nLike arrays, lists are not copied when they are assigned: two variables "
   "can hold the same list, and a change made through one is seen through the "
   "other.\n\n"
   + prog("let l = List[1, 2, 3]\nlet m = l\nHd[m] := 99\nWrite[l, '*n']"))

# ------------------------------------------------------ labels and the switch

qa("How do labels and go to work in CPL?",
   "Any command can be given a label, which is a name followed by a colon, and "
   "`go to` (or `goto`) transfers control to it. After `if` or `unless` the `do` "
   "may be left out before `go to`.\n\n"
   + prog("prefer integer\nlet n = 1\nLoop: Write[n, ' ']\nn := 2n\n"
          "if n < 100 go to Loop\nWrite['*n']") +
   "\n\nThe manual says that conditional commands and loops \"remove to some "
   "extent the need for explicit labels in a program (and should be exploited)\". "
   "But CPL's labels can do things that loops cannot, because a label is a value.")

qa("Is a label a value in CPL?",
   "Yes. A label is a data item. It can be put in a variable of type label, passed "
   "as a parameter, and `go to` can be given any expression whose value is a "
   "label.\n\n"
   + prog("let x = -5\ngo to (x < 0 -> Neg, x = 0 -> Zero, Pos)\n\n"
          "Neg:  Write['negative*n'];  go to Done\n"
          "Zero: Write['zero*n'];      go to Done\n"
          "Pos:  Write['positive*n']\nDone: Write['done*n']") +
   "\n\nA label variable is, in the manual's words, \"a link which may be set to "
   "hold different command labels under different conditions and may later be "
   "used in transfer commands\":\n\n"
   + prog("prefer integer\nlet L be label\nlet n = 0\n\nL := First\n"
          "Again: n := n + 1\ngo to L\n\nFirst:  Write['first ']\n"
          "        L := Second\n        go to Again\nSecond: Write['second ']\n"
          "        L := Third\n        go to Again\n"
          "Third:  Write['third, after ', n, ' turns*n']"))

qa("How does CPL handle errors inside a routine?",
   "By passing a label to the routine, which jumps to it when something goes "
   "wrong. The Elementary Programming Manual's equation solver takes a label to "
   "go to \"if there is no solution\":\n\n"
   + prog("routine LINEQ[real a, b, c, a', b', c', ref x, y, val label L] be\n"
          "$( let DET = ab' - a'b\n   if Mod[DET] < 1@-6 go to L\n"
          "   x := (cb' - c'b)/DET\n   y := (ac' - a'c)/DET $)\n\n"
          "let V, W = 0, 0\nLINEQ[1, 2, 3, 4, 5, 6, V, W, ERROR]\n"
          "Write['V = ', V, ', W = ', W, '*n']\n"
          "LINEQ[1, 2, 3, 2, 4, 7, V, W, ERROR]\nWrite['this is not printed*n']\n"
          "finish\n\nERROR: Write['no solution*n']") +
   "\n\nThe jump leaves the routine and does not come back. `finish` ends the "
   "program; without it the first part would run on into the commands labelled "
   "`ERROR`.")

qa("Can a CPL jump leave several function calls at once?",
   "Yes. A jump to a label unwinds activations until it reaches the one its label "
   "belongs to. Here a recursive function is three calls deep when it finds the "
   "bad member:\n\n"
   + prog("prefer general\n\nlet rec Sum[l, label Bad] =\n        Null[l] -> 0,\n"
          "        value of $( if Type[Hd[l]] = string go to Bad\n"
          "                    result is Hd[l] $) + Sum[Tl[l], Bad]\n\n"
          "Write[Sum[List[1, 2, 3], Oops], '*n']\n"
          "Write[Sum[List[1, 2, 'three', 4], Oops], '*n']\nfinish\n\n"
          "Oops: Write['that list has a string in it*n']") +
   "\n\nLanguages since have invented exceptions to do this. C has `setjmp` and "
   "`longjmp`, which come from BCPL's `level` and `longjump`, which were Richards' "
   "replacement for exactly this feature.")

qa("What exactly is a label value in CPL?",
   "A position in the program together with the activation it belongs to. The "
   "reference manual (section 9.3.2) gives this routine, which calls itself `n` "
   "times, each call passing on its own label `N` in place of one of the two it "
   "was given. Executing `R[10, Even, Odd]` jumps to `Even`, whereas "
   "`R[9, Even, Odd]` jumps to `Odd`.\n\n"
   + prog("let rec routine R[integer n, label L, M] be\n"
          "   $( if n > 0 do R[n - 1, M, N]\n   N: goto L $)\n\nlet Try = 10\n"
          "Again: R[Try, Even, Odd]\n\nEven: Write[Try, ' is even*n'];  goto Next\n"
          "Odd:  Write[Try, ' is odd*n']\nNext: Try := Try - 1\n"
          "      if Try >= 7 goto Again") +
   "\n\nWhen the calls unwind, each `goto L` goes to the `N` of the call that made "
   "it, with its variables as they were. A label does not outlive its activation; "
   "jumping to one whose routine has already returned is an error:\n\n"
   + fails("let Saved be label\nlet rt Keep[] be\n   $( Saved := Here\n      return\n"
           "Here: Write['back inside*n'] $)\n\nKeep[]\ngoto Saved"))

qa("How does the switch-command work in CPL?",
   "The switch-command was the last addition to CPL, in 1966, described on a "
   "single page at the back of the Working Papers as \"designed as an extension to "
   "the test-command\". It is written `switch on E into` followed by a section "
   "with `case` and `default` labels.\n\n"
   + prog("for n = 1 to 5 do\n   $( switch on n into\n"
          "      $( case 1: Write['one*n']\n                 go to Out\n"
          "         case 2: Write['two*n']\n                 go to Out\n"
          "         case 3:\n         case 4: Write['three or four*n']\n"
          "                 go to Out\n         default: Write['many*n'] $)\n Out: $)") +
   "\n\nA `case` is a kind of label. Control goes to the one that matches and "
   "carries on from there, into the next case if nothing stops it: hence the "
   "`go to` at the end of each. BCPL took this command from CPL and C took it "
   "from BCPL, falling through and all.")

qa("Does break leave a switch in CPL?",
   "No. In the manual as it stands `break` means only \"leave the loop\", so inside "
   "a switch within a `for` it would end the `for`. Use `go to` a label after the "
   "switch, or inside a function use `result is`, which leaves the switch along "
   "with everything else. BCPL invented `endcase`, and C made `break` do both "
   "jobs.\n\n"
   + prog("let Days[index m] = value of\n   $( switch on m into\n"
          "      $( case 2: result is 28\n"
          "         case 4: case 6: case 9: case 11: result is 30\n"
          "         default: result is 31 $) $)\n\n"
          "for m = 1 to 12 do Write[Days[m], ' ']\nWrite['*n']") +
   "\n\nThe editor's note at the front of the manual says that the rules about "
   "`break`, `return` and `result is` had been altered \"to allow a more general "
   "use\", but the new rules are not given anywhere, so this implementation "
   "follows the old ones.")

qa("Can a CPL switch use strings as cases?",
   "Yes, in this implementation the cases may be strings as well as numbers.\n\n"
   + prog("let Sound[string a] = value of\n   $( switch on a into\n"
          "      $( case 'cat': result is 'miaow'\n"
          "         case 'dog': result is 'woof'\n"
          "         default: result is '...' $) $)\n\n"
          "Write[Sound['dog'], ' ', Sound['cat'], ' ', Sound['fish'], '*n']"))

qa("How do I end a CPL program early?",
   "With the command `finish`, which ends the program wherever it is obeyed.\n\n"
   + prog("Write['one*n']\nfinish\nWrite['two*n']"))

# ------------------------------------------------------ LH functions

qa("What is an LH function in CPL?",
   "A function whose calls can stand on the left of an assignment as well as on "
   "the right: a variable of your own design. It has two parts. `load` says how "
   "to get the value; `update` says what to do when something is assigned, and in "
   "it `rhs` stands for the value being assigned.\n\n"
   + prog("let c = 100                    || a temperature, kept in Celsius\n\n"
          "let function Fahrenheit[] be\n"
          "       load $( result is 9c/5 + 32 $)\n"
          "       update $( c := 5(rhs - 32)/9 $)\n\n"
          "Write[Fahrenheit[], '*n']\nFahrenheit[] := 32\nWrite[c, '*n']\n"
          "Fahrenheit[] := Fahrenheit[] + 18\nWrite[c, '*n']") +
   "\n\n`Fahrenheit[]` behaves like a variable. There is no such variable: there "
   "is only `c`, seen through a pair of functions. Modern languages call these "
   "properties, or getters and setters. BCPL left them out.")

qa("Can an LH function be passed by reference in CPL?",
   "Yes. A call of an LH function is a real L-value, so it can be given a name by "
   "reference, and passed to a routine that expects a variable.\n\n"
   + prog("let c = 0\nlet function Fahrenheit[] be\n"
          "       load $( result is 9c/5 + 32 $)\n"
          "       update $( c := 5(rhs - 32)/9 $)\n\n"
          "let rt Double[ref x] be x := 2x\n\nlet f ~~ Fahrenheit[]\nf := 106\n"
          "Double[f]\nWrite[c, '*n']") +
   "\n\n`Double` knows nothing about temperatures. It was handed something to load "
   "from and update, and did so.")

qa("What does fix do in a CPL LH function?",
   "The `fix` part is worked out once, when the call is made, and settles which "
   "variable is meant; `load` and `update` then use it. An LH function with "
   "parameters is an array whose storage you design. This one is a symmetric "
   "matrix, which keeps only half its elements: `Sym[i, j]` and `Sym[j, i]` are "
   "the same variable.\n\n"
   + prog("let t = Newarray[real, (1, 10)]\nfor k = 1 to 10 do t[k] := 0\n\n"
          "let function Sym[index i, j] be\n"
          "    $( fix k = (i >= j -> Quot[i(i - 1), 2] + j, Quot[j(j - 1), 2] + i)\n"
          "       load $( result is t[k] $)\n       update $( t[k] := rhs $) $)\n\n"
          "Sym[1, 3] := 7\nSym[4, 2] := 5\n"
          "Write[Sym[3, 1], ' ', Sym[2, 4], ' ', Sym[4, 4], '*n']") +
   "\n\nFixing matters when the parameters change between the call and the "
   "use:\n\n"
   + prog("let t = Formarray[real, (1, 3)][10, 20, 30]\n"
          "let function Elem[index i] be\n    $( fix k = i\n"
          "       load $( result is t[k] $)\n       update $( t[k] := rhs $) $)\n\n"
          "let n = 1\nlet e ~~ Elem[n]\nn := 3\ne := 99\n"
          "Write[t[1], ' ', t[2], ' ', t[3], '*n']") +
   "\n\n`e` was fixed as element 1 when it was defined, and stayed element 1.")

# ------------------------------------------------------ input and output

qa("How do I read input in CPL?",
   "`Read` takes variables, by reference, and puts the next numbers from the "
   "input into them. Numbers are separated by spaces, commas or new lines. Input "
   "comes from standard input: the keyboard, or a file, as in "
   "`cpl examples/statistics.cpl < examples/statistics.in`.\n\n"
   + prog("let a, b = 0, 0\nRead[a, b]\nWrite[a, ' + ', b, ' = ', a + b, '*n']", "17 25\n"))

qa("How do I read numbers until the end of the list in CPL?",
   "There is no way to ask whether there is another number, so a list of numbers "
   "needs something to mark its end. A zero is traditional.\n\n"
   + prog("let x, s, n = 0, 0, 0\n$( Read[x]\n   if x = 0 break\n"
          "   s, n := s + x, n + 1 $) repeat\n"
          "Write['the mean of ', n, ' numbers is ', s/n, '*n']", "3 5 8 13\n21 0\n") +
   "\n\nReading past the end of the input stops the program with the message "
   "`Read: the input stream is exhausted`.")

qa("How do I read a character at a time in CPL?",
   "`Readchar` reads one character into a string variable. At the end of the "
   "input it gives the empty string, so a loop can stop there. This copies its "
   "input with every letter made a capital:\n\n"
   + prog("let c = ''\n$( Readchar[c]\n   if c = '' break\n"
          "   Write['a' <= c <= 'z' -> String[Index[c] - 32], c] $) repeat",
          "Cambridge and London\n"))

qa("How do I print numbers in columns in CPL?",
   "`Write` prints a number as briefly as it can. For tables there is "
   "`Printd[x, i, j]`, which prints in \"fixed point decimal style, with i figures "
   "before the decimal point, j figures after\". Zeros that are not needed are "
   "printed as spaces, so numbers line up in columns, and with `j` zero there is "
   "no decimal point.\n\n"
   + prog("Write['   x     Sqrt      Exp*n']\nfor x = step 0, 0.5, 3 do\n"
          "   $( Printd[x, 2, 1]\n      Printd[Sqrt[x], 2, 5]\n"
          "      Printd[Exp[x], 3, 4]\n      Write['*n'] $)"))

qa("What does Printf do in CPL?",
   "`Printf[x, i]` prints in \"floating decimal style\" with `i` significant "
   "figures. A number too big for the `Printd` it was given is printed this way "
   "too.\n\n"
   + prog("Printf[299792458, 4];  Write['*n']\nPrintf[-0.000123, 3];  Write['*n']\n"
          "Printd[123456, 3, 0];  Write['*n']") +
   "\n\nThe project notes that `Printd` and `Printf` follow the description in the "
   "supplement to the Elementary Programming Manual, and that the exact layout of "
   "sign and exponent is a guess.")

qa("How does Write print values of different types in CPL?",
   "`Write` takes any number of parameters of any type. A whole number is printed "
   "without a decimal point and a real number with one, so the output shows the "
   "type. Twelve significant figures are given, and very large or small numbers "
   "use `@` for \"times ten to the\".\n\n"
   + prog("Write['a string', ' ', 42, ' ', 2.5, ' ', true, ' ', #o777, ' ', "
          "Cpx[1, 2], '*n']\n"
          "Write[List[1, 'two'], ' ', real, ' ', 1/3, ' ', 1@20, ' ', 2/1@7, '*n']") +
   "\n\nThe routines of the supplement were spelt in capitals, and both spellings "
   "work: `WRITE` or `Write`. `Writechar` prints one character, given as a string "
   "or as its number:\n\n"
   + prog("Writechar['C']; Writechar[80]; Writechar['L']; Newline[]"))

qa("What input and output does this CPL not have?",
   "The designers of CPL had large plans for input and output, with streams and "
   "files and a `Layout` function, and never finished them; they were never "
   "specified, and are not implemented. What is provided is the handful of "
   "routines described in the two-page supplement to the Elementary Programming "
   "Manual for the first system that ran: `Write`, `Read`, `Readchar`, "
   "`Writechar`, `Printd` and `Printf`, with `Newline` and `Space`. "
   "`Readchar[stream, x]` accepts a stream name, and there is one stream. `Write` "
   "prints its arguments plainly rather than in the fixed-width formats of the "
   "pre-pilot system.")

# ------------------------------------------------------ common mistakes

qa("Why does let total = 0 give an error in CPL?",
   "Because a small letter is a whole name. `total` is five names side by side, a "
   "product, so after `t` the system expected the rest of a definition and found "
   "the name `o` instead.\n\n" + fails("let total = 0") +
   "\n\nA name longer than one letter must begin with a capital: write `Total`.")

qa("Why can't I multiply i by n by writing in in CPL?",
   "The words of the language (`let`, `if`, `for`, `do`, `in`, `result` and the "
   "rest) are written in small letters and are reserved. A run of small letters "
   "that spells one of them is that word, so `in` is the word `in`. For `i` times "
   "`n`, put a space or a star between them.\n\n"
   + prog("let i, n = 3, 4\nWrite[i n, ' ', i*n, '*n']"))

qa("What does 'used before it has been given a value' mean in CPL?",
   "A variable defined with `be` has a type and no value, and cannot be used "
   "until it has been assigned to.\n\n" + fails("let n be integer\nWrite[n + 1]") +
   "\n\nThe same happens with the elements of a new array, because `Newarray` "
   "does not give the elements values:\n\n"
   + fails("let a = Newarray[real, (1, 3)]\nWrite[a[2]]"))

qa("What does 'the name is not defined here' mean in CPL?",
   "A name is used outside the part of the program it belongs to. Common causes "
   "are using a `for` variable after its loop, a name from inside a block after "
   "the block has ended, and a recursive function defined without `rec`. The "
   "mistake is found when the program is compiled, so nothing is printed at "
   "all.\n\n" + fails("for i = 1 to 3 do Write[i, ' ']\nWrite[i]") +
   "\n\n" + fails("$( let y = 3\n   Write[y, '*n'] $)\nWrite[y]"))

qa("What does 'the real number is not integral' mean in CPL?",
   "A real value that is not a whole number is being put into an integer or index "
   "variable or parameter. A transfer only happens automatically when nothing is "
   "lost. Use `Round` or `Intpt`, or make the variable real.\n\n"
   + fails("let n be integer\nn := 1/3") +
   "\n\nIn this implementation a real transfers to an integer only if it is "
   "integral to within 1 part in 10⁹.")

qa("What does 'assignment to a constant' mean in CPL?",
   "The thing on the left of `:=` may not be changed. That happens with a name "
   "defined by `let constant`, with a function defined by `let`, and when an "
   "expression that has no L-value, such as `a + 1`, is passed by reference to a "
   "routine that assigns to it.\n\n"
   + fails("let rt Double[ref x] be x := 2x\n\nlet a = 5\nDouble[a + 1]"))

qa("What happens with runaway recursion in CPL?",
   "A program may have 100,000 activations live at once; beyond that it is taken "
   "to be in a runaway recursion and stopped. There is no tail-call "
   "elimination.\n\n"
   + fails("let rec Down[integer n] = Down[n - 1]\nWrite[Down[1]]"))

qa("What happens if I forget to close a section bracket in CPL?",
   "The program does not compile.\n\n"
   + fails("for i = 1 to 3 do\n   $( Write[i]") +
   "\n\n`cpl --check` gives the line and column of the bracket that was left "
   "open:\n\n" + check("loop.cpl", "for i = 1 to 3 do\n   $( Write[i]"))

qa("What happens if a section bracket tag does not match in CPL?",
   "A closing bracket with a tag closes the nearest open section with the same "
   "tag. If no open section has that tag, the program does not compile. A dot "
   "counts as part of a tag, so `R.T` and `R.T.` are different tags; the "
   "preprocessor of 1966 makes exactly this slip three times.\n\n"
   + fails("$(R.T Write['hello*n'] $)R.T."))

qa("What does division by zero do in CPL?",
   "It is an alarm: the program stops with a message.\n\n" + fails("Write[1/0]"))

qa("What happens on integer overflow in CPL?",
   "In this implementation index is 32 bits and integer 64, and index and integer "
   "overflow is an alarm.\n\n"
   + fails("let n be integer\nn := 9000000000000000000\nWrite[n + n]"))

qa("Why does my CPL function not see a later change to a variable?",
   "Because it was defined with `=`, which makes a fixed function: the values of "
   "its free variables are taken when the definition is obeyed, and kept. Define "
   "it with `==` to make a free function, which shares the variables.\n\n"
   + prog("let Rate = 10\nlet Fixed[x] = Rate x\nlet Free[x] == Rate x\n"
          "Rate := 20\nWrite[Fixed[2], ' ', Free[2], '*n']"))

qa("Why did my CPL routine not change the variable I passed to it?",
   "Parameters are called by value unless you say otherwise, so the routine was "
   "working on its own copy. Mark the parameter `ref`.\n\n"
   + prog("let rt Clear[x] be x := 0\nlet rt Clear'[ref x] be x := 0\n\n"
          "let a, b = 1, 2\nClear[a]\nClear'[b]\nWrite[a, ' ', b, '*n']"))

# ------------------------------------------------------ CPL and BCPL

qa("What did BCPL take out of CPL?",
   "Martin Richards describes BCPL as \"CPL with all the difficult bits "
   "removed\". The bits were these. CPL's types, with transfer functions inserted "
   "automatically, became one type, the machine word. The type `general`, a value "
   "carrying its type, went: nothing carries a type. Definition and parameters by "
   "reference became pointers, with `@` and `!`. Functions with free variables, "
   "fixed or free, went: BCPL has no dynamic free variables at all. LH functions "
   "were removed. `where`, and `in` for private variables, were removed, and "
   "`static` was added. Jumps out of function activations through label values "
   "became `goto` within a function, and `longjump`. `rec` is no longer required: "
   "every function is recursive. And juxtaposition for multiplication with "
   "one-letter names gave way to long names.")

qa("What survived from CPL into BCPL?",
   "Section brackets, `value of` and `result is` (as `valof` and `resultis`), "
   "`test`, `unless`, `repeat while`, simultaneous assignment and simultaneous "
   "declarations all survived into BCPL, along with `let`, `:=`, `if`, `while`, "
   "`until`, `repeat`, `for`, `break`, `return`, conditional expressions, and the "
   "L-value and the R-value, which became the address of a word and its contents. "
   "The switch went the other way: Richards says `SWITCHON` was included in BCPL "
   "and retro-fitted into CPL in 1966, where it is the switch-command of the "
   "1966 appendix. It still falls through in C.")

qa("What is the difference between CPL and BCPL?",
   "BCPL, \"Basic CPL\", is the small language Martin Richards designed for "
   "writing compilers after working on the Cambridge CPL compiler; he described it "
   "as CPL with all the difficult bits removed. CPL has many types with automatic "
   "transfer functions and a type `general` checked as the program runs; BCPL has "
   "one type, the machine word, and nothing is checked. CPL has definition and "
   "parameters by reference; BCPL passes parameters by value only, and the "
   "address of a variable can be taken with `@` and followed with `!`. CPL "
   "functions carry free variables, fixed or free, and can be returned as "
   "results; a BCPL function may not use the local variables of another at all. "
   "CPL has LH functions, `where` and `in`; BCPL does not, though it kept "
   "simultaneous declarations and `and`. CPL jumps out of functions through label values; BCPL has `level` and "
   "`longjump`. CPL needs `rec` for recursion; in BCPL every function may be "
   "recursive. CPL has one-letter names and multiplication without a sign; BCPL "
   "has long names in small letters. Richards had found that most of the "
   "difficulty of compiling CPL lay in its types, and a language for writing "
   "compilers did not need them.")

qa("What does the same program look like in CPL, BCPL and C?",
   "The highest common factor, in the CPL of 1966:\n\n"
   + prog("let rec HCF[integer n, m] =\n        (m > n -> HCF[m, n],\n"
          "         m = 0 -> n,\n         HCF[m, Rem[n, m]])\n\n"
          "Write[HCF[1071, 462], '*n']") +
   "\n\nIn BCPL, a few years later:\n\n"
   + block("LET HCF(N, M) = M > N -> HCF(M, N),\n                M = 0 -> N,\n"
           "                HCF(M, N REM M)") +
   "\n\nAnd in C:\n\n"
   + block("int hcf(int n, int m) {\n    return m > n ? hcf(m, n)\n"
           "         : m == 0 ? n\n         : hcf(m, n % m);\n}", "c") +
   "\n\nThe square brackets became round, `->` and the comma became `?` and `:`, "
   "and the types went away and came back.")

qa("What does C owe to CPL?",
   "`{` and `}` come from CPL's section brackets, § and its struck-through twin. "
   "`lvalue` and `rvalue`, in C compilers' error messages, are Strachey's L-value "
   "and R-value. `a ? b : c` is CPL's `a -> b, c`. `switch`, `case`, `default` and "
   "falling through come from the switch-command of 1966. `break`, `return`, "
   "`while`, `for` and `if` are the same words in CPL. `&x` and `*p` come from "
   "BCPL's `@` and `!`, which replaced definition by reference. An array name "
   "meaning the address of its elements comes from the CPL array as a pointer. "
   "`//` comments come from BCPL's `//`, standing in for CPL's `||`. What is not "
   "from CPL is assignment as an expression, as in `x = y = 0` and `+=`: in CPL, "
   "assignment was strictly a command.")

qa("What can CPL do that C cannot?",
   "A function can return a function that remembers. This needs a function to "
   "carry its free variables with it, which was one of the difficult bits that "
   "BCPL removed, and it cannot be written in BCPL, in B, or in C.\n\n"
   + prog("prefer integer\n\nlet Counter[] = value of\n   $( let n = 0\n"
          "      let Next[] == value of $( n := n + 1\n"
          "                                result is n $)\n      result is Next $)\n\n"
          "let c = Counter[]\nc[]; c[]\nWrite[c[], '*n']") +
   "\n\nOther things in CPL that have since come back into ordinary programming "
   "are values that carry their types, references, properties with getters and "
   "setters (LH functions), definitions in parallel, the matching of names "
   "against structures, and jumps that unwind many calls at once.")

# ------------------------------------------------------ the 1966 preprocessor

qa("What is the preprocessor in Appendix 1 of the CPL Working Papers?",
   "The CPL Working Papers of July 1966 contain one CPL program of any size: "
   "Appendix 1 of the Reference Manual draft, \"The Preprocessor\". It is about 190 "
   "lines, and is \"intended to be both a description of the transformation from "
   "an implementation alphabet to canonical CPL and a general example of a CPL "
   "program\". Across the top of its first page someone has written, by hand, "
   "\"This draft has not been fully checked\". The project's `appendix1/` "
   "directory has that program, transcribed from the scans of pages 2 to 6 of the "
   "appendix, and running: `preprocessor.cpl` is the program with its slips put "
   "right, each marked `|| printed:`, and `preprocessor-as-printed.cpl` is the "
   "same with the slips as printed, which does not compile.")

qa("What does the 1966 CPL preprocessor do?",
   "It reads a CPL program a character at a time, as typed on a Flexowriter, and "
   "writes the canonical symbols one after another, each followed by `ø`. Names, "
   "numbers and strings are marked with a struck-through N, K or S. Along the way "
   "it removes comments and spaces; puts in the `;` that the end of a line stands "
   "for, when the symbol before can end a command and the symbol after can start "
   "one; puts in the `do` that may be left out before `goto`, `break` and the "
   "like; tells a monadic `+` or `-` (`pos`, `neg`) from a dyadic one; joins "
   "words, so that `value of` becomes `valof` and `then do` becomes `do`; replaces "
   "the comma of a conditional expression by `comma`, and a row of dots in a "
   "for-list by `dot string`; and closes the sections that a tagged closing "
   "bracket implies. Run as\n\n"
   + block("cpl appendix1/preprocessor.cpl", "bash") + "\n\non the input\n\n"
   + block("l̲e̲t̲ a, b = 1, 2; i̲f̲ a > b g̲o̲t̲o̲ L") + "\n\nit prints:\n\n"
   + tool(["appendix1/preprocessor.cpl"], "l̲e̲t̲ a, b = 1, 2; i̲f̲ a > b g̲o̲t̲o̲ L\n"))

qa("How do I run the 1966 preprocessor?",
   "Build the system and give the preprocessor a program in the implementation "
   "alphabet on standard input:\n\n"
   + block("cargo build --release\n"
           "target/release/cpl appendix1/preprocessor.cpl < appendix1/sample.pub\n"
           "target/release/cpl appendix1/preprocessor.cpl < appendix1/itself.pub", "bash") +
   "\n\n`sample.pub` is a small program that shows each thing the preprocessor "
   "does:\n\n" + block((ROOT / "appendix1" / "sample.pub").read_text(encoding="utf-8")) +
   "\n\nand the preprocessor makes this of it:\n\n"
   + tool(["appendix1/preprocessor.cpl"],
          (ROOT / "appendix1" / "sample.pub").read_text(encoding="utf-8")) +
   "\n\n`tests/appendix1.rs` runs all of this and checks each of the eleven "
   "corrections.")

qa("How does the 1966 preprocessor use labels?",
   "It closes sections by recursion. `Preprocessor` calls itself for each opening "
   "section bracket, and is given a label to jump to \"if a closing section "
   "bracket is found with a tag which does not match with the current section\". "
   "The label belongs to the calling activation, so the jump unwinds one level, "
   "which writes its own closing bracket and compares its own tag, and so on "
   "outwards. This is the feature of section 9.3 of the manual, labels as values, "
   "used in earnest. The program is also one big switch on the character just "
   "read, and every one of its cases ends by jumping back to the top.")

qa("How many slips are there in the 1966 CPL preprocessor as printed?",
   "Eleven differences separate the printed text from the one that works. Seven "
   "are slips of the typewriter that stop the program from compiling at all: "
   "three mismatched section bracket tags, `Proprocessor` for `Preprocessor`, a "
   "`case` with no colon, `goto switch` for `goto Switch`, and `standard` for "
   "`Standard`. Two let it run and give the wrong answer: one takes the first "
   "letter of every underlined word twice, and one loses the character after "
   "every comma. And in two places the informal definitions of Part 3 of the "
   "appendix disagree with the program of Part 2. The corrections written on the "
   "copy by hand (`:=` for `: =` throughout, `return` for `goto Switch` on page 3, "
   "`Switch` for `Next` on page 4) are taken as part of the text and are not "
   "counted. The list is the reading of a scan of a typescript by one reader, and "
   "each slip can be checked against the page given. The first nine are slips, "
   "wrong under any reading; the last two are conflicts between two parts of the "
   "appendix, which could have been resolved another way.")

qa("Which slips stop the 1966 preprocessor from compiling?",
   "Seven, numbered 1 to 7 in the project's list, with the page of the appendix:\n\n"
   "1. Page 2: `ReadTag` opens `§R.T` and closes with the tag `R.T.`. No open "
   "section is tagged `R.T.`.\n"
   "2. Page 3: `ReadUnderlinedWord` opens `§R.U.W` and closes with `R.U.W.`. "
   "Likewise.\n"
   "3. Page 3: `ReadSpaces` opens `§R.S.` (the last mark is faint) and closes with "
   "`R.S`. Likewise.\n"
   "4. Page 3: `Proprocessor ['§', s, Mismatch]` should be `Preprocessor`. The "
   "name is not defined.\n"
   "5. Page 3: `case ')'` with no colon should be `case ')' :`. A `:` is "
   "expected.\n"
   "6. Page 4: `goto switch` should be `goto Switch`. A small-letter name is one "
   "letter, so this is not a label.\n"
   "7. Page 5: `s := standard [s <=> s']` should be `Standard`. The same: "
   "`standard` is eight names side by side.\n\n"
   "A dot counts as part of a tag by the program's own rule (`Letter.Digit.Dot`), "
   "which is why 1 to 3 are mismatches and not full stops. Running the text as "
   "printed stops at the first of them:\n\n"
   + block("cpl appendix1/preprocessor-as-printed.cpl", "bash") + "\n\n"
   + block(cpl(["appendix1/preprocessor-as-printed.cpl"])[2]))

qa("What does cpl --check say about the 1966 preprocessor as printed?",
   "It reports the three mismatched tags of slips 1 to 3, with line and column in "
   "the transcription, and exits with status 1. The other slips are only reached "
   "once these are put right.\n\n"
   + block("cpl --check appendix1/preprocessor-as-printed.cpl", "bash") + "\n\n"
   + tool(["--check", "appendix1/preprocessor-as-printed.cpl"], status=1))

qa("Which slips make the 1966 preprocessor give the wrong answer?",
   "Two, numbered 8 and 9, both on page 3 of the appendix. They let the program "
   "run and give the wrong answer.\n\n"
   "8. `ReadUnderlinedWord` begins `s := c` where it should be `s := ''`. The "
   "first letter of every word is taken twice, so `let` comes out with its first "
   "letter doubled.\n"
   "9. After a comma, the program calls `ReadSpaces [c]` and then `Output [','] ; "
   "NextChar [c]`; there should be no `NextChar`. `ReadSpaces` has already moved "
   "on, so the character after every comma is lost: `a, b = 1, 2` comes out as "
   "`a , = 1 ,`.")

qa("Where does Part 3 of the preprocessor appendix disagree with the program?",
   "In two places, numbered 10 and 11 in the project's list, both on page 6. Part "
   "3 defines informally the functions that Part 2 uses.\n\n"
   "10. Definition (c) says `CannotPreceedCommand` (so the scan reads; Part 2 "
   "calls it `CannotProceedCommand`) \"is true if LastOutput is any "
   "of\" `;` `§` `do` `:` `or`. It is taken as true if it is none of them. Those "
   "are the symbols that can precede a command; as printed a `do` is put in after "
   "every semicolon and never where it is wanted.\n"
   "11. Definition (f) says `Standard` gives `go` \"if x is\" `goto`. That row is "
   "taken out. Definitions (b) and (d) list `goto`, so as printed no `do` is put "
   "in before a jump.\n\n"
   "These are conflicts, better called open questions than slips, since Part 3 "
   "says of itself that it is informal, and neither correction is the only one "
   "possible: for 10, Part 2 could as well negate its call; for 11, the row could "
   "be kept and `goto` changed to `go` elsewhere.")

qa("What are the eleven slips in the 1966 CPL preprocessor?",
   "In the project's numbering, with pages of the appendix:\n\n"
   "1. Page 2: `ReadTag` opens a section tagged `R.T` and closes it as `R.T.`.\n"
   "2. Page 3: `ReadUnderlinedWord` opens `R.U.W` and closes `R.U.W.`.\n"
   "3. Page 3: `ReadSpaces` opens `R.S.` and closes `R.S`.\n"
   "4. Page 3: `Proprocessor` for `Preprocessor`.\n"
   "5. Page 3: `case ')'` with no colon.\n"
   "6. Page 4: `goto switch` for `goto Switch`.\n"
   "7. Page 5: `standard` for `Standard`.\n"
   "8. Page 3: `ReadUnderlinedWord` begins `s := c` for `s := ''`, so the first "
   "letter of every word is taken twice.\n"
   "9. Page 3: an extra `NextChar [c]` after a comma, so the character after "
   "every comma is lost.\n"
   "10. Page 6: Part 3 (c) says `CannotPreceedCommand` (Part 2's "
   "`CannotProceedCommand`) is true if `LastOutput` is "
   "any of `;` `§` `do` `:` `or`; it is taken as true if it is none of them.\n"
   "11. Page 6: Part 3 (f) says `Standard` gives `go` if x is `goto`; there "
   "should be no such row.\n\n"
   "The first seven stop the program from compiling, 8 and 9 let it run and give "
   "the wrong answer, and 10 and 11 are places where the informal definitions of "
   "Part 3 disagree with the program of Part 2.")

qa("What had to be supplied to make the 1966 preprocessor run?",
   "Part 3 of the appendix defines thirteen of the functions that Part 2 uses "
   "\"informally\", in a sentence or a list each; they are written out at the head "
   "of `preprocessor.cpl`. Seven more identifiers are used and never defined: "
   "`CPL.Source`, `EndofStreamChar`, `Report`, `BigLetter`, `SmallLetter`, and "
   "`UnderlinedLetter` with `Small.UnderlinedLetter`. They are supplied too. "
   "`Report[n]` writes `?? Report n` and carries on, since the program plainly "
   "expects it to return. \"The basic symbol set of canonical CPL\", which "
   "`Standardizable` needs, is taken to be the list called `Canonical`. These "
   "definitions are the reconstruction's own. Three go beyond the text: (a) ends "
   "its list with \"etc.\", read as the other types and `true` and `false`; (c) "
   "also accepts the empty string that `LastOutput` holds at the start; and "
   "`Canonical` has `goto`, not `go`, to agree with the resolution of slip 11.")

qa("How are underlined and struck-through characters represented in this CPL?",
   "An underlined letter, and a letter or sign with a stroke through it, were "
   "single characters on the Flexowriter, typed by overstriking. Here they are a "
   "character followed by a Unicode combining mark (U+0332 underline, U+0338 "
   "stroke), and `Readchar` returns the pair as one CPL character, so the "
   "preprocessor's `UnderlinedLetter[c]` and its case for the struck-through "
   "section sign work as written. `cpl --publication FILE` prints any program in "
   "this form; that is how the `.pub` files in `appendix1/` were made.")

qa("What happens when the 1966 preprocessor is given its own text?",
   "Corrected, the preprocessor can be given its own text. `itself.out` is the "
   "corrected preprocessor run on its own Part 2: 189 lines in, some 1,100 "
   "canonical symbols out, ending with `finish`. `itself-as-printed.out` is the "
   "corrected preprocessor run on Part 2 as printed. It stops a little over 200 "
   "symbols in, at slip 1: the closing bracket of `ReadTag` matches no open "
   "section, every level jumps to its mismatch label, and the outermost one is "
   "the main program's `Error: finish`. No message is written, because the "
   "program has none for this case.")

qa("Does the 1966 preprocessor know about the switch-command it is written with?",
   "Not quite, and this is not counted as a slip. The program is written with the "
   "switch-command, which had only just been added to the language; its "
   "definition is on the last page of the same appendix. Part 3 was not brought "
   "up to date with it: `jump` is not `Combinable`, so the preprocessor, given its "
   "own text, writes `jump on` and `the rest` as two symbols each.")

qa("Was the 1966 CPL preprocessor ever run at the time?",
   "The documents kept with the project do not say. The handwritten note at the "
   "head of it, \"This draft has not been fully checked\", suggests that this "
   "draft, at least, was not. As printed it does not compile; with seven slips of "
   "the typewriter put right it runs and is wrong in two ways; with all eleven "
   "differences put right it runs, on a sample and on its own text.")

# ------------------------------------------------------ choices and limits

qa("What parts of CPL are not implemented in this system?",
   "The project lists them. The rules for `break`, `return` and `result is` that "
   "replaced section 9.5 of the manual, because the new rules are not given; 9.5 "
   "is followed as written. Running the declarations of a block when jumping into "
   "it past them; the jump is made, and a variable whose definition was skipped "
   "is reported when used. The numerical constant prefixes, the basic symbol *i*, "
   "and `plus`, `minus` and `mult` of 1963. Input and output streams, `Layout` and "
   "files, which were never specified. The representation functions such as "
   "`Bitpattern` and `Formreal`, which depend on the word of a machine that is "
   "not being imitated. A substitution parameter is only passed unevaluated when "
   "the function called is named directly. \"Structure matching\" beyond one "
   "level. And checking of types before the program runs, beyond the deduction "
   "needed to insert transfer functions. No rearrangement of expressions is done, "
   "so every block behaves as if marked `sic`.")

qa("What extensions does this CPL add to the language of the papers?",
   "A few small ones. `let rec f[..] = .. and g[..] = ..` makes the group "
   "mutually recursive, where the manual wants the group in section brackets. "
   "Declarations may appear anywhere in a block. "
   "`Type[x]` gives the current type of a data item and `Cpx[a, b]` makes a "
   "complex number; neither is in the papers. List equality with `=` compares "
   "members. `#x` for hexadecimal logical constants is an extra, and `{` and `}` "
   "are accepted as section brackets.")

qa("What are the limits of this CPL implementation?",
   "A program may have 100,000 activations live at once; beyond that it is taken "
   "to be in a runaway recursion. There is no tail-call elimination. The load and "
   "update parts of LH functions, and substituted expressions, may nest 10,000 "
   "deep (120 in WebAssembly). Activations are kept in a list on the heap, not on "
   "the stack of the host, so a CPL program can recurse 100,000 deep in a "
   "browser.")

qa("How does a where-clause bind in this CPL?",
   "The manual says a where-clause qualifies \"the longest command, definition or "
   "expression immediately preceding it\", and twice notes that the rule \"will be "
   "changed\". Strachey's `Product` needs each `where` to qualify the definiens "
   "just before it, inside the scope of that definition's parameters. That is the "
   "rule used here: after a definiens, `where` belongs to that definiens; after a "
   "command, to the command.\n\n"
   + prog("let Hyp[a, b] = Sqrt[s] where s = a a + b b\nWrite[Hyp[3, 4], '*n']"))

qa("What are the basic functions and routines of CPL?",
   "Arithmetic: `Sqrt Exp Log Sin Cos Tan Arctan Arcsin Arccos Mod Sign Intpt "
   "Frpt Round Quot Rem`. Complex: `Cpx Realpt Imagpt`. Transfer functions: "
   "`Index Integer Real Double Complex DoubleComplex Logical LongLogical String "
   "Boolean`. Values: `RValue LValue Type`. Logical: `Lshift Rshift Rotate Ones "
   "Longones Field Mask Bit`. Strings: `Length First Last Character Initials "
   "Finals`. Arrays: `Newarray Formarray Copy Fix Bounds Equal`. Lists: `NIL Cons "
   "List List1 Hd Tl Null Atom Members Append Length`. Output: `Write Writechar "
   "Printd Printf Space Newline`. Input: `Read Readchar`.")

qa("What are the commands of CPL?",
   "`L := E` is assignment, and `L1, L2 := E1, E2` is simultaneous. `R[E, ...]` is "
   "a routine call. `$( ... $)` is a compound command or block, and the brackets "
   "may have tags. `if B do C` and `unless B do C`, where `then` may replace `do`. "
   "`test B then C1 or C2`. `while B do C` and `until B do C`. `C repeat`, "
   "`C repeat while B` and `C repeat until B`. `for v = list do C`, where the list "
   "is made of values, `a to b`, `step a, b, c` and `a, b, ..., c`; `for ext` uses "
   "an existing variable. `break` leaves the smallest enclosing loop, `return` "
   "leaves the routine, `result is E` leaves the `value of` block with a value, "
   "and `finish` ends the program. `Name: C` is a label and `go to E` jumps to a "
   "label value. `switch on E into $( case K: ... default: ... $)` chooses among "
   "cases.")

qa("What forms of definition does CPL have?",
   "`let x = E` makes a new variable, by value. `let x ~~ E` makes a new name for "
   "an existing variable, by reference. `let x == E` defines by substitution, the "
   "expression itself (1963). `let x be T`, `let x, y both be T` and `let x, y, z "
   "all be T` give a type and no value. `let x, y = E1, E2` defines several at "
   "once. `let constant x = E` may not be assigned to. `let F[P] = E` is a fixed "
   "function, with free variables by value, and `let F[P] == E` a free function, "
   "with free variables shared. `let routine R[P] be C` is a routine (`rt` for "
   "short; the `let` is optional). `let function F[P] be load B update B` is an LH "
   "function, with an optional `fix`. `let rec D` lets the names defined be used "
   "in their own definitions. `D1 and D2` defines in parallel, `D1 in D2` makes "
   "the names of D1 known only in D2, and `C where D` or `(E where D)` makes the "
   "names of D known only in C or E. `prefer T` sets the type assumed when none is "
   "given.")

qa("What is the order of precedence of CPL operators?",
   "From loosest to tightest: the conditional `B -> E1, E2`; `==` and `~==` "
   "(equivalent, not equivalent); `\\/` (or); `/\\` (and); the relations `=` `~=` "
   "`<` `<=` `>` `>=` `<<` `>>`, which may be chained; `<=>` (joining strings); "
   "`+` and `-`; `*`, `/` and writing side by side; `-` in front; `**`; `~` "
   "(not); and application, `F[E, ...]`, for functions and arrays. A `/` takes "
   "everything after it that is multiplied or divided, so `a/bc` is `a/(bc)`, "
   "and the right operand of `**` takes in multiplication and division too, so "
   "`a**bc` is `a**(bc)` (6.1.4).\n\n"
   + prog(r"""
Write[1 + 2 * 3 ** 2, ' ', 1 < 2 /\ 2 < 3 \/ false, ' ', 'a' <=> 'b' = 'ab', '*n']
"""))

# ------------------------------------------------------ programs

TASKS = {
    "hello": "Write Hello World in CPL.",
    "manual-demonstration": "What did the \"simple example of a CPL job\" from the "
                            "pre-pilot system supplement look like?",
    "manual-hcf": "Show me the Euclidean algorithm from the CPL Elementary Programming Manual.",
    "manual-lineq": "Show me the CPL Elementary Programming Manual's routine for two "
                    "simultaneous equations.",
    "manual-scalar-product": "Show me the scalar product example from the CPL Elementary "
                             "Programming Manual.",
    "manual-gaussquad": "Write a CPL program that integrates a function passed as a "
                        "parameter by Gaussian quadrature.",
    "manual-polynomial": "Show me the polynomial example from the CPL Elementary "
                         "Programming Manual.",
    "factorial": "Show me the example functions from \"The main features of CPL\" (1963).",
    "euler": "Write a CPL program that sums a series by Euler's transformation.",
    "labels": "Show me the CPL Reference Manual's routine that jumps to Even or Odd.",
    "product": "Write the Cartesian product of a list of lists in CPL.",
    "lvalues": "Write a CPL program that demonstrates L-values and R-values.",
    "fixed-and-free": "Write a CPL program that demonstrates fixed and free functions.",
    "lh-functions": "Write a CPL program that demonstrates LH functions.",
    "sparse-array": "Write a sparse array in CPL using an LH function.",
    "types": "Write a CPL program that demonstrates types, transfer functions and the "
             "polymorphic operators.",
    "general": "Write a CPL program that demonstrates the type general.",
    "type-dispatch": "Write a CPL routine that describes a data item of any type.",
    "own-variables": "Write a CPL program that demonstrates in, and, where and rec.",
    "sections": "Write a CPL program that demonstrates tagged section brackets.",
    "arrays": "Write a CPL program that demonstrates arrays.",
    "strings": "Write a CPL program that demonstrates strings and the string functions.",
    "bits": "Write a CPL program that demonstrates logical values and the bit functions.",
    "bitsets": "Write a CPL program that uses long logicals as sets.",
    "lists": "Write a CPL program that demonstrates lists with map, filter and fold.",
    "matching": "Write a CPL program that demonstrates functions with several results and "
                "the matching of names against lists.",
    "substitution": "Write a CPL program that demonstrates call by substitution and "
                    "Jensen's device.",
    "switch": "Write a CPL program that demonstrates the switch-command.",
    "state-machine": "Write a state machine in CPL using a label variable.",
    "error-exits": "Write a CPL program that uses a label parameter as the way out of a "
                   "deep recursion.",
    "counters": "Write CPL functions that return functions which remember.",
    "compose": "Write a CPL program that composes functions.",
    "streams": "Write a CPL program that uses lists without end to find primes.",
    "ackermann": "Write Ackermann's function in CPL.",
    "big-factorial": "Write a CPL program that calculates 52 factorial digit by digit.",
    "binary-search": "Write a binary search in CPL.",
    "bottles": "Write a CPL program that prints the Ten Green Bottles song.",
    "bubble-sort": "Write a bubble sort in CPL.",
    "caesar": "Write a Caesar cipher in CPL.",
    "calculator": "Write an expression calculator in CPL.",
    "calendar": "Write a CPL program that prints a calendar.",
    "change": "Write a CPL program that counts the ways of making change.",
    "collatz": "Write a CPL program that prints a Collatz sequence.",
    "diamond": "Write a CPL program that prints a diamond of stars.",
    "differentiate": "Write a CPL program that differentiates expressions symbolically.",
    "fibonacci": "Write a CPL program that prints Fibonacci numbers.",
    "fizzbuzz": "Write FizzBuzz in CPL.",
    "gauss": "Write Gaussian elimination in CPL.",
    "guessing": "Write a CPL program that guesses a number by bisection.",
    "hanoi": "Write the Towers of Hanoi in CPL.",
    "josephus": "Write a CPL program that solves the Josephus problem.",
    "life": "Write Conway's Life in CPL.",
    "list-sort": "Write insertion sort and merge sort on lists in CPL.",
    "magic-square": "Write a CPL program that prints a magic square.",
    "mandelbrot": "Write a CPL program that draws the Mandelbrot set.",
    "multiplication-table": "Write a CPL program that prints a multiplication table.",
    "newton": "Write a CPL program that finds square roots by Newton's method.",
    "palindromes": "Write a CPL program that tests for palindromes.",
    "pascal-triangle": "Write a CPL program that prints Pascal's triangle.",
    "perfect-numbers": "Write a CPL program that finds perfect numbers.",
    "permutations": "Write a CPL program that lists the permutations of a list.",
    "pi-digits": "Write a CPL program that prints digits of pi.",
    "primes": "Write a CPL program that prints prime numbers.",
    "quadratic": "Write a CPL program that solves quadratic equations.",
    "queens": "Write a CPL program that solves the eight queens problem.",
    "quicksort": "Write quicksort in CPL.",
    "random": "Write a CPL program that generates random numbers.",
    "roman-numerals": "Write a CPL program that converts numbers to Roman numerals.",
    "simpson": "Write a CPL program that integrates by Simpson's rule.",
    "statistics": "Write a CPL program that prints the mean and deviation of numbers "
                  "read from the input.",
    "temperature": "Write a CPL program that prints a table of temperatures.",
    "truth-tables": "Write a CPL program that prints truth tables.",
    "word-count": "Write a CPL program that counts the lines, words and characters of "
                  "its input.",
}
missing = sorted(p.stem for p in EXAMPLES.glob("*.cpl") if p.stem not in TASKS)
if missing:
    sys.exit(f"examples with no question in TASKS: {', '.join(missing)}")
for name, question in TASKS.items():
    qa(question, example(name))

questions = [row["messages"][1]["content"] for row in rows]
repeated = sorted({q for q in questions if questions.count(q) > 1})
if repeated:
    sys.exit(f"questions asked twice: {repeated}")

text = "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in rows)
if "--check" in sys.argv[1:]:
    if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
        sys.exit(f"{OUT} is out of date: run python3 tutorial/build_dataset.py")
    print(f"{OUT} is up to date ({len(rows)} examples)")
else:
    OUT.parent.mkdir(exist_ok=True)
    OUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {len(rows)} examples to {OUT}")
