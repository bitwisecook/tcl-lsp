# KCS: E004 — Why does the analyser flag extra words after `else`?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why do I see a red squiggle on extra words following `else` in an `if` statement?

## Why

Tcl's `if` command expects the `else` clause to be followed by a braced body. Bare words after `else` are not valid syntax and will cause a runtime error.

## Symptoms

- A red squiggle appears after the `else` keyword, with the message "invalid argument count: extra words after 'else'".

## Example that triggers it

```tcl
if {$x} {puts yes} else extra
```

The analyser reports **`E004`** on the unexpected word `extra`.

## Fix

```tcl
if {$x} {puts yes} else {puts no}
```

Wrap the else-branch body in braces so the parser recognises it as a script block.

## How to suppress

Add `# noqa: E004` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E001`
