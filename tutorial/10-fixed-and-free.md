# 10. Fixed and free functions

## Free variables

In `let f[x] = a x + b` the names `a` and `b` are not parameters. They are
**free variables** of `f`: names whose meaning comes from outside. What
should happen if `a` is changed after `f` has been defined?

CPL makes the programmer choose, by the sign in the definition.

```cpl
let a = 1
let f[x] = a x
let g[x] == a x

a := 100
Write[f[5], ' ', g[5], '*n']
```
```output
5.0 500.0
```

`f`, defined with `=`, is a **fixed function**. When the definition is
obeyed, the *values* of its free variables are taken and kept: `f` goes on
multiplying by 1 whatever happens to `a` afterwards. It is the function
that the definition meant at the time, for good.

`g`, defined with `==`, is a **free function**. (CPL printed the sign with
three bars, ≡.) It holds on to the *variables*, and looks at `a` afresh
every time it is called.

In the words of the last chapter: the free variables of a fixed function
are taken by R-value, and those of a free function by L-value. It is the
same choice as between calling by value and calling by reference, and
between `=` and `~~` in a definition.

## A fixed function cannot change the world

A fixed function has only copies of its free variables, so it would make
no sense for it to assign to them, and the system will not allow it:

```cpl
let Count = 0
let Next[] = value of $( Count := Count + 1
                         result is Count $)
Write[Next[]]
```
```output
! line 2: Count is a free variable of the fixed function Next, which may not assign to it (a function defined with == is free and may)
```

With `==` it is allowed, and works:

```cpl
let Count = 0
let Next[] == value of $( Count := Count + 1
                          result is Count $)
Next[]; Next[]
Write[Next[], ' ', Count, '*n']
```
```output
3.0 3.0
```

This gives fixed functions a valuable property. A fixed function whose
parameters are called by value cannot alter anything outside itself, and
the value of a call depends on nothing but the parameters. It behaves like
a function in mathematics.

Routines are always free: a routine exists to change things.

## Functions that make functions

A function can be the result of a function. Each call of `Adder` makes a
new function, with its own copy of `n` fixed inside it:

```cpl
let Adder[n] = f where f[x] = x + n

let Add1, Add10 = Adder[1], Adder[10]
Write[Add1[5], ' ', Add10[5], ' ', Adder[100][5], '*n']
```
```output
6.0 15.0 105.0
```

`Adder[100][5]` calls `Adder` to get a function and then calls that.

In most languages of the 1960s this was impossible. A function was a piece
of program and nothing more, and when `Adder` returned, its `n` was gone.
In CPL a function is a piece of program *together with its free
variables*, and so it can outlive the call that made it. This is what is
now called a closure.

```cpl
let Compose[function f, g] = h where h[x] = f[g[x]]
let Inc[x] = x + 1
let Sq[x] = x x

Write[Compose[Inc, Sq][3], ' ', Compose[Sq, Inc][3], '*n']
```
```output
10.0 16.0
```

## Functions that remember

Put the two ideas together: a free function, made by another function,
sharing a variable that belongs to nobody else.

```cpl
prefer integer

let Counter[Start] = value of
   $( let n = Start
      let Next[] == value of $( n := n + 1
                                result is n $)
      result is Next $)

let a, b = Counter[0], Counter[100]
Write[a[], ' ', a[], ' ', a[], '*n']
Write[b[], ' ', b[], '*n']
Write[a[], '*n']
```
```output
1 2 3
101 102
4
```

Each call of `Counter` makes a new variable `n` and a new function that
shares it. The two counters do not disturb each other, and nothing else in
the program can touch either `n`.

## Functions in variables

A function can be assigned to a variable of type function, and the
variable called:

```cpl
let f be function
for i = 1 to 3 do
   $( f := (i = 1 -> Sin, i = 2 -> Cos, Sqrt)
      Write[f[0], ' '] $)
Write['*n']
```
```output
0.0 1.0 0.0 
```

A function defined by `let` is a constant, so that its meaning cannot be
changed behind the back of the code that uses it:

```cpl
let f[x] = x + 1
f := Sqrt
```
```output
! line 2: assignment to a constant (in main program)
```

Next: [Arrays](11-arrays.md)
