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

## Inside a TclOO method body: `[self class]`

O129 also answers the one introspection keyword whose value the *enclosing
method frame* fixes rather than its arguments — `self class`, the class that
defines the running method implementation:

```tcl
oo::class create ::ticklecharts::C {
    method render {} { puts [self class] }
}
```

becomes `puts ::ticklecharts::C`. The value is the class whose definition body
lexically encloses the method, and real Tcl keeps it that class through
inheritance, mixins, and `next` — each link of a `next` chain reports its own
definer, and a mixin-provided method reports the mixin, not the receiver
(verified identical on tclsh 9.0.4 and 8.6.16). Constructor and destructor
bodies fold the same way, as do methods added by `oo::define` (which fold to
the class `oo::define` names) and classes created under `namespace eval`.

This is registry-driven: `self`'s spec declares which of its words is a frame
fact (`oo_context_facts`), so the optimiser never matches a command or
subcommand by name.

Four shapes deliberately do **not** fold, because the value is either not
static or not a value at all:

- **`self object` / `self namespace`** (and a bare `self`) name the *receiving
  instance* — a fresh `::oo::ObjNN` per `new`. The chain words (`method`,
  `call`, `caller`, `filter`, `next`, `target`) are reshaped at run time.
- **A method on the class object** — `self method NAME …`, `classmethod`, or an
  `oo::objdefine` instance method. There `self class` *raises*
  ("method not defined by a class"), so there is no name to fold to.
- **A renamed class command.** `self class` answers with the class's *current*
  name, so a module containing `rename ::R ::R2` anywhere makes every
  `[self class]` written under `::R` unfoldable.
- **A dynamic class name** (`oo::class create $nm { … }`), which the compiler
  never resolves to a class in the first place.

The method-body fold carries no constants map, so it introduces no variable
propagation inside a method: an instance variable is object state that outlives
the frame and any `my …` call may rewrite it, which the per-function lattice
does not model.

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

## File-path anchors

- `rust/tcl-compiler/src/optimiser/propagation.rs` — `fold_builtin_cmd_subst_raw`, and `run_oo_method_folds` / `oo_frame_for` / `oo_context_fact_fold` for the method-frame half
- `rust/tcl-registry/src/spec.rs` — `CommandSpec::oo_context_facts`, `OoContextFact` (with the oracle transcript)
- `rust/tcl-registry/src/commands/tcl/oo_self.rs` — `self`'s declaration

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O102`, `O116`, `O118`
