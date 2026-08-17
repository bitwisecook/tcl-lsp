# KCS: IRULE2001 — Why does the analyser flag `matchclass` as deprecated?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that `matchclass` is deprecated?

## Why

`matchclass` was removed after BIG-IP v10. It does not exist on current platforms and will raise a runtime error.

## Symptoms

- A squiggle appears on the `matchclass` command, with the message "deprecated matchclass".

## Example that triggers it

```tcl
matchclass $data $class
```

The analyser reports **`IRULE2001`** on the `matchclass` token.

## Fix

Use the modern `class match` command:

```tcl
class match -- $data equals $class
```

## How to suppress

Add `# noqa: IRULE2001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE2002`, `IRULE2003`
