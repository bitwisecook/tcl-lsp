# KCS: O111 — Brace expression performance hints

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, lexing

## Profiles

readability, standard, full

## Question

What does O111 rewrite, and when does it fire?

## Why

Braced expressions compile to bytecode; unbraced ones are re-parsed on every call, which is slower and risks double substitution.

## Before

```tcl
expr $x + 1
```

## After

```tcl
expr {$x + 1}
```

O111 rewrites nothing itself. It rides on every
[`W100`](kcs-diagnostic-w100-unbraced-expression.md) the analyser reports,
adding an information-level note over the same range that explains the
performance cost; bracing the expression is your edit.

## Safety conditions

- Reported only where `W100` is. An expression already braced, or one the
  analyser does not read as an expression word, draws neither.

## How to disable

Set `tclLsp.optimiser.O111` to `false`, or turn the optimiser off entirely
with `tclLsp.optimiser.enabled`. See the
[optimiser feature](../features/kcs-feature-optimiser.md) for the profile
options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Lexing](../../GLOSSARY.md#lexing)
- Related codes: `O110`, `O115`
