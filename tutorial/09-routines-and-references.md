# 9. Routines and references

## Routines

A function is an expression with a name: calling it gives a value. A
**routine** is a command with a name: calling it does something.

```cpl
let routine Rule[index n] be
   $( for i = 1 to n do Write['-']
      Write['*n'] $)

Rule[20]
Write['Combined Programming*n']
Rule[20]
```
```output
--------------------
Combined Programming
--------------------
```

`routine` may be shortened to `rt`, and the `let` may be left out, as the
Elementary Programming Manual does. A routine is left when its command
ends, or sooner by the command `return`:

```cpl
routine Countdown[index n] be
   $( if n > 5 do
         $( Write['too long to wait*n']
            return $)
      for i = step n, -1, 1 do Write[i, ' ']
      Write['go*n'] $)

Countdown[3]
Countdown[10]
```
```output
3 2 1 go
too long to wait
```

Routines may be recursive, with `rec`, like functions:

```cpl
let rec rt Hanoi[index n, string a, b, c] be
   $( if n = 0 return
      Hanoi[n - 1, a, c, b]
      Write[a, ' to ', b, '*n']
      Hanoi[n - 1, c, b, a] $)

Hanoi[2, 'left', 'right', 'middle']
```
```output
left to middle
left to right
middle to right
```

## Called by value

Here is a routine that does not do what its author hoped:

```cpl
let rt Double[x] be x := 2x

let a = 5
Double[a]
Write[a, '*n']
```
```output
5.0
```

Nothing happened to `a`. A parameter is normally **called by value**: the
routine gets a new variable of its own, `x`, which starts with the value
of the actual parameter. Assigning to `x` changes that private variable
and nothing else.

## Called by reference

```cpl
let rt Double[ref x] be x := 2x

let a = 5
Double[a]
Write[a, '*n']
```
```output
10.0
```

With `ref` (in full, `reference`) the routine is given not the value of
`a` but `a` itself. Inside the routine `x` is another name for the same
variable, and assigning to `x` assigns to `a`.

The manual's own example is a routine that solves a pair of simultaneous
equations. Six parameters are numbers going in, called by value; two are
variables for the answers to come out in, called by reference:

```cpl
routine LINEQ[real a, b, c, a', b', c', ref x, y] be
$( let DET = ab' - a'b
   x := (cb' - c'b)/DET
   y := (ac' - a'c)/DET $)

let V, W = 0, 0
LINEQ[1, 2, 3, 4, 5, 6, V, W]
Write[V, ' ', W, '*n']
```
```output
-1.0 2.0
```

`ref` and `val` apply, like types, to the names that follow them.

## L-values and R-values

Why does `a` mean one thing in `Double[a]` when the parameter is by value
and another when it is by reference? Strachey's answer, worked out for
CPL, has been part of the vocabulary of programming languages ever since.

Look at `x := x + 1`. The `x` on the right stands for a number, the
contents of the variable. The `x` on the left does not stand for a number
at all: it stands for the place where the number is kept. So an expression
has two values. Its **R-value** is what it means on the right of an
assignment: a number, a string, a truth value. Its **L-value** is what it
means on the left: a location.

Every expression has an R-value. Only some have L-values: a variable has
one, and so (as we shall see) does an array element; `x + 1` does not.

A parameter called by value receives an R-value. A parameter called by
reference receives an L-value. What if the actual parameter has no
L-value to give?

```cpl
let rt Show[ref x] be Write['x is ', x, '*n']
let rt Double[ref x] be x := 2x

let a = 5
Show[a + 1]
Double[a + 1]
```
```output
x is 6.0
! line 2: assignment to a constant (in Double)
```

A location is made to hold the value of `a + 1`, so `Show` works. But it
is a **constant** location: it may be read and not assigned to, which
catches the mistake of handing an expression to a routine that means to
change a variable.

## Definition by reference

The same choice exists for definitions. `let b = a` defines `b` by value:
a new variable, starting with the value of `a`. `let b ~~ a` defines `b`
by reference: a new *name* for the variable `a`. (CPL printed this sign as
an equals sign with a wavy line over it, ≃.)

```cpl
let a = 1
let b ~~ a
let c = a
b := 2
Write[a, ' ', b, ' ', c, '*n']
```
```output
2.0 2.0 1.0
```

Assigning to `b` changed `a`, because they are the same variable; `c` is a
different one and kept the old value.

## Expressions with L-values

A conditional expression has an L-value if its arms have. So it can be
assigned to, and the assignment goes to whichever variable is chosen:

```cpl
let p, q = 0, 0
for x = 3, -2, 5, -1 do
   (x > 0 -> p, q) := (x > 0 -> p, q) + x
Write[p, ' ', q, '*n']
```
```output
8.0 -3.0
```

and it can be passed by reference:

```cpl
let rt Clear[ref x] be x := 0

let a, b = 1, 2
Clear[(a > b -> a, b)]
Write[a, ' ', b, '*n']
```
```output
1.0 0.0
```

`value of` gives a value; its counterpart `reference of` gives a variable:

```cpl
let a, b, c = 5, 9, 2
reference of $( if a >= b /\ a >= c result is a
                if b >= c result is b
                result is c $) := 0
Write[a, ' ', b, ' ', c, '*n']
```
```output
5.0 0.0 2.0
```

That block picked out the largest of three variables, and the assignment
set it to zero.

## Free variables of routines

A routine can use variables that are neither its parameters nor defined
inside it. These are its **free variables**, and a routine works on them
directly:

```cpl
let Total = 0
let rt Add[x] be Total := Total + x

Add[5]; Add[10]; Add[20]
Write[Total, '*n']
```
```output
35.0
```

For functions the matter is more interesting. That is the next chapter.

Next: [Fixed and free functions](10-fixed-and-free.md)
