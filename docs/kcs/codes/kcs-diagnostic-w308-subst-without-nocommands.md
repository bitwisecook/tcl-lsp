# KCS: W308 — Does subst without -nocommands allow code execution?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when `subst` is called without `-nocommands`?

## Why

Unprotected `[cmd]` sequences in the input execute as Tcl code, allowing an attacker to run arbitrary commands.

## Symptoms

- A blue squiggle (hint) appears under the `subst` call, with the message "subst without -nocommands".

## Example that triggers it

```tcl
subst $template
```

The analyser reports **`W308`** on the `subst` call.

## Fix

```tcl
subst -nocommands $template
```

Add `-nocommands` to prevent embedded command substitution.

## How to suppress

Add `# noqa: W308` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W102`, `W309`
