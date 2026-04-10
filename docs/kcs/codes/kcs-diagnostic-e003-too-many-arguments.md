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

## Symptoms

- A red squiggle appears under the extra arguments, with the message "too many arguments for 'incr'".

## Example that triggers it

```tcl
incr x 1 2
```

The analyser reports **`E003`** on the surplus argument `2`.

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
- Related codes: `E001`, `E002`
