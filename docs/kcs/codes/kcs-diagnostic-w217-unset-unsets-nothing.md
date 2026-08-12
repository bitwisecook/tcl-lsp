# KCS: W217 — Why does the analyser say my `unset` unsets nothing?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser warn that my `unset` command unsets nothing?

## Why

`unset` accepts the options `-nocomplain` and `--` before its variable names. When **every** argument looks like an option, Tcl consumes them all as options and no variable name remains, so nothing is unset. This happens when a variable is genuinely named with a leading dash (for example `set -x 1; unset -x` — the `-x` is read as an unknown option and the command errors, or with `-nocomplain` alone nothing happens at all).

## Symptoms

- A yellow squiggle appears under the `unset` command, with the message "`unset` unsets nothing".

## Example that triggers it

```tcl
unset -nocomplain
```

The analyser reports **`W217`** — every argument was consumed as an option, so no variable is named.

## Fix

```tcl
unset -nocomplain -- -x
```

Add `--` to end option processing, then name the variable — a `-`-named variable must appear after `--`.

## How to suppress

Add `# noqa: W217` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
