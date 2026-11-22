# 1. Introduction

CPL is the Combined Programming Language. It was designed between 1962 and
1966 by a group at the University Mathematical Laboratory in Cambridge and
the University of London Institute of Computer Science: Christopher
Strachey, David Barron, John Buxton, David Hartley, Eric Nixon and others.
It was to be the language for two new machines, Titan at Cambridge and
Atlas in London, and it was meant to do everything: numerical work like
ALGOL 60, on which it was based, and also the non-numerical work that ALGOL
could not do.

It never quite arrived. The language was too big for the machines and the
compiler writers of its day; only subsets were ever implemented, and the
reference manual was still a draft when the project ended. But one of the
people who worked on the Cambridge compiler, Martin Richards, took what he
had learnt and designed a small language for writing compilers in: "Basic
CPL", or BCPL. Ken Thompson cut BCPL down to make B, and Dennis Ritchie
built B up again to make C. Much of what a C programmer takes for granted
reached C by way of CPL. Some of it CPL had from ALGOL 60: assignment with
`:=`, `if`, `for` and `while`. Some of it is not in ALGOL 60 and came to C
from CPL: Strachey's L-value and R-value, the distinction between the two
sides of an assignment; `break`, `return` and the block as an expression.
`switch` and `case` came by way of BCPL. The curly brackets descend from CPL's section
brackets.

This tutorial teaches the language as the 1966 manual describes it, as far
as an implementation written from that manual sixty years later goes. The
[README](../README.md#choices-deviations-and-omissions) lists what it
leaves out and where it departs from the manual. Every program in it is run by the test suite, and the output shown
is what the program really printed.

## Running a program

Build the system once:

```bash
cargo build --release
```

A CPL program is a text file. This is one, `examples/hello.cpl`:

```cpl
|| The first program.

Write['Hello, world!*n']
```
```output
Hello, world!
```

Run it by giving the file name:

```bash
target/release/cpl examples/hello.cpl
```

There are two other ways to hand over a program, which are useful for
trying things out: `-e` takes the program itself, and `-` reads it from
standard input.

```bash
target/release/cpl -e "Write[6 * 7]"
echo "Write[6 * 7]" | target/release/cpl -
```

Or use the playground in a browser: see the [README](../README.md).

## How to read this tutorial

A program is shown like this, followed by what it prints:

```cpl
Write['The answer is ', 6 * 7, '*n']
```
```output
The answer is 42
```

When a program goes wrong, the output ends with a line beginning `!` which
gives the message and the line of the program it came from:

```cpl
Write['one*n']
Write[1/0]
```
```output
one
! line 2: alarm: division by zero (in main program)
```

At the command line that message is written to standard error, and the exit
status is 1.

## The look of the language

CPL was written on a Flexowriter, a typewriter that punched paper tape. It
had 162 printing characters to play with, because it could underline and
overstrike: the words of the language were underlined (<u>let</u>,
<u>value of</u>), and the brackets that group commands were a section sign
§ and the same sign struck through.

Here the words of the language are simply written in small letters, and
the section brackets are `$(` and `$)`. The full list of what stands for
what is in [chapter 19](19-reference.md), but one rule must be learnt at
once, because it is the first thing that surprises everybody:

> **A small letter is a whole name.** `x` is a name, and so is `y`. `xy` is
> not a name: it is `x` times `y`. A longer name must begin with a capital
> letter: `Total`, `Count`, `Fib`.

Mathematicians write *ab* for the product of *a* and *b*, and CPL lets you
do the same. [Chapter 3](03-numbers.md) has more on this.

Next: [Hello, world](02-hello-world.md)
