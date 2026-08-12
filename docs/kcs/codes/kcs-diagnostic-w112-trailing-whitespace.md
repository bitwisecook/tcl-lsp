# KCS: W112 — Why does the analyser flag trailing whitespace?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about trailing whitespace at the end of a line?

## Why

Trailing whitespace adds no value, clutters diffs, and can cause problems with backslash-newline continuation where a space after the backslash silently breaks the continuation.

## Symptoms

- A yellow squiggle appears at the end of the line, with the message "trailing whitespace".

## Example that triggers it

```tcl
set x 42   
```

The analyser reports **`W112`** on the trailing spaces after `42`.

## Fix

```tcl
set x 42
```

Remove the trailing whitespace. Most editors can be configured to strip it automatically on save.

## How to suppress

Add `# noqa: W112` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W111`, `W118`
