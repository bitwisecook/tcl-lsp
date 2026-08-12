# KCS: W300 — Does source with a variable path allow code execution?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when `source` is called with a variable argument?

## Why

Sourced files execute as Tcl code; an attacker-controlled path leads to arbitrary code execution.

## Symptoms

- A yellow squiggle appears under the `source` call, with the message "source with variable argument".

## Example that triggers it

```tcl
source $filepath
```

The analyser reports **`W300`** on the `source` call.

## Fix

```tcl
source [file join $safe_dir $name]
```

Constrain the path to a known safe directory before sourcing.

## How to suppress

Add `# noqa: W300` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W101`, `W313`
