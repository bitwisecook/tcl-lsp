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

## How to suppress

Add `# noqa: W210` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W211`, `W213`, `W220`
