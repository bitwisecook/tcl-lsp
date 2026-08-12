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

- A yellow squiggle appears at the end of the comment, with the message "backslash-newline in comment swallows the next line".

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

Remove the trailing backslash from the comment line.

## How to suppress

Add `# noqa: W115` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W112`, `W111`
