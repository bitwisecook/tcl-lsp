# KCS: IRULE5007 — Why does the analyser warn about an event command outside a when block?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag an event-context command used at the top level outside a `when` block?

## Why

Commands like `HTTP::uri` require an active event context; calling them at the top level raises a runtime error.

## Symptoms

- A squiggle appears under the command, with the message "event-context command used outside when block".

## Example that triggers it

```tcl
set uri [HTTP::uri]
```

The analyser reports **`IRULE5007`** because `HTTP::uri` is called at the top level with no event context.

## Fix

Wrap the command in an appropriate `when` block:

```tcl
when HTTP_REQUEST {
  set uri [HTTP::uri]
}
```

## How to suppress

Add `# noqa: IRULE5007` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5005`, `IRULE5006`
