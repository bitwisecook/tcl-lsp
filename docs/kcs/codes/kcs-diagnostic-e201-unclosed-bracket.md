# KCS: E201 — Why does the analyser flag an unterminated command substitution?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle saying a `[` command substitution is never closed?

## Why

An unclosed bracket causes the parser to absorb all subsequent text as part of the substitution. This hides the real commands that follow and produces misleading errors elsewhere in the file.

## Symptoms

- A red squiggle appears at or after the opening `[`, with the message "unterminated '[' command substitution".

## Example that triggers it

```tcl
set cmd [expr $x +
```

The analyser reports **`E201`** on the unclosed `[` because no matching `]` is found.

## Fix

```tcl
set cmd [expr {$x + 1}]
```

Add the missing `]` to terminate the command substitution, and brace the expression for safety.

## How to suppress

Add `# noqa: E201` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E200`, `E202`
