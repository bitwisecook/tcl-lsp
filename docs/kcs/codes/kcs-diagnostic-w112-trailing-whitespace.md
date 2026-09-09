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

- A hint underline over the trailing spaces, with the message "Trailing
  whitespace".
- A **Remove trailing whitespace** quick fix on the diagnostic.

## Example that triggers it

```tcl
set x 42   
```

The analyser reports **`W112`** on the trailing spaces after `42`.

## Fix

```tcl
set x 42
```

Apply the quick fix, or configure your editor to strip trailing whitespace on save. A trailing carriage return is stripped before the check runs, so CRLF endings are not flagged.

## How to suppress

Add `# noqa: W112` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W111`, `W118`
