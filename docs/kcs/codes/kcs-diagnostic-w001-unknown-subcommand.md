# KCS: W001 — Why does the analyser flag an unknown subcommand?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why do I see a warning on a subcommand that the analyser does not recognise?

## Why

An unrecognised subcommand usually means a typo or a version mismatch. At runtime, Tcl will raise an error because the ensemble or command does not support that subcommand.

## Symptoms

- A yellow squiggle appears under the subcommand token, with the message "unknown subcommand 'foo' for 'string'".

## Example that triggers it

```tcl
string mach $a $b
```

The analyser reports **`W001`** on the `mach` token.

## Fix

```tcl
string match $a $b
```

Correct the subcommand name to one the command actually supports.

## How to suppress

Add `# noqa: W001` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E001`, `W002`
