# KCS: IRULE2003 — Why does the analyser flag an unsafe iRules command?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that a command is unsafe in iRules?

## Why

The command allows context escalation or namespace escape, breaking iRules isolation. It can access or modify state outside the iRule sandbox.

## Symptoms

- A squiggle appears on the unsafe command, with the message "unsafe iRules command".

## Example that triggers it

```tcl
global ns
```

The analyser reports **`IRULE2003`** because `global` escapes the iRules namespace.

## Fix

Use `static::` for shared state or per-connection storage:

```tcl
set static::ns "value"
```

## How to suppress

Add `# noqa: IRULE2003` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE2001`, `IRULE2002`
