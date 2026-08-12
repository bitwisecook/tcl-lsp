# KCS: W309 — Does eval with subst create a double-substitution vulnerability?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser flag `eval [subst ...]` as an error?

## Why

`subst` followed by `eval` creates a double-substitution injection vector; this is almost always a vulnerability.

## Symptoms

- A red squiggle appears under the `eval` call, with the message "eval/uplevel with subst = double substitution".

## Example that triggers it

```tcl
eval [subst $template]
```

The analyser reports **`W309`** on the `eval` call.

## Fix

```tcl
eval [string map [list %name $name] $template]
```

Use `string map` or `format` to perform safe placeholder replacement instead.

## How to suppress

Add `# noqa: W309` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W101`, `W301`, `W308`
