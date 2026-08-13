# KCS: E202 — Why does the analyser flag an unterminated string literal?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle saying a `"` string literal is never closed?

## Why

An unclosed quote causes the parser to treat all subsequent text — including other commands — as part of one string. This silently swallows the rest of the file and produces confusing secondary errors.

## Symptoms

- A red squiggle appears at or after the opening `"`, with the message "unterminated '\"' string literal".

## Example that triggers it

```tcl
set str "unclosed
```

The analyser reports **`E202`** on the unclosed `"` because no matching closing quote is found.

## Fix

```tcl
set str "closed"
```

Add the missing closing `"` so the string is properly terminated.

## How to suppress

Add `# noqa: E202` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E200`, `E201`
