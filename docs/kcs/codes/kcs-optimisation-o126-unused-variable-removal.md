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
- Skipped when evaluating the right-hand side could raise an error, because deleting the statement would drop the error. A value that reads a variable is kept unless every variable it reads is certainly set to a scalar where it is read (a parameter, a variable assigned on every path, or the target of a command that always writes it, such as `gets` or `lassign`), and an `expr` value, or a value with a command substitution, is kept unless it folds to a constant: `set x [lindex {a b} 1.5]` raises whatever it reads, and so does a call to a procedure, however pure (`set unused [add x 1]` raises where `add` adds its arguments), unless the procedure provably completes whatever its arguments hold: its body runs straight through, reads only its parameters and variables it set, and runs only commands that complete whatever they are given, such as `string length` or another such procedure, with no recursion; every word of its body is one the release's parser accepts (`"a"b`, `{a}b`, and `{*}$x` under Tcl 8.4, where `{*}` is a braced word, are compile errors when the body first runs); and its `proc` statement, and those of the procedures it calls, surely ran before the call: each is a direct statement of the top level, or of a `namespace eval` that is one — not under `if`, `catch` or a loop, not inside another procedure, not in another file — and comes before the first place the script may run the caller, or the caller runs only as a callback, after the script; so with `proc len {x} {return [string length $x]}` an unused `set n [len abc]` goes. A command that a `# tcl-lsp: stub` declares completes only as far as the declaration says, and a stub's `-pure` says the command changes nothing, not that it completes, so a call to it is kept. A store the analysis proves raises inside a `catch` or `try` body always stays, though nothing reads it: the raise is its effect, and the body stops there, so in `catch {set x [expr {1/0}]; set y 2}` the `set y 2` never runs. An `incr` is kept unless its amount is a literal integer and its variable holds an integer wherever it is set; under a dialect that includes Tcl 8.4, where `incr` of an unset variable raises, the variable must also be set where the `incr` reads it. So `set y $x` with `x` unset, `set y [expr {$v + 1}]` with `v` a parameter, `set y [expr {1/0}]`, and, under 8.4, an `incr n` that may find `n` unset all stay.
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
