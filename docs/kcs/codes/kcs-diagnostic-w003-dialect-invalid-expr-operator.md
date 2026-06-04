# KCS: W003 — Expression operator not available in the active dialect

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that an expression operator is not available in my dialect?

## Why

Tcl added new expression operators in specific releases. The `in` and `ni` membership operators were introduced in Tcl 8.5 (TIP 201); the string-comparison operators `lt`, `le`, `gt`, and `ge` were introduced in Tcl 9.0 (TIP 461). When the active dialect is set to an earlier version, using these operators causes a runtime `syntax error in expression` or `invalid bareword` error in the real Tcl interpreter.

## Symptoms

- A yellow squiggle appears under the expression argument, with a message such as: "Expression operator 'in' is not available in the active dialect (tcl8.4); requires Tcl 8.5+ (TIP 201)."

## Example that triggers it

```tcl
# dialect: tcl8.4
expr {2 in {1 2 3}}
```

The analyser reports **`W003`** on the expression argument.

## Fix

```tcl
# Use a dialect-compatible alternative:
expr {[lsearch -exact {1 2 3} 2] >= 0}
```

Replace the dialect-restricted operator with a form supported by the active dialect, or raise the dialect setting to a version that supports the operator.

## How to suppress

Add `# noqa: W003` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W002`, `W004`
