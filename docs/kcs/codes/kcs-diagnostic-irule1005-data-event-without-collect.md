# KCS: IRULE1005 — Why does the analyser flag a data event without a collect call?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report that a data event handler has no matching `collect` call?

## Why

The data event only fires after `collect` has been called in an earlier event. Without a preceding `collect`, the data handler never executes and all its code is dead.

## Symptoms

- A squiggle appears on the data event name, with the message "data event without collect".

## Example that triggers it

```tcl
when CLIENT_DATA { log [TCP::payload] }
```

The analyser reports **`IRULE1005`** because no `TCP::collect` call exists in `CLIENT_ACCEPTED`.

## Fix

Add a `TCP::collect` call in the connection-setup event:

```tcl
when CLIENT_ACCEPTED { TCP::collect 1024 }
when CLIENT_DATA { log [TCP::payload] }
```

## How to suppress

Add `# noqa: IRULE1005` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1006`, `IRULE1007`, `IRULE1008`
