# 6. Loops

Most of the programs in this chapter count things, so they begin with
`prefer integer` to make their variables whole numbers
([chapter 4](04-definitions.md)).

## while and until

`while b do C` obeys `C` again and again for as long as `b` is true. If
`b` is false to begin with, `C` is not obeyed at all.

```cpl
prefer integer
let n = 1
while n < 100 do n := 2n
Write[n, '*n']
```
```output
128
```

`until b do C` is the same with the condition the other way up:

```cpl
prefer integer
let n = 27
until n = 1 do
   $( n := Rem[n, 2] = 0 -> Quot[n, 2], 3n + 1
      Write[n, ' '] $)
Write['*n']
```
```output
82 41 124 62 31 94 47 142 71 214 107 322 161 484 242 121 364 182 91 274 137 412 206 103 310 155 466 233 700 350 175 526 263 790 395 1186 593 1780 890 445 1336 668 334 167 502 251 754 377 1132 566 283 850 425 1276 638 319 958 479 1438 719 2158 1079 3238 1619 4858 2429 7288 3644 1822 911 2734 1367 4102 2051 6154 3077 9232 4616 2308 1154 577 1732 866 433 1300 650 325 976 488 244 122 61 184 92 46 23 70 35 106 53 160 80 40 20 10 5 16 8 4 2 1 
```

## repeat

To test at the end instead, so that the command is obeyed at least once,
write the command first:

```cpl
prefer integer
let n = 1000
$( n := Quot[n, 2]
   Write[n, ' '] $) repeat until n < 10
Write['*n']
```
```output
500 250 125 62 31 15 7 
```

`C repeat while b` and `C repeat until b` are the two forms. `C repeat`
with no condition goes on for ever, or until something stops it from
inside. That something is usually `break`, which leaves the smallest loop
it is in:

```cpl
prefer integer
let n, s = 0, 0
$( n := n + 1
   s := s + n
   if s > 100 break $) repeat
Write['1 + 2 + ... + ', n, ' = ', s, '*n']
```
```output
1 + 2 + ... + 14 = 105
```

## for

`for` gives a variable each of a list of values in turn.

```cpl
for i = 1 to 5 do Write[i, ' ']
Write['*n']
```
```output
1 2 3 4 5 
```

The variable belongs to the loop. It is defined afresh for each cycle,
its type comes from the values it is given, and when the loop is over it
no longer exists:

```cpl
for i = 1 to 3 do Write[i, ' ']
Write[i]
```
```output
! line 2: the name i is not defined here
```

(The `i` in the last line could only be a name from outside the loop, and
there is none. The mistake is found when the program is compiled, so
nothing is printed at all.)

There are four ways to say what the values are.

**A list.** Any expressions, of any type:

```cpl
for x = 0, 1.7, 2.51, 10 do Write[x, ' ']
Write['*n']
for s = 'red', 'green', 'blue' do Write[s, ' ']
Write['*n']
```
```output
0 1.7 2.51 10 
red green blue 
```

**From one number to another**, in steps of one:

```cpl
prefer integer
let n = 4
for i = n - 2 to n + 2 do Write[i, ' ']
Write['*n']
```
```output
2 3 4 5 6 
```

**With a step.** `step a, b, c` starts at `a` and goes up by `b` towards
`c`:

```cpl
for i = step 0, 5, 30 do Write[i, ' ']
Write['*n']
for i = step 10, -1, 1 do Write[i, ' ']
Write['*n']
for x = step 0, 0.25, 1 do Write[x, ' ']
Write['*n']
```
```output
0 5 10 15 20 25 30 
10 9 8 7 6 5 4 3 2 1 
0.0 0.25 0.5 0.75 1.0 
```

**With dots**, as one would write it by hand. The first two values give
the step:

```cpl
for i = 1, 3, ..., 11 do Write[i, ' ']
Write['*n']
```
```output
1 3 5 7 9 11 
```

The forms can be mixed in one list:

```cpl
for i = 1, 2, 3, step 10, 10, 50, 99 do Write[i, ' ']
Write['*n']
```
```output
1 2 3 10 20 30 40 50 99 
```

## The list is fixed before the loop starts

"The expressions are evaluated once and for all before the cycle is
started; it is not possible for the cycle to change the increment or the
end condition."

```cpl
let n = 3
for i = 1 to n do
   $( n := 100
      Write[i, ' '] $)
Write['*n']
```
```output
1 2 3 
```

One consequence needs care. With `step a, b, c` the number of cycles is
worked out in advance as (c - a)/b, **rounded to the nearest whole
number**, plus one. That is very sensible for real numbers, where a step
of 0.1 might not land exactly on the end, but it means that the last value
can overshoot:

```cpl
for i = step 0, 2, 7 do Write[i, ' ']
Write['*n']
```
```output
0 2 4 6 8 
```

(7 - 0)/2 is 3.5, which rounds to 4, so there are five cycles and the last
is 8. When the step may not divide the distance, use `while`.

## Loops inside loops

```cpl
for i = 1 to 4 do
   $( for j = 1 to i do Write['**']
      Write['*n'] $)
```
```output
*
**
***
****
```

```cpl
for i = 1 to 5 do
   $( for j = 1 to 5 do Printd[i j, 3, 0]
      Write['*n'] $)
```
```output
   1   2   3   4   5
   2   4   6   8  10
   3   6   9  12  15
   4   8  12  16  20
   5  10  15  20  25
```

`break` leaves only the loop it is in:

```cpl
for i = 1 to 3 do
   $( for j = 1 to 10 do
         $( if j > i break
            Write[j, ' '] $)
      Write['*n'] $)
```
```output
1 
1 2 
1 2 3 
```

## Using a variable from outside

Because the loop variable disappears, its last value cannot be used
afterwards. If it is wanted, write `for ext` and the loop will use a
variable that already exists:

```cpl
prefer integer
let k = 0
for ext k = 1 to 100 do
   if k k > 200 break
Write['the first square over 200 is that of ', k, '*n']
```
```output
the first square over 200 is that of 15
```

Next: [Blocks and scope](07-blocks-and-scope.md)
