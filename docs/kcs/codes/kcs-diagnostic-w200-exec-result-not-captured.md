# KCS: W200 — Why does the analyser warn about an uncaptured exec result?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that the result of `exec` is not captured?

## Why

An `exec` whose output is not captured discards the command's result. This is usually a mistake — either the output should be stored, or the command's exit status should be checked to detect failures.

## Symptoms

- A yellow squiggle appears under the `exec` command, with the message "exec result is not captured".

## Example that triggers it

```tcl
exec ls /tmp
```

The analyser reports **`W200`** on the `exec` call.

## Fix

```tcl
set files [exec ls /tmp]
```

Capture the result in a variable, or redirect output explicitly if you genuinely want to discard it.

## How to suppress

Add `# noqa: W200` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W126`, `W201`
