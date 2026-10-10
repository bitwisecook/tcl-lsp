# KCS: O101 — Fold constant integer expressions

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O101 rewrite, and when does it fire?

## Why

Evaluating arithmetic at compile time avoids runtime computation, producing smaller, faster code.

## Before

```tcl
puts [expr {2 + 3}]
```

## After

```tcl
puts 5
```

## Safety conditions

- Skipped when any operand is not a compile-time constant.
- A nested command must have a supported evaluator and stable binding. An
  unsupported command or an observable write prevents an effect-free fold.
- Skipped when a leading-zero operand (`010`) is ambiguous between octal and decimal and the document's Tcl dialect/version isn't known.
- Skipped on a domain error, such as `0.0/0.0` or a negative exponent of `0`.
- Skipped when `expr` itself, or a math function used in the expression (`abs`, `sqrt`, …), has been renamed, aliased, or redefined by a `proc`.
- Skipped when a variable's value can't be proven unchanged across an intervening call — for example, a helper procedure that writes the same global/namespace variable, or a variable under a `trace` installed anywhere in the file.

## Standalone expression limitation

A standalone `expr {2 + 3}` is currently rewritten to the bare command `5`,
which Tcl cannot execute. The safe example above folds a command
substitution inside `puts`. For code with standalone expressions, disable
O101 until the command wrapper is preserved. See the
[precision limitations](../../design/compiler/precision-limitations.md#open--a-standalone-expression-fold-drops-its-command-wrapper).

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O100`, `O102`
