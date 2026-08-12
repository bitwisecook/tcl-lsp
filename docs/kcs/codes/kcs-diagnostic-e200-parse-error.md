# KCS: E200 — Why does the analyser report a general parse error?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle saying the parser cannot recover from a syntax error?

## Why

An unclosed delimiter prevents the parser from determining where one command ends and the next begins. Everything after the unclosed token is misinterpreted, so no further analysis is reliable until the delimiter is matched.

## Symptoms

- A red squiggle appears at or near the unclosed delimiter, with the message "parse error: unclosed brace, bracket, or quote".

## Example that triggers it

```tcl
set data {unclosed
```

The analyser reports **`E200`** at the end of the file because the opening `{` is never closed.

## Fix

```tcl
set data {closed}
```

Close the brace, bracket, or quote so the parser can process the rest of the file correctly.

## How to suppress

Add `# noqa: E200` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E201`, `E202`, `E203`
