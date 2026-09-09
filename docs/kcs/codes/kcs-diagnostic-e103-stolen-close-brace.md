# KCS: E103 — Why does the analyser say a nested body stole a closing brace?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle saying a closing brace was consumed by the wrong scope?

## Why

When a nested construct steals a brace from an outer scope, the rest of the script is parsed incorrectly. This causes cascading errors that are difficult to diagnose without tooling.

## Symptoms

- A red squiggle appears on the stolen `}`, with the message "Missing '}' — a
  nested body consumed this closing brace".

## Example that triggers it

```tcl
proc report {x} {
    if {$x > 0} {
        puts yes
    }
```

The analyser reports **`E103`** on the `}` that closes the `if` body: the
`proc` body was never closed, so that brace was consumed as the procedure's
own closer and the procedure runs to the end of the file.

## Fix

```tcl
proc report {x} {
    if {$x > 0} {
        puts yes
    }
}
```

Add the missing `}` so the inner body and the outer body each get their own
closing brace.

`E103`'s auto-fix only fires when the missing brace swallowed exactly one
nested construct — the common case of a single `if`/`switch`/`while`/`for`
block stealing the enclosing scope's closer. When more than one top-level
statement got swallowed (for example a sibling `proc` defined right after
the unclosed one), which brace was "stolen" becomes ambiguous, so the
analyser falls back to the generic **`E200`** ("missing close-brace")
instead of guessing a fix location that could silently nest the following
statement(s) inside the wrong scope.

## How to suppress

`E103` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E103` directive at the top of the file, or for a
whole project with `disabled = E103` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E100`, `E101`
