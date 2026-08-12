# KCS: W233 — Division or modulo by a provably-zero divisor

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, sccp

## Profiles

default

## Question

Why does the analyser warn that a division or modulo will raise "divide by zero" at runtime?

## Why

The analyser uses interval-based value propagation to determine what range a variable can take at each program point. When the divisor in a `/` or `%` expression is provably zero — either a literal `0` or a variable whose interval domain has been narrowed to the single value `0` — Tcl will always raise a `divide by zero` error at runtime. The analyser reports this ahead of time so you can fix it before running the code.

The check only fires inside SCCP-reachable blocks: divisions inside statically-dead branches (e.g. `if {0} { expr {1/0} }`) are not reported.

## Symptoms

- A yellow squiggle appears under the expression containing the division or modulo, with the message: "Division by a provably-zero divisor — raises 'divide by zero' at runtime." (or "Modulo by a provably-zero divisor …" for `%`).

## Example that triggers it

```tcl
set x [expr {1 / 0}]
```

The analyser reports **`W233`** on the `expr {1 / 0}` call.

## Fix

```tcl
set x [expr {1 / 2}]
```

Replace the zero divisor with a non-zero value, or add a guard before the division:

```tcl
if {$divisor != 0} {
    set result [expr {$dividend / $divisor}]
}
```

## How to suppress

Add `# noqa: W233` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [sccp](../../GLOSSARY.md#sccp)
- Related codes: `W230`, `W231`, `W232`
