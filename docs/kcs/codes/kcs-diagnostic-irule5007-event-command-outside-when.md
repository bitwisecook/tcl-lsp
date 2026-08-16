# KCS: IRULE5007 — Why does the analyser reject executable code at iRules top level?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag an executable command at the top level of an iRule?

## Why

iRules top level is a declaration surface. Only `when`, `proc`, `timing`, and `priority` are permitted there. Executable commands must be inside a `when` event body or a top-level `proc`; user procs must then be invoked with `call`.

## Symptoms

- A squiggle appears under an executable top-level command.

## Example that triggers it

```tcl
set uri [HTTP::uri]
```

The analyser reports **`IRULE5007`** because `set` is executable and appears at the declaration-only top level.

## Fix

Move executable code into an appropriate `when` block:

```tcl
when HTTP_REQUEST {
  set uri [HTTP::uri]
}
```

## How to suppress

Add `# noqa: IRULE5007` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5005`, `IRULE5006`
