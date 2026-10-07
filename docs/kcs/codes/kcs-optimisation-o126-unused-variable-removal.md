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
- Skipped when evaluating the right-hand side could raise an error, because deleting the statement would drop the error. A value that reads a variable is kept unless every variable it reads is certainly set to a scalar where it is read (a parameter, a variable assigned on every path, or the target of a command that always writes it, such as `gets` or `lassign`), and an `expr` value, or a value with a command substitution, is kept unless it folds to a constant: `set x [lindex {a b} 1.5]` raises whatever it reads, and so does a call to a procedure, however pure (`set unused [add x 1]` raises where `add` adds its arguments). A store the analysis proves raises inside a `catch` or `try` body always stays, though nothing reads it: the raise is its effect, and the body stops there, so in `catch {set x [expr {1/0}]; set y 2}` the `set y 2` never runs. An `incr` is kept unless its amount is a literal integer and its variable holds an integer wherever it is set; under a dialect that includes Tcl 8.4, where `incr` of an unset variable raises, the variable must also be set where the `incr` reads it. So `set y $x` with `x` unset, `set y [expr {$v + 1}]` with `v` a parameter, `set y [expr {1/0}]`, and, under 8.4, an `incr n` that may find `n` unset all stay.
- Skipped when the variable has a [trace](../../GLOSSARY.md#trace) attached.
- Skipped when the variable could be read via `upvar`, `uplevel`, or other dynamic access — among them a call to a procedure of the file that links the variable it is handed, or whose parameter's default names it where the call leaves that argument out (`proc bumpd {{name n}} {upvar 1 $name v; incr v}` called as `bumpd` reads `n`).
- Skipped when an existence check or an unset observes the stored value — `info exists v`, `array exists v`, `unset v` or `array unset v`, whether it is a command of its own, a condition, or a `[…]` substitution inside another command — because removing the store changes the check's answer or makes the `unset` fail.
- Skipped at the top level of a file, where another file or an interactive session can still read the variable.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Dead-code elimination](../../GLOSSARY.md#dce)
- Related codes: `O124`, `O125`, `O127`
