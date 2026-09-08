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
- Skipped when a variable in the chain is read by live code elsewhere.
- Skipped at the top level of a file, where another file or an interactive session can still read the variables.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [DCE](../../GLOSSARY.md#dce)
- Related codes: `O107`, `O109`
