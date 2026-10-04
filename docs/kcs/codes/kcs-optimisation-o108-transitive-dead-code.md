# KCS: O108 — Eliminate transitively dead code

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, dce

## Profiles

full

## Question

What does O108 rewrite, and when does it fire?

## Why

A chain of assignments that feeds only dead code is itself dead; removing the entire chain keeps the source clean and avoids wasted computation.

## Before

```tcl
proc report {x} {
    set a 1
    set b [expr {$a + 1}]
    return $x
}
```

## After

```tcl
proc report {x} {
    return $x
}
```

`b` feeds nothing, so `a` feeds nothing either and the whole chain goes.

## Safety conditions

- Skipped when any statement in the chain has observable side effects.
- Skipped when evaluating the right-hand side could raise an error, because deleting the statement would drop the error. A value that reads a variable is kept unless every variable it reads is certainly set to a scalar where it is read (a parameter, a variable assigned on every path, or the target of a command that always writes it, such as `gets` or `lassign`), and an `expr` value, or a value with a command substitution, is kept unless it folds to a constant: `set x [lindex {a b} 1.5]` raises whatever it reads, and so does a call to a procedure, however pure (`set unused [add x 1]` raises where `add` adds its arguments). A store the analysis proves raises inside a `catch` or `try` body always stays, though nothing reads it: the raise is its effect, and the body stops there, so in `catch {set x [expr {1/0}]; set y 2}` the `set y 2` never runs. An `incr` is kept unless its amount is a literal integer and its variable holds an integer wherever it is set; under a dialect that includes Tcl 8.4, where `incr` of an unset variable raises, the variable must also be set where the `incr` reads it. So `set y $x` with `x` unset, `set y [expr {$v + 1}]` with `v` a parameter, `set y [expr {1/0}]`, and, under 8.4, an `incr n` that may find `n` unset all stay.
- Skipped when a variable in the chain is read by live code elsewhere.
- Skipped when an existence check or an unset observes the stored value — `info exists v`, `array exists v`, `unset v` or `array unset v`, whether it is a command of its own, a condition, or a `[…]` substitution inside another command — because removing the store changes the check's answer or makes the `unset` fail.
- Skipped at the top level of a file, where another file or an interactive session can still read the variables.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [DCE](../../GLOSSARY.md#dce)
- Related codes: `O107`, `O109`
