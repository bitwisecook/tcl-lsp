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

- A hint at line 1, column 1, with the message "Mixed line endings: LF (1),
  CRLF (1); expected LF". A file using one non-expected style throughout reads
  "File uses CRLF line endings (12); expected LF".

## Example that triggers it

A file whose first line ends `\r\n` and whose second ends `\n`.

The analyser reports **`W118`** once, at the start of the file. The expected
style is LF.

## Fix

Normalise the file to use a single line-ending style. Most editors and version-control systems can do this automatically (e.g. `.gitattributes` with `* text=auto`).

## How to suppress

Add `# noqa: W118` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W112`, `W111`
