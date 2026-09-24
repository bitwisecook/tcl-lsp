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

- A yellow squiggle appears under the variable name in the `unset` call, with a
  message like `Variable 'maybe_defined' may not exist; use 'unset -nocomplain'
  to suppress the error`.
- Where the variable certainly does not exist — nothing set it, or an earlier
  `unset` removed it — the message is definite: `Variable 'tmp' does not exist
  here; use 'unset -nocomplain' to suppress the error`.

## Example that triggers it

```tcl
unset maybe_defined
```

The analyser reports **`W213`** because `maybe_defined` may not exist at that point.

A second `unset` of the same variable fails on every Tcl release, so it is
reported as definite:

```tcl
proc cleanup {} {
    set tmp 1
    unset tmp
    unset tmp   ;# W213: 'tmp' does not exist here
}
```

The analyser reads whether the variable exists where the `unset` runs: it
reports nothing where the variable certainly exists, "may not exist" where
only some paths set it, and "does not exist here" where none do.
`unset -nocomplain` never reports, and it is not a read of the variable, so
it draws no `W210` either.

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
