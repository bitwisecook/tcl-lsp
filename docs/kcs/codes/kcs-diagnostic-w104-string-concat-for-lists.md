# KCS: W104 — Why not build lists with string concatenation?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn when I build a list by concatenating strings?

## Why

String concatenation is fragile and does not handle special characters (spaces, braces, backslashes) correctly. Elements containing those characters will silently corrupt the list structure.

## Symptoms

- A yellow squiggle appears under the concatenation, with the message "string concatenation used to build a list".

## Example that triggers it

```tcl
set mylist "$mylist $newitem"
```

The analyser reports **`W104`** on the string concatenation.

## Fix

```tcl
lappend mylist $newitem
```

Use `lappend` or `list` to build lists so that special characters are properly quoted.

## How to suppress

Add `# noqa: W104` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W105`
