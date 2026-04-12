# KCS: W214 — Why does the analyser warn about an unused proc parameter?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, liveness

## Profiles

default

## Question

Why does the analyser flag a procedure parameter that is declared but never read?

## Why

An unused parameter may indicate a signature mismatch or missing logic that was meant to use it.

## Symptoms

- A yellow squiggle appears under the parameter name, with the message "proc parameter declared but never used".

## Example that triggers it

```tcl
proc greet {name greeting} {
    puts "Hello"
}
```

The analyser reports **`W214`** on the `greeting` parameter.

## Fix

Use the parameter or remove it from the signature:

```tcl
proc greet {name greeting} { puts "$greeting, $name" }
```

## How to suppress

Add `# noqa: W214` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W211`, `W220`
