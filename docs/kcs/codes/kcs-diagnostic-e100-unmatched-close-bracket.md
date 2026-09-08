# KCS: E100 — Why does the analyser flag an unmatched closing bracket?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle on a `]` that does not have a matching `[`?

## Why

A stray closing bracket usually means a command substitution was intended but the opening `[` was omitted, or the bracket was left behind after an edit. Either way the script will not behave as expected.

## Symptoms

- A red squiggle appears under the stray `]`, with the message "Unmatched ']' — missing opening '['?".

## Example that triggers it

```tcl
set result value]
```

The analyser reports **`E100`** on the unmatched `]`.

## Fix

```tcl
set result [value]
```

Add the missing `[` to form a complete command substitution, or remove the stray `]` if no substitution was intended.

## How to suppress

`E100` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E100` directive at the top of the file, or for a
whole project with `disabled = E100` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E101`, `E200`
