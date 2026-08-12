# KCS: IRULE1002 — Why does the analyser flag an unknown event name?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser report an unknown event name in a `when` block?

## Why

The event does not exist in the iRules event catalogue. The `when` block will never fire, so all code inside it is dead.

## Symptoms

- A squiggle appears under the event name, with the message "unknown event name".

## Example that triggers it

```tcl
when INVALID_EVENT {
  set x 1
}
```

The analyser reports **`IRULE1002`** because `INVALID_EVENT` is not a recognised iRules event.

## Fix

Use a valid event name:

```tcl
when HTTP_REQUEST {
  set x 1
}
```

## How to suppress

Add `# noqa: IRULE1002` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1001`, `IRULE1003`
