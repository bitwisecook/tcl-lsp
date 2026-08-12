# KCS: IRULE1003 — Why does the analyser flag a deprecated event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser report that an event is deprecated?

## Why

The event is no longer supported. The iRule will not fire on current BIG-IP versions, so the handler is silently ignored.

## Symptoms

- A squiggle appears under the event name, with the message "deprecated event".

## Example that triggers it

```tcl
when LOGOUT { log "bye" }
```

The analyser reports **`IRULE1003`** because `LOGOUT` is a deprecated event.

## Fix

Use the modern replacement event recommended by the diagnostic message:

```tcl
when ACCESS_SESSION_CLOSED { log "bye" }
```

## How to suppress

Add `# noqa: IRULE1003` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1001`, `IRULE1002`
