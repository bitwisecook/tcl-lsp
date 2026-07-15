# KCS: E003 — Why does the analyser say a command has too many arguments?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why do I see a red squiggle saying a command was called with too many arguments?

## Why

Passing more arguments than a command accepts will raise a runtime error. The extra words are never silently ignored, so the script will fail.

This check is not limited to builtin commands: it also applies to same-file `proc` calls, `interp alias` targets (shifted by any prepended arguments), `rename`d commands (which keep the original's arity — and, if the old name is later re-declared as a fresh `proc`, the *new* declaration's own arity, not the original's), TclOO methods and `forward`s (including `forward NAME my TARGET ?ARG…?`, the idiom for forwarding to a sibling or inherited method), TclOO constructor calls (`ClassName new ?args?` / `ClassName create name ?args?` / `ClassName createWithNamespace name ::ns ?args?`, checked against the nearest explicit `constructor` in the class's inheritance chain), `next`/`nextto` calls inside a method body (checked against the resolved next-in-MRO method or `nextto`'s named target — see the [E002 page](kcs-diagnostic-e002-too-few-arguments.md#tcloo-next--nextto-context)), a Tk/ttk widget's own instance command when the receiver traces back to its creating constructor (see the [E002 page](kcs-diagnostic-e002-too-few-arguments.md#tk-widget-instance-dispatch-context); `configure`/`cget` are never arity-checked), and direct calls to an inline `apply {{params} body} ?args?` lambda.

## Symptoms

- A red squiggle appears under the extra arguments, with a message like "Too
  many arguments for 'incr': expected at most 2, got 3 — usage: incr varName
  ?increment?".
- The " — usage: …" tail quotes the command's synopsis when the analyser has
  a registry signature for the command; calls to your own `proc`s, TclOO
  methods, and `apply` lambdas keep the count-only message — see the note on
  the [E002 page](kcs-diagnostic-e002-too-few-arguments.md#symptoms).
- The same squiggle can appear on a call to a `proc` you defined earlier in the file, an `interp alias`, a `rename`d command, or a `$obj method` call — not just builtin commands.

## Example that triggers it

```tcl
incr x 1 2
```

The analyser reports **`E003`** on the surplus argument `2`.

```tcl
proc greet {name} {
    return "hello $name"
}
greet Alice Bob
```

The analyser reports **`E003`** on the call, since `greet` only accepts one argument.

## Command-prefix callback context

`E003` also fires on a **callback proc that is too small for its command
prefix**. When a command invokes a callback (`lsort -command cb`, `trace add
… cb`, `$graph walk … -command cb`), it appends a fixed number of arguments; if
the referenced proc accepts fewer, the runtime call raises "too many
arguments". Here the squiggle is under the **callback proc name** (the head of
the prefix), not under a surplus word at the call site — look at the proc it
names, not the calling command's own argument list.

```tcl
proc oneArg {a} { return 0 }
lsort -command oneArg {3 1 2}   ;# lsort appends 2 → E003 on `oneArg`
```

Fix by widening the callback's parameter list (here to `{a b}`) or giving it a
variadic tail (`{args}`) so it absorbs every appended argument.

**This specific check requires cross-file diagnostics to be enabled** (the
`crossFileResolution` setting — off by default) — see the identical note on
the [E002 page](kcs-diagnostic-e002-too-few-arguments.md#command-prefix-callback-context).
Every other E003 case on this page fires unconditionally.

## Fix

```tcl
incr x 1
```

Remove the surplus arguments so the call matches the command's signature.

## How to suppress

Add `# noqa: E003` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E001`, `E002`, `E005` (wrong argument-count *shape* — an
  in-range count that doesn't fit a key/value-pair or paired-argument
  command like `dict create`/`foreach`/`switch`)
