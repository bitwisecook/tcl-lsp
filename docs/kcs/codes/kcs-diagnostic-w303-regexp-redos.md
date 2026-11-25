# KCS: W303 — Why might this regexp be slow?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn about a regular expression pattern?

## Why

A backreference may require the regexp engine to try several ways to split an input before it can reject the match. Nested repetition and repeated alternatives can amplify that work.

## Symptoms

- A yellow squiggle appears under the regexp pattern, with the message *"A regular expression with repeated alternatives or nested repetition and a backreference can take substantially longer to reject some inputs (backtracking)."*

## Example that triggers it

```tcl
regexp {^((a+)+)\1$} $input
```

The analyser reports **`W303`** on the regexp pattern.

## Fix

```tcl
regexp {^(a+)\1$} $input
```

Simplify the repeated group while preserving the match. For this example, one or more runs of `a` describe the same sequence as a single run.

## How to suppress

Add `# noqa: W303` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W306`, `W100`
