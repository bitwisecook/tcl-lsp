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

## Chaining `[self class]` onward: `namespace qualifiers` / `namespace tail`

`namespace qualifiers` and `namespace tail` split a *string* at its last `::`
— pure string arithmetic that never consults the interpreter's namespace table
— so they fold on any provably-constant word, including the `[self class]`
value the fold resolves first. That makes the real-corpus ticklecharts idiom a
single O129 rewrite:

```tcl
oo::class create ::ticklecharts::Gauge {
    method render {} { set ns [namespace qualifiers [self class]] }
}
```

becomes `set ns ::ticklecharts` (and `namespace tail` would give `Gauge`). The
edge cases are pinned byte-identical on tclsh 9.0.4 and 8.6.14 — `:::` → `{}` /
`{}`, `a:::b` → `a` / `b`, `::a::b::` → `::a::b` / `{}`, `::x:y` → `{}` /
`x:y`, and the empty string → `{}` / `{}` — by a unit table in the registry and
a `tclsh`-differential matrix replayed against both interpreters.

## Variable propagation inside a method body

The method-body walk now carries a constants map, but only over variables the
compilation unit can prove are **method-local**. An instance variable is object
state that outlives the frame: the constructor or any other method may have
written it, and a `my …` dispatch may rewrite it between two statements here.
So for this pass SCCP is re-run for the method with the class's instance
variables (its `variable` declarations, plus any the method itself makes with
`variable` / `my variable`) in its escaping set, which forces them — and
everything derived from them — to `Overdefined`. What is left over is a
private local, and a `my` / `next` / `[self …]` dispatch cannot reach it.

```tcl
oo::class create ::A {
    variable n
    method bump {} { incr n }
    method m {} { set n 1 ; my bump ; puts $n }   ;# `$n` never folds
    method k {} { set v 42  ; puts $v }           ;# `$v` folds to 42
}
```

One further whole-module gate, on the principle that an incomplete picture of
what a `my` / `next` dispatch can do means no method body's locals are
propagated anywhere in the module. It fires when any method body can reach its
caller's frame (an `upvar` at any level above the current one, or an `uplevel`
— counted whatever the dynamism of the names involved, since a computed name is
more dangerous, not less), or when any method was redefined (the lowering keeps
only the first body, so a replacement is invisible to every scan). A proc
callee that reaches its caller is modelled per call site; a method reached
through `my` is not, because the dispatch does not name it. `global` /
`variable` / `namespace upvar` reach a namespace rather than a caller frame and
deliberately do not trip the gate. See the "evidence rules" section of
`docs/design/compiler/sccp-core-analyses.md`.

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

- `rust/tcl-compiler/src/optimiser/propagation.rs` — `fold_builtin_cmd_subst_raw`, and `run_oo_method_folds` / `oo_frame_for` / `oo_context_fact_fold` / `oo_method_constants` / `methods_reach_caller_frames` for the method-frame half
- `rust/tcl-compiler/src/cfg_builder/upvar_info.rs` — `reaches_caller_frame` (the barrier's caller-frame evidence)
- `rust/tcl-compiler/src/lowering/mod.rs` — `Lowerer::class_instance_vars` (per-class instance-variable union across definition blocks)
- `rust/tcl-registry/src/commands/tcl/namespace_.rs` — `fold_qualifiers` / `fold_tail` and the oracle table
- `rust/tcl-registry/src/spec.rs` — `CommandSpec::oo_context_facts`, `OoContextFact` (with the oracle transcript)
- `rust/tcl-registry/src/commands/tcl/oo_self.rs` — `self`'s declaration

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O102`, `O116`, `O118`
