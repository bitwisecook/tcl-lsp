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

A Unicode character that looks like an ASCII one — Cyrillic `х` for Latin `x`,
a full-width digit, a non-breaking space — makes two different identifiers
read as the same name. Invisible and direction-altering characters can make
reviewed text lie about the code beside it.

## What `tclLsp.style.nonAscii` controls

`confusables` is the default for Tcl: it flags Unicode look-alikes and
copy-paste artefacts. `common` allows intentional Unicode letters, digits, and
symbols, flagging only look-alikes and control characters. `strict` — the
default for F5 iRules and iApps, whose platforms expect ASCII-only files —
flags every non-ASCII character, including in comments. `off` disables the
check.

Ordinary non-ASCII prose in a comment (an em-dash, a smart quote, an accented
word) is not flagged outside `strict`. Bidirectional override and isolate
controls, zero-width characters, and Unicode line separators are always
flagged, in comments as well as code.

## Symptoms

- A yellow squiggle under the character, with the message "Non-ASCII character
  U+0445 'х' — outside the standard ASCII printable/whitespace set".
- A **Replace with ASCII equivalent** quick fix when the character has one.

## Example that triggers it

```tcl
set х 1
puts $х
```

The `х` is Cyrillic U+0445, not Latin `x`. The analyser reports **`W108`** on
each occurrence.

## Fix

```tcl
set x 1
puts $x
```

Retype the identifier in ASCII rather than pasting it.

## How to suppress

Add `# noqa: W108` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W111`, `W112`
