# KCS: W306 — Can a substitution in a literal-expected position cause issues?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn about a substitution in a position that expects a literal?

## Why

A regexp pattern or class name undergoes unintended variable or command substitution, which can alter the match semantics or execute code.

## Symptoms

- A yellow squiggle appears under the argument, with the message "substitution in literal-expected argument position".

## Example that triggers it

```tcl
regexp "$pattern" $string
```

The analyser reports **`W306`** on the pattern argument.

## Fix

```tcl
regexp {$pattern} $string
```

Use braces instead of double quotes so the pattern is treated as a literal.

## How to suppress

Add `# noqa: W306` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W100`, `W303`
