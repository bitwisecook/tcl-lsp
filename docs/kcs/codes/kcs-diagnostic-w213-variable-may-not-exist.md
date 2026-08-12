# KCS: W213 — Why does the analyser warn that a variable may not exist?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, liveness

## Profiles

default

## Question

Why does the analyser suggest using `unset -nocomplain` instead of plain `unset`?

## Why

`unset` on a non-existent variable raises an error; `-nocomplain` prevents the crash.

## Symptoms

- A yellow squiggle appears under the `unset` call, with the message "variable may not exist — use unset -nocomplain".

## Example that triggers it

```tcl
unset maybe_defined
```

The analyser reports **`W213`** because `maybe_defined` may not exist at that point.

## Fix

```tcl
unset -nocomplain maybe_defined
```

## How to suppress

Add `# noqa: W213` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W210`, `W211`
