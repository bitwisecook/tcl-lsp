# KCS: T102 — Why does the analyser warn about tainted data in option position?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default, irule

## Question

Why does the analyser flag user-controlled data passed in option position without a `--` terminator?

## Why

A value starting with `-` is interpreted as an option, letting the attacker change command behaviour.

## Symptoms

- A yellow squiggle appears under the argument, with the message "tainted data in option position without -- terminator".

## Example that triggers it

```tcl
set path [HTTP::uri]
file exists $path
```

The analyser reports **`T102`** because `path` could start with `-`.

## Fix

```tcl
set path [HTTP::uri]
file exists -- $path
```

Add `--` before the argument to end option parsing.

## How to suppress

Add `# noqa: T102` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T100`, `T101`, `W304`
