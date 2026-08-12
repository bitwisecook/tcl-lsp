# KCS: IRULE1006 — Why does the analyser flag payload access without a collect call?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, dataflow

## Profiles

default, dialect:irule

## Question

Why does the analyser report that payload data is accessed without a preceding `collect`?

## Why

HTTP, TCP, SSL, MQTT, message routing (MR), RTSP, SCTP, and WebSocket payload
access can need an earlier matching `collect` call. Without it, the payload may
be empty or the command may fail at runtime. The analyser takes the lifecycle,
event side, and command call-form rules from the command registry.

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

`UDP::payload` is the current datagram. ASM, CACHE, DIAMETER, GTP, REWRITE,
SIP, and XML payload commands receive their data from the current protocol or
profile event, so they do not produce this warning. `MQTT::payload length`,
`replace`, and `prepend` operate on the current PUBLISH message; the bare and
`append` forms require collected data. The analyser abstains when a dynamic
argument prevents it from selecting the MQTT form.

## How to suppress

Add `# noqa: IRULE1006` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1005`, `IRULE1007`, `IRULE1008`
