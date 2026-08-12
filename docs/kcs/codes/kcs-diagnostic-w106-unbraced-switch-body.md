# KCS: W106 — Why is an unbraced switch body dangerous?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about an unbraced `switch` body?

## Why

An unbraced `switch` body undergoes substitution before `switch` parses it, which can execute arbitrary code, misinterpret patterns, and prevents byte-compilation of the arms.

## Symptoms

- A yellow squiggle appears under the switch body, with the message "dangerous unbraced switch body".

## Example that triggers it

```tcl
switch $x a { puts A } b { puts B }
```

The analyser reports **`W106`** on the inline switch body.

## Fix

```tcl
switch $x {
    a { puts A }
    b { puts B }
}
```

Wrap the entire pattern–body list in braces.

## How to suppress

Add `# noqa: W106` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W105`
