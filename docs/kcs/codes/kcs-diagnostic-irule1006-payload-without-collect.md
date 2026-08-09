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

HTTP, TCP, and SSL payload access needs an earlier matching `collect` call.
Without it, the payload may be empty or the command may fail at runtime. The
analyser takes this requirement from the command registry.

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

## Limits

`UDP::payload` is the current datagram and `ASM::payload` does not use the
HTTP/TCP/SSL collection lifecycle, so neither command produces this warning.
The analyser is conservative around dynamic command names and indirect calls.

## How to suppress

Add `# noqa: IRULE1006` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1005`, `IRULE1007`, `IRULE1008`
