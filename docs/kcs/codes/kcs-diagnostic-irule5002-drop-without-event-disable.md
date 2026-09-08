# KCS: IRULE5002 — Why does the analyser warn about drop without event disable?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `drop`, `reject`, or `discard` without `event disable all` or `return`?

## Why

`drop` ends the connection but not the event. Other iRules and later priorities on the same virtual server keep running, and their commands can raise TCL errors on a connection that is already gone.

## Symptoms

- A yellow squiggle appears under the `drop`, with the message "'drop' without
  'event disable all' or 'return' — other iRules and later priorities in this
  event will still execute, which may cause TCL errors."

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
  drop
  event disable all
  return
}
```

Both words come *after* the `drop`: `event disable all` stops the remaining
iRules, `return` stops the rest of this one. The editor offers **Add 'event
disable all' + 'return'** as a code action.

## How to suppress

Add `# noqa: IRULE5002` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5001`, `IRULE5004`
