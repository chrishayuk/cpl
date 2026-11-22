# 7. Blocks and scope

## Section brackets

`$(` and `$)` make several commands into one. CPL printed them as the
section sign § and the same sign struck through; C took them from BCPL as
`{` and `}`, and those are accepted here too.

If there are definitions among the commands, the section is called a
**block**, and the names defined in it belong to it:

```cpl
let x = 1
$( let x = 2
   let y = 3
   Write['inside: ', x, ' ', y, '*n'] $)
Write['outside: ', x, '*n']
```
```output
inside: 2.0 3.0
outside: 1.0
```

Inside the block, `x` means the new variable. The old one is still there,
untouched, and is seen again when the block ends. `y` does not exist
outside the block at all:

```cpl
$( let y = 3
   Write[y, '*n'] $)
Write[y]
```
```output
! line 3: the name y is not defined here
```

## Tags

In a long program it is easy to lose track of which bracket closes which.
A section bracket can carry a **tag**, written straight after it, and the
closing bracket must carry the same one:

```cpl
$(1 for i = 1 to 2 do
    $(1.1 for j = 1 to 2 do
          $(1.2 Write[i, j, ' '] $)1.2
          Write['*n'] $)1.1
$)1
```
```output
11 12 
21 22 
```

A closing bracket with a tag also closes any untagged or inner sections
that are still open, so that one bracket can end several:

```cpl
$(Outer
   for i = 1 to 3 do
      $( if i = 2 do
            $( Write['two*n']
$)Outer
Write['done*n']
```
```output
two
done
```

## Definitions in parallel: and

Definitions are obeyed in order, and each can use the ones before it:

```cpl
let a = 5
let b = a + 1
Write[b, '*n']
```
```output
6.0
```

Definitions joined by `and` are made together, and none of them can see
the others. The Elementary Programming Manual shows the difference with
this pair:

```cpl
$( let a = 5
   $( let a = 10
      let b = a
      Write[b, ' '] $)
   $( let a = 10 and b = a
      Write[b, '*n'] $) $)
```
```output
10.0 5.0
```

In the first inner block `b` is defined after the new `a` and gets 10. In
the second the two definitions are simultaneous, so the `a` on the right
is the outer one, and `b` gets 5.

## where

A definition can be put after the thing that uses it. This is how a
mathematician writes, and it keeps the main line of the thought first:

```cpl
let a, b, c = 1, -3, 2
Write[(-b + d)/(2a), ' ', (-b - d)/(2a), '*n'] where d = Sqrt[b b - 4a c]
```
```output
2.0 1.0
```

The name `d` exists only for that one command. A where-clause can also
qualify an expression in round brackets:

```cpl
let x = 3
let y = (t t + 1/t where t = 2x + 1)
Write[y, '*n']
```
```output
49.1428571429
```

and it may hold several definitions joined by `and`:

```cpl
Write[(a a + b b where a = 3 and b = 4), '*n']
```
```output
25.0
```

## Private variables: in

`D1 in D2` makes the names defined by `D1` visible in `D2` and nowhere
else. Here `Count` is known only to the function `Next`:

```cpl
let Count = 0 in Next[] == value of $( Count := Count + 1; result is Count $)

Next[]; Next[]
Write[Next[], '*n']
```
```output
3.0
```

The variable keeps its value from one call to the next, because it was
created once, when the definition was obeyed, and not inside the function.
Nobody else can reach it:

```cpl
let Count = 0 in Next[] == value of $( Count := Count + 1; result is Count $)
Write[Count]
```
```output
! line 2: the name Count is not defined here
```

(The pieces of that definition, `value of`, `result is` and the `==`, are
explained in the next three chapters.)

Next: [Functions](08-functions.md)
