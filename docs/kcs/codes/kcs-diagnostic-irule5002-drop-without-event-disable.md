# KCS: IRULE5002 — Why does the analyser warn about drop without event disable?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `drop`, `reject`, or `discard` without `event disable all` or `return`?

## Why

Other iRules on the same virtual server continue executing after `drop`, potentially sending a response on a dropped connection.

## Symptoms

- A squiggle appears under the `drop` call, with the message "drop without event disable or return".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  drop
}
```

The analyser reports **`IRULE5002`** because `drop` is not followed by `event disable all` or `return`.

## Fix

Disable further event processing, or add `return` after `drop`:

```tcl
when HTTP_REQUEST {
  event disable all
  drop
}
```

## How to suppress

Add `# noqa: IRULE5002` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5001`, `IRULE5004`
