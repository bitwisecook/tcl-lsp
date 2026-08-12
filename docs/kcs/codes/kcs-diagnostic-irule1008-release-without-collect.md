# KCS: IRULE1008 — Why does the analyser flag a release call without a matching collect?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report an error for a `release` call that has no matching `collect`?

## Why

Releasing data that was never collected is a logic error. At runtime this raises a TCL error or silently does nothing, depending on the BIG-IP version.

## Symptoms

- A red squiggle (error severity) appears on the `release` call, with the message "release without collect".

## Example that triggers it

```tcl
when CLIENT_ACCEPTED { TCP::release }
```

The analyser reports **`IRULE1008`** because no `TCP::collect` precedes the release.

## Fix

Only call `release` after a matching `collect`:

```tcl
when CLIENT_ACCEPTED { TCP::collect 1024 }
when CLIENT_DATA { TCP::release }
```

## How to suppress

Add `# noqa: IRULE1008` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1005`, `IRULE1006`, `IRULE1007`
