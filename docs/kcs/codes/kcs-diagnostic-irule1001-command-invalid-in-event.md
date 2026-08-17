# KCS: IRULE1001 — Why does the analyser flag a command as invalid in this event?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that a command is not valid in this event context?

## Why

The command has no effect or raises an error when called in the wrong event context. Each iRules command is only meaningful inside the events that support it.

## Symptoms

- A squiggle appears under the command, with the message "command not valid in this event".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  RULE_INIT
}
```

The analyser reports **`IRULE1001`** because `RULE_INIT` is not a command that can be called inside `HTTP_REQUEST`.

## Fix

Move the command to the appropriate event:

```tcl
when RULE_INIT {
  # initialisation logic here
}
```

## How to suppress

Add `# noqa: IRULE1001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE1002`, `IRULE1003`
