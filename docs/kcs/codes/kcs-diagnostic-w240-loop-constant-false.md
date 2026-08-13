# KCS: W240 — Why does the analyser warn that my loop never executes?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default

## Question

Why does the analyser flag a `while` or `for` whose condition is a
constant-false expression?

## Why

A loop whose condition is a literal `0`, `false`, `no`, or `off` is
never entered. The body is effectively dead code and is almost always
the result of a typo, a left-over debugging tweak, or a forgotten
comparison.

## Example that triggers it

```tcl
while {0} { puts "never runs" }
for {set i 0} {false} {incr i} { puts hi }
```

The analyser reports **`W240`** on the condition.

## Fix

Replace the constant with the intended predicate, or delete the loop
if it is truly unused.

```tcl
while {$running} { puts "still going" }
```

## How to suppress

Add `# noqa: W240` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [W241 — provably infinite loop](kcs-diagnostic-w241-loop-provably-infinite.md)
- [W242 — loop termination not provable](kcs-diagnostic-w242-loop-termination-unprovable.md)
