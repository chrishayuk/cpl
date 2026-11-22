# 14. Lists

The lists in this chapter come from the first published account of the
language, "The main features of CPL" (1963), which promised lists "similar
in many respects to those of the LISP system". The 1966 reference manual
left all such structures for a later edition that was never written. They
are included here because CPL without them is not the language its
designers described; see the [README](../README.md) for the details.

The programs in this chapter begin with `prefer general`, so that
parameters can be lists, or the members of lists, without saying so.

## Making lists

```cpl
let l = List[1, 2, 3]
let m = List['one', 2, 3.5, true, List['a', 'b']]
Write[l, '*n', m, '*n', NIL, '*n']
```
```output
(1, 2, 3)
('one', 2, 3.5, true, ('a', 'b'))
NIL
```

`List` makes a list of its parameters. The members can be of any type, and
can be lists. `NIL` is the empty list.

A list is taken apart with `Hd` (the head: the first member) and `Tl` (the
tail: the list of all the others), and built up with `Cons`, which puts
one more member on the front:

```cpl
let l = List[1, 2, 3]
Write[Hd[l], ' ', Tl[l], ' ', Hd[Tl[l]], ' ', Cons[0, l], '*n']
Write[Null[l], ' ', Null[NIL], ' ', Null[Tl[Tl[Tl[l]]]], '*n']
```
```output
1 (2, 3) 2 (0, 1, 2, 3)
false true true
```

`Null[l]` is true if `l` is empty. `Atom[x]` is true if `x` is not a list
at all, and `Length` counts the members.

## Functions on lists

Nearly every function on lists has the same shape: say what the answer is
for the empty list, and say how to get the answer for a list from its head
and the answer for its tail.

```cpl
prefer general

let rec Sum[l] = Null[l] -> 0, Hd[l] + Sum[Tl[l]]
let rec Count[l] = Null[l] -> 0, 1 + Count[Tl[l]]
let rec Join[a, b] = Null[a] -> b, Cons[Hd[a], Join[Tl[a], b]]
let rec Reverse[l] = Null[l] -> NIL, Join[Reverse[Tl[l]], List[Hd[l]]]

let l = List[3, 1, 4, 1, 5]
Write[Sum[l], ' ', Count[l], '*n']
Write[Join[l, List[9, 2, 6]], '*n']
Write[Reverse[l], '*n']
```
```output
14 5
(3, 1, 4, 1, 5, 9, 2, 6)
(5, 1, 4, 1, 3)
```

```cpl
prefer general

let rec Upto[a, b] = a > b -> NIL, Cons[a, Upto[a + 1, b]]
let rec Member[x, l] = Null[l] -> false, Hd[l] = x -> true, Member[x, Tl[l]]

Write[Upto[1, 10], '*n']
Write[Member[7, Upto[1, 10]], ' ', Member[11, Upto[1, 10]], '*n']
```
```output
(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
true false
```

## Functions that take functions

Three functions do most of the work. `Map` applies a function to every
member; `Filter` keeps the members that pass a test; `Fold` combines all
the members with a function of two parameters.

```cpl
prefer general

let rec Map[f, l] = Null[l] -> NIL, Cons[f[Hd[l]], Map[f, Tl[l]]]
let rec Filter[p, l] = Null[l] -> NIL,
                       p[Hd[l]] -> Cons[Hd[l], Filter[p, Tl[l]]],
                       Filter[p, Tl[l]]
let rec Fold[f, z, l] = Null[l] -> z, f[Hd[l], Fold[f, z, Tl[l]]]
let rec Upto[a, b] = a > b -> NIL, Cons[a, Upto[a + 1, b]]

let l = Upto[1, 10]
Write[Map[Sq, l], '*n']             where Sq[x] = x x
Write[Filter[Even, l], '*n']        where Even[x] = Rem[x, 2] = 0
Write[Fold[Plus, 0, l], '*n']       where Plus[a, b] = a + b
Write[Fold[Times, 1, l], '*n']      where Times[a, b] = a b
Write[Map[Sqrt, List[1, 4, 9]], '*n']
```
```output
(1, 4, 9, 16, 25, 36, 49, 64, 81, 100)
(2, 4, 6, 8, 10)
55
3628800
(1.0, 2.0, 3.0)
```

The `where` at the end of each line defines the little function that the
line needs, on the spot.

## Strachey's Cartesian product

This program is often quoted as an example of what CPL could say. Given a
list of lists, it produces every way of choosing one member from each. It
is the function `Lit` (the same as `Fold` above) used three times over,
with three nested where-clauses, each of which can see the parameters of
the function it qualifies:

```cpl
prefer general

let rec Lit[F, z, L] = Null[L] -> z, F[Hd[L], Lit[F, z, Tl[L]]]

let Product[L] = Lit[f, List1[NIL], L]
    where f[k, z] = Lit[g, NIL, k]
          where g[x, y] = Lit[h, y, z]
                where h[p, q] = Cons[Cons[x, p], q]

Write[Product[List[List[1, 2], List['a', 'b', 'c']]], '*n']
Write[Length[Product[List[List[1, 2, 3], List[4, 5], List[6, 7, 8, 9]]]], '*n']
```
```output
((1, 'a'), (1, 'b'), (1, 'c'), (2, 'a'), (2, 'b'), (2, 'c'))
24
```

(`List1[x]` is a list of one member.) No loops, no assignments, no
variables that change: only functions, defined in terms of one another.

## Lists and names

A list can be spread out into separate names. In a definition or an
assignment with several names on the left and one list on the right, the
members are matched to the names:

```cpl
let p = List['Strachey', 1916, true]
let a, b, c = p
Write[a, ' was born in ', b, '*n']

let x, y = 0, 0
x, y := List[10, 20]
Write[x + y, '*n']
```
```output
Strachey was born in 1916
30.0
```

The numbers must agree:

```cpl
let a, b = List[1, 2, 3]
```
```output
! line 1: a list of 3 member(s) is matched against 2 name(s) (in main program)
```

`Members` does the same thing inside a list of parameters, where it turns
a list into so many separate parameters:

```cpl
let Add3[a, b, c] = a + b + c
let l = List[1, 2, 3]
Write[Add3[Members[l]], ' ', List[0, Members[l], 4], '*n']
```
```output
6.0 (0, 1, 2, 3, 4)
```

## Changing a list

`Hd[l]` and `Tl[l]` have L-values, so the cells of a list can be assigned
to:

```cpl
let l = List[1, 2, 3]
Hd[l] := 'one'
Hd[Tl[l]] := 'two'
Tl[Tl[Tl[l]]] := List[4, 5]
Write[l, '*n']
```
```output
('one', 'two', 3, 4, 5)
```

Like arrays, lists are not copied when they are assigned: two variables
can hold the same list, and a change made through one is seen through the
other.

```cpl
let l = List[1, 2, 3]
let m = l
Hd[m] := 99
Write[l, '*n']
```
```output
(99, 2, 3)
```

Next: [Labels and jumps](15-labels-and-jumps.md)
