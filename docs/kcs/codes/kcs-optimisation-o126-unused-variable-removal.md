# KCS: O126 — Remove set statements for variables never read

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, dce

## Profiles

full

## Question

What does O126 rewrite, and when does it fire?

## Why

A variable that is set but never read is dead code; removing it simplifies the script and eliminates a wasted assignment.

## Before

```tcl
proc handle {} {
    set unused 42
    puts done
}
```

## After

```tcl
proc handle {} {
    puts done
}
```

## Safety conditions

- Skipped when the `set` command's right-hand side has [side effects](../../GLOSSARY.md#side-effects) that must be preserved.
- Skipped when evaluating the right-hand side could raise an error, because deleting the statement would drop the error. A value that reads a variable is kept unless every variable it reads is definitely set here (a parameter, or assigned on every path), and an `expr` or `incr` value is kept unless it folds to a constant. So `set y $x` with `x` unset, `set y [expr {$v + 1}]` with `v` a parameter, and `set y [expr {1/0}]` all stay.
- Skipped when the variable has a [trace](../../GLOSSARY.md#trace) attached.
- Skipped when the variable could be read via `upvar`, `uplevel`, or other dynamic access.
- Skipped at the top level of a file, where another file or an interactive session can still read the variable.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Dead-code elimination](../../GLOSSARY.md#dce)
- Related codes: `O124`, `O125`, `O127`
