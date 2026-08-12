# KCS: W121 — Why does the analyser flag a non-contiguous subnet mask?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about non-contiguous bits in a subnet mask?

## Why

A valid subnet mask must consist of contiguous 1-bits followed by contiguous 0-bits. Non-contiguous masks (e.g. `255.0.255.0`) are rejected by most network stacks and almost always indicate a typo.

## Symptoms

- A yellow squiggle appears under the mask literal, with the message "non-contiguous subnet mask bits".

## Example that triggers it

```tcl
set mask "255.0.255.0"
```

The analyser reports **`W121`** on the subnet mask literal.

## Fix

```tcl
set mask "255.255.255.0"
```

Use a valid contiguous subnet mask.

## How to suppress

Add `# noqa: W121` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W124`
