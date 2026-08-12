# KCS: W303 — Can this regexp cause catastrophic backtracking?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn about a regular expression pattern?

## Why

Nested quantifiers like `(a+)+` cause exponential backtracking on crafted input, freezing the application (a ReDoS attack).

## Symptoms

- A yellow squiggle appears under the regexp pattern, with the message "regexp vulnerable to catastrophic backtracking (ReDoS)".

## Example that triggers it

```tcl
regexp {(a+)+} $input
```

The analyser reports **`W303`** on the regexp pattern.

## Fix

```tcl
regexp {a+} $input
```

Remove nested quantifiers so the pattern matches in linear time.

## How to suppress

Add `# noqa: W303` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W306`, `W100`
