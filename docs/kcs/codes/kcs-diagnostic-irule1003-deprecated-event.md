# KCS: IRULE1003 — Why does the analyser flag a deprecated event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that an event is deprecated?

## Why

F5 has marked the event deprecated. It still fires today, but it is frozen and a future release can drop it, taking the handler with it.

## Symptoms

- The event name is struck through and carries a yellow squiggle, with the
  message "'AUTH_SUCCESS' event is deprecated as of BIG-IP 9.4.0."

## Example that triggers it

```tcl
when AUTH_SUCCESS { log local0. "authenticated" }
```

The analyser reports **`IRULE1003`** on `AUTH_SUCCESS`, deprecated since
BIG-IP 9.4.0.

## Fix

Move the handler to the supported event for the same point in the flow — for
authentication, the `AUTH_RESULT` event:

```tcl
when AUTH_RESULT { log local0. "authenticated" }
```

The message names the release the event was deprecated in, not a replacement:
check F5's documentation for the event that replaced it.

## How to suppress

Add `# noqa: IRULE1003` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1001`, `IRULE1002`
