# KCS: W307 — Can a non-literal command name execute anything?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn when a command name is built from a variable or substitution?

## Why

A command name built from a variable or command substitution cannot be statically verified and may execute anything, including attacker-controlled code.

## Symptoms

- A yellow squiggle appears under the command word, with the message *"Non-literal command name — cannot statically analyse"*.

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

The warning is an abstention, not a verdict, so it stays silent whenever the analyser can resolve the dispatch. A variable the object-type lattice proves holds a TclOO object does not warn:

```tcl
oo::class create B { method greet {} { return "hi" } }
set b [B new]
$b greet   ;# no W307 — `b` is provably a ::B, so the method is validated instead
```

An unknown method on such a handle reports `W308`, matching what hover and go-to-definition say about the same receiver. A handle the analyser cannot type — one returned by a method, for example — still warns.

## How to suppress

Add `# noqa: W307` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W101`, `W123`
