# KCS: W108 — Why does the analyser flag non-ASCII characters?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn when a token contains non-ASCII characters?

## Why

Non-ASCII characters in command or variable names are often the result of copy-pasting from rich-text sources and can introduce invisible characters that cause hard-to-debug runtime failures.

## What about comments?

Comments are prose, so ordinary non-ASCII text in a comment (an em-dash, a smart quote, or accented words) is **not** flagged in the default `confusables` mode or in `common` mode. Inside comments the analyser flags only invisible or direction-altering characters — bidirectional override and isolate controls, zero-width characters, and Unicode line separators — because those can make the reviewed text lie about the code next to it (the "Trojan Source" attack). Those characters are also flagged in code even when they have no ASCII replacement.

In `strict` mode (the default for F5 iRules and iApps, whose platforms expect ASCII-only files) every non-ASCII character is still flagged, including inside comments.

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

Add `# noqa: W108` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W111`, `W112`
