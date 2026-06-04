# KCS: O129 — Fold pure builtin command substitutions with constant arguments

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, const-fold

## Profiles

standard, full

## Question

What does O129 rewrite, and when does it fire?

## Why

When a builtin command is called with all-constant arguments and produces a
deterministic result with no side effects, the optimiser can evaluate it at
compile time and replace the call with the resulting string literal. This
eliminates the command-dispatch overhead at runtime and makes the intent of the
code clearer. Commands covered include `string length`, `string toupper`,
`string tolower`, `join`, `format`, `dict get`, `dict size`, `list`, and
similar pure builtins.

## Before

```tcl
puts [string length abcde]
```

## After

```tcl
puts 5
```

## Safety conditions

- All arguments to the inner command must be compile-time constants (no
  variable references, no nested command substitutions whose values are
  unknown).
- The command must be a known pure builtin — user-defined procs and commands
  with observable side effects (such as `clock`, `rand`, `file`) are not
  folded.
- Skipped when the command substitution appears inside an unbraced expression
  that is itself not constant.
- The special cases `[list …]` and `[lindex …]` with all-constant arguments
  are handled by `O116` and `O118` respectively; those codes take precedence
  over O129 for those commands.

## How to disable

Toggle the optimiser profile in your editor settings. See the
[optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O102`, `O116`, `O118`
