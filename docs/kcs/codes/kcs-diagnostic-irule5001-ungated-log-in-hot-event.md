# KCS: IRULE5001 — Why does the analyser warn about an ungated log in a high-frequency event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `log` call inside a per-request event with no guard?

## Why

Logging on every request generates millions of log lines, overwhelming syslog and slowing the BIG-IP.

## Symptoms

- A hint squiggle appears under the `log` call, with the message "ungated log in high-frequency event".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  log local0. "req: [HTTP::uri]"
}
```

The analyser reports **`IRULE5001`** because the log runs unconditionally on every request.

## Fix

Gate the log with a debug flag:

```tcl
when HTTP_REQUEST {
  if {$static::debug} { log local0. "req: [HTTP::uri]" }
}
```

## How to suppress

Add `# noqa: IRULE5001` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5002`, `IRULE4004`
