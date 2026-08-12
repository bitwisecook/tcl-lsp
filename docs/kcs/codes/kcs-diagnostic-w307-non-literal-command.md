# KCS: W307 — Can a non-literal command name execute anything?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when a command name is built from a variable or substitution?

## Why

A command name built from a variable or command substitution cannot be statically verified and may execute anything, including attacker-controlled code.

## Symptoms

- A yellow squiggle appears under the command invocation, with the message "non-literal command name".

## Example that triggers it

```tcl
$computed_name $arg
```

The analyser reports **`W307`** on the command invocation.

## Fix

```tcl
switch $action {
    run  { run_cmd $arg }
    stop { stop_cmd $arg }
}
```

Use a literal command name or a validated dispatch table instead.

## When it does not fire

The warning is an abstention, not a verdict, so it stays silent whenever the analyser can actually resolve the dispatch. In particular, a variable the object-type lattice proves holds a TclOO object does not warn — including a handle returned by a method and captured into a variable:

```tcl
oo::class create A { method make {} { return [B new] } }
oo::class create B { method greet {} { return "hi" } }
set a [A new]
set b [$a make]
$b greet   ;# no W307 — `b` is provably a ::B, so the method is validated instead
```

An unknown method on such a handle reports `W308`, matching what hover and go-to-definition say about the same receiver.

## How to suppress

Add `# noqa: W307` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W101`, `W123`
