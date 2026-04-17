# KCS: W220 — Why does the analyser warn about a dead store?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, dce

## Profiles

default

## Question

Why does the analyser flag a variable that is set but overwritten before being read?

## Why

The first assignment is wasted work; the value is thrown away before anything reads it.

## Symptoms

- A yellow squiggle appears under the first assignment, with the message "variable set but overwritten before being read".

## Example that triggers it

```tcl
set x 1
set x 2
puts $x
```

The analyser reports **`W220`** on `set x 1` because the value `1` is never read.

## Fix

```tcl
set x 2
puts $x
```

Remove the redundant first assignment.

## How to suppress

Add `# noqa: W220` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [DCE](../../GLOSSARY.md#dce)
- Related codes: `W210`, `W211`
