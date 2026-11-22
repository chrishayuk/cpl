# 19. Reference

## Writing CPL in ASCII

| Publication CPL | Here |
| --- | --- |
| underlined words: **let**, **value of** | small letters: `let`, `value of` |
| § and § struck through | `$(` and `$)`, or `{` and `}` |
| ≃ (by reference) | `~~` |
| ≡ (free function; if and only if) | `==` |
| ≢ | `~==` |
| → | `->` |
| × ↑ | `*`, `**` or `^` |
| ≠ ≤ ≥ | `~=` `<=` `>=` |
| ∧ ∨ ∼ | `/\` `\/` `~` |
| subscript ten | `@` |
| bold **8** and **2** before bits | `#o` and `#b` (and `#x`) |
| `\|\|` comment | `\|\|` |

The Unicode characters `→ ≡ ≢ ≃ ∧ ∨ ∼ ≠ ≤ ≥ × ↑` are accepted as well, and so is
publication CPL itself: underlined words (a letter followed by the
combining underline) and § with its struck-through twin. `cpl
--publication` prints a program that way.

Names: one small letter with optional primes (`x`, `a'`), or a capital
followed by letters and digits (`Total`). Small letters side by side are
multiplied.

## Definitions

| | |
| --- | --- |
| `let x = E` | a new variable, by value |
| `let x ~~ E` | a new name for an existing variable, by reference |
| `let x == E` | by substitution: the expression itself (1963) |
| `let x be T`, `let x, y both be T`, `let x, y, z all be T` | a type and no value |
| `let x, y = E1, E2` | several at once |
| `let constant x = E` | may not be assigned to |
| `let F[P] = E` | a fixed function: free variables by value |
| `let F[P] == E` | a free function: free variables shared |
| `let routine R[P] be C` | a routine (`rt` for short; `let` optional) |
| `let function F[P] be load B update B` | an LH function, with an optional `fix` |
| `let rec D` | the names defined may be used in their own definitions |
| `D1 and D2` | in parallel |
| `D1 in D2` | the names of D1 are known only in D2 |
| `C where D`, `(E where D)` | the names of D are known only in C or E |
| `prefer T` | the type assumed when none is given |

A parameter may be preceded by a type and by `val`, `ref` or `subst`; each
applies to the parameters after it.

## Commands

| | |
| --- | --- |
| `L := E` | assignment; `L1, L2 := E1, E2` is simultaneous |
| `R[E, ...]` | a routine call |
| `$( ... $)` | a compound command or block; brackets may have tags |
| `if B do C`, `unless B do C` | `then` may replace `do` |
| `test B then C1 or C2` | |
| `while B do C`, `until B do C` | |
| `C repeat`, `C repeat while B`, `C repeat until B` | |
| `for v = list do C` | the list: values, `a to b`, `step a, b, c`, `a, b, ..., c` |
| `for ext v = list do C` | uses an existing variable |
| `break` | leave the smallest enclosing loop |
| `return` | leave the routine |
| `result is E` | leave the `value of` block with a value |
| `finish` | end the program |
| `Name: C` | a label |
| `go to E` | jump to a label value |
| `switch on E into $( case K: ... default: ... $)` | |

## Expressions, from loosest to tightest

| | |
| --- | --- |
| `B -> E1, E2` | conditional |
| `==` `~==` | equivalent, not equivalent |
| `\/` | or; both operands are evaluated |
| `/\` | and; both operands are evaluated |
| `=` `~=` `<` `<=` `>` `>=` `<<` `>>` | relations; may be chained |
| `<=>` | joining strings |
| `+` `-` | |
| `*` `/` and writing side by side | `a/bc` is `a/(bc)`, `a/b/c` is `a/(b/c)` |
| `-` in front | |
| `**` | its right operand takes in `*` and `/`: `a**bc` is `a**(bc)` |
| `~` | not |
| `F[E, ...]` | application: functions, arrays |

`value of B` and `reference of B` make an expression from a block.

## Types

`index`, `integer`, `real`, `double`, `complex`, `double complex`,
`Boolean`, `logical`, `long logical`, `string`, `label`, `function`,
`routine`, `type`, `general`, and arrays: `T vector`, `T matrix`,
`T n array`.

## Basic functions and routines

| | |
| --- | --- |
| Arithmetic | `Sqrt Exp Log Sin Cos Tan Arctan Arcsin Arccos Mod Sign Intpt Frpt Round Quot Rem` |
| Complex | `Cpx Realpt Imagpt` |
| Transfer | `Index Integer Real Double Complex DoubleComplex Logical LongLogical String Boolean` |
| Values | `RValue LValue Type` |
| Logical | `Lshift Rshift Rotate Ones Longones Field Mask Bit` |
| Strings | `Length First Last Character Initials Finals` |
| Arrays | `Newarray Formarray Copy Fix Bounds Equal` |
| Lists | `NIL Cons List List1 Hd Tl Null Atom Members Append Length` |
| Output | `Write Writechar Printd Printf Space Newline` |
| Input | `Read Readchar` |

## The command line

```text
cpl [OPTIONS] FILE.cpl      run a program
cpl [OPTIONS] -e PROGRAM    run the program text given
cpl [OPTIONS] -             read the program from standard input
cpl --check FILE.cpl...     say what is wrong with each program, without running it

--tokens         print the basic symbols instead of running
--ast            print the syntax tree instead of running
--code           print the abstract machine code instead of running
--publication    print the program as it would be printed
--max-steps N    stop with an error after N machine steps
```

The exit status is 0 when the program finishes, 1 for an error in the
program, and 2 for a mistake on the command line.

## Messages you will meet

```cpl
let count = 0
```
```output
! line 1: '=', '~~', '==' or 'be' expected in a definition (found the name o)
```

A long name in small letters. Write `Count`.

```cpl
let n be integer
n := 1/3
```
```output
! line 2: alarm: the real number 0.333333333333 is not integral (use Round or Intpt to make an integer of it) (in main program)
```

A value that will not go into the type of the variable. Use `Round` or
`Intpt`, or make the variable real.

```cpl
let f[x] = x = 0 -> 1, x f[x - 1]
Write[f[3]]
```
```output
! line 1: the name f is not defined here
```

A recursive function without `rec`.

```cpl
let Total = 0
let Add[x] = value of $( Total := Total + x; result is Total $)
Write[Add[1]]
```
```output
! line 2: Total is a free variable of the fixed function Add, which may not assign to it (a function defined with == is free and may)
```

A fixed function assigning to a free variable. Define it with `==`.

```cpl
let rec Down[integer n] = Down[n - 1]
Write[Down[1]]
```
```output
! line 1: too many nested activations (runaway recursion?) (in Down)
```

A recursion with no way out.

```cpl
let a = Newarray[real, (1, 3)]
Write[a[2]]
```
```output
! line 2: a variable is used before it has been given a value (in main program)
```

`Newarray` does not give the elements values.

```cpl
for i = 1 to 3 do
   $( Write[i]
```
```output
! line 3: 1 section bracket(s) left open at the end of the program
```

A section bracket that is never closed.

Back to the [introduction](01-introduction.md), or on to the
[examples](../examples).
