# KCS: O103 — Fold static procedure calls

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, ipa

## Profiles

standard, full

## Question

What does O103 rewrite, and when does it fire?

## Why

When all arguments to a pure proc are constant, the call can be replaced with its return value, removing the function-call overhead entirely.

## Before

```tcl
proc double {n} { expr {$n * 2} }
set x [double 21]
```

## After

```tcl
proc double {n} { expr {$n * 2} }
set x 42
```

A loop in the procedure does not stop the fold where the arguments bound it:
the analyser runs the procedure over the call's constant arguments, its loops
to their end, and reads the value each `return` gives where it returns.

```tcl
proc total {n} { set t 0; for {set i 0} {$i < $n} {incr i} {incr t 2}; return $t }
set r [total 3]   ;# becomes set r 6
```

A procedure whose return is computed, not written as a literal, folds when it
computes the same value for every caller: the analyser runs it with its
parameters unknown and reads what each `return` gives.

```tcl
proc prefix {} { set x [string range foobar 0 2]; return $x }
puts [prefix]     ;# becomes puts foo
```

The folded value is spelled exactly as the procedure returns it: `return 1.0`
folds to `1.0`, `return 007` to `007`, and `return " 5"` to `{ 5}`.

## Safety conditions

- Skipped when the proc has observable side effects.
- Skipped when any argument is not a compile-time constant, or the call passes
  a number of arguments the proc's parameters do not accept: such a call can
  raise, or change a variable, before the proc runs.
- Skipped when the proc body cannot be summarised by [interprocedural analysis](../../GLOSSARY.md#ipa).
- Skipped when a `return` runs inside a command the analyser keeps whole — an
  arm of `switch -glob` or `switch -regexp` whose subject it cannot read —
  since the value that `return` gives is not read. A subject it can read
  decides the `switch`, and the arm's `return` then folds like any other.
- Skipped when the analyser proves the procedure reaches none of its
  `return`s, as after a loop that ends only by raising.
- Skipped when the proc's bare name is anywhere `rename`d over, `rename`d away, or shadowed by an `interp alias` — the call site can no longer be trusted to run that proc's body.
- A proc with no explicit `return` still folds when it falls through: the value is whatever Tcl's "result of the last command executed" rule would leave (the `double` example above relies on exactly this).

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [IPA](../../GLOSSARY.md#ipa)
- Related codes: `O100`, `O102`
