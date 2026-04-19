# KCS: E001 — Why does the analyser flag a bare command with no subcommand?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why do I see a red squiggle on a command like `string` with no subcommand?

## Why

Commands such as `string`, `array`, and `dict` require a subcommand to do anything useful. A bare invocation is always an error at runtime and will raise a Tcl exception.

## Symptoms

- A red squiggle appears under the bare command, with the message "missing subcommand for 'string'".

## Example that triggers it

```tcl
string
```

The analyser reports **`E001`** on the bare `string` token.

## Fix

```tcl
string length $x
```

Provide the required subcommand so the command knows which operation to perform.

## How to suppress

Add `# noqa: E001` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E002`, `E003`
