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

- A yellow squiggle appears under the space-padded value, with the message "append with space-separated values looks like list construction".
- For the simple shape below, a quick fix titled "Rewrite with `lappend`" is offered.

## Example that triggers it

```tcl
append mylist " $newitem"
```

The analyser reports **`W104`** on the space-padded value.

## Fix

```tcl
lappend mylist $newitem
```

Use `lappend` or `list` to build lists so that special characters are properly quoted.

The quick fix rewrites the whole command, and is offered only for the mechanical shape: `append var " piece"` — one quoted value, one leading pad space, and one piece free of spaces, braces, quotes, brackets, backslashes, and semicolons. On a non-empty list the rewrite is byte-for-byte equivalent; on the first append it also drops the stray leading separator, which is almost always the intent. A trailing pad (`append msg "item "`), several value words, or extra padding stay message-only — those shapes have no unambiguous `lappend` mapping.

## How to suppress

Add `# noqa: W104` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W105`
