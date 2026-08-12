# KCS: IRULE2101 — Why does the analyser hint about a heavy regexp in a high-frequency event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser show a hint about a complex regular expression in a hot event?

## Why

A complex regular expression runs on every request, consuming CPU on every connection. In high-frequency events such as `HTTP_REQUEST`, this can degrade throughput across the entire virtual server.

## Symptoms

- A blue squiggle (hint severity) appears on the `regexp` call, with the message "heavy regexp in high-frequency event".

## Example that triggers it

```tcl
when HTTP_REQUEST { regexp {complex} [HTTP::uri] }
```

The analyser reports **`IRULE2101`** because `regexp` is used in a hot event path.

## Fix

Use `string match` or a [data-group](../features/kcs-feature-refactor-extract-datagroup.md) lookup:

```tcl
when HTTP_REQUEST {
  if {[string match "*/api/*" [HTTP::uri]]} {
    pool api_pool
  }
}
```

## How to suppress

Add `# noqa: IRULE2101` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE2001`, `IRULE2002`
