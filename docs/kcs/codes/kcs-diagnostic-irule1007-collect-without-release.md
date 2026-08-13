# KCS: IRULE1007 — Why does the analyser flag a collect call without a matching release?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report an error for a `collect` call that has no matching `release`?

## Why

Collected data is never freed. The connection leaks memory and buffers until the connection times out, which can exhaust TMM resources.

## Symptoms

- A red squiggle (error severity) appears on the `collect` call, with the message "collect without release".

## Example that triggers it

```tcl
when CLIENT_ACCEPTED { TCP::collect 1024 }
```

The analyser reports **`IRULE1007`** because no `TCP::release` exists in a data handler.

## Fix

Add a `TCP::release` call in the corresponding data event:

```tcl
when CLIENT_ACCEPTED { TCP::collect 1024 }
when CLIENT_DATA { TCP::release }
```

## Limits

TCP and SSL collections require an explicit matching `release`. HTTP releases
its collected data implicitly when its matching request or response data event
finishes, so a complete HTTP data-event handler does not produce this warning.
If that data event issues another `HTTP::collect`, the new collection needs a
later data event or an explicit `HTTP::release`. The analyser cannot follow
releases through dynamically constructed commands.

## How to suppress

Add `# noqa: IRULE1007` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1005`, `IRULE1006`, `IRULE1008`
