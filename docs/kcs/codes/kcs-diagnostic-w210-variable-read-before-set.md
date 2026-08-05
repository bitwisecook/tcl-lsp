# KCS: W210 — Why does the analyser warn about a variable used before being set?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, liveness, dataflow

## Profiles

default

## Question

Why does the analyser flag a variable that is read before it has been assigned a value?

## Why

Reading an undefined variable causes a runtime error and stops the script.

## Symptoms

- A yellow squiggle appears under the variable reference, with the message "variable used before being set".

## Example that triggers it

```tcl
puts $x
```

The analyser reports **`W210`** because `x` is never set before it is read.

## Fix

```tcl
set x ""
puts $x
```

Assign the variable before using it.

## Existence checks are not reads

`[info exists X]` tests whether `X` exists, and `[array exists X]` tests whether
`X` exists *as an array variable* (a scalar `set X 1` makes `info exists` true
but leaves `array exists` false). Both *test* rather than read the value, so
neither raises `W210`. A check also informs the branches it guards: inside
`if {[info exists X]} { … }` the variable is known to exist, so reading `$X`
there is safe; on the `else` side it is still unset, so a read there is still
flagged. When existence is statically provable, the check is folded to a
constant and reported as
[`I230`](kcs-diagnostic-i230-constant-existence-check.md) instead — though a
scalar assignment only proves `info exists`, never `array exists`.

The same branch narrowing applies to the `info vars` / `info locals` membership
idioms for a single exact name: `[info vars X] ne ""`,
`[llength [info vars X]]`, and `[lsearch [info vars] X] > -1`; and to
`catch {set _ $X}`, whose no-error (false) branch proves `$X` was readable.
(`info globals` is not used — it proves the *global* exists, not the bare-`$X`
local — and glob patterns are not statically decidable.)

## Braces are not substitution

A word wrapped in braces is passed through exactly as written — Tcl performs
no `$` substitution inside it — so the names it mentions are not read there:

```tcl
puts {$y}              ;# prints the two characters $y; not flagged
puts {set y 1; puts $y}  ;# prints the line; not flagged
mydefiner ::a::b {optlist} {set y 1; return $y}   ;# not flagged
```

That holds for any command the analyser knows: `puts`, `string match`,
`lsort -command`, and every other command in its registry say what each
argument is, and none of them substitutes a braced one.

Two shapes are the exception, because the command really does evaluate the
braced word against *your* variables: an expression (`expr {$a + $b}`,
`if {$a} …`) and a body that shares your frame (`if`, `while`, `foreach`,
`catch`). Those are read normally and still flagged.

A command the analyser does **not** know — one of your own procedures, or a
definer from a library it has no description of — is the middle case. Its
braced argument might be a script, and if the procedure passes it on to
something that runs it with `uplevel`, it runs in your frame:

```tcl
proc wrapper {script} { real_worker $script }   ;# real_worker uplevels it
wrapper { puts $myf }                            ;# flagged: myf is never set
```

so a read there is still reported. The exception is a name the braced word
sets itself — that is the script's own variable whichever frame it ends up
in, and it is not flagged:

```tcl
mydefiner ::a::b {optlist} { set y 1; return $y }   ;# not flagged
```

A braced mention does still count as *use* for
[`W211`](kcs-diagnostic-w211-variable-set-not-used.md) and
[`W220`](kcs-diagnostic-w220-dead-store.md): the text may be evaluated later,
so the assignment feeding it is not reported dead.

## Variables set inside a loop

A variable assigned inside a loop body and read *after* the loop is **not**
flagged, as long as the body sets it on every iteration:

```tcl
foreach item $items {
    lappend result $item
}
puts $result        ;# not flagged — assumed the loop ran at least once
```

The analyser assumes a loop that *might* run does run, matching how the code
behaves on real (non-empty) data. Two cases still fire, because they are
genuine errors:

- A **provably empty** loop never runs its body, so the variable is definitely
  unset: `foreach x {} { set y $x }; puts $y`, or a `while 0` / a `for` whose
  condition is false on entry.
- A read **inside** the loop body, *before* the body's own assignment, is a
  first-iteration read-before-set: `foreach x $items { puts $y; set y $x }`.

A body that assigns the variable only under an inner condition
(`foreach x $items { if {$x} { set y 1 } }; puts $y`) is also still flagged —
the variable can be unset even when the loop runs.

## Commands that set a variable inside a condition

A command that writes a variable named by one of its own arguments does so
before either branch of the `if` (or the first turn of the `while`) can run,
so the guarded body may read that variable safely:

```tcl
proc find {lst} {
    if {[set idx [lsearch $lst foo]] > -1} {
        puts $idx          ;# not flagged — the condition set it
    }
}
```

Which argument writes which variable comes from the command registry, so this
covers every command it knows: `set`, `incr`, `append`, `lappend`, `lset`,
`catch`, `gets`, `scan`, `regexp`, `regsub`, `lassign`, `binary scan`, and the
rest. `unset` is the exception — it removes a variable rather than creating
one, so a read after it is still flagged.

## Computed variable names silence the check

Tcl can compute a variable's *name* at run time:

```tcl
proc handle {name} {
    set $name 1        ;# sets whatever variable $name spells
    puts $foo           ;# not flagged — `$name` may have been "foo"
}
```

Once a proc contains a write like that, no local in it can still be proved
unset, so `W210` goes silent for the whole proc. That is deliberate: a warning
that cannot be proved is worse than a missing one. Spell the name out to get
the check back.

## A procedure you call can set your variables

Tcl lets a procedure write its **caller's** variables, so a variable nothing in
your own code assigns can still be set by the time you read it:

```tcl
proc setdef {dictVar key value} {
    upvar 1 $dictVar d       ;# `d` is an alias for the caller's variable
    dict set d $key $value
}
proc build {} {
    setdef options name blue ;# creates `options` in `build`
    return [dict get $options name]
}
```

The analyser works out, once per procedure, which of your variables a call to
it writes, and treats the call as the assignment. That covers `upvar` with a
literal name or a by-name parameter, `uplevel 1 {…}`, and
`uplevel 1 [list set …]`. It also follows one hop of `uplevel 1 [list helper
…]`, where `helper`'s own `upvar` reaches one frame further out than a plain
call would.

Which frame the write lands in matters, and only a write to *your* frame
counts. `upvar #0` writes a global, `upvar 0` writes the callee's own local,
and `upvar 2` writes your caller's caller — none of them is an assignment to
your variable.

When the name cannot be worked out — `uplevel 1 $script`, a computed
`upvar` target, or `argparse`, which names its variables from its own
definition list — the procedure could write *any* of your variables, so
`W210` goes silent for the whole calling procedure, exactly as it does for a
computed name.

Note the difference between `eval` and `uplevel`: `eval $script` runs in the
procedure that writes it, so it can set that procedure's own locals;
`uplevel 1 $script` runs one frame up, so it cannot.

## How to suppress

Add `# noqa: W210` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W211`, `W213`, `W220`
