# 18. What became of it

## The end of the project

By 1966 there were two compilers. London had one for a subset called CPL1
which, in the words of the people who wrote it, had "no types general,
complex or double length", no lists, and restrictions on functions.
Cambridge had another, built in an unusual way: the compiler was written
in CPL itself and translated by hand into the macros of Strachey's General
Purpose Macrogenerator. Martin Richards, who worked on it, wrote later
that it "did not result in a satisfactory implementation". The reference
manual, edited by Strachey, was circulated as a draft in the *CPL Working
Papers* of July 1966, with a list of "changes foreseen" at the back. There
was no later edition.

So the programs in this tutorial have, for the most part, never been run
before. Nobody had a CPL system that would accept the whole language.

The same is true of the one large program in the Working Papers, a
preprocessor of about 190 lines given as "a general example of a CPL
program". Someone wrote across the top of it, "This draft has not been
fully checked". It runs now, after eleven corrections:
[appendix1](../appendix1/README.md) has the story.

## BCPL

Richards went to MIT in 1966 and took with him the idea of the Cambridge
compiler without its difficulties. He called the result BCPL, "Basic
CPL", and described it as CPL with all the difficult bits removed. The
difficult bits were these.

| CPL | BCPL |
| --- | --- |
| Sixteen types, with transfer functions put in automatically ([13](13-types.md)) | One type: the machine word |
| The type `general`, checked as the program runs | Nothing is checked |
| Definition and parameters by reference ([9](09-routines-and-references.md)) | Parameters by value only; the address of a variable can be taken with `@` and followed with `!` |
| Fixed and free functions with free variables ([10](10-fixed-and-free.md)) | A function may not use the local variables of another at all |
| Functions as results | Only the address of the code |
| LH functions ([16](16-lh-functions.md)) | Gone |
| `where`, `in`, simultaneous definitions ([7](07-blocks-and-scope.md)) | Gone |
| Jumps out of functions through label values ([15](15-labels-and-jumps.md)) | `level` and `longjump` |
| `rec` needed for recursion | Every function may be recursive |
| One-letter names and multiplication without a sign | Long names in small letters |
| Lists (1963) | Vectors of words |

What stayed is just as striking: section brackets, `let`, `:=`, `valof`
and `resultis`, `if`, `unless`, `test`, `while`, `until`, `repeat`, `for`,
`break`, `return`, `switchon` with `case` and `default`, conditional
expressions, and the L-value and the R-value, which became the address of
a word and its contents.

## The same program, three times

The highest common factor, in the CPL of 1966:

```cpl
let rec HCF[integer n, m] =
        (m > n -> HCF[m, n],
         m = 0 -> n,
         HCF[m, Rem[n, m]])

Write[HCF[1071, 462], '*n']
```
```output
21
```

In BCPL, a few years later:

```
LET HCF(N, M) = M > N -> HCF(M, N),
                M = 0 -> N,
                HCF(M, N REM M)
```

And in C:

```c
int hcf(int n, int m) {
    return m > n ? hcf(m, n)
         : m == 0 ? n
         : hcf(m, n % m);
}
```

The shape has not changed in sixty years. The square brackets became
round, `->` and the comma became `?` and `:`, and the types went away and
came back.

Now a program that did not survive the journey. In CPL a function can
return a function that remembers:

```cpl
prefer integer

let Counter[] = value of
   $( let n = 0
      let Next[] == value of $( n := n + 1
                                result is n $)
      result is Next $)

let c = Counter[]
c[]; c[]
Write[c[], '*n']
```
```output
3
```

This cannot be written in BCPL, in B, or in C. It needs a function to
carry its free variables with it, which was one of the difficult bits.
Programmers who were not using LISP waited about thirty years to get it
back.

## What C still owes

| In C | Comes from |
| --- | --- |
| `{` and `}` | the section brackets § and its struck-through twin |
| `lvalue` and `rvalue`, in every C compiler's error messages | Strachey's L-value and R-value |
| `a ? b : c` | `a -> b, c` |
| `switch`, `case`, `default`, and falling through | the switch-command of 1966 |
| `break`, `return`, `while`, `for`, `if` | the same words in CPL |
| `&x` and `*p` | BCPL's `@` and `!`, which replaced definition by reference |
| an array name meaning the address of its elements | the CPL array as a pointer ([11](11-arrays.md)) |
| `x = y = 0`, `+=`, and assignment as an expression | not CPL: there, assignment was strictly a command |
| `//` comments | BCPL's `//`, standing in for CPL's `\|\|` |

## What was lost, and found again

Read down the first column of the table in the BCPL section again and
count how many of the entries have since come back into ordinary
programming: values that carry their types, references, functions that
close over their variables, properties with getters and setters,
definitions in parallel and pattern-like matching of names against
structures, exceptions that unwind many calls at once. The designers of
CPL had all of it on paper by 1966. They were trying to build it in a
system that gave a running program 3500 words of working space and
compiled, by its own account, "about 1 line per second".

Next: [Reference](19-reference.md)
