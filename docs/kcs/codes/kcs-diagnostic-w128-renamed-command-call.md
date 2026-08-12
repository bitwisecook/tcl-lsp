# KCS: W128 — Command called after it was renamed or deleted earlier in this file

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, cfg

## Profiles

default

## Question

Why does the analyser warn that I am calling a command that was already renamed or deleted?

## Why

In Tcl, `rename` can remove or replace a command. If a command was bound in this file (as a proc or built-in) and was later `rename`d away, any subsequent call to the original name does not raise an immediate error — it falls through to the `unknown` handler. That handler may do nothing useful or may raise a confusing secondary error. The analyser tracks this flow-sensitively: only names that were actually bound in the current file are flagged; calls to external library commands that were never defined here remain silent.

## Symptoms

- A yellow squiggle appears under the command call, with the message: "Command 'myproc' was renamed or deleted earlier in this file; this call falls through to the 'unknown' handler."

## Example that triggers it

```tcl
proc myproc {} { return 1 }
myproc
rename myproc gone
myproc
```

The analyser reports **`W128`** on the final `myproc` call.

## Fix

```tcl
proc myproc {} { return 1 }
myproc
rename myproc gone
gone
```

Call the command under its new name after the `rename`, or remove the stale call.

## How to suppress

Add `# noqa: W128` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [cfg](../../GLOSSARY.md#cfg)
- Related codes: `W001`, `W002`
