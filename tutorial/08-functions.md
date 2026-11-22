# 8. Functions

## Definitions

A function is defined like a variable, with its parameters in square
brackets after its name:

```cpl
let F[x] = 3x**2 + 4x + 1
Write[F[0], ' ', F[1], ' ', F[2.5], '*n']
```
```output
1.0 8.0 29.75
```

That is the example from section 18 of the Elementary Programming Manual.
`x` is the **formal parameter**; in the call `F[2.5]`, 2.5 is the **actual
parameter**. A function may have any number of parameters, including none,
but the brackets are always written:

```cpl
let Mean[a, b] = (a + b)/2
let Seven[] = 7
Write[Mean[3, 4], ' ', Seven[], '*n']
```
```output
3.5 7.0
```

A function call is an expression and can go wherever an expression can:

```cpl
let Sq[x] = x x
Write[Sq[Sq[2] + 1], ' ', Sqrt[Sq[3] + Sq[4]], '*n']
```
```output
25.0 5.0
```

## The types of parameters

A parameter is of the preferred type, real, unless something else is
said. A type written in the parameter list applies to the names after it,
until another type is written:

```cpl
let Q[index i, j] = i(i + 1) + j
let Initial[string s, real x] = First[s] <=> '.'
Write[Q[3, 4], ' ', Type[Q[3, 4]], ' ', Initial['Strachey', 0], '*n']
```
```output
16 index S.
```

The actual parameter is converted to the type of the formal one, just as
in an assignment:

```cpl
let Half[integer n] = Quot[n, 2]
Write[Half[10.0], '*n']
Write[Half[10.5], '*n']
```
```output
5
! line 3: parameter n of Half: alarm: the real number 10.5 is not integral (use Round or Intpt to make an integer of it) (in main program)
```

## Functions that need commands: value of

The body of a function is an expression. When working it out needs
commands (a loop, say) the expression can be a block, introduced by
`value of`. The command `result is E` ends the block and gives its value.

```cpl
prefer integer

let Sum[index n] = value of
   $( let s = 0
      for i = 1 to n do s := s + i
      result is s $)

Write[Sum[100], '*n']
```
```output
5050
```

"Throughout CPL there is a sharp distinction between commands and
expressions", says the manual, and `value of` is the bridge from one to
the other. It can be used anywhere, not only in function definitions:

```cpl
let x = 10
let y = 1 + value of $( let t = x x
                        result is t + 1 $)
Write[y, '*n']
```
```output
102.0
```

The first `result is` to be obeyed ends the block, so it can be used to
leave early:

```cpl
let Prime[index n] = value of
   $( if n < 2 result is false
      for d = 2 to n - 1 do
         $( if d d > n break
            if Rem[n, d] = 0 result is false $)
      result is true $)

for n = 1 to 30 do if Prime[n] do Write[n, ' ']
Write['*n']
```
```output
2 3 5 7 11 13 17 19 23 29 
```

`valof` and `resultis`, each as one word, are how BCPL later spelt these,
and they are accepted too.

## Recursion

A function that uses itself must say so, with `rec`:

```cpl
let rec Fact[integer n] = n = 0 -> 1, n Fact[n - 1]
Write[Fact[5], ' ', Fact[20], '*n']
```
```output
120 2432902008176640000
```

This is the Euclidean algorithm exactly as the manual gives it:

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

Why must it be said? Because a definition normally cannot see itself: the
names on the right of `let f[x] = ...` are the names that existed before
the definition was made. Without `rec`, an `f` on the right is some other,
older `f`:

```cpl
let f[n] = n + 1
$( let f[n] = n = 0 -> 100, f[n - 1]
   Write[f[5], '*n'] $)
```
```output
5.0
```

The inner `f` calls the outer one, which adds 1 to 4. With `rec` it would
have called itself and given 100.

Functions that call one another are defined together, with `and`:

```cpl
let rec Even[index n] = n = 0 -> true, Odd[n - 1]
    and Odd[index n] = n = 0 -> false, Even[n - 1]
Write[Even[10], ' ', Odd[10], '*n']
```
```output
true false
```

## More than one result

A function can give several results. They are received by a definition or
an assignment with the same number of names on the left:

```cpl
let Divide[integer a, b] = Quot[a, b], Rem[a, b]

let q, r = Divide[17, 5]
Write[q, ' remainder ', r, '*n']
```
```output
3 remainder 2
```

```cpl
let Minmax[a, b, c] = value of
   $( let l, h = a, a
      for x = b, c do
         $( if x < l do l := x
            if x > h do h := x $)
      result is l, h $)

let l, h = Minmax[3, 9, 1]
Write[l, ' to ', h, '*n']
```
```output
1.0 to 9.0
```

## Functions as parameters

A function is a value like any other. It can be handed to another
function:

```cpl
let Twice[function f, real x] = f[f[x]]
let Inc[x] = x + 1

Write[Twice[Inc, 5], ' ', Twice[Sqrt, 16], '*n']
```
```output
7.0 2.0
```

```cpl
let Table[function f] = value of
   $( for x = 0 to 4 do Write[f[x], ' ']
      result is '*n' $)

Write[Table[Sq]] where Sq[x] = x x
Write[Table[Exp]]
```
```output
0.0 1.0 4.0 9.0 16.0 
1.0 2.71828182846 7.38905609893 20.0855369232 54.5981500331 
```

[Chapter 10](10-fixed-and-free.md) and [chapter 14](14-lists.md) take this
much further.

Next: [Routines and references](09-routines-and-references.md)
