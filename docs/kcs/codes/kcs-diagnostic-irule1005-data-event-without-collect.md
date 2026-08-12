# KCS: IRULE1005 — Why does the analyser flag a data event without a collect call?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report that a data event handler has no matching `collect` call?

## Why

Some data events only fire after their protocol's `collect` command has run
in an earlier event. Without that call, the data handler never executes and
all its code is dead. The analyser reports this only where the selected
protocol is known to require collection.

## Symptoms

- A squiggle appears on the data event name, with the message "data event without collect".

## Example that triggers it

```tcl
when HTTP_REQUEST_DATA { log [HTTP::payload] }
```

The analyser reports **`IRULE1005`** because no `HTTP::collect` call exists in
`HTTP_REQUEST`.

## Fix

Add an `HTTP::collect` call in the matching request event:

```tcl
when HTTP_REQUEST { HTTP::collect 1024 }
when HTTP_REQUEST_DATA { log [HTTP::payload] }
```

## Limits

`CLIENT_DATA` and `SERVER_DATA` can represent either TCP or UDP traffic. UDP
delivers each datagram without a `collect` call, so the analyser does not warn
for those ambiguous event names. It cannot prove data flow through dynamic
event names, aliases, or commands assembled with `eval`.

## How to suppress

Add `# noqa: IRULE1005` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1006`, `IRULE1007`, `IRULE1008`
