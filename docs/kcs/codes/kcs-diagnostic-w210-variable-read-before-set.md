# KCS: W210 — Why does the analyser warn about a variable used before being set?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, liveness

## Profiles

default

## Question

Why does the analyser flag a variable that is read before it has been assigned a value?

## Why

Reading an undefined variable causes a runtime error and stops the script.

## Symptoms

- A yellow squiggle appears under the variable reference, with the message "variable used before being set".

## Example that triggers it

```tcl
puts $x
```

The analyser reports **`W210`** because `x` is never set before it is read.

## Fix

```tcl
set x ""
puts $x
```

Assign the variable before using it.

## How to suppress

Add `# noqa: W210` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W211`, `W213`, `W220`
