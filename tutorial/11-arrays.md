# 11. Arrays

## Making an array

An array is made by calling a function. `Newarray` is given the type of
the elements and, for each dimension, the lowest and highest subscript:

```cpl
let a = Newarray[real, (1, 5)]
for i = 1 to 5 do a[i] := i i
for i = 1 to 5 do Write[a[i], ' ']
Write['*n']
```
```output
1.0 4.0 9.0 16.0 25.0 
```

An element is written with its subscript in square brackets, and has an
L-value: it can be assigned to, and passed by reference.

Because the bounds are ordinary expressions worked out when the program
runs, the size of an array can depend on anything, and the lower bound
need not be 1 or 0:

```cpl
let n = 3
let t = Newarray[index, (-n, n)]
for i = -n to n do t[i] := i i
for i = -n to n do Write[t[i], ' ']
Write['*n']
```
```output
9 4 1 0 1 4 9 
```

`Newarray` gives no values to the elements. `Formarray` makes an array and
fills it from a list:

```cpl
let p = Formarray[index, (1, 6)][2, 3, 5, 7, 11, 13]
Write[p[1] + p[6], '*n']
```
```output
15
```

`Formarray[index, (1, 6)]` is a function, which is then applied to the six
values: that is why there are two pairs of brackets.

## Subscripts are checked

```cpl
let a = Newarray[real, (1, 5)]
a[6] := 0
```
```output
! line 2: subscript 6 is outside the bounds (1, 5) (in main program)
```

## More dimensions

```cpl
let m = Formarray[real, (1, 2), (1, 3)][1, 2, 3,
                                        4, 5, 6]
for i = 1 to 2 do
   $( for j = 1 to 3 do Write[m[i, j], ' ']
      Write['*n'] $)
```
```output
1.0 2.0 3.0 
4.0 5.0 6.0 
```

The types of arrays have names: `real vector` or `real 1 array` for one
dimension, `real matrix` or `real 2 array` for two, `index 3 array` and so
on. They are used for parameters:

```cpl
let Trace[matrix m, index n] = value of
   $( let s = 0
      for i = 1 to n do s := s + m[i, i]
      result is s $)

let m = Formarray[real, (1, 2), (1, 2)][8, 10, 12, -16]
Write[Trace[m, 2], '*n']
```
```output
-8.0
```

`Bounds` gives the lowest and highest subscript of an array, so a function
need not be told them:

```cpl
let Total[vector v] = value of
   $( let l, h = Bounds[v]
      let s = 0
      for i = step l, 1, h do s := s + v[i]
      result is s $)

Write[Total[Formarray[real, (0, 3)][1, 2, 3, 4]], '*n']
```
```output
10.0
```

## The matrix product

This is the example that closes section 14 of the Elementary Programming
Manual: "suppose we have three two-dimensional square arrays A, B, C,
whose subscripts go from 1 to n, then the following program sets C equal
to the matrix product AB". The capital letters `A`, `B` and `C` are large
names, so the product of two elements needs no sign.

```cpl
let n = 2
let A = Formarray[real, (1, n), (1, n)][1, 2, 3, 4]
let B = Formarray[real, (1, n), (1, n)][0, 1, 1, 0]
let C = Newarray[real, (1, n), (1, n)]

for i = 1 to n do
$(1.1 for j = 1 to n do
   $(1.2 let a = 0
         for k = 1 to n do
         $(1.3 a := a + A[i, k] B[k, j] $)1.3
         C[i, j] := a $)1.1

Write[C[1, 1], ' ', C[1, 2], '*n', C[2, 1], ' ', C[2, 2], '*n']
```
```output
2.0 1.0
4.0 3.0
```

## An array is a pointer to its elements

This is the thing to understand about CPL arrays. The *value* of an array
variable is not the elements; it is the place where they are. So
assignment and definition do not copy the elements:

```cpl
let v = Formarray[index, (1, 3)][1, 2, 3]
let w = v
w[1] := 99
Write[v[1], '*n']
```
```output
99
```

`w` and `v` are two variables holding the same array. To have a separate
one, ask for it with `Copy`:

```cpl
let v = Formarray[index, (1, 3)][1, 2, 3]
let w = Copy[v]
w[1] := 99
Write[v[1], ' ', w[1], '*n']
```
```output
1 99
```

For the same reason a routine can change the elements of an array that
was passed to it by value. What was passed was the pointer:

```cpl
let rt Clear[vector v] be
   $( let l, h = Bounds[v]
      for i = step l, 1, h do v[i] := 0 $)

let v = Formarray[real, (1, 3)][1, 2, 3]
Clear[v]
Write[v[1], v[2], v[3], '*n']
```
```output
0.00.00.0
```

and an array variable can be given a different array, of a different
size, at any time:

```cpl
let v = Newarray[real, (1, 3)]
v := Newarray[real, (1, 100)]
v[100] := 1
Write[v[100], '*n']
```
```output
1.0
```

C programmers will recognise all of this. It is where C's arrays come
from, by way of BCPL.

`=` between arrays asks whether they are the same array; `Equal` asks
whether they have the same elements:

```cpl
let v = Formarray[index, (1, 3)][1, 2, 3]
let w = Copy[v]
Write[v = w, ' ', Equal[v, w], '*n']
```
```output
false true
```

## Rows, and arrays that are not rectangles

`a[i, j]` means the same as `a[i][j]`: a matrix is a vector of vectors,
and `a[i]` is a row.

```cpl
let a = Formarray[real, (1, 2), (1, 3)][1, 2, 3, 4, 5, 6]
let r ~~ a[2]
Write[r[1], ' ', r[3], ' ', a[2][2], '*n']
```
```output
4.0 6.0 5.0
```

Since each row is an array in its own right, the rows need not be the same
length. Here is a triangle:

```cpl
let t = Newarray[general, (1, 4)]
for i = 1 to 4 do
   $( t[i] := Newarray[index, (1, i)]
      for j = 1 to i do t[i][j] := i j $)

for i = 1 to 4 do
   $( for j = 1 to i do Write[t[i][j], ' ']
      Write['*n'] $)
```
```output
1 
2 4 
3 6 9 
4 8 12 16 
```

Next: [Strings and bits](12-strings-and-bits.md)
