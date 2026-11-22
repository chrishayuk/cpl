# 4. Names and definitions

## let

A variable is brought into existence by a definition, which begins with
`let`, and its value is changed by an assignment, which uses `:=`.

```cpl
let x = 5
Write[x, '*n']
x := x + 1
Write[x, '*n']
```
```output
5.0
6.0
```

Notice the two signs. `=` in a definition says what the variable is to
start as. `:=` is a command: work out the right hand side, and put the
result in the variable on the left. (`=` on its own, in an expression, is
a test for equality.) The distinction between the two sides of `:=` turns
out to be one of the deepest ideas in the language, and
[chapter 9](09-routines-and-references.md) comes back to it.

## Names

A **small name** is one small letter, which may be followed by primes:
`x`, `y`, `a'`, `a''`. A **large name** begins with a capital letter and
goes on with letters and digits: `Total`, `Fib`, `X2`, `ERROR`.

```cpl
let a, a' = 1, 2
let Total = a + a'
Write[Total, '*n']
```
```output
3.0
```

What happens if you forget, and write a long name in small letters?

```cpl
let total = 0
```
```output
! line 1: '=', '~~', '==' or 'be' expected in a definition (found the name o)
```

`total` is five names side by side, a product, so after `t` the system
expected the rest of a definition and found the name `o` instead.

The words of the language itself (`let`, `if`, `for`, `do`, `result` and
the rest) are written in small letters and are reserved. So `in` is the
word `in`, and if you want `i` times `n` you must put a space or a star
between them.

## Several at once

```cpl
let x, y, z = 1, 2, 3
Write[x + y + z, '*n']
x, y := y, x
Write[x, ' ', y, '*n']
```
```output
6.0
2.0 1.0
```

The last command is a **simultaneous assignment**: all the right hand
sides are worked out first, and then all the assignments are made. So
`x, y := y, x` exchanges two variables with no third one to help.

## Types

Every variable has a type, fixed when it is defined. `let x = 5` did not
say what type `x` was to be, so the system used the **preferred type**,
which is real. That is why this prints a decimal point:

```cpl
let x = 5
Write[x, ' is of type ', Type[x], '*n']
```
```output
5.0 is of type real
```

To have a variable of some other type, say so. A definition by `be` gives
the type and no value:

```cpl
let n be integer
let s be string
n := 5
s := 'five'
Write[n, ' ', s, '*n']
```
```output
5 five
```

```cpl
let i, j both be index
let x, y, z all be real
let n, s be integer, string
i, j, x, y, z, n, s := 1, 2, 3, 4, 5, 6, 'seven'
Write[i, j, ' ', x, y, z, ' ', n, s, '*n']
```
```output
12 3.04.05.0 6seven
```

A variable that has been defined but not yet given a value cannot be used:

```cpl
let n be integer
Write[n + 1]
```
```output
! line 2: the variable n is used before it has been given a value (in main program)
```

If the right hand side of a definition is plainly not a number, the
variable takes its type from it:

```cpl
let s = 'a string'
let b = 2 > 1
let c = Cpx[1, 2]
Write[Type[s], ', ', Type[b], ', ', Type[c], '*n']
```
```output
string, Boolean, complex
```

The types are index, integer, real, double, complex, double complex,
Boolean, logical, long logical, string, label, function, routine, array,
type and general. They each get their turn in later chapters.

## Changing the preferred type

A program that only counts is tidier in whole numbers. `prefer` changes
the type that is assumed when nothing is said:

```cpl
prefer integer
let n = 5
Write[n, ' is of type ', Type[n], '*n']
```
```output
5 is of type integer
```

## A variable keeps its type

When a value is assigned to a variable it is first converted to the
variable's type, if that can be done without loss:

```cpl
let n be integer
let x be real
n := 6.0
x := 6
Write[n, ' ', x, '*n']
```
```output
6 6.0
```

If it cannot, the program stops. The manual calls this an alarm.

```cpl
let n be integer
n := 6.5
```
```output
! line 2: alarm: the real number 6.5 is not integral (use Round or Intpt to make an integer of it) (in main program)
```

```cpl
let n be integer
n := 'six'
```
```output
! line 2: only a string of length one (a character) transfers to a number (in main program)
```

## Constants

A definition can say that a name is a constant, which may not be assigned
to:

```cpl
let constant Pi = 3.14159265359
Write[2Pi, '*n']
Pi := 3
```
```output
6.28318530718
! line 3: assignment to a constant (in main program)
```

Next: [Conditions](05-conditions.md)
