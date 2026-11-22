# 16. New kinds of variable

## LH functions

[Chapter 9](09-routines-and-references.md) said that an expression has an
L-value, what it means on the left of `:=`, and an R-value, what it means
on the right. A function call normally has only an R-value. CPL lets the
programmer define a function whose calls can stand on the left as well:
an **LH function**. It has two parts. `load` says how to get the value;
`update` says what to do when something is assigned, and in it `rhs`
stands for the value being assigned.

```cpl
let c = 100                    || a temperature, kept in Celsius

let function Fahrenheit[] be
       load $( result is 9c/5 + 32 $)
       update $( c := 5(rhs - 32)/9 $)

Write[Fahrenheit[], '*n']
Fahrenheit[] := 32
Write[c, '*n']
Fahrenheit[] := Fahrenheit[] + 18
Write[c, '*n']
```
```output
212.0
0.0
10.0
```

`Fahrenheit[]` behaves like a variable. There is no such variable: there
is only `c`, seen through a pair of functions.

Because it is a real L-value, it can do whatever an L-value can. It can
be given a name by reference, and passed to a routine that expects a
variable:

```cpl
let c = 0
let function Fahrenheit[] be
       load $( result is 9c/5 + 32 $)
       update $( c := 5(rhs - 32)/9 $)

let rt Double[ref x] be x := 2x

let f ~~ Fahrenheit[]
f := 106
Double[f]
Write[c, '*n']
```
```output
100.0
```

`Double` knows nothing about temperatures. It was handed something to
load from and update, and did so.

## Subscripts of your own

An LH function with parameters is an array whose storage you design. This
one is a symmetric matrix, which keeps only half its elements: `Sym[i, j]`
and `Sym[j, i]` are the same variable.

```cpl
let t = Newarray[real, (1, 10)]
for k = 1 to 10 do t[k] := 0

let function Sym[index i, j] be
    $( fix k = (i >= j -> Quot[i(i - 1), 2] + j, Quot[j(j - 1), 2] + i)
       load $( result is t[k] $)
       update $( t[k] := rhs $) $)

Sym[1, 3] := 7
Sym[4, 2] := 5
Write[Sym[3, 1], ' ', Sym[2, 4], ' ', Sym[4, 4], '*n']
```
```output
7.0 5.0 0.0
```

The `fix` part is worked out once, when the call is made, and settles
*which* variable is meant; `load` and `update` then use it. That matters
when the parameters change between the call and the use:

```cpl
let t = Formarray[real, (1, 3)][10, 20, 30]
let function Elem[index i] be
    $( fix k = i
       load $( result is t[k] $)
       update $( t[k] := rhs $) $)

let n = 1
let e ~~ Elem[n]
n := 3
e := 99
Write[t[1], ' ', t[2], ' ', t[3], '*n']
```
```output
99.0 20.0 30.0
```

`e` was fixed as element 1 when it was defined, and stayed element 1.

The example `sparse-array.cpl` uses an LH function to make a vector of a
million elements that stores only the ones in use.

Modern languages call these properties, or getters and setters. In CPL
they were not a feature of objects; they came straight out of taking the
two sides of an assignment seriously. BCPL left them out.

## Call by substitution

There is a third way to pass a parameter, older than the other two. ALGOL
60 called it call by name: the actual parameter is not worked out at all
when the function is called, but handed over as an expression, to be
worked out afresh each time the function uses it. CPL had it in 1963,
under the name **substitution**, and had dropped it by 1966. It is here
because it is instructive.

```cpl
let Sum[ref i, val a, b, subst t] = value of
   $( let s = 0
      for k = step a, 1, b do
         $( i := k
            s := s + t $)
      result is s $)

let i = 0
Write[Sum[i, 1, 10, i i], '*n']
Write[Sum[i, 1, 100, 1/i], '*n']
```
```output
385.0
5.18737751764
```

This trick is called Jensen's device. `Sum` sets `i` and then asks for
`t`; `t` is the expression `i i`, which is worked out with the new `i`.
So the one function adds up the squares, or the reciprocals, or anything
else that can be written in terms of `i`.

A definition can be by substitution too. Here `w` is not a number; it is
an expression, which has a different value whenever `x` has:

```cpl
let x, y = 1, 2
let w == x x + 2x y + y y
Write[w, ' ']
x := 10
Write[w, '*n']
```
```output
9.0 144.0
```

The designers came to prefer passing a function, which says the same
thing more plainly, and that is how it would be done in the CPL of 1966:

```cpl
let Sum[index a, b, function f] = value of
   $( let s = 0
      for k = step a, 1, b do s := s + f[k]
      result is s $)

Write[Sum[1, 10, Sq], '*n'] where Sq[x] = x x
```
```output
385.0
```

Next: [Input and output](17-input-and-output.md)
