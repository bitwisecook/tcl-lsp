# KCS: W302 — Does catch without a result variable hide errors?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when `catch` is used without a result variable?

## Why

Errors are silently discarded, hiding bugs and security issues that would otherwise surface during execution.

## Symptoms

- A blue squiggle (hint) appears under the `catch` call, with the message "catch without result variable".

## Example that triggers it

```tcl
catch {risky_command}
```

The analyser reports **`W302`** on the `catch` call.

## Fix

```tcl
catch {risky_command} result
```

Capture the result so that errors can be logged, inspected, or re-raised.

## How to suppress

Add `# noqa: W302` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W125`, `W200`
