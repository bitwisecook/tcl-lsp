# KCS: W110 — Why should I use eq/ne instead of ==/!= for strings?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn about using `==` or `!=` to compare strings?

## Why

The `==` and `!=` operators attempt numeric comparison first. If one operand looks numeric and the other does not, the result may be surprising or raise an error. Using `eq` and `ne` forces a string comparison, which is both clearer and safer.

## Symptoms

- A yellow squiggle appears under the operator, with the message "use eq/ne for string comparison, not ==/!=".

## Example that triggers it

```tcl
if {$name == "admin"} { puts "welcome" }
```

The analyser reports **`W110`** on the `==` operator.

## Fix

```tcl
if {$name eq "admin"} { puts "welcome" }
```

Use `eq` for equality and `ne` for inequality when comparing strings.

## How to suppress

Add `# noqa: W110` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W100`, `W114`
