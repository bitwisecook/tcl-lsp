# KCS: E101 — Why does the analyser flag a missing opening brace after `switch`?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle when `switch` cases are listed without an enclosing brace?

## Why

Without braces around the switch body, Tcl treats each case as a separate argument. This leads to argument-count errors or silently wrong pattern matching at runtime.

## Symptoms

- A red squiggle appears after the `switch` variable, with the message "missing '{' for switch body".

## Example that triggers it

```tcl
switch $x
  1 {puts one}
```

The analyser reports **`E101`** on the line following `switch $x`.

## Fix

```tcl
switch $x {
  1 {puts one}
}
```

Wrap the entire set of cases in braces so the parser recognises them as a single switch body.

## How to suppress

Add `# noqa: E101` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E103`, `E200`
