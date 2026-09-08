# KCS: W115 — Why does a backslash-newline in a comment matter?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about a backslash-newline at the end of a comment?

## Why

Tcl's parser treats backslash-newline as a line continuation even inside comments. This silently swallows the next line, turning real code into part of the comment — a common source of mysterious missing-command bugs.

## Symptoms

- A yellow squiggle over the comment and the line it swallows, with the message
  "Backslash-newline in comment silently swallows the next line".
- A **Convert to per-line comments** quick fix on the diagnostic.

## Example that triggers it

```tcl
# This is a long comment \
set x 42
```

The analyser reports **`W115`** on the backslash at the end of the comment.

## Fix

```tcl
# This is a long comment
set x 42
```

Remove the trailing backslash, or apply the quick fix, which comments out
every continued line instead.

## How to suppress

Add `# noqa: W115` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W112`, `W111`
