# KCS: E102 — Why does the analyser flag an unmatched closing brace?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle on a `}` that does not have a matching `{`?

## Why

A stray closing brace indicates a structural mismatch in the script. It typically means an extra brace was left behind after refactoring, which will cause a parse error at runtime.

## Symptoms

- A red squiggle appears under the stray `}`, with the message "unmatched '}' without opening '{'".

## Example that triggers it

```tcl
puts "hello"
}
```

The analyser reports **`E102`** on the unmatched `}` on the second line.

A `}` does not need a line to itself to be flagged — it is just as stray when
it appears in the middle of a word:

```tcl
set x foo}bar
```

The analyser reports **`E102`** on the `}` inside `foo}bar`.

## Fix

Remove the stray `}` so that every opening brace has exactly one matching
close. When the `}` sits alone on its own line, the quick fix removes the
whole line; a `}` embedded in a larger word has no automatic fix — remove or
correct it by hand.

## How to suppress

Add `# noqa: E102` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E100`, `E103`
