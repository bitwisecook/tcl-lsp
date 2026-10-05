# KCS: W241 — Why does the analyser warn that my loop is provably infinite?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default

## Question

Why does the analyser flag a `while` or `for` as provably infinite?

## Why

The analyser can see enough of the loop shape to prove it will never
terminate. It reports **`W241`** in these cases:

- A `while {1}` / `while {true}` whose body never leaves the loop —
  no `break`, and nothing that terminates the enclosing block or frame
  (`return` / `error` / `exit` / `throw` / `tailcall`). A `continue`
  does *not* count: it restarts the loop, so the loop is still infinite.
- A loop whose condition the analyser proves true at every test, though it is
  not a literal — `set go 1; while {$go} {...}`, or a `for` whose counter the
  step never changes (`for {set i 0} {$i < 10} {} {...}`) — and which no path
  leaves. An exit the analyser can find, in the flow graph or in the text of
  the body (a `break` inside a `catch` body, say), keeps the loop from being
  reported, however the word that holds it is written: `if {$i < 0} break`
  and `if {$i < 0} "break"` leave the loop as `if {$i < 0} {break}` does.
- A loop whose counter cannot reach its bound. The condition compares the
  counter with a literal (`$v OP N`), and every pass adds the same literal
  step: a `for`'s step script `incr v STEP`, or the one `incr v STEP` at the
  top level of a `while` body. The counter starts at the value the analyser
  proves it holds when the loop is reached, after a `for`'s start script —
  `for {set i 5} …`, `set start 5; for {set i $start} …` and `set i 5;
  while {$i < 10} {incr i -1}` all start at 5:
  - `incr v 0` — the counter never changes.
  - The counter moves *away* from the bound (`$v < N` with a
    negative step, `$v > N` with a positive step).
  - The counter skips the bound (`$v != N` with a step whose
    direction or magnitude never lands exactly on `N`).

The step must be the only write to the counter in the loop. The analyser
reads what writes it from the dataflow it builds for the code around the
loop, not from the loop's text, so any other write there leaves the loop
undecided, in a braced, quoted or bare body word alike: `set v …`, a nested
`incr v`, `lset v …`, a second `incr v` in a `while` body, an `[incr v]` or
`[set v …]` inside another command's word, a `foreach`, `lmap` or `dict for`
that binds `v`, a `switch` arm that sets it, and a call to a procedure the
file defines that sets the caller's `v` with `upvar` or `uplevel`. So does a
write the analyser cannot place, as below. A loop whose code the analyser
does not follow, such as one inside a `catch` body, is reported only when its
condition is a literal true with no exit: `while {1} {…}`.

Each proof but the literal one is about the value the condition's variable
holds, so a write the analyser cannot place leaves the loop undecided and
draws no `W241`: a variable an arm of a `switch` in the loop sets (`-glob`,
`-regexp`, `-nocase`, a fall-through arm, `case`, directly or through a
procedure that sets the caller's variable with `upvar`), one a callback script
stored anywhere in the file sets (`after`, `fileevent`, `bind`, a variable
trace's callback, a procedure named as a callback, a command prefix built with
`list`, a script spelled as several words), one a callback the analyser cannot
read may set (`after 100 $script`, a command the file does not define), and
one a call to a command the file does not define, a call whose command is
computed and a call inside a `catch` body may set. At the top level `go` is
the global `$::go`, which such a call may set; in a procedure the call may set
the local `go` too, through `upvar 1` — a procedure an autoloader or the
unknown handler brings in can do so on every release — so `while {$go} { foo
}` is not reported there either, nor is a loop around a `source`, which runs
its file in the procedure's frame. A procedure the file defines is read for
what it writes, so a loop around a call to one that leaves `go`, or the
counter, alone is still reported.

## Example that triggers it

Each loop is in a procedure of its own: the code after a loop that never ends
never runs, and a loop there is not reported.

```tcl
proc forever {} {
    while {1} {
        puts "forever"
    }
}

proc step_is_zero {} {
    for {set i 0} {$i < 10} {incr i 0} {
        puts "step is zero"
    }
}

proc wrong_direction {} {
    for {set i 0} {$i < 10} {incr i -1} {
        puts "wrong direction"
    }
}

proc skips_ten {} {
    for {set i 0} {$i != 10} {incr i 3} {
        puts "skips 10"
    }
}

proc counts_away {} {
    set start 5
    set i $start
    while {$i < 10} {
        incr i -1
    }
}

proc still_forever {} {
    set go 1
    while {$go} {
        puts "still forever"
    }
}
```

## Fix

Add a `break` / `return` path, fix the step direction, or use `<` /
`<=` in place of `!=` so the loop ends cleanly.

```tcl
for {set i 0} {$i < 10} {incr i} {
    puts $i
}
```

## How to suppress

Add `# noqa: W241` on the line **above** the offending command. (Genuine
event-loop servers that really do loop forever are the intended
suppression target.)

## Related

- [KCS codes index](README.md)
- [W240 — loop condition is constant false](kcs-diagnostic-w240-loop-constant-false.md)
- [W242 — loop termination not provable](kcs-diagnostic-w242-loop-termination-unprovable.md)
- `IRULE5003` — iRules loop condition `!= 0` with decrement
