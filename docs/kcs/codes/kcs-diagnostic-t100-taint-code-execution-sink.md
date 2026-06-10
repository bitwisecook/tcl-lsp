# KCS: T100 — Why does the analyser warn about tainted data in a code-execution sink?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, taint

## Profiles

default, irule

## Question

Why does the analyser flag user-controlled data flowing into `eval`, `uplevel`, `subst`, unbraced `expr`, `exec`, or a *direct* operand of braced `expr`?

## Why

T100 covers two related taint-into-evaluation hazards:

1. **Code execution.** User-controlled input reaching `eval`,
   `uplevel`, `subst`, **unbraced** `expr` (``expr $cmd + 1``), or
   `exec` is re-parsed as Tcl syntax and can execute arbitrary
   commands.  This is the classic injection vector.

2. **Numeric / type coercion.** A tainted value used as a *direct*
   operand of **braced** `expr` (``expr {$data + 1}``) is NOT
   re-parsed as code -- Tcl evaluates the expression once with
   ``$data`` as a single operand.  But the value still flows through
   Tcl's numeric coercion: ``"inf"`` returns inf, ``"0xff"`` parses
   as 255 even if base-10 was intended, ``"0/0"`` raises a domain
   error.  Decisions taken on the result can be subverted without
   any arbitrary-code execution.

The same code (T100) covers both because the underlying defence is
identical: validate / pin the value's shape *before* it reaches the
expression.  The emitted diagnostic message names the specific
hazard ("expr operand: numeric coercion" vs "eval: code injection")
so the appropriate remediation is obvious.

## Symptoms

- A yellow squiggle appears under the sink command, with one of:
  - "Tainted variable ${var} used in eval; possible code injection"
  - "Tainted variable ${var} flows into expr operand; numeric
    coercion may misinterpret value (use Tcl numeric-validation
    guards)"

## Example that triggers it

```tcl
set uri [HTTP::uri]
eval $uri
```

The analyser reports **`T100`** because `uri` carries tainted data into `eval`.

## Fix

```tcl
set uri [HTTP::uri]
# Validate or sanitise the input; avoid passing it to eval.
if {$uri in $allowed_commands} {
    eval $uri
}
```

## How to suppress

Add `# noqa: T100` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T101`, `T102`, `W300`
