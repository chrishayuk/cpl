# 2. Hello, world

```cpl
Write['Hello, world!*n']
```
```output
Hello, world!
```

`Write` is a routine, and `Write[...]` is a call of it. The parameters of
a function or routine go in **square** brackets, always; round brackets
are only for grouping. A string is written between single quotes.

## Special characters

A star inside a string means that the next character is special. `*n` is a
new line, and `Write` does not start a new line unless it is told to:

```cpl
Write['one ']
Write['two ']
Write['three*n*nfour*n']
```
```output
one two three

four
```

| In a string | Means |
| --- | --- |
| `*n` | new line |
| `*s` | space |
| `*t` | tab |
| `*'` | a quote |
| `**` | a star |

```cpl
Write['It*'s 2 ** 3*n']
```
```output
It's 2 * 3
```

## Several things at once

`Write` takes any number of parameters, of any type, and writes them one
after another with nothing in between:

```cpl
Write['Two and two make ', 2 + 2, '.*n']
```
```output
Two and two make 4.
```

`Newline[]` and `Space[n]` do what they say:

```cpl
Write['a']
Space[5]
Write['b']
Newline[]
```
```output
a     b
```

## Commands and lines

A program is a sequence of commands. The end of a line ends a command, and
several commands can share a line if there are semicolons between them:

```cpl
Write['one*n']; Write['two*n']; Write['three*n']
```
```output
one
two
three
```

A command may run over several lines if it is clear that it has not
finished, because a bracket is still open or because the line ends with an
operator:

```cpl
Write['The sum is ',
      1 + 2 +
      3 + 4,
      '*n']
```
```output
The sum is 10
```

## Comments

Two vertical bars begin a comment, which runs to the end of the line:

```cpl
|| This program prints a greeting.
Write['Hello!*n']       || this is the greeting
```
```output
Hello!
```

## A limerick

Strings can be as long as you like, and a program can be nothing but
`Write` commands:

```cpl
Write['There was a young man of Japan*n']
Write['Whose limericks never would scan.*n']
Write['*tWhen asked why this was,*n']
Write['*tHe replied *'It*'s because*n']
Write['I always try to fit as many syllables into the last line as I possibly can.*'*n']
```
```output
There was a young man of Japan
Whose limericks never would scan.
	When asked why this was,
	He replied 'It's because
I always try to fit as many syllables into the last line as I possibly can.'
```

Next: [Numbers](03-numbers.md)
