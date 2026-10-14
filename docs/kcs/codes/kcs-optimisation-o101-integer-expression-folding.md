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
- Skipped when the expression uses an operator or math function your Tcl
  version does not have — `**`, `in` and `ni` before 8.5, `lt`/`le`/`gt`/`ge`
  before 9.0, `max` or `entier` before 8.5 — because the program raises
  an error there. `W003` reports the operator.
- A floating-point result is written the way your Tcl version prints it:
  `1e-05` for Tcl 8.4 and Jim, `1e-5` from Tcl 8.5. It is skipped when that
  text would not read back as the same number — `1.0/3` under Tcl 8.4 and
  Jim (`0.333333333333`), and a few exact powers of two from 8.5, such as
  `2.0**64` (`1.844674407370955e+19`).
- Floating-point results are skipped under Tcl 8.4 to 8.6 when the file sets
  `tcl_precision`, or writes a variable whose name it computes.
- Skipped when the result is `-9223372036854775808` under Tcl 8.4 or 8.5,
  which print that computed value as `-8`.
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
