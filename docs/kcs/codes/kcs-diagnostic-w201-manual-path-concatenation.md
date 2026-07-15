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
- When the value splits cleanly into `/`-separated segments, a quick fix titled "Rewrite with `file join`" is offered.

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

The quick fix replaces exactly the concatenated value, and is offered only when every `/`-separated segment is a plain word or a simple `$var` reference. A leading `/` is kept on the first segment, so an absolute path stays absolute (`"/tmp/$x"` becomes `[file join /tmp $x]`). No fix is offered for mixed segments (`$name.log`), command substitutions, glob characters, backslashes, protocol-like values (`http://...`), or consecutive or trailing slashes — `file join` would silently normalise those, changing the built string.

## How to suppress

Add `# noqa: W201` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint](../../GLOSSARY.md#taint)
- Related codes: `W200`, `W104`
