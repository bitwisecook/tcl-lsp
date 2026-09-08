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

- A red squiggle appears at or after the opening `"`, with the message `missing "`.

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

`E202` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E202` directive at the top of the file, or for a
whole project with `disabled = E202` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E200`, `E201`
