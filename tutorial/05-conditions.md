# 5. Conditions

## True and false

A condition is an expression of type **Boolean**, whose value is `true` or
`false`.

```cpl
Write[2 < 3, ' ', 2 > 3, ' ', 2 = 2, ' ', 2 ~= 2, '*n']
```
```output
true false true false
```

| Relation | |
| --- | --- |
| `=` `~=` | equal, not equal |
| `<` `<=` `>` `>=` | less, less or equal, greater, greater or equal |
| `<<` `>>` | much less, much greater |

Relations can be strung together as they are in mathematics. `a < b <= c`
means that both `a < b` and `b <= c`:

```cpl
let x = 5
Write[0 < x < 10, ' ', 0 < x < 3, ' ', 1 < 2 = 2 < 3, '*n']
```
```output
true false true
```

`x << y` is for real numbers, and asks whether `x` is so small beside `y`
that adding it would make no difference. It is how a loop that is closing
in on an answer knows when to stop:

```cpl
Write[1 << 1000, ' ', 1 << 1@20, '*n']
```
```output
false true
```

## And, or, not

| Operator | Printed as | |
| --- | --- | --- |
| `~` | ∼ | not |
| `/\` | ∧ | and |
| `\/` | ∨ | or |
| `==` | ≡ | if and only if |
| `~==` | ≢ | exclusive or |

```cpl
let p, q = true, false
Write[p /\ q, ' ', p \/ q, ' ', ~p, ' ', p == q, ' ', p ~== q, '*n']
```
```output
false true false false true
```

A Boolean value can be kept in a variable, "to record the result of a test
for later or repeated use":

```cpl
let x, y = 3, 4
let b be Boolean
b := x < y
Write[b, '*n']
```
```output
true
```

## Conditional expressions

This is the form CPL is fondest of. `b -> E1, E2` is an expression whose
value is `E1` if `b` is true and `E2` if it is not:

```cpl
let x = -7
Write[(x < 0 -> -x, x), '*n']
```
```output
7.0
```

The second arm can be another conditional, and so on, which gives a list
of cases to be tried in order:

```cpl
let Grade[n] = n >= 70 -> 'first',
               n >= 50 -> 'second',
               n >= 40 -> 'third',
               'fail'
Write[Grade[82], ' ', Grade[55], ' ', Grade[12], '*n']
```
```output
first second fail
```

(`let Grade[n] = ...` defines a function; see [chapter 8](08-functions.md).)

Inside a larger expression a conditional needs round brackets:

```cpl
let x = 3
Write[10 + (x > 2 -> 1, 0), '*n']
```
```output
11
```

C still has this expression, spelt `b ? E1 : E2`.

## Conditional commands

`if b do C` obeys the command `C` if `b` is true, and `unless b do C` if
it is false. `then` may be written for `do`.

```cpl
let x = 5
if x > 3 do Write['big*n']
unless x > 3 do Write['small*n']
if x > 9 then Write['very big*n']
```
```output
big
```

To choose between two commands there is a different word, `test`, with
`or` before the second command:

```cpl
let x = 5
test x > 9 then Write['big*n'] or Write['small*n']
```
```output
small
```

CPL keeps `if` and `test` apart so that there is never any doubt which
`if` an `else` belongs to: a problem ALGOL had, and C still has. A chain
of tests reads well set out like this:

```cpl
let n = 15
test Rem[n, 15] = 0 then Write['FizzBuzz*n']
or test Rem[n, 5] = 0 then Write['Buzz*n']
or test Rem[n, 3] = 0 then Write['Fizz*n']
or Write[n, '*n']
```
```output
FizzBuzz
```

## More than one command

To make several commands into one, put them between the section brackets
`$(` and `$)`:

```cpl
let x, y = 7, 3
if x > y do
   $( Write['exchanging*n']
      x, y := y, x $)
Write[x, ' ', y, '*n']
```
```output
exchanging
3.0 7.0
```

Next: [Loops](06-loops.md)
