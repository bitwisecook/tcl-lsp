# KCS: W118 — Why does the analyser flag mixed line endings?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about mixed LF and CRLF line endings?

## Why

Inconsistent line endings cause noisy diffs, confuse some Tcl parsers, and create merge conflicts. A file should use one style consistently.

## Symptoms

- A yellow squiggle appears on the first inconsistent line, with the message "mixed LF and CRLF line endings".

## Example that triggers it

```tcl
set a 1\r\n
set b 2\n
```

The analyser reports **`W118`** when the file contains both `\r\n` and `\n` line endings.

## Fix

Normalise the file to use a single line-ending style. Most editors and version-control systems can do this automatically (e.g. `.gitattributes` with `* text=auto`).

## How to suppress

Add `# noqa: W118` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W112`, `W111`
