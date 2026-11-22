# 17. Input and output

The designers of CPL had large plans for input and output, with streams
and files and a `Layout` function, and never finished them. What a CPL
programmer actually had, in the first system that ran, was a handful of
routines described in a two-page supplement
to the Elementary Programming Manual. Those are the ones provided here.
The supplement spelt them in capitals, and both spellings work: `WRITE`
or `Write`.

## Write

`Write` takes any number of parameters of any type.

```cpl
Write['a string', ' ', 42, ' ', 2.5, ' ', true, ' ', #o777, ' ', Cpx[1, 2], '*n']
Write[List[1, 'two'], ' ', real, ' ', 1/3, ' ', 1@20, ' ', 2/1@7, '*n']
```
```output
a string 42 2.5 true #o777 (1.0, 2.0)
(1, 'two') real 0.333333333333 1.0@20 2.0@-7
```

A whole number is printed without a decimal point and a real number with
one, so the output shows the type. Twelve significant figures are given,
and very large or small numbers use `@` for "times ten to the".

This is the "simple example of a CPL job" that ends the supplement:

```cpl
WRITE['*nDEMONSTRATION PROGRAM*n']
$( let x = 1
   WRITE[2x, '*tEnd of test*n'] $)
```
```output

DEMONSTRATION PROGRAM
2.0	End of test
```

## Printd and Printf

`Printd[x, i, j]` prints in "fixed point decimal style, with i figures
before the decimal point, j figures after". Zeros that are not needed are
printed as spaces, so numbers line up in columns.

```cpl
for x = 1, 12.5, -3.14159, 1234.5678 do
   $( Printd[x, 4, 2]
      Write['*n'] $)
```
```output
    1.00
   12.50
   -3.14
 1234.57
```

With `j` zero there is no decimal point:

```cpl
for n = 1, 10, 100, 1000 do
   $( Printd[n, 4, 0]
      Printd[n n, 7, 0]
      Write['*n'] $)
```
```output
    1       1
   10     100
  100   10000
 1000 1000000
```

`Printf[x, i]` prints in "floating decimal style" with `i` significant
figures. A number too big for the `Printd` it was given is printed this
way too.

```cpl
Printf[299792458, 4];  Write['*n']
Printf[-0.000123, 3];  Write['*n']
Printd[123456, 3, 0];  Write['*n']
```
```output
  2.998@+08
 -1.23@-04
  1.23@+05
```

`Writechar` prints one character, given as a string or as its number, and
`Space[n]` and `Newline[]` are as before:

```cpl
Writechar['C']; Writechar[80]; Writechar['L']; Newline[]
```
```output
CPL
```

## A table

```cpl
Write['   x     Sqrt      Exp*n']
for x = step 0, 0.5, 3 do
   $( Printd[x, 2, 1]
      Printd[Sqrt[x], 2, 5]
      Printd[Exp[x], 3, 4]
      Write['*n'] $)
```
```output
   x     Sqrt      Exp
  0.0  0.00000   1.0000
  0.5  0.70711   1.6487
  1.0  1.00000   2.7183
  1.5  1.22474   4.4817
  2.0  1.41421   7.3891
  2.5  1.58114  12.1825
  3.0  1.73205  20.0855
```

## Read

`Read` takes variables, by reference, and puts the next numbers from the
input into them. Numbers are separated by spaces, commas or new lines.

```cpl
let a, b = 0, 0
Read[a, b]
Write[a, ' + ', b, ' = ', a + b, '*n']
```
```input
17 25
```
```output
17.0 + 25.0 = 42.0
```

In this tutorial the box between the program and its output is what the
program was given to read. At the command line, input comes from the
keyboard, or from a file:

```bash
target/release/cpl examples/statistics.cpl < examples/statistics.in
```

There is no way to ask whether there is another number, so a list of
numbers needs something to mark its end. A zero is traditional:

```cpl
let x, s, n = 0, 0, 0
$( Read[x]
   if x = 0 break
   s, n := s + x, n + 1 $) repeat
Write['the mean of ', n, ' numbers is ', s/n, '*n']
```
```input
3 5 8 13
21 0
```
```output
the mean of 5.0 numbers is 10.0
```

Reading past the end of the input stops the program:

```cpl
let x = 0
Read[x]
Read[x]
```
```input
1
```
```output
! line 3: Read: the input stream is exhausted (in main program)
```

## Readchar

`Readchar` reads one character into a string variable. At the end of the
input it gives the empty string, so a loop can stop there:

```cpl
let c = ''
let n be index
n := 0
$( Readchar[c]
   if c = '' break
   if c = 'e' do n := n + 1 $) repeat
Write['there were ', n, ' letter e*'s*n']
```
```input
The quick brown fox jumps over the lazy dog,
and never implements the whole language.
```
```output
there were 10 letter e's
```

This copies its input with every letter made a capital:

```cpl
let c = ''
$( Readchar[c]
   if c = '' break
   Write['a' <= c <= 'z' -> String[Index[c] - 32], c] $) repeat
```
```input
Cambridge and London
```
```output
CAMBRIDGE AND LONDON
```

The example `calculator.cpl` reads whole lines this way and works out the
expressions on them.

Next: [What became of it](18-what-became-of-it.md)
