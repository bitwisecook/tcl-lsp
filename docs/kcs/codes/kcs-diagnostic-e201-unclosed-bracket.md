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

- A red squiggle appears at or after the opening `[`, with the message "missing close-bracket".

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

`E201` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E201` directive at the top of the file, or for a
whole project with `disabled = E201` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E200`, `E202`
