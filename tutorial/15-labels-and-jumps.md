# 15. Labels and jumps

## Labels

Any command can be given a label, which is a name followed by a colon, and
`go to` (or `goto`) transfers control to it:

```cpl
prefer integer
let n = 1
Loop: Write[n, ' ']
n := 2n
if n < 100 go to Loop
Write['*n']
```
```output
1 2 4 8 16 32 64 
```

After `if` or `unless` the `do` may be left out before `go to`.

The manual is frank about this: conditional commands and loops "remove to
some extent the need for explicit labels in a program (and should be
exploited)". But CPL's labels can do things that loops cannot.

## Label variables

A label is a data item. It can be put in a variable, and `go to` can be
given any expression whose value is a label:

```cpl
let x = -5
go to (x < 0 -> Neg, x = 0 -> Zero, Pos)

Neg:  Write['negative*n'];  go to Done
Zero: Write['zero*n'];      go to Done
Pos:  Write['positive*n']
Done: Write['done*n']
```
```output
negative
done
```

```cpl
prefer integer
let L be label
let n = 0

L := First
Again: n := n + 1
go to L

First:  Write['first ']
        L := Second
        go to Again
Second: Write['second ']
        L := Third
        go to Again
Third:  Write['third, after ', n, ' turns*n']
```
```output
first second third, after 3 turns
```

`L` is "a link which may be set to hold different command labels under
different conditions and may later be used in transfer commands". The
example `state-machine.cpl` uses this to recognise numbers.

## Labels as parameters: the way out

A label can be passed to a routine, and the routine can jump to it. This
is how CPL deals with things going wrong. The manual's equation solver,
complete this time, takes a label to go to "if there is no solution":

```cpl
routine LINEQ[real a, b, c, a', b', c', ref x, y, val label L] be
$( let DET = ab' - a'b
   if Mod[DET] < 1@-6 go to L
   x := (cb' - c'b)/DET
   y := (ac' - a'c)/DET $)

let V, W = 0, 0
LINEQ[1, 2, 3, 4, 5, 6, V, W, ERROR]
Write['V = ', V, ', W = ', W, '*n']
LINEQ[1, 2, 3, 2, 4, 7, V, W, ERROR]
Write['this is not printed*n']
finish

ERROR: Write['no solution*n']
```
```output
V = -1.0, W = 2.0
no solution
```

The jump leaves the routine and does not come back. `finish` ends the
program; without it the first part would run on into the commands labelled
`ERROR`.

Such a jump can leave any number of calls at once. Here a recursive
function is three calls deep when it finds the bad member:

```cpl
prefer general

let rec Sum[l, label Bad] =
        Null[l] -> 0,
        value of $( if Type[Hd[l]] = string go to Bad
                    result is Hd[l] $) + Sum[Tl[l], Bad]

Write[Sum[List[1, 2, 3], Oops], '*n']
Write[Sum[List[1, 2, 'three', 4], Oops], '*n']
finish

Oops: Write['that list has a string in it*n']
```
```output
6
that list has a string in it
```

Languages since have invented exceptions to do this. C has `setjmp` and
`longjmp`, which come from BCPL's `level` and `longjump`, which were
Richards' replacement for exactly this feature.

## A label remembers where it came from

The reference manual gives this as a "trivial example" of how ambiguities
arise with recursive routines (section 9.3.2). The routine calls itself
`n` times, and each call passes on its *own* label `N` in place of one of
the two it was given. "The effect of executing R [10, Even, Odd], say, is
to jump to the destination "Even", whereas R [9, Even, Odd] jumps to
the destination "Odd"."

```cpl
let rec routine R[integer n, label L, M] be
   $( if n > 0 do R[n - 1, M, N]
   N: goto L $)

let Try = 10
Again: R[Try, Even, Odd]

Even: Write[Try, ' is even*n'];  goto Next
Odd:  Write[Try, ' is odd*n']
Next: Try := Try - 1
      if Try >= 7 goto Again
```
```output
10.0 is even
9.0 is odd
8.0 is even
7.0 is odd
```

When the calls unwind, each `goto L` goes to the `N` of the call that made
it: not just to a place in the program, but to that place *in that
particular call*, with its variables as they were. A label value is a
position together with the activation it belongs to.

A label does not outlive its activation. Jumping to one whose routine has
already returned is an error:

```cpl
let Saved be label
let rt Keep[] be
   $( Saved := Here
      return
Here: Write['back inside*n'] $)

Keep[]
goto Saved
```
```output
! jump to the label Here, whose activation is no longer live
```

## The switch

The last addition to CPL, in 1966, was a command for choosing among many
cases. It is described on a single page at the back of the Working
Papers, "designed as an extension to the test-command":

```cpl
for n = 1 to 5 do
   $( switch on n into
      $( case 1: Write['one*n']
                 go to Out
         case 2: Write['two*n']
                 go to Out
         case 3:
         case 4: Write['three or four*n']
                 go to Out
         default: Write['many*n'] $)
 Out: $)
```
```output
one
two
three or four
three or four
many
```

A `case` is a kind of label. Control goes to the one that matches and
carries on from there, into the next case if nothing stops it: hence the
`go to` at the end of each. If that seems familiar, it should. BCPL took
this command from CPL and C took it from BCPL, falling through and all.
What the later languages added was a tidier way out. In the manual as it
stands `break` means only "leave the loop", so here it would have ended
the `for`. (The editor's note at the front says that this section "is now
largely wrong as we have altered the rules about break, return and result
is to allow a more general use", and that the change was too complicated
to write in. The new rules are not given anywhere, so the old ones are
followed here.) BCPL invented `endcase`, and C made `break` do both jobs.

The only large CPL program in those papers, a preprocessor of about 190
lines ([appendix1](../appendix1/README.md)), is one big switch on the character just read, and
every one of its cases ends by jumping back to the top.

Inside a function, `result is` leaves the switch along with everything
else:

```cpl
let Days[index m] = value of
   $( switch on m into
      $( case 2: result is 28
         case 4: case 6: case 9: case 11: result is 30
         default: result is 31 $) $)

for m = 1 to 12 do Write[Days[m], ' ']
Write['*n']
```
```output
31 28 31 30 31 30 31 31 30 31 30 31 
```

The cases may be strings as well as numbers:

```cpl
let Sound[string a] = value of
   $( switch on a into
      $( case 'cat': result is 'miaow'
         case 'dog': result is 'woof'
         default: result is '...' $) $)

Write[Sound['dog'], ' ', Sound['cat'], ' ', Sound['fish'], '*n']
```
```output
woof miaow ...
```

Next: [New kinds of variable](16-lh-functions.md)
