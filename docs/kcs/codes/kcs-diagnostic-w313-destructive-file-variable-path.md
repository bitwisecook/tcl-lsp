# KCS: W313 — Can a destructive file operation with a variable path be exploited?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser warn about a destructive file operation with a variable path?

## Why

An attacker who controls the path can delete, rename, or create files outside the intended directory.

## Symptoms

- A yellow squiggle appears under the file operation, with the message "destructive file operation with variable path".

## Example that triggers it

```tcl
file delete $userPath
```

The analyser reports **`W313`** on the `file delete` call.

## Fix

```tcl
set safe [file normalize $userPath]
file delete -- $safe
```

Normalise and validate the path, and use `--` to prevent option injection.

## How to suppress

Add `# noqa: W313` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `W300`, `W304`
