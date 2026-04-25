# KCS: W108 — Why does the analyser flag non-ASCII characters?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn when a token contains non-ASCII characters?

## Why

Non-ASCII characters in command or variable names are often the result of copy-pasting from rich-text sources and can introduce invisible characters that cause hard-to-debug runtime failures.

## Symptoms

- A yellow squiggle appears under the token, with the message "non-ASCII character in token".

## Example that triggers it

```tcl
set café "latte"
```

The analyser reports **`W108`** on the `café` token.

## Fix

```tcl
set cafe "latte"
```

Replace non-ASCII characters with their ASCII equivalents or remove them.

## How to suppress

Add `# noqa: W108` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W111`, `W112`
