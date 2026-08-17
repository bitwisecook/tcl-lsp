# KCS: IRULE2002 — Why does the analyser flag a deprecated iRules command?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default, dialect:irule

## Question

Why does the analyser report that an iRules command is deprecated?

## Why

The command is no longer supported and may not work on current BIG-IP versions. Continued use risks runtime errors after a BIG-IP upgrade.

## Symptoms

- A squiggle appears on the deprecated command, with the message "deprecated iRules command".

## Example that triggers it

```tcl
when HTTP_REQUEST {
  use deprecated_command_here
}
```

The analyser reports **`IRULE2002`** on the deprecated command token.

## Fix

Replace the deprecated command with its modern equivalent, as indicated by the diagnostic message.

## How to suppress

Add `# noqa: IRULE2002` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE2001`, `IRULE2003`
