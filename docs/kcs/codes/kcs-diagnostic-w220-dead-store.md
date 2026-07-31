# KCS: W220 — Why does the analyser warn about a dead store?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, dce, dataflow

## Profiles

default

## Question

Why does the analyser flag a variable that is set but overwritten before being read?

## Why

The first assignment is wasted work; the value is thrown away before anything reads it.

## Symptoms

- A yellow squiggle appears under the first assignment, with the message "variable set but overwritten before being read".

## Example that triggers it

```tcl
set x 1
set x 2
puts $x
```

The analyser reports **`W220`** on `set x 1` because the value `1` is never read.

Here `x` *is* read later (by `puts $x`), so only the first, overwritten
assignment is a dead store. When a variable is never read at all — a single
`set x 1` with no later use — the analyser reports the more informative
[`W211`](kcs-diagnostic-w211-variable-set-not-used.md) ("set but never used")
on that assignment instead, and suppresses the co-located `W220` so the line
carries a single, clearer hint rather than two.

## Fix

```tcl
set x 2
puts $x
```

Remove the redundant first assignment.

## Computed variable names silence the check

Tcl can compute a variable's *name* at run time, and a read through a computed
name can reach any local at all:

```tcl
proc dump {} {
    set alpha 10
    set beta 20
    foreach v [info locals] {
        puts [set $v]      ;# reads alpha and beta, with no `$alpha` anywhere
    }
}
```

`[set $v]` — and the same idiom spelled `[subst $[subst $v]]`, or a `subst`
over a template held in a variable — names its target from run-time data, so
no assignment in the proc can still be proved unread. Once one appears,
`W220` and [`W211`](kcs-diagnostic-w211-variable-set-not-used.md) both go
silent for the whole proc, and the optimiser stops removing stores there
([`O109`](kcs-optimisation-o109-dead-store.md) /
[`O126`](kcs-optimisation-o126-unused-variable-removal.md)).
Removing a store the analyser cannot see the reader for would change what the
program prints, so the analysis abstains instead.

## A brace-quoted name is a *literal* name, not a computed one

`{$n}` is not a computed name — braces suppress substitution, so `set {$n} 1`
creates a variable literally called `$n`, unrelated to `n`:

```tcl
set {$n} v
info exists {$n}      ;# 1
info exists n         ;# 0  — a different variable entirely
```

Such a name is fully static, so it does **not** silence the check the way a
computed `[set $v]` does: the assignment is analysed like any other, and
`W220` / [`W211`](kcs-diagnostic-w211-variable-set-not-used.md) name the cell
by its real spelling (`Assignment to '$n' is never read`). Reads of it —
`[set {$n}]` and `${$n}` alike — count as reads of that cell and of no other,
so an unrelated `set n …` nearby is judged entirely on its own merits.

## A procedure you call can read your variables

A procedure can run a whole script in its **caller's** frame, so a store the
calling code never appears to read may well be read there:

```tcl
proc runner {script} { uplevel 1 $script }
proc host {expr} {
    set threshold 10       ;# not a dead store — `$script` may read it
    runner $expr
}
```

When the script is not readable — `uplevel 1 $script`, or a computed `upvar`
target — the callee could read *any* of your variables, so `W220` and
[`W211`](kcs-diagnostic-w211-variable-set-not-used.md) both go silent for the
whole calling procedure and the optimiser stops removing stores there. A
brace-quoted script (`runner {puts $threshold}`) is ordinary source text the
analyser reads directly, so it does not silence anything.

The frame the script runs in decides whose variables are at risk. Inside
`runner` itself, `uplevel 1 $script` runs one frame *up*, so `runner`'s own
locals stay provable — a `set` in `runner` that nothing in `runner` reads is
still a real dead store. `eval $script` is the other way round: it runs where
it is written, so it protects that procedure's own locals instead.

## How to suppress

Add `# noqa: W220` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [DCE](../../GLOSSARY.md#dce)
- Related codes: `W210`, `W211`
