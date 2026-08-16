# KCS: IRULE5004 — Why does the analyser warn about DNS::return without return?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `DNS::return` that is not followed by `return`?

## Why

`DNS::return` queues the response but does not exit the handler; remaining code still runs unnecessarily.

## Symptoms

- A squiggle appears under the `DNS::return` call, with the message "DNS::return without return".

## Example that triggers it

```tcl
when DNS_REQUEST {
  DNS::return "1.2.3.4"
  log local0. "still runs"
}
```

The analyser reports **`IRULE5004`** because `DNS::return` is not followed by a `return` statement.

## Fix

Add `return` after `DNS::return`:

```tcl
when DNS_REQUEST { DNS::return "1.2.3.4"; return }
```

## How to suppress

Add `# noqa: IRULE5004` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5002`, `IRULE5005`
