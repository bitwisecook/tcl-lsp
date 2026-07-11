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

This also applies to the vendor dialects (F5 iApps/tmsh, the EDA-tool shells) according to their documented base Tcl version, and to F5 iRules according to its embedded Tcl 8.4.6 runtime — not the newer command signature it otherwise advertises.

## Symptoms

- A yellow squiggle appears under the operator itself (not the whole expression), with a message such as: "Expression operator 'in' is not available in dialect 'tcl8.4'; requires Tcl 8.5+ (TIP 201)."
- Each occurrence of a gated operator gets its own squiggle — an expression using the same or different gated operators more than once reports one diagnostic per occurrence, each anchored at its own operator.

## Example that triggers it

```tcl
# tcl-dialect: tcl8.4
expr {2 in {1 2 3}}
```

The analyser reports **`W003`** on the `in` operator.

## Fix

For the common case — the whole expression is exactly one gated comparison with plain operands (a literal, a variable, a string/list literal, or a `[command]` substitution) — the editor offers a quick fix that rewrites it to a portable form:

```tcl
# Before:
expr {2 in {1 2 3}}
# After (quick fix):
expr {([lsearch -exact {1 2 3} 2] >= 0)}
```

```tcl
# Before:
if {$x lt $y} { ... }
# After (quick fix):
if {([string compare $x $y] < 0)} { ... }
```

When the operator is nested inside a larger expression, appears more than once, or has an operand that isn't a plain value (e.g. a function call like `max($a, $b)`), no quick fix is offered — rewrite it by hand, or raise the dialect setting to a version that supports the operator.

## How to suppress

Add `# noqa: W003` at the end of the offending line. Suppression is line-based: it silences every W003 occurrence reported on that line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W002`, `W004`
