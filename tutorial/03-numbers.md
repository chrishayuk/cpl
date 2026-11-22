# 3. Numbers

## Arithmetic

```cpl
Write[2 + 3, ' ', 7 - 10, ' ', 6 * 7, ' ', 2 ** 10, '*n']
```
```output
5 -3 42 1024.0
```

`**` raises to a power (CPL wrote an upward arrow), and its result is
always a real number, which is why 1024 has a decimal point. Multiplication and
division are done before addition and subtraction, and round brackets
group:

```cpl
Write[2 + 3 * 4, ' ', (2 + 3) * 4, '*n']
```
```output
14 20
```

## Whole numbers and real numbers

CPL has several types of number. The two that matter at first are the
whole numbers, which it calls **index** (small ones, for counting and
subscripts) and **integer** (large ones), and the **real** numbers. A
constant is the smallest type that will hold it:

```cpl
Write[Type[7], ' ', Type[7.5], ' ', Type[5000000000], '*n']
```
```output
index real integer
```

Adding, subtracting and multiplying whole numbers gives a whole number.
Dividing does not: `/` always gives a real result, and a real number is
always printed with a decimal point.

```cpl
Write[7 + 2, ' ', 7 * 2, ' ', 7 / 2, ' ', 8 / 2, '*n']
```
```output
9 14 3.5 4.0
```

For division of whole numbers there are two functions: `Quot` gives the
quotient and `Rem` the remainder.

```cpl
Write[Quot[17, 5], ' remainder ', Rem[17, 5], '*n']
```
```output
3 remainder 2
```

A very large or very small number is written with a power of ten. The
manual printed a small subscript 10; here it is `@`. (The last of these is
a whole number, however it is written, and so it is an index.)

```cpl
Write[6.02@23, ' ', 1@-6, ' ', 1.5@3, '*n']
```
```output
6.02@23 1.0@-6 1500
```

## Multiplication without a sign

In mathematics *2x* means 2 times *x*, and so it does in CPL. Writing two
things side by side multiplies them:

```cpl
let x, y = 3, 4
Write[2x, ' ', x y, ' ', xy, ' ', 2x(y + 1), ' ', (x + y)(x - y), '*n']
```
```output
6.0 12.0 12.0 30.0 -7.0
```

(`let x, y = 3, 4` defines two variables; that is the next chapter. A
variable defined this way is real, which is why the answers have decimal
points.)

This is why a name longer than one letter must begin with a capital: `xy`
is a product. It also explains a trap. Division and multiplication are
done from left to right, so `a/bc` is *a/b* times *c*, not *a* over *bc*:

```cpl
let a, b, c = 12, 2, 3
Write[a/b c, ' ', a/(b c), '*n']
```
```output
18.0 2.0
```

A minus sign in front of something is treated as multiplication by -1, so
it is done after a power:

```cpl
Write[-2 ** 2, ' ', (-2) ** 2, '*n']
```
```output
-4.0 4.0
```

## The standard functions

```cpl
Write[Sqrt[2], '*n']
Write[Exp[1], ' ', Log[10], '*n']
Write[Sin[0], ' ', Cos[0], ' ', 4 Arctan[1], '*n']
```
```output
1.41421356237
2.71828182846 2.30258509299
0.0 1.0 3.14159265359
```

| Function | |
| --- | --- |
| `Sqrt[x]` | square root |
| `Exp[x]`, `Log[x]` | exponential and natural logarithm |
| `Sin[x]`, `Cos[x]`, `Tan[x]` | the angle is in radians |
| `Arctan[x]`, `Arcsin[x]`, `Arccos[x]` | |
| `Mod[x]` | modulus: the absolute value |
| `Sign[x]` | -1, 0 or 1 |
| `Intpt[x]`, `Frpt[x]` | integral part and fractional part |
| `Round[x]` | the nearest whole number |
| `Quot[a, b]`, `Rem[a, b]` | quotient and remainder |

```cpl
Write[Mod[-3.5], ' ', Sign[-3.5], ' ', Intpt[3.7], ' ', Frpt[3.7], ' ', Round[3.7], '*n']
```
```output
3.5 -1 3 0.7 4
```

`Intpt` and `Round` give whole numbers; that is the way to get from a real
number to an integer.

## Printing numbers in columns

`Write` prints a number as briefly as it can. For tables there is
`Printd[x, i, j]`, which prints `x` with room for `i` figures before the
decimal point and `j` after it:

```cpl
Printd[3.14159, 3, 2];  Write['|*n']
Printd[42, 5, 0];       Write['|*n']
Printd[-0.5, 2, 3];     Write['|*n']
```
```output
   3.14|
    42|
 -0.500|
```

There is more about output in [chapter 17](17-input-and-output.md).

Next: [Names and definitions](04-definitions.md)
