# KCS: W002 — Why is a command disabled in the active dialect?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that a command is disabled in the current dialect?

## Why

Some Tcl dialects (e.g. iRules) deliberately restrict the set of available commands. Using a disabled command will fail at runtime in that environment, even though it works in standard Tcl.

## Symptoms

- A yellow squiggle appears under the command name, with the message "command 'exec' is disabled in the iRules dialect".

## Example that triggers it

```tcl
# dialect: irules
exec ls /tmp
```

The analyser reports **`W002`** on the `exec` token.

## Fix

```tcl
# Remove or replace with a dialect-appropriate alternative.
log local0. "listing not available in iRules"
```

Use only commands permitted by the active dialect.

## How to suppress

Add `# noqa: W002` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W001`, `E001`
