# 12. Strings and bits

## Strings

A string is a data item like a number. It can be kept in a variable,
passed to a function and given back as a result, and it can be as long as
you like.

```cpl
let s = 'Combined'
let t = 'Programming Language'
Write[s, ' ', t, '*n']
```
```output
Combined Programming Language
```

There is one operator on strings: `<=>` joins two of them end to end.

```cpl
let s = 'Combined'
s := s <=> ' ' <=> 'Programming'
Write[s <=> '!', '*n']
```
```output
Combined Programming!
```

Everything else is done with functions.

| Function | |
| --- | --- |
| `Length[s]` | the number of characters |
| `First[s]`, `Last[s]` | the first character, the last |
| `Character[i, s]` | the i-th character |
| `Initials[i, s]`, `Finals[i, s]` | the first i characters, the last i |

```cpl
let s = 'Strachey'
Write[Length[s], ' ', First[s], ' ', Last[s], ' ', Character[4, s], ' ',
      Initials[3, s], ' ', Finals[3, s], '*n']
```
```output
8 S y a Str hey
```

A character is simply a string of length one. A string is taken apart with
a loop:

```cpl
let s = 'Atlas'
for i = step Length[s], -1, 1 do Write[Character[i, s]]
Write['*n']
```
```output
saltA
```

or by recursion:

```cpl
let rec Reverse[string s] = s = '' -> '', Reverse[Finals[Length[s] - 1, s]] <=> First[s]
Write[Reverse['Titan'], '*n']
```
```output
natiT
```

## Comparing strings

The relations work on strings, in dictionary order. "A longer string is
greater than a shorter string identical to its starting characters."

```cpl
Write['cat' < 'dog', ' ', 'cat' < 'catalogue', ' ', 'cat' = 'cat', ' ', 'Cat' < 'cat', '*n']
```
```output
true true true true
```

```cpl
let Kind[string c] = 'a' <= c <= 'z' -> 'small letter',
                     'A' <= c <= 'Z' -> 'capital',
                     '0' <= c <= '9' -> 'digit',
                     'something else'
for c = 'x', 'Q', '7', '?' do Write[c, ' is a ', Kind[c], '*n']
```
```output
x is a small letter
Q is a capital
7 is a digit
? is a something else
```

## Characters and numbers

Every character has a number. `Index` of a character gives it, and
`String` of a number gives the character back:

```cpl
Write[Index['A'], ' ', Index['a'], ' ', Index['0'], ' ', String[67], String[80], String[76], '*n']
```
```output
65 97 48 CPL
```

That is enough to turn a capital into a small letter, or a number into
its digits:

```cpl
let Small[string c] = 'A' <= c <= 'Z' -> String[Index[c] + 32], c

let rec Digits[integer n] = n < 10 -> String[48 + n],
                            Digits[Quot[n, 10]] <=> String[48 + Rem[n, 10]]

Write[Small['Q'], Small['q'], ' ', Digits[1966] <=> ' was the year', '*n']
```
```output
qq 1966 was the year
```

## Logical values

The other non-numerical type is **logical**: a row of 24 bits, the length
of half a word on the Atlas computer. (**long logical** has 48.) The manual
says that logical variables "provide the means whereby most non-numerical
work is carried out in CPL".

A logical constant is written in octal or binary. CPL printed a bold 8 or
2 in front of the digits; here it is `#o` or `#b`, with `#x` for
hexadecimal as an extra.

```cpl
let p = #o2736
Write[p, ' ', #b010111011110, ' ', #x5DE, '*n']
```
```output
#o2736 #o2736 #o2736
```

The Boolean operators work on logicals one bit at a time:

```cpl
let a, b = #b1100, #b1010
Write[a /\ b, ' ', a \/ b, ' ', a ~== b, ' ', ~a /\ #b1111, '*n']
```
```output
#o10 #o16 #o6 #o3
```

So `c /\ #o77` keeps the bottom six bits of `c` and clears the rest, and
this, from the manual, "replaces the field in c specified by the ones in
b, by the corresponding field of a":

```cpl
let a, b, c = #o1234, #o0770, #o5555
c := (c /\ ~b) \/ (b /\ a)
Write[c, '*n']
```
```output
#o5235
```

The functions number the bits from 0 at the right hand end.

| Function | |
| --- | --- |
| `Lshift[p, j]`, `Rshift[p, j]` | shift j places; zeros come in at the end |
| `Rotate[p, j]` | shift left j places, round and round |
| `Ones[j, k]` | ones in bits j to k, zeros elsewhere |
| `Field[p, j, k]` | bits j to k of p, moved to the right hand end |
| `Mask[p, j, k]` | bits j to k of p, left where they are |
| `Bit[p, j]` | bit j of p |

```cpl
let p = #o2736
Write[Lshift[p, 3], ' ', Rshift[p, 3], ' ', Rotate[p, 22], '*n']
Write[Ones[3, 0], ' ', Field[p, 6, 10], ' ', Mask[p, 6, 10], ' ', Bit[p, 1], '*n']
```
```output
#o27360 #o273 #o40000567
#o17 #o27 #o2700 #o1
```

"The functions Bit and Field may be used on the left-hand side of
assignment commands": when the logical is a variable, they name part of
it.

```cpl
let p = #o0
Bit[p, 0] := #o1
Field[p, 3, 5] := #o7
Write[p, '*n']
```
```output
#o71
```

`Index` turns a logical into the number it represents in binary, and
`Logical` turns a number into bits:

```cpl
Write[Index[#b1010], ' ', Logical[10], ' ', Index[#o777], '*n']
```
```output
10 #o12 511
```

```cpl
|| Count the ones in a word.
let Ones'[logical w] = value of
   $( let n be index
      n := 0
      until w = #o0 do
         $( n := n + Index[Bit[w, 0]]
            w := Rshift[w, 1] $)
      result is n $)

Write[Ones'[#o2736], ' ', Ones'[#o77777777], '*n']
```
```output
8 24
```

This is the part of CPL that BCPL kept when it threw the types away. In
BCPL *everything* is a row of bits, and it is up to the programmer to
remember which rows are numbers.

Next: [Types and the type general](13-types.md)
