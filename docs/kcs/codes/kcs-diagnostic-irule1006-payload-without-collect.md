# KCS: IRULE1006 — Why does the analyser flag payload access without a collect call?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report that payload data is accessed without a preceding `collect`?

## Why

Payload is empty because no `collect` call reserved the data. The command returns an empty string or raises an error at runtime.

## Symptoms

- A squiggle appears on the payload access command, with the message "payload access without collect".

## Example that triggers it

```tcl
when HTTP_REQUEST { set p [HTTP::payload] }
```

The analyser reports **`IRULE1006`** because `HTTP::collect` was not called first.

## Fix

Call `HTTP::collect` before accessing the payload:

```tcl
when HTTP_REQUEST { HTTP::collect 1024 }
when HTTP_REQUEST_DATA { set p [HTTP::payload] }
```

## How to suppress

Add `# noqa: IRULE1006` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1005`, `IRULE1007`, `IRULE1008`
