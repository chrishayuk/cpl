# cpl

An implementation of **CPL**, the Combined Programming Language designed at
Cambridge and London between 1962 and 1966 by Christopher Strachey, David
Barron, John Buxton, David Hartley, Eric Nixon and others. CPL is the
ancestor of BCPL, and through BCPL of B and C. The whole language was not
implemented in the 1960s: the London compiler was for a subset, CPL1, with
"no types general, complex or double length" and no lists, and the
Cambridge one, in Martin Richards' words, "did not result [in] a
satisfactory implementation" (his text has "result is", plainly a slip).

This one is written from the documents: chiefly the unfinished *Reference
Manual* in the *CPL Working Papers* of July 1966. It is a compiler to an
abstract stack machine, and an interpreter for that machine, in Rust with no
dependencies.

```
let rec HCF[integer n, m] =
        (m > n -> HCF[m, n],
         m = 0 -> n,
         HCF[m, Rem[n, m]])

Write['HCF[1071, 462] = ', HCF[1071, 462], '*n']
```

```text
HCF[1071, 462] = 21
```

That is the Euclidean algorithm from section 24 of the *Elementary
Programming Manual*, changed only in the character set and in one bracket:
the manual prints the first recursive call as `HCF m, n]`.

- **New to CPL?** Start with the [tutorial](tutorial/01-introduction.md):
  19 chapters, from hello world to LH functions.
- **Want something to run?** There are [73 example programs](#examples),
  ten of them taken from the documents of 1963 to 1966.
- **Want the 1966 program?** [Appendix 1](appendix1/README.md): the one
  large CPL program in the documents, run here, slips and all.
- **No terminal handy?** The [browser playground](#in-the-browser) runs the
  same system in a web page.
- **Writing some?** There is a [language server and a Visual Studio Code
  extension](#in-your-editor).

## Install

You need [Rust](https://rustup.rs). To install the system and the language
server from this repository:

```bash
cargo install --git https://github.com/chrishayuk/cpl cpl cpl-lsp
```

That puts `cpl` and `cpl-lsp` in `~/.cargo/bin`. Then:

```bash
cpl -e "Write['Hello, world!*n']"
```

For Visual Studio Code, download `cpl-language-0.1.1.vsix` from the
[v0.1.1 release](https://github.com/chrishayuk/cpl/releases/tag/v0.1.1)
and install it:

```bash
code --install-extension cpl-language-0.1.1.vsix
```

To build the extension yourself instead, see
[`editors/vscode`](editors/vscode/README.md).

Open any `.cpl` file and you get highlighting, hover help, problems as
you type, and a run button. The extension carries its own copy of the
system and the language server, so it works without the `cargo install`
above. For Neovim, Helix and other editors see
[`editors/README.md`](editors/README.md).

To install nothing at all, there is a playground that runs CPL in your
browser at <https://chrishayuk.com/demos/cpl>, and a note about the 1966
preprocessor at
<https://chrishayuk.com/notebook/a-1966-program-said-it-had-not-been-checked>.

## Quick start

To work on the system itself, clone the repository and:

```bash
cargo build --release

target/release/cpl examples/hello.cpl
target/release/cpl examples/product.cpl
target/release/cpl examples/mandelbrot.cpl
target/release/cpl examples/calculator.cpl < examples/calculator.in
```

Or install it as `cpl` with `cargo install --path .`.

```text
Usage: cpl [OPTIONS] <FILE.cpl>
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
  -V, --version        Show the version
```

```bash
target/release/cpl -e "let x = 3; Write[2x + 1, '*n']"
```

`Read` and `Readchar` take the program's input from standard input. The
exit status is 0 when the program finishes, 1 on an error in the program
(reported on standard error with its line number), 2 for bad arguments.

## A taste of the language

Small letters are whole names, so multiplication needs no sign; a
definition can follow the thing it explains.

```
let a, b, c = 1, -3, 2
Write[(-b + d)/(2a), ' ', (-b - d)/(2a), '*n'] where d = Sqrt[b b - 4a c]
```

```text
2.0 1.0
```

A function is code together with its free variables. Defined with `=` it
takes their values for good; defined with `==` it shares the variables.

```
let a = 1
let f[x] = a x
let g[x] == a x
a := 100
Write[f[5], ' ', g[5], '*n']
```

```text
5.0 500.0
```

An LH function is a variable of your own design: `load` says what its
value is, `update` what assigning to it does.

```
let c = 100
let function Fahrenheit[] be
       load $( result is 9c/5 + 32 $)
       update $( c := 5(rhs - 32)/9 $)

Fahrenheit[] := 32
Write[c, '*n']
```

```text
0.0
```

A label is a value. Passed to a routine, it is the way out when something
goes wrong, however many calls deep. This is from the Elementary
Programming Manual:

```
routine LINEQ[real a, b, c, a', b', c', ref x, y, val label L] be
$( let DET = ab' - a'b
   if Mod[DET] < 1@-6 go to L
   x := (cb' - c'b)/DET
   y := (ac' - a'c)/DET $)
```

Each of these is in the tutorial, where the test suite runs it.

## What is implemented

Section numbers are those of the Reference Manual draft. Each program in
`examples/` has its expected output beside it, and the test suite runs
them all.

| Feature | Manual | Program |
| --- | --- | --- |
| L-values and R-values; definition by value `=` and by reference `≃`; parameters by `value` and by `reference`; L-mode conditionals and `reference of` blocks; constants | 4.2, 7.2, 7.3, 10.3.3 | `lvalues.cpl` |
| Fixed functions (`=`, free variables taken by value when defined) and free functions (`≡`, free variables shared) | 10.3.4 | `fixed-and-free.cpl` |
| LH functions: programmer-defined L-values with `load`, `update`, `rhs`, `fix` | 10.3.5 | `lh-functions.cpl` |
| Types index, integer, real, double, complex, double complex, Boolean, logical, long logical, string, label, function, routine, array, type, general; transfer functions; polymorphic operators (Rule A); compound relations; juxtaposition | 3, 6 | `types.cpl` |
| The type `general` | 3.2.4, 3.3.4 | `general.cpl` |
| Functions and routines as data items; `rec`; `and`, `in`, `where` | 8, 10 | `own-variables.cpl`, `lists.cpl` |
| Labels as data items, jumps out of activations, jumps into cycles | 9.3 | `labels.cpl` |
| `if`, `unless`, `test`, `while`, `until`, `repeat`, `repeat while`, `repeat until`, `for` with `step`, `to` and dot strings, `break`, `return`, `finish`, `value of` / `result is` | 9, 4.4 | `factorial.cpl`, `euler.cpl`, `queens.cpl` |
| The switch-command | App. 1, p. 8 | `switch.cpl` |
| Arrays as functions; `Newarray`, `Formarray`, `Copy`, `Fix`, `Bounds`, `Equal`; non-rectangular arrays | 11 | `arrays.cpl` |
| Functions with several results; simultaneous assignment | 4.5, 9.2 | `matching.cpl` |
| Strings and the string functions | Elem. Manual 26 | `strings.cpl` |
| Logical and long logical values; `Lshift`, `Rshift`, `Rotate`, `Ones`, `Longones`, `Field`, `Mask`, `Bit` | Elem. Manual 25 | `bits.cpl`, `bitsets.cpl` |
| Routine headings without `let`; `for ext`; `<<` and `>>` | Elem. Manual 13, 20, 21 | `manual-lineq.cpl`, `manual-scalar-product.cpl` |
| `Bit` and `Field` on the left of an assignment; bar-justified logical constants (`#o\|273`) | Elem. Manual 25.3, 25.4 | `bits.cpl` |
| Publication CPL itself as input: underlined basic symbols, § and its struck-through twin, `≠ ∧ ∨ → ×`; overstruck characters read as one | 1.2, 2.1 | `appendix1/` |
| The preprocessor program | App. 1 | `appendix1/preprocessor.cpl` |
| `Write`, `Read`, `Readchar`, `Writechar`, `Printd`, `Printf` | Elem. Manual, supplement | `manual-demonstration.cpl`, `temperature.cpl` |
| Tagged section brackets; layout as terminator; comments | 2.1 | `sections.cpl` |

Three things come from the 1963 paper *The Main Features of CPL* and are not
in the 1966 manual:

| Feature | 1963 | Program |
| --- | --- | --- |
| Call by substitution (ALGOL call by name), and definition by substitution. Richards: "by mid 1966 call by substitution and the corresponding form of variable definition were removed" | 16, 19 | `substitution.cpl` |
| Lists: `Cons`, `Hd`, `Tl`, `Null`, `List`, `Members`. The 1966 manual defers all "compound data structures" | 23 | `lists.cpl`, `product.cpl` |
| A list of names matched against a list value: "the transfer function Members is automatically invoked" | 24 | `matching.cpl` |

`product.cpl` runs the Cartesian product attributed to Strachey, three
nested where-clauses and all, in the form in which it is usually quoted (the
text was supplied with the brief for this project and has not been checked
against a primary source):

```
let Product[L] = Lit[f, List1[NIL], L]
    where f[k, z] = Lit[g, NIL, k]
          where g[x, y] = Lit[h, y, z]
                where h[p, q] = Cons[Cons[x, p], q]
```

## What BCPL took out

Martin Richards describes BCPL as "CPL with all the difficult bits removed".
These are the bits, and where to see each one working.

| In CPL | In BCPL | Program |
| --- | --- | --- |
| Types, with transfer functions inserted automatically | One type, the machine word | `types.cpl` |
| `general`: a value carrying its type | Nothing carries a type | `general.cpl` |
| Definition and parameters by reference | Pointers, `@` and `!` | `lvalues.cpl` |
| Functions with free variables, fixed or free | No dynamic free variables at all | `fixed-and-free.cpl`, `lists.cpl` |
| LH functions | Removed | `lh-functions.cpl` |
| `where`, and `in` for private variables | Removed; `static` added | `own-variables.cpl` |
| Jumps out of function activations through label values | `goto` within a function; `longjump` | `labels.cpl` |
| `rec` required for recursion | Every function recursive | `own-variables.cpl` |
| Juxtaposition for multiplication, one-letter names | Long names | `types.cpl` |

Section brackets, `value of` / `result is`, `test`, `unless`, `repeat
while`, simultaneous assignment and simultaneous declarations all survived
into BCPL. The switch went the other way: Richards says `SWITCHON` was
included in BCPL and "retro-fitted into CPL in 1966", which fits its
appearing only in the last pages of the 1966 appendix. The switch that
still falls through in C is in `switch.cpl`.

## Writing CPL in ASCII

CPL was written on a Flexowriter with 162 printing characters, underlining
and overstriking. "Any practical implementation of CPL will probably accept
an input which differs to some extent from both publication and canonical
CPL" (1.2.1), and this one does:

| Publication CPL | Here |
| --- | --- |
| underlined basic symbols: **let**, **value of** | reserved lower case words: `let`, `value of` (or `valof`) |
| § and its struck-through closing form, with tags | `$(` and `$)`, with tags (`$(1.2 ... $)1.2`), or `{` and `}` |
| `≃` definition by reference | `~~` |
| `≡` free function, or equivalence | `==` |
| `→` in a conditional | `->` |
| `×`, `↑`, `≠`, `≤`, `≥` | `*`, `**` or `^`, `~=`, `<=`, `>=` |
| `∧`, `∨`, `≢`, `∼` | `/\`, `\/`, `~==`, `~` |
| subscript ten: 2.5₁₀−8 | `2.5@-8` |
| logical constants **8** 777, **2** 1010 | `#o777`, `#b1010`, `#x1FF` |
| string escapes | `*n` `*s` `*t` `*b` `*'` `**` |
| comment | `||` to the end of the line |

The Unicode forms `→ ≡ ≢ ≃ ∧ ∨ ¬ ≠ ≤ ≥ × ↑` are accepted too.

**Names follow the CPL rule.** A small name is one lower case letter,
optionally primed (`x`, `y'`). A large name begins with a capital (`Total`,
`Fact2`, `Digit.Dot`). So `ax` is `a` times `x`, and `count` is a product of
five variables. Because the basic symbols cannot be underlined, a run of
lower case letters that spells one *is* one: `in` is the basic symbol, and
`i` times `n` must be written `i n` or `i*n`.

Layout is as in 2.1.2: the end of a line ends a command unless the command
could not end there. An expression continues over a line break after an
operator or before any operator that cannot begin a command; a line
beginning with a name, a number, `(`, `+` or `-` starts a new command.
`do` (or `then`) may be left out before `goto`, `break`, `return` and the
like.

Names of basic functions are those of the manuals: `Sqrt Exp Log Sin Cos Tan
Arctan Arcsin Arccos Mod Intpt Frpt Quot Rem Sign Round`, the transfer
functions `Index Integer Real Double Complex DoubleComplex Logical
LongLogical String Boolean`, `RValue LValue`, `Length First Last Character
Initials Finals`, `Lshift Rshift Rotate Ones Longones Field Mask Bit`, and `Write Read
Readchar Writechar Printd Printf Newline Space`. The full list is in
[chapter 19 of the tutorial](tutorial/19-reference.md).

## The preprocessor of 1966

The Working Papers contain one CPL program of any size: the preprocessor
of Appendix 1, about 190 lines, headed in handwriting "This draft has not
been fully checked". [`appendix1/`](appendix1/README.md) has it transcribed
from the scans and running:

```bash
target/release/cpl appendix1/preprocessor.cpl < appendix1/sample.pub
```

```text
l̲e̲t̲ a, b = 1071, 462
i̲f̲ a < b g̲o̲t̲o̲ Swap
```

```text
letøN̸aø,øN̸bø=øK̸1071ø,øK̸462ø; øiføN̸aø<øN̸bødo øgotoøN̸Swapø
```

As printed it does not compile: seven slips of the typewriter stop it
(mismatched bracket tags, `Proprocessor`, `goto switch`). With those put
right it runs and is wrong in two ways, one of which loses the character
after every comma. Two more changes resolve places where the informal
definitions of Part 3 disagree with the program of Part 2. These eleven
differences are listed, with page numbers and their kind (slip or
conflict), in the appendix's README, and `tests/appendix1.rs` checks what
each one does. The program also needs about twenty definitions that the
appendix gives only informally or not at all; those are written for this
reconstruction, and the README says where they go beyond the text.
Corrected, the preprocessor can be given its own text; given its text as
printed, it stops silently at its own first mismatched bracket.

`cpl --publication FILE` prints any program the way the papers print
them, and the result is itself accepted as a program:

```bash
target/release/cpl --publication examples/manual-lineq.cpl
```

```text
r̲o̲u̲t̲i̲n̲e̲ LINEQ[r̲e̲a̲l̲ a, b, c, a', b', c', r̲e̲f̲ x, y, v̲a̲l̲ l̲a̲b̲e̲l̲ L] b̲e̲
§ l̲e̲t̲ DET = ab' - a'b
   i̲f̲ Mod[DET] < 1@-6 g̲o̲ t̲o̲ L
   x := (cb' - c'b)/DET
   y := (ac' - a'c)/DET §̸
```

## Examples

Every program in `examples/` has a `.out` file with its expected output,
and a `.in` file if it reads input. `cargo test` runs them all. A program
whose `.out` ends with a line beginning `!` ends, on purpose, with that
error.

From the documents. Each file's header says what was changed from the
printed text, besides the character set and the notation of 1966:

| Program | |
| --- | --- |
| `manual-demonstration.cpl` | The "simple example of a CPL job" from the supplement for the pre-pilot system |
| `manual-hcf.cpl` | Elementary Programming Manual 24: the Euclidean algorithm |
| `manual-lineq.cpl` | Elementary Programming Manual 20.1: two simultaneous equations, with a label for "no solution" |
| `manual-scalar-product.cpl` | Elementary Programming Manual 21 (a): a scalar product, with a label for "orthogonal" |
| `manual-gaussquad.cpl` | Elementary Programming Manual 21 (b): Gaussian 3-point integration of a function parameter |
| `manual-polynomial.cpl` | Elementary Programming Manual 22: a polynomial as a result expression |
| `factorial.cpl` | The example functions of "The main features of CPL" (1963), in the notation of 1966 |
| `euler.cpl` | Euler's transformation of a series, from the 1963 paper: an adaptation, with the names changed |
| `labels.cpl` | Reference Manual 9.3.2: the routine that jumps to `Even` or `Odd` |

And one that is not from the documents read here:

| Program | |
| --- | --- |
| `product.cpl` | The Cartesian product attributed to Strachey, as usually quoted; not checked against a primary source |

One for each part of the language:

| Program | |
| --- | --- |
| `lvalues.cpl` | L-values and R-values; definition and parameters by value and by reference |
| `fixed-and-free.cpl` | Fixed functions take the values of their free variables, free functions share them |
| `lh-functions.cpl` | Variables defined by `load` and `update` |
| `sparse-array.cpl` | An LH function: a vector of a million elements that keeps only those in use |
| `types.cpl` | Types, transfer functions and the polymorphic operators |
| `general.cpl` | The type `general`: a value that carries its type |
| `type-dispatch.cpl` | A routine that describes a data item of any type |
| `own-variables.cpl` | `in`, `and`, `where`, `rec`: the scope rules for definitions |
| `sections.cpl` | Tagged section brackets |
| `arrays.cpl` | Arrays as functions; `Newarray`, `Formarray`, `Copy`; arrays that are not rectangles |
| `strings.cpl` | Strings and the string functions |
| `bits.cpl` | Logical values and the bit functions |
| `bitsets.cpl` | Sets as long logicals, and a sieve in 48 bits |
| `lists.cpl` | The lists of 1963, with map, filter and fold |
| `matching.cpl` | Several results; lists of names matched against lists of values |
| `substitution.cpl` | Call by substitution and Jensen's device |
| `switch.cpl` | The switch-command of 1966, falling through as it still does in C |
| `state-machine.cpl` | A label variable as the state of a machine that recognises numbers |
| `error-exits.cpl` | A label parameter as the way out of a deep recursion |
| `counters.cpl` | Functions that return functions which remember |
| `compose.cpl` | Composition, repeated application, a derivative as a function of functions |
| `streams.cpl` | Lists without end, and the sieve of all the primes |

Familiar programs:

| Program | |
| --- | --- |
| `hello.cpl` | Hello, world |
| `ackermann.cpl` | Ackermann's function |
| `big-factorial.cpl` | 52 factorial, digit by digit |
| `binary-search.cpl` | Binary search, with two results |
| `bottles.cpl` | Ten green bottles |
| `bubble-sort.cpl` | Bubble sort |
| `caesar.cpl` | A Caesar cipher |
| `calculator.cpl` | An expression calculator by recursive descent, reading lines of input |
| `calendar.cpl` | A calendar for July 1966 |
| `change.cpl` | Ways of paying a pound in the coins of 1966 |
| `collatz.cpl` | The Collatz sequence |
| `diamond.cpl` | A diamond of stars |
| `differentiate.cpl` | Symbolic differentiation with lists |
| `fibonacci.cpl` | Fibonacci numbers three ways |
| `fizzbuzz.cpl` | FizzBuzz, with a conditional expression of mixed type |
| `gauss.cpl` | Gaussian elimination |
| `guessing.cpl` | The computer guesses your number |
| `hanoi.cpl` | The Towers of Hanoi |
| `josephus.cpl` | The Josephus problem, on a list joined into a ring |
| `life.cpl` | Conway's Life: a glider |
| `list-sort.cpl` | Insertion sort and merge sort on lists, with the ordering as a parameter |
| `magic-square.cpl` | A magic square |
| `mandelbrot.cpl` | The Mandelbrot set, in complex arithmetic |
| `multiplication-table.cpl` | A multiplication table |
| `newton.cpl` | Square roots by Newton's method |
| `palindromes.cpl` | Palindromes |
| `pascal-triangle.cpl` | Pascal's triangle |
| `perfect-numbers.cpl` | Perfect numbers and amicable pairs |
| `permutations.cpl` | Permutations and sublists of a list |
| `pi-digits.cpl` | Forty digits of pi by a spigot |
| `primes.cpl` | The sieve of Eratosthenes |
| `quadratic.cpl` | Roots of a quadratic, real or complex |
| `queens.cpl` | The eight queens |
| `quicksort.cpl` | Quicksort |
| `random.cpl` | Random numbers, and pi by throwing darts |
| `roman-numerals.cpl` | Roman numerals |
| `simpson.cpl` | Integration by Simpson's rule |
| `statistics.cpl` | Mean and deviation of numbers read from the input |
| `temperature.cpl` | A table of temperatures |
| `truth-tables.cpl` | Truth tables, and De Morgan's law |
| `word-count.cpl` | Lines, words and characters of the input |

## Tutorial and dataset

[`tutorial/`](tutorial/01-introduction.md) is a 19-chapter tutorial that
assumes no CPL and not much programming.

| | |
| --- | --- |
| [1](tutorial/01-introduction.md) Introduction | [11](tutorial/11-arrays.md) Arrays |
| [2](tutorial/02-hello-world.md) Hello, world | [12](tutorial/12-strings-and-bits.md) Strings and bits |
| [3](tutorial/03-numbers.md) Numbers | [13](tutorial/13-types.md) Types and the type general |
| [4](tutorial/04-definitions.md) Names and definitions | [14](tutorial/14-lists.md) Lists |
| [5](tutorial/05-conditions.md) Conditions | [15](tutorial/15-labels-and-jumps.md) Labels and jumps |
| [6](tutorial/06-loops.md) Loops | [16](tutorial/16-lh-functions.md) New kinds of variable |
| [7](tutorial/07-blocks-and-scope.md) Blocks and scope | [17](tutorial/17-input-and-output.md) Input and output |
| [8](tutorial/08-functions.md) Functions | [18](tutorial/18-what-became-of-it.md) What became of it |
| [9](tutorial/09-routines-and-references.md) Routines and references | [19](tutorial/19-reference.md) Reference |
| [10](tutorial/10-fixed-and-free.md) Fixed and free functions | |

It contains 205 complete programs, each followed by its output. The test
`tests/tutorial.rs` runs every one and compares, so the tutorial cannot
drift from the implementation; `CPL_BLESS=1 cargo test --test tutorial`
rewrites the outputs after a deliberate change.

`tutorial/dataset/cpl.jsonl` is a small chat-format training set about
CPL. `python3 tutorial/build_dataset.py` rebuilds it; every output in an
answer comes from running the program.

## In the browser

`web/` is a playground that runs CPL in a web page: an editor, the
examples in a menu, a box for the program's input, **Run**, **Stop**, and
**Code** to see the stack machine a program is compiled to, and **Printed**
to see the program as the papers would have set it. The system is
compiled from the same source for WASI (`wasm32-wasip1`, about 700 KB), and
`web/wasi.js` is a small host that supplies the arguments, standard input
and standard output, so no other library is needed. Each run happens in a
worker, which is how an endless program can be stopped.

`web/editor.js` turns the program box into an editor with colours, problems
as you type, hover help and completion. It loads CodeMirror from a CDN, so
that part needs a network connection; without one the plain box still works.
`web/lsp-worker.js` runs the language server in a page the same way as the
system: it is the WASI build of `cpl-lsp`, given one program and a list of
questions per run.

```bash
node web/build.mjs                  # builds web/cpl.wasm, web/cpl-lsp.wasm and web/examples.json
node web/test.mjs                   # runs every example through the wasm build, and questions the server
python3 -m http.server -d web 8000  # then open http://localhost:8000
```

Add `?example=mandelbrot` to the address to open a particular example, and
`&run` to run it straight away.

## In your editor

`cpl --check program.cpl` reads a program without running it and reports
what is wrong, with the line and the column: a string or a section bracket
that is never closed, a tag that matches nothing, a mistake of syntax, a
name that is not defined. The messages are the system's own. Given

```
let Total, n = 0, 3
for i = 1 to n do
   $( Total := Total + i
Write['Total is ', total, '*n]
```

```bash
cpl --check sum.cpl
```

```text
sum.cpl:3:4: error: this section bracket is never closed
sum.cpl:4:27: error: string constant has no closing quote
```

The exit status is 1 if any program has an error. Every example passes
but the one that is there to show a mistake the compiler finds:

```bash
target/release/cpl --check examples/*.cpl
```

```text
examples/fixed-and-free.cpl:23:25: error: Count is a free variable of the fixed function Bad, which may not assign to it (a function defined with == is free and may)
```

The same analysis drives a language server, `cpl-lsp` in `lsp/`, with
hover help for every basic symbol and basic function, go to definition,
references, rename, completion and more. `editors/vscode` is a Visual
Studio Code extension that bundles the server and the system as
WebAssembly and adds syntax highlighting, for publication CPL in `.pub`
files too, and a run button. See [`editors/README.md`](editors/README.md).

The system itself still has no dependencies. The language server is a
separate crate in the same workspace and uses `serde_json`.

## WebAssembly + WASI

The same crate builds for any WASI preview 1 runtime:

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1

wasmtime run --dir=. target/wasm32-wasip1/release/cpl.wasm examples/queens.cpl
wasmtime run target/wasm32-wasip1/release/cpl.wasm -e "Write[2**10, '*n']"
```

`--dir=.` lets the sandbox read the `.cpl` file; with `-e` it needs no
access to anything. Two checks keep the WASI build honest:

```bash
cargo test --target wasm32-wasip1   # the whole test suite, run inside wasmtime
scripts/wasi-check.sh               # every example: native and WASI output must be identical
```

## Tests

```bash
cargo test
```

| | |
| --- | --- |
| `tests/programs.rs` | runs all 73 examples against their `.out` files (`CPL_BLESS=1` rewrites them), and 32 tests of single features: juxtaposition, the two kinds of function, type deduction, labels, recursion 50,000 deep, the bit functions, the output routines |
| `tests/tutorial.rs` | runs the 205 programs of the tutorial |
| `python3 tutorial/build_dataset.py --check` | rebuilds the dataset from real runs and fails if `tutorial/dataset/cpl.jsonl` would change |
| `tests/appendix1.rs` | runs the 1966 preprocessor on a sample and on itself, and checks each of the eleven corrections to it |
| `tests/check.rs` | `cpl --check` and the analysis behind it: every example passes, and on the examples, the tutorial's programs and a set of broken ones the check finds an error exactly when the system does, with the same message |
| `cargo test --target wasm32-wasip1` | the same, inside wasmtime |
| `scripts/wasi-check.sh` | native and WASI builds agree on every example, byte for byte |
| `node web/test.mjs` | the browser's WASI host runs every example, reports errors, survives deep recursion; the language server answers through the code the page's editor uses |

```bash
cargo test -p cpl-lsp                    # the language server
cd editors/vscode && npm install && npm run build && npm test
```

`.github/workflows/ci.yml` runs all of these, with `cargo fmt --check` and
`cargo clippy -- -D warnings`. `npm run test:vscode`, which drives the
extension inside a real Visual Studio Code, is run by hand. The page itself
(`index.html`, `worker.js`, `editor.js`) has no automated test.

## Choices, deviations and omissions

No CPL compiler survives to test against, so this is faithful to the
documents and to nothing else. Where they are silent, contradictory, or
were not followed, the choice is listed here.

**Which CPL.** The 1966 Reference Manual is the authority. Where the 1963
paper disagrees, 1966 wins: brackets round part of an expression list have
no significance (4.5), although in 1963 they made a sublist. On `a/bc` the
two agree: juxtaposition is replaced by `×` "before any further analysis"
(6.1.3), and the table of precedences of 6.1.4 then groups `a/b×c` as
`a/(b×c)`, as the 1963 paper does. The same table makes `a/b/c` mean
`a/(b/c)` and `a↑bc` mean `a↑(bc)`, and that is what is done. (Up to
version 0.1.1 this README said 1966 made `a/bc` mean `(a/b)c`, and the
system did so; that was a misreading, found by the [independent
review](docs/independent-review.md).) The three 1963 features above were
added because the manual leaves a hole where they were.

**Types.** CPL was to be compiled with every type known in advance. Here
every L-value has a type fixed when it is created and every assignment and
parameter passes through the transfer functions, but this is checked when
the program runs. Types are deduced at compile time only as far as needed to
insert the transfers of 3.3.3: the arms of a conditional, the results of a
`value of` block, and the result of a recursive function (found by
repetition, as 10.3.6 prescribes). A type error is therefore reported when
the offending command is obeyed, not before. Function types are not
distinguished by their result types.

**Where-clauses.** The manual says a where-clause qualifies "the longest
command, definition or expression immediately preceding it", and twice notes
that the rule "will be changed", without saying how. The `Product` quoted
above needs each `where` to qualify the definiens just before it, inside the
scope of that definition's parameters. That is the rule used: after a
definiens, `where` belongs to that definiens; after a command, to the
command. This is a departure from the manual's stated rule, not a reading
of it, and it is made to run a program whose source has not been checked.

**Implementation-dependent quantities.** index is 32 bits and integer 64;
real and double are both IEEE double precision, so double gains nothing;
logical is 24 bits and long logical 48, as on Atlas. Index and integer
overflow is an alarm. A real transfers to an integer only if it is integral
to within 1 part in 10⁹; otherwise use `Round` or `Intpt`.

**Small extensions.** `let rec f[..] = .. and g[..] = ..` makes the group
mutually recursive (the manual wants the group in section brackets).
Declarations may appear anywhere in a block. `Type[x]` gives the current type of a
data item and `Cpx[a, b]` makes a complex number; neither is in the papers.
List equality with `=` compares members.

**Boolean operators.** `/\` and `\/` evaluate both their operands, as 4.2
requires: an operator "may be applied to its operands at any time after each
of them has been evaluated". Up to version 0.1.1 they skipped the second
operand when the first settled the result, and this README said 4.2 permitted
that; it does not, and the short cut has been removed. A test that must not
evaluate its second part is written as a conditional expression,
`(i <= n -> A[i] > 0, false)`.

**Not implemented.**

- The rules for `break`, `return` and `result is` that replaced section
  9.5. The editor's note says that section "is now largely wrong as we
  have altered the rules ... to allow a more general use", and the new
  rules are not given; 9.5 is followed as written.
- Running the declarations of a block when jumping into it past them
  (9.3.2). The jump is made; a variable whose definition was skipped is
  reported when used.
- The numerical constant prefixes; the basic symbol *i*; `plus`, `minus`,
  `mult` of 1963. The sources say too little about them to build from.
- Input and output streams, `Layout`, and files (1963, sections 25 to 27),
  which were never specified. `Readchar[stream, x]` accepts a stream name
  and there is one stream. `Write` prints its arguments plainly rather
  than in the fixed-width formats of the pre-pilot system; `Printd` and
  `Printf` follow the supplement's description, whose exact layout of sign
  and exponent is a guess.
- The representation functions `Bitpattern`, `Formreal` and so on (3.3.5),
  which depend on the word of a machine that is not being imitated.
- A substitution parameter is only passed unevaluated when the function
  called is named directly; through a function variable it degrades to a
  call by reference.
- "Structure matching" beyond one level. The sources read here describe the
  matching of a list of names against a list (`matching.cpl`) and nothing
  more elaborate.
- Checking of types before the program runs, beyond what is described
  under **Types** above.

Rearrangement of expressions (1.1.5) is a freedom the manual gives the
compiler, not a duty; none is done, so every block behaves as if marked
`sic`.

**Limits.** A program may have 100,000 activations live at once; beyond
that it is taken to be in a runaway recursion. There is no tail-call
elimination. The load and update parts of LH functions, and substituted
expressions, may nest 10,000 deep (120 in WebAssembly).

## How it works

```
source ── lexer.rs ──▶ basic symbols ── parser.rs ──▶ tree ── compile.rs ──▶ code ── vm.rs
```

The Cambridge compiler of 1965 translated CPL into calls of GPM macros for a
stack machine (`LRX` load R-value, `LLX` load L-value, `RV`, `Fn`, ...),
in which, as Richards describes it, a function "was represented at run time
by a pointer to the first compiled instruction of its body combined with a
pointer to its free variable list": R-values in the list for a fixed
function, L-values for a free one. The machine here has the same shape.
`cpl --code` prints it:

```text
Function HCF (fixed)  [2 locals; free variables: HCF by reference]
     0  LoadLocal 1 (m)
     1  LoadLocal 0 (n)
     2  Rel([Gt])
     3  JumpIfFalse(10)
     4  LoadFree 0 (HCF)
     5  LvalLocal 1 (m)
     6  LvalLocal 0 (n)
     7  Call { n: 2, lmode: false }
     8  Transfer(Integer)
     ...
```

Actual parameters are evaluated in L-mode and the function called decides,
parameter by parameter, whether to take the L-value or its contents (5.3.2).
An L-value is a location, or a load-update pair made by an LH function, or
an unevaluated expression. `value of` blocks are activations of their own,
which is what makes a label's scope "the smallest enclosing routine body or
block-expression" and lets `result is` and a jump both end one cleanly. A
jump unwinds activations until it reaches the one its label belongs to.

Activations are kept in a list on the heap, not on the stack of the host,
so a CPL program can recurse 100,000 deep in a browser, whose own stack
would give out after about a thousand.

| File | |
| --- | --- |
| `src/lexer.rs` | the ASCII representation, tagged brackets, two-word symbols |
| `src/parser.rs` | syntax of sections 4 to 10 |
| `src/compile.rs` | scope rules, modes, free variable lists, type deduction |
| `src/code.rs` | the abstract machine code |
| `src/vm.rs` | activations, L-values, jumps |
| `src/value.rs` | data items, transfer functions, polymorphic operators |
| `src/builtins.rs` | basic functions and routines |
| `src/analysis.rs` | a program read without being run: tokens, definitions and their scopes, problems; behind `cpl --check` and the language server |
| `lsp/` | the language server, a crate of its own |
| `editors/` | the Visual Studio Code extension, and notes for other editors |

## Corrections to the sources

Every place where a program or quotation here differs from its source,
beyond the character set and the ASCII forms above, is listed here, with
its kind: a **slip** is wrong under any reading of the source; a
**conflict** is a place where the sources say two things and the printed
text follows the other one; an **interpretation** is our reading of what
was meant; **ours** is a consequence of choices made in this
reconstruction.

| Source | Printed | Here | Kind |
| --- | --- | --- | --- |
| Elementary Programming Manual 24 (`manual-hcf.cpl`) | `HCF m, n]` | `HCF[m, n]` | slip |
| Elementary Programming Manual 21 (b) (`manual-gaussquad.cpl`) | heading with no `be` | `be` added | slip |
| Elementary Programming Manual 20.1, 21 (a), 21 (b) | parameters as typed | the `ref` and `val` written in by hand | conflict |
| Elementary Programming Manual 22 (`manual-polynomial.cpl`) | `for i = Step[10, -1, 0]` | `for i = step 10, -1, 0`, as in the block printed just before | conflict |
| Elementary Programming Manual 22 | `f[x] = ...` | `let f[x] = ...` | ours |
| Main Features 29 (`factorial.cpl`) | `function Exp[x]`, `real s, t, r` | `Exp1`, so as not to hide the basic function; no type, so real by default | ours |
| Main Features 29 (`euler.cpl`) | `Fct`, `Eps`, `Tim`, `Mn`, `Ds`, `Sum`, `index n = 0` | other names, `integer n`, set to zero together | ours |
| Reference Manual 8.3.5 | `where` qualifies the longest preceding command, definition or expression | after a definiens, that definiens only | ours |
| Richards, p. 1 | "did not result is a satisfactory implementation" | quoted as "result [in]" | slip |
| Appendix 1 | eleven corrections | see [`appendix1/README.md`](appendix1/README.md#the-slips): nine slips and two conflicts | slip, conflict |
| Appendix 1, Part 3 | informal definitions | three readings beyond the text: one interpretation, two ours | interpretation, ours |

Up to version 0.1.1 `factorial.cpl` also replaced the 1963 stopping test
`t < 1₁₀−8` by `t << 1`, which changed the algorithm; the paper's test is
now used.

## Sources

[`sources/README.md`](sources/README.md) says where each was fetched from.
The documents themselves are not in this repository; `appendix1/scans/`
has the few details of pages that the list of slips refers to.

- *CPL Working Papers*, University of London Institute of Computer Science
  and the Mathematical Laboratory, Cambridge, July 1966. Contains the
  Reference Manual draft edited by Strachey.
- *CPL Elementary Programming Manual*, Edition II (Cambridge), January 1966,
  "including corrections and revisions"; the copy read has handwritten
  corrections and the supplement for the pre-pilot system.
- D. W. Barron, J. N. Buxton, D. F. Hartley, E. Nixon and C. Strachey, "The
  main features of CPL", *The Computer Journal* 6 (1963), 134-143.
- M. Richards, "How BCPL evolved from CPL", written for *The Computer
  Journal*, 2011.
- G. F. Coulouris, T. J. Goodey, R. W. Hill, R. W. Keeling and D. Levin,
  "The London CPL1 compiler", *The Computer Journal*, received 1967. Not
  used for the language definition, but it confirms two readings made here:
  that the list processing of 1963 is "not part of CPL as defined by the
  reference manual", and that `b -> 1, 3.24` is real.

## Licence

Apache License 2.0 - see [LICENSE](LICENSE). That covers the code, the
examples and the tutorial. The details of pages in `appendix1/scans` are
reproduced for reference and remain the copyright of their authors and
publishers.
