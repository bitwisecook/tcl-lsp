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

Add `# noqa: E103` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E100`, `E101`
