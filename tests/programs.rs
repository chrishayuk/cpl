//! Every program in examples/ is run, with examples/<name>.in as its input
//! if there is one, and its output compared with examples/<name>.out. A program that ends in an error has the
//! message as a last line beginning "! ". Run with CPL_BLESS=1 to rewrite
//! the expected files.

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

fn transcript(src: &str, input: &str) -> String {
    let (mut out, result) = cpl::run_source(src, input, Some(50_000_000));
    if let Err(e) = result {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("! {}\n", e));
    }
    out
}

#[test]
fn sample_programs() {
    let dir = root().join("examples");
    let bless = std::env::var("CPL_BLESS").is_ok();
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "cpl"))
        .collect();
    names.sort();
    assert!(names.len() >= 60, "example programs are missing");
    let mut failed = Vec::new();
    for path in names {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(&path).unwrap();
        let input = std::fs::read_to_string(dir.join(format!("{}.in", name))).unwrap_or_default();
        let got = transcript(&src, &input);
        let want_path = dir.join(format!("{}.out", name));
        if bless {
            std::fs::write(&want_path, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&want_path).unwrap_or_default();
        if got != want {
            eprintln!("---- {} ----\nexpected:\n{}\ngot:\n{}", name, want, got);
            failed.push(name);
        }
    }
    assert!(failed.is_empty(), "programs with unexpected output: {:?}", failed);
}

fn run(src: &str) -> String {
    transcript(src, "")
}

#[test]
fn juxtaposition_is_multiplication() {
    assert_eq!(
        run("let a, b = 3, 4\nWrite[a b, ' ', 2a, ' ', a(b + 1), ' ', a/b b]"),
        "12.0 6.0 15.0 3.0"
    );
}

#[test]
fn keywords_are_reserved_words_and_other_runs_are_products() {
    // `ab` is a times b; `in` is the basic symbol, so i times n needs a space.
    assert_eq!(run("let a, b, i, n = 2, 3, 4, 5\nWrite[ab, ' ', i n]"), "6.0 20.0");
}

#[test]
fn definitions_by_value_and_by_reference() {
    assert_eq!(
        run("let a = 1\nlet b ~~ a\nlet c = a\nb := 2\nWrite[a, b, c]"),
        "2.02.01.0"
    );
}

#[test]
fn fixed_functions_take_values_and_free_functions_take_locations() {
    let src = "let a = 1\nlet f[] = a\nlet g[] == a\na := 2\nWrite[f[], ' ', g[]]";
    assert_eq!(run(src), "1.0 2.0");
}

#[test]
fn a_fixed_function_may_not_assign_to_a_free_variable() {
    let out = run("let a = 1\nlet f[] = value of $( a := 2; result is a $)\nWrite[f[]]");
    assert!(out.contains("free variable of the fixed function f"), "{}", out);
}

#[test]
fn recursion_needs_rec() {
    let out = run("let f[integer n] = n = 0 -> 1, n f[n - 1]\nWrite[f[3]]");
    assert!(out.contains("the name f is not defined here"), "{}", out);
    assert_eq!(run("let rec f[integer n] = n = 0 -> 1, n f[n - 1]\nWrite[f[5]]"), "120");
}

#[test]
fn result_type_of_a_recursive_function_is_found_by_iteration() {
    // The arm 1 is an index, the other arm real; both become real.
    assert_eq!(
        run("let rec f[x] = x = 0 -> 1, x f[x - 1]\nWrite[f[0], ' ', Type[f[0]]]"),
        "1.0 real"
    );
}

#[test]
fn simultaneous_assignment() {
    assert_eq!(
        run("let a, b, c = 1, 2, 3\na, b, c := c, a, b\nWrite[a, b, c]"),
        "3.01.02.0"
    );
}

#[test]
fn assignment_transforms_to_the_type_of_the_left_hand_side() {
    assert_eq!(run("let n be integer\nn := 4.0\nWrite[n, ' ', Type[n]]"), "4 integer");
    assert!(run("let n be integer\nn := 4.5").contains("not integral"));
    assert!(run("let n be integer\nn := true").contains("no transfer function from Boolean to integer"));
}

#[test]
fn general_variables_take_any_type() {
    assert_eq!(run("let g be general\ng := 1; g := 'x'; g := Sqrt\nWrite[g[4]]"), "2.0");
}

#[test]
fn lh_functions_make_new_kinds_of_l_value() {
    let src = "let a = 0\n\
               let function Twice[] be load $( result is 2a $) update $( a := rhs/2 $)\n\
               Twice[] := 10\nWrite[a, ' ', Twice[]]";
    assert_eq!(run(src), "5.0 10.0");
}

#[test]
fn call_by_substitution() {
    let src = "let i = 0\n\
               let Sum[ref k, val n, subst t] = value of\n\
               $( let s = 0\n   for j = 1 to n do $( k := j; s := s + t $)\n   result is s $)\n\
               Write[Sum[i, 4, i i]]";
    assert_eq!(run(src), "30.0");
}

#[test]
fn jump_out_of_a_function_activation() {
    let src = "let rt R[label l] be goto l\nR[Out]\nWrite['skipped']\nOut: Write['landed']";
    assert_eq!(run(src), "landed");
}

#[test]
fn compound_relations() {
    assert_eq!(run("Write[1 < 2 < 3, 3 > 2 > 2, 1 = 1 ~= 2]"), "truefalsetrue");
}

#[test]
fn tagged_section_brackets_close_inner_sections() {
    assert_eq!(run("$(A $( $( Write['x'] $)A\nWrite['y']"), "xy");
    assert!(run("$(A Write['x'] $)B").contains("no open section bracket is tagged B"));
}

#[test]
fn end_of_line_ends_a_command_only_where_it_could() {
    // A line beginning with ( starts a new command: b is a, not a(a).
    assert_eq!(run("let a = 5\nlet b = a\n(a) := 7\nWrite[a, b]"), "7.05.0");
    assert_eq!(run("let a = 1 +\n 2\nWrite[a\n *2]"), "6");
}

#[test]
fn read_takes_numbers_from_the_input_stream() {
    let (out, r) = cpl::run_source(
        "let a, n = 0, 0\nlet k be index\nRead[a, n, k]\nWrite[a + n, ' ', k]",
        "1.5 2\n7\n",
        None,
    );
    assert!(r.is_ok());
    assert_eq!(out, "3.5 7");
}

#[test]
fn step_limit() {
    assert!(run("$( $) repeat").contains("step limit"));
}

#[test]
fn recursion_is_not_carried_on_the_host_stack() {
    let src = "let rec Sum[integer n] = (n = 0 -> 0, n + Sum[n - 1])\nWrite[Sum[50000]]";
    assert_eq!(run(src), "1250025000");
}

#[test]
fn runaway_recursion_is_reported() {
    let out = run("let rec Down[integer n] = Down[n - 1]\nWrite[Down[1]]");
    assert!(out.contains("runaway recursion"), "{}", out);
}

#[test]
fn a_long_list_is_built_compared_and_freed() {
    let src = "let rec Upto[integer n, general l] = (n = 0 -> l, Upto[n - 1, Cons[n, l]])\n\
               let l = Upto[60000, NIL]\nWrite[Length[l], ' ', l = Upto[60000, NIL]]";
    assert_eq!(run(src), "60000 true");
}

#[test]
fn routine_headings_of_the_elementary_manual() {
    // No let; ref and val written among the parameters; a label parameter.
    let src = "let V = 0\n\
               routine Halve[real a, ref x, val label L] be\n\
               $( if a = 0 go to L\n   x := a/2 $)\n\
               Halve[5, V, Bad]\nWrite[V]\nHalve[0, V, Bad]\nWrite['no']\nfinish\nBad: Write[' bad']";
    assert_eq!(run(src), "2.5 bad");
}

#[test]
fn for_ext_uses_an_existing_variable() {
    assert_eq!(
        run("let k, s = 0, 0\nfor ext k = step 1, 2, 11 do s := s + k\nWrite[k, ' ', s]"),
        "11.0 36.0"
    );
}

#[test]
fn the_number_of_cycles_of_a_step_is_rounded() {
    // (E3 - E1)/E2 "rounded to the nearest integer" (Elementary Manual 13.2).
    assert_eq!(run("for i = step 0, 2, 7 do Write[i, ' ']"), "0 2 4 6 8 ");
    assert_eq!(run("for i = step 0, 2, 6.9 do Write[i, ' ']"), "0 2 4 6 ");
    assert_eq!(run("for i = step 5, 1, 1 do Write[i]\nWrite['none']"), "none");
}

#[test]
fn logical_functions_of_section_25() {
    let src = "let p = #o2736\n\
               Write[Lshift[p, 3], Rshift[p, 3], Lshift[p, -3], Rotate[p, 20], Ones[3, 0], Ones[0, 3],\n\
                     Field[p, 6, 10], Field[p, 10, 6], Mask[p, 6, 10], Bit[p, 1], Longones[0, 30]]";
    assert_eq!(
        run(src),
        "#o27360#o273#o273#o70000135#o17#o17#o27#o27#o2700#o1#o17777777777"
    );
    assert!(run("Write[Bit[#o1, 24]]").contains("there is no bit 24 in 24 bits"));
}

#[test]
fn output_routines_of_the_pre_pilot_system() {
    assert_eq!(
        run("Printd[42, 5, 0]; Printd[-3.14159, 3, 2]; Printd[0.5, 2, 3]; Write['|']"),
        "    42  -3.14  0.500|"
    );
    // Too big for its field: floating decimal style.
    assert_eq!(run("Printd[123456, 3, 0]; Write['|']"), "  1.23@+05|");
    assert_eq!(
        run("Printf[3.14159, 4]; Printf[-0.00123, 3]; Write['|']"),
        "  3.142@+00 -1.23@-03|"
    );
    assert_eq!(run("WRITE['a']; WRITECHAR['b']; Writechar[67]"), "abC");
}

#[test]
fn function_is_a_type_that_can_be_compared() {
    assert_eq!(
        run("let f[x] = x\nWrite[Type[f] = function, Type[Sqrt] = function, Type[1] = function]"),
        "truetruefalse"
    );
}

#[test]
fn input_is_read_by_read_and_readchar() {
    let out = transcript(
        "let a, b = 0, 0\nlet c = ''\nRead[a, b]\nReadchar[c]\nWrite[a + b, c]\nReadchar[c]\nWrite[c = '']",
        "3, 4 x",
    );
    assert_eq!(out, "7.0xtrue");
}

#[test]
fn bit_and_field_may_stand_on_the_left() {
    let src = "let p = #o2736\nBit[p, 0] := #o1\nField[p, 6, 10] := #o0\n\
               let rt Set[ref b] be b := #o1\nSet[Bit[p, 23]]\nWrite[p, Field[p, 0, 2], Bit[#o4, 2]]";
    assert_eq!(run(src), "#o40000037#o7#o1");
}

#[test]
fn a_bar_puts_a_logical_constant_at_the_top() {
    assert_eq!(run("Write[#o|273, #b|1, #o273]"), "#o27300000#o40000000#o273");
}

#[test]
fn an_overstruck_character_is_read_as_one() {
    // l underlined, then a section sign struck through, then x.
    let out = transcript(
        "let c = ''\nfor i = 1 to 4 do $( Readchar['INPUT', c]; Write[Length[c]] $)",
        "l\u{332}\u{a7}\u{338}x",
    );
    assert_eq!(out, "2210");
}

#[test]
fn publication_cpl_is_accepted() {
    let src = "l\u{332}e\u{332}t\u{332} x = 2\n\u{a7}1 I\u{332}f\u{332} x \u{2260} 3 d\u{332}o\u{332} Write[x \u{d7} x] \u{a7}\u{338}1";
    assert_eq!(run(src), "4.0");
    assert_eq!(
        cpl::lexer::publication("let x = 2\n$(1 if x ~= 3 do Write[x * x] $)1"),
        src.replace("I\u{332}", "i\u{332}")
    );
}
