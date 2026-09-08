# KCS: E203 — Why does the analyser flag an unterminated brace group?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle saying a `{` brace group is never closed?

## Why

An unclosed brace group absorbs all subsequent text as literal content, preventing the parser from recognising any further commands. This makes the entire remainder of the file unparseable.

## Symptoms

- A red squiggle appears at or after the opening `{`, with the message "missing close-brace".

## Example that triggers it

```tcl
set data {multi
line unclosed
```

The analyser reports **`E203`** on the unclosed `{` because no matching `}` is found before the end of the file.

## Fix

```tcl
set data {multi
line closed}
```

Add the missing `}` to terminate the brace group so the parser can continue processing the file.

## How to suppress

`E203` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E203` directive at the top of the file, or for a
whole project with `disabled = E203` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E200`, `E201`
