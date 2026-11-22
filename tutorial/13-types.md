# 13. Types and the type general

## The numerical types

There are six types of number: **index**, **integer**, **real**,
**double**, **complex** and **double complex**. Each can do everything the
one before it can. The arithmetic operators are the same for all of them,
and when the two operands are of different types the result is of the
"larger" one:

```cpl
let i, n, x be index, integer, real
i, n, x := 2, 3, 4
Write[Type[i + i], ', ', Type[i + n], ', ', Type[n + x], ', ', Type[i x], '*n']
```
```output
index, integer, real, real
```

Two operators always give at least a real result, because their answers
are not usually whole numbers: `/` and `**`.

```cpl
Write[Type[6/3], ' ', Type[2**3], ' ', 6/3, ' ', 2**3, '*n']
```
```output
real real 2.0 8.0
```

The same rule settles the type of a conditional expression. Its two arms
might be of different types, and the result has to be of one type
whichever arm is taken, so both are brought up to the larger:

```cpl
let b = true
Write[(b -> 1, 2.5), ' ', Type[(b -> 1, 2.5)], '*n']
```
```output
1.0 real
```

## Complex numbers

`Cpx[a, b]` makes the complex number *a + ib*. It is printed as a pair.

```cpl
let z = Cpx[3, 4]
let i = Cpx[0, 1]
Write[z, ' ', z + 1, ' ', 2z, ' ', z z, ' ', i i, '*n']
Write[Mod[z], ' ', Realpt[z], ' ', Imagpt[z], ' ', z/z, '*n']
```
```output
(3.0, 4.0) (4.0, 4.0) (6.0, 8.0) (-7.0, 24.0) (-1.0, 0.0)
5.0 3.0 4.0 (1.0, 0.0)
```

Nothing in a program needs to change for it to work on complex numbers
instead of real ones. This function was written with real numbers in mind:

```cpl
let Poly[general x] = x x - 2x + 2

Write[Poly[3], ' ', Poly[0.5], ' ', Poly[Cpx[1, 1]], '*n']
```
```output
5 1.25 (0.0, 0.0)
```

and 1 + *i* turns out to be a root. (`general` is explained below.)

## Transfer functions

A value is changed from one type to another by a **transfer function**.
The system puts them in for itself wherever a particular type is needed:
on assignment, for a parameter called by value, and for the result of a
function. They can also be written out, and are named after the type
they produce:

```cpl
Write[Real[3], ' ', Integer[3.0], ' ', Complex[3], ' ', Index['A'], ' ', String[65], '*n']
Write[Logical[5], ' ', Index[#b101], ' ', Boolean[1], ' ', Index[true], '*n']
```
```output
3.0 3 (3.0, 0.0) 65 A
#o5 5 true 1
```

A transfer only happens automatically when nothing is lost. To go from a
real number that is not whole to an integer you must say how:

```cpl
let n be integer
n := Round[2.7]
Write[n, ' ']
n := Intpt[2.7]
Write[n, '*n']
n := 2.7
```
```output
3 2
! line 6: alarm: the real number 2.7 is not integral (use Round or Intpt to make an integer of it) (in main program)
```

## The type general

Sometimes the type of a thing cannot be known when the program is
written. A variable of type **general** can hold a value of any type, and
the value carries its type with it:

```cpl
let g be general
g := 3;        Write[g, ' is of type ', Type[g], '*n']
g := 2.5;      Write[g, ' is of type ', Type[g], '*n']
g := 'three';  Write[g, ' is of type ', Type[g], '*n']
g := true;     Write[g, ' is of type ', Type[g], '*n']
g := Sqrt;     Write[g, ' is of type ', Type[g], '*n']
```
```output
3 is of type index
2.5 is of type real
three is of type string
true is of type Boolean
<basic function Sqrt> is of type function
```

The reference manual calls it "a data item whose type may vary
dynamically", and admits that "the introduction of the type general
carries with it the need for type checking at run time".

An operator given general operands decides what to do when it sees what
they are:

```cpl
let Add[general a, b] = a + b
Write[Add[1, 2], ' ', Add[1, 2.5], ' ', Add[Cpx[1, 1], 1], '*n']
Write[Add['one', 2]]
```
```output
3 3.5 (2.0, 1.0)
! line 1: the operator + is not defined for string and index (in Add)
```

A type is itself a data item, so the type of a value can be looked at and
acted on:

```cpl
let Describe[general x] = Type[x] = string -> 'the string ' <=> x,
                          Type[x] = Boolean -> (x -> 'yes', 'no'),
                          Type[x] = index -> 'a small whole number',
                          Type[x] = real -> 'a real number',
                          'something else'

for x = 'abc', 2 > 1, 42, 4.2, Cpx[4, 2] do Write[Describe[x], '*n']
```
```output
the string abc
yes
a small whole number
a real number
something else
```

The conditional expression in `Describe` has arms of unlike types that
have no larger type in common, and so is general itself. A `for` list of
unlike values makes a general loop variable in the same way.

## prefer general

With `prefer general` at its head, a program need hardly mention types at
all: every variable and parameter can hold anything. This is the style of
LISP, and the style of the next chapter.

```cpl
prefer general

let Twice[f, x] = f[f[x]]
let Exclaim[s] = s <=> '!'
Write[Twice[Sqrt, 16], ' ', Twice[Exclaim, 'CPL'], '*n']
```
```output
2.0 CPL!!
```

## What BCPL did

It took the types out. All of them. In BCPL, and in B after it, there is
one kind of value, the machine word, and `+` always means the addition of
words. Richards had found that most of the difficulty of compiling CPL
lay in its types, and a language for writing compilers did not need them.
C put types back, more cautiously, with a different idea of what they
were for.

Next: [Lists](14-lists.md)
