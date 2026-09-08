# KCS: W201 — Why not build file paths with / or \ manually?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser warn about manual path concatenation with `/` or `\`?

## Why

Manually joining path segments with separator characters is fragile and non-portable. It mishandles double separators, trailing slashes, and platform differences between Unix and Windows.

## Symptoms

- A hint underline under the concatenation, with the message "Possible manual
  path concatenation. Use [file join] for portable path construction."
- A **Rewrite with `file join`** quick fix when the value splits cleanly into
  `/`-separated segments.

## Example that triggers it

```tcl
set dir /tmp
set filename report.txt
set path "$dir/$filename"
puts $path
```

The analyser reports **`W201`** on the string containing the `/` separator.

## Fix

```tcl
set dir /tmp
set filename report.txt
set path [file join $dir $filename]
puts $path
```

Use `file join` to build paths safely and portably.

The quick fix replaces exactly the concatenated value, and is offered only when
every `/`-separated segment is a plain word or a simple `$var` reference. A
leading `/` is kept, so an absolute path stays absolute (`"/tmp/$x"` becomes
`[file join /tmp $x]`). No fix is offered for mixed segments (`$name.log`),
command substitutions, glob characters, backslashes, protocol-like values
(`http://...`), or consecutive or trailing slashes: `file join` would normalise
those, changing the built string.

## How to suppress

Add `# noqa: W201` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint](../../GLOSSARY.md#taint-analysis)
- Related codes: `W200`, `W104`
