# KCS: IRULE1004 — Why does the analyser hint about a missing event priority?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser show a hint that a `when` block is missing an explicit priority?

## Why

Without a priority, execution order across multiple iRules bound to the same virtual server is unpredictable. Explicit priorities make the ordering deterministic.

## Symptoms

- A blue squiggle (hint severity) appears on the `when` keyword, with the message "when block missing explicit priority".

## Example that triggers it

```tcl
when HTTP_REQUEST { pool main }
```

The analyser reports **`IRULE1004`** because no `priority` clause is specified.

## Fix

Add an explicit priority value:

```tcl
when HTTP_REQUEST priority 500 { pool main }
```

## How to suppress

Add `# noqa: IRULE1004` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1001`, `IRULE1002`
