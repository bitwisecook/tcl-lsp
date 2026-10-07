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

A procedure that hands one of its own variables to another procedure, which
changes it through `upvar`, changes nothing its caller can see, so it still
folds. The called procedure runs from the value the variable holds, under
the call's constant arguments too:

```tcl
proc bump {name} {upvar 1 $name v; incr v}
proc two {} {set n 1; bump n; return $n}
proc step {x} {set n $x; bump n; return $n}
puts [two]        ;# becomes puts 2
puts [step 5]     ;# becomes puts 6
```

A recursive procedure folds where the call's constant arguments end the
recursion. The analyser follows at most 32 nested calls, and runs at most
4096 calls for one file; a deeper or longer recursion is left as written:

```tcl
proc fact {n} {if {$n <= 1} {return 1}; expr {$n * [fact [expr {$n - 1}]]}}
puts [fact 5]     ;# becomes puts 120
puts [fact 40]    ;# left as written: 40 nested calls
```

## Safety conditions

- Skipped when the proc has observable side effects.
- Skipped when any argument is not a compile-time constant, or the call passes
  a number of arguments the proc's parameters do not accept: such a call can
  raise, or change a variable, before the proc runs.
- Skipped when the proc body cannot be summarised by [interprocedural analysis](../../GLOSSARY.md#ipa).
- Skipped when a `return` runs inside a command the analyser keeps whole — an
  arm of `switch -glob` or `switch -regexp` whose subject it cannot read, or
  the body of a `foreach` or `lmap` over a namespace-qualified variable such
  as `::x` — since the value that `return` gives is not read. A subject it
  can read decides the `switch`, and the arm's `return` then folds like any
  other.
- Skipped when the analyser proves the procedure reaches none of its
  `return`s, as after a loop that ends only by raising.
- Skipped when the analyser proves a command of the procedure raises —
  `incr` of a value that is not an integer, `expr {1/0}`, `error` — or a
  call it makes to another procedure of the file raises under that call's
  arguments, or passes it a number of arguments its parameters do not
  accept: the call never reaches its `return`. A procedure whose call to
  one that writes its variables the analyser cannot decide completes, such
  as `bump n` after `set n $x`, has no value every caller shares; a call to
  it with constant arguments is still run with them, and folds where it
  completes.
- Skipped when the proc's bare name is anywhere `rename`d over, `rename`d away, or shadowed by an `interp alias` — the call site can no longer be trusted to run that proc's body.
- A proc with no explicit `return` still folds when it falls through: the value is whatever Tcl's "result of the last command executed" rule would leave (the `double` example above relies on exactly this).

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [IPA](../../GLOSSARY.md#ipa)
- Related codes: `O100`, `O102`
