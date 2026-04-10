# KCS: W201 — Why not build file paths with / or \ manually?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser warn about manual path concatenation with `/` or `\`?

## Why

Manually joining path segments with separator characters is fragile and non-portable. It mishandles double separators, trailing slashes, and platform differences between Unix and Windows.

## Symptoms

- A yellow squiggle appears under the concatenation, with the message "manual path concatenation with '/' or '\\'".

## Example that triggers it

```tcl
set path "$dir/$filename"
```

The analyser reports **`W201`** on the string containing the `/` separator.

## Fix

```tcl
set path [file join $dir $filename]
```

Use `file join` to build paths safely and portably.

## How to suppress

Add `# noqa: W201` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint](../../GLOSSARY.md#taint)
- Related codes: `W200`, `W104`
