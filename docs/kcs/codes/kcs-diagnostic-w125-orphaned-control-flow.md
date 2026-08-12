# KCS: W125 — Why does the analyser flag an orphaned control-flow keyword?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn about `else`, `elseif`, `then`, `on`, `trap`, or `finally` used as a standalone command?

## Why

Keywords like `else`, `elseif`, `then`, `on`, `trap`, and `finally` are not
standalone commands in Tcl — they are syntactic parts of `if` or `try` that
must appear on the same logical line as the preceding clause. When a newline
separates the closing brace of one clause from the keyword that starts the
next, Tcl parses the keyword as a separate command and either calls the
`unknown` handler (if no proc with that name is defined) or does something
unexpected. The most common cause is placing `else` or `elseif` on its own
line after a closing `}`.

## Symptoms

- A yellow squiggle appears under the orphaned keyword, with a message such as: '"else" used as standalone command — should be part of "if" (check for misplaced newline).'

## Example that triggers it

```tcl
if {1} {puts a}
else {puts b}
```

The analyser reports **`W125`** on the `else` token.

## Fix

```tcl
if {1} {puts a} else {puts b}
```

Move the keyword onto the same logical line as the preceding clause, or use a
backslash continuation to join the lines.

## How to suppress

Add `# noqa: W125` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E200`, `W001`
