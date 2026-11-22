# Independent review

## Who reviewed, and what

On 22 November 2026 the repository, as of version 0.1.1 (commit
`e12f6cc`), was reviewed by OpenAI Codex CLI (`codex-cli 0.166.0`): an
independent second model, not a person. It was asked to check the
implementation, the tutorial and the README against the scanned documents
in `sources/`, and to run everything the README describes.

It read the code, ran the programs, and compared 28 passages with the
rendered pages of the sources: the eleven numbered corrections to
Appendix 1, the three corrections written by hand on the appendix, eleven
example programs or functions, the matrix-product passage of the
tutorial, and two quotations. It could not reach the npm registry, so the
editor extension's build and tests were not run, and it could not read the
notebook note on chrishayuk.com.

It made ten numbered findings. Each was then checked again, against the
code, by running programs, and against the scans, before anything was
changed. That second check looked at 25 places in the sources: the table
of precedences (Working Papers 6.1.4), the rules of evaluation (4.2) and
the logical operators (6.3); the precedence rules of the 1963 paper; pages
2 and 6 of Appendix 1, for slip 1, the hand corrections, Part 3 (a), (c)
and (f) and the two spellings of `CannotProceedCommand`; 3.1.2 on `:=`;
`Exp`, `Fact1`, `Fact2` and `Euler` in the 1963 paper; Gaussquad, the
scalar product, the polynomial and HCF in the Elementary Programming
Manual; the example of 9.3.2; and four passages of Richards. The ALGOL 60
report and the PAL manual were not opened; the changes that depend on them
rely on Richards and on the general content of the ALGOL report. The
other passages the review found to match (LINEQ, the demonstration job,
the matrix product, the 9.5 warning) were not checked again.

## The findings, and what was done

**1. Division was grouped wrongly, and a test enforced it.** Accepted.
The table of 6.1.4 gives `×` left and right precedences 9 and 9, `/` 9
and 8, and `↑` 10 and 8. Worked through the grouping procedure of 6.1.4,
`a/bc` is `a/(bc)` and `a/b/c` is `a/(b/c)`; the 1963 paper says the same
of `a/bc`, so there was no conflict between 1963 and 1966. The same table
makes `a↑bc` mean `a↑(bc)`, which the review did not mention; that is
done too. Fixed in `e93a4cb`; the test `juxtaposition_is_multiplication`
now expects `a/b b` to be 0.1875, and
`grouping_follows_the_table_of_precedences` checks nine cases. The
README, tutorial chapters 3 and 19, the hover help and the dataset were
corrected, and `examples/types.cpl` shows `a/b/c`.

**2. `/\` and `\/` skipped their second operand.** Accepted. 4.2 lets an
operator be applied "at any time after each of them has been evaluated";
it does not permit leaving one out. The review suggested describing the
short cut as a deviation; it was removed instead, to match the manual.
Fixed in `d495a19`, with the test `and_and_or_evaluate_both_operands`.
`examples/calculator.cpl` relied on the short cut and now guards its two
tests with conditional expressions.

**3. The preprocessor lost the `;` after `true` or `false`.** Accepted.
Reproduced exactly. Part 3 (a) ends its list with "index integer real
etc."; `true` and `false` are now read as part of the "etc.", which is
labelled an interpretation. Fixed in `855e9b4`, with the test
`a_line_ending_in_a_boolean_constant_ends_a_command`. The preprocessor's
output for its own text (`appendix1/itself.out`) gained three `;` that it
had been losing.

**4. `Exp1` changed the 1963 algorithm.** Accepted. The paper's test is
`t < 1₁₀−8`; the example used `t << 1`. The paper's test is now used
(`4af37c5`), and the example prints `Exp1[1] = 2.7182818262`, as the
review predicted. The renaming of `Exp` to `Exp1` and the dropped `real`
declaration are disclosed in the file and in the README's table of
corrections.

**5. "Unchanged" and "as printed" concealed editorial changes.**
Accepted. Each case was confirmed on the page: `HCF m, n]` in the
Elementary Programming Manual; Gaussquad's heading without `be`; `let`
and `Step[10, -1, 0]` in the polynomial; the renamed variables of
`Euler`; the tidied quotation in `labels.cpl`; and Richards' "result is".
Every example header now says what was changed (`4af37c5`, `c1143df`),
the README's HCF sentence is withdrawn, the Richards quotation is marked
"[in]", and the README has a table of every correction with its kind.

**6. Two of the eleven Appendix 1 corrections are conflicts, and their
implementation was not fully described.** Accepted. Slips 1 to 9 are
labelled slips (3 on its reading of a faint mark); 10 and 11 are labelled
conflicts, with the alternative corrections named. The empty string added
to (c) and the change of `go` to `goto` in `Canonical` are described as
ours, and the twelfth entry in the test's list is explained. The scan
does spell the function differently in Part 3 (it reads
`CannotPreceedCommand`); the appendix README now says so (`855e9b4`).

**7. Other corrections and choices.** Accepted in part. The `where` rule
is now described as a departure from the manual, made for a program whose
source has not been checked, and the missing `jump` and `the` in
`Combinable` are labelled a conflict left as printed. On `: =`, the
review called the hand correction a normalisation, since 3.1.2 lets `:=`
take two print positions. That is possible, but the program's own
`case ':'` reads `:` followed by a space as `:`, so on the program's own
terms the space is a slip; the appendix README now gives both readings.

**8. Historical claims contradicted the cited sources.** Accepted. Richards
says `SWITCHON` went into BCPL and was "retro-fitted into CPL in 1966",
and that BCPL allowed simultaneous declarations and `and`; ALGOL 60 had
assignment, `if`, `for` and `while` before CPL; and PAL and ML had
functions that return functions long before the thirty years the
tutorial gave. The README, tutorial chapters 1 and 18 and the dataset were
corrected (`4af37c5`).

**9. Claims of completeness and first execution.** Accepted. "Never been
run before" is withdrawn: what the documents support is that no system of
the time accepted the whole language, and later implementations were not
looked for. The tutorial no longer claims to teach "the full language".
`product.cpl` is listed apart from the programs taken from the documents,
and the README now says nine examples, not ten, come from them.

**10. The documented checks pass, except the editor's, which could not
install.** Confirmed, with the network available: `npm install`,
`npm run build`, `npm test` (17 passed) and `npm run test:vscode` all
pass. Nothing needed fixing.

## The checks after the changes

| Check | Result |
| --- | --- |
| `cargo build --release` | passes |
| `cargo test --release` | 73 passed (70 before; three new tests) |
| `cargo test --release --workspace` | 88 passed |
| `cargo test --release --target wasm32-wasip1` | 67 passed |
| `scripts/wasi-check.sh` | 73 programs compared, 0 differ |
| `python3 tutorial/build_dataset.py --check` | up to date (251 examples) |
| `node web/build.mjs`; `node web/test.mjs` | 73 examples checked, 0 checks failed |
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `editors/vscode`: `npm test`; `npm run test:vscode` | 17 passed; passes |
