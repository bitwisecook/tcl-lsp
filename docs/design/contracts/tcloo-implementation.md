# TclOO implementation

## Summary

The TclOO subsystem provides class hierarchy analysis for the LSP and runtime
execution in the bytecode VM.  It covers `oo::class create`, `oo::define`,
`oo::objdefine`, constructors, destructors, methods, mixins, filters, private
variables/methods (TIP 500), properties (TIP 558), and configurable support.

## Architecture

### Registry-driven definer grammar (`tcl-registry`)

Per `AGENTS.md`, member **recognition** and **argument layout** come entirely
from the definer's `definition_body` grammar (`tcl-registry/src/definer.rs`):
member sub-keywords (`method`, `constructor`, `variable`, …) with their
body / param / var layout (`MemberKind::Flat`), nested-member wrappers
(`self`, itcl's access modifiers — `MemberKind::Wrapper`), and flag-keyed
forms (`property` — `MemberKind::FlagKeyed`).  TclOO, snit, and [incr Tcl]
are pure registry data; the shared walkers hold no member-keyword lists.

The list-valued definition words — `filter`, `superclass`, `mixin`,
`variable` — are **slots** (`oo::Slot` instances in real Tcl), not
assignments, and their behaviour is registry data too
(`MemberSpec::slot`, a `SlotSpec`): each slot carries its
C-pinned default operation (`-append` for `filter`/`variable`, `-set` for
`superclass`/`mixin` — identical in 8.6.16 and 9.0.4) plus a dedup rule
(`variable` dedups, `filter` keeps duplicates), and the explicit
`-set` / `-append` / `-appendifnew` / `-prepend` / `-remove` / `-clear`
operations fold through the single `SlotSpec::apply` fold.  Every consumer
(the instance `filters`, the class-object `class_filters`, `superclasses`,
`mixins`, and declared `variables`) takes the identical fold, so `filter a;
filter b` keeps both live and `superclass -append B` extends rather than
being dropped as a flag.  Known limit: an unrecognised leading `-op` word
aborts the whole definition in real Tcl; the fold conservatively leaves the
slot unchanged rather than modelling the abort.

### Member arguments that name commands (`record_member_command_references`)

A member argument that *names a command* is a first-class command reference,
recorded during the class-body walk so navigation reaches it exactly as a
direct call does.  Which arguments name commands is registry data, never a
member keyword the walker knows by name:

- `MemberSpec::all_args_ref == Some(MemberRefKind::Class)` — every argument is
  a class (`superclass A B`, `mixin ?-append? M …`, `[incr Tcl]`'s
  `inherit Base`).  A class is a command in `TclOO`, so each resolves in the
  referencing class's namespace (the one-hop call-site rule) and is recorded as
  a `command_invocation`.
- an `ArgRole::CommandName` position — one argument names a command
  (`forward NAME TARGET …`: the delegated command).

The recorder (`analyser/oo.rs::record_member_command_references`) unwraps a
`MemberKind::Wrapper` prefix (`self mixin …`) and skips flags (`-append`) and
dynamic names (`superclass $base`).  Because these references land in the same
`command_invocations` collection as ordinary calls, find-references, rename,
go-to-definition, and call-hierarchy resolve them across files through the
workspace index — and rename and references can never disagree about a
`superclass` / `mixin` / `inherit` site.

### LSP analysis layer (`rust/tcl-compiler/src/analyser/`)

The analyser (`oo.rs`, `class_hierarchy.rs`) recognises `oo::class create` /
`oo::define` / `oo::objdefine` during static analysis, building `ClassDef`
entries in the semantic model.  These feed the `rust/tcl-lsp-core` providers:

- **Hover** (`hover.rs`) — class hierarchy, method signatures, inherited
  methods.
- **Go-to-definition** (`definition.rs`) — method bodies and class
  definitions, including the cross-file method paths.
- **Completion** (`completion.rs`) — methods in `my` and `self` contexts.
- **Type hierarchy** (`type_hierarchy.rs`) — supertypes and subtypes, from
  the owner-aware class-hierarchy index.
- **Folding + semantic tokens** — the shared `oo_body.rs` walker, dispatching
  on `MemberKind`, never a keyword.

### MRO algorithm (`rust/tcl-syntax/src/mro.rs`)

Method resolution order uses a linearisation matching C Tcl's algorithm.  It
lives in `tcl-syntax` so the analyser and the bytecode VM share one
implementation without the VM depending on the compiler.

### VM runtime layer (`rust/tcl-vm/src/cmd_oo.rs`)

The VM manages the object/class registry at runtime:

- **Object lifecycle** — creation, destruction, per-object instance
  namespaces (`oo::Obj<N>`).
- **Method dispatch** — walks the shared MRO, applies filter chains, handles
  `next`/`nextto`.
- **Variable binding** — `my variable`, `my varname`, private variable
  mangling using creation IDs (TIP 500).
- **Introspection** — `info object` / `info class` subcommands
  (`rust/tcl-vm/src/cmd_info.rs`).

Class name resolution during `oo::define` body evaluation resolves relative
names in the namespace where `oo::define` was invoked (`cmd_oo.rs::
resolve_class`, mirroring C's `GetClassInOuterContext` — the one-hop
call-site rule), not the `::oo::define` evaluation namespace.

## Test conformance

The behavioural suites live in `rust/tcl-vm/tests/cmd_oo_e2e.rs` (tclsh-pinned
end-to-end vectors) and the analyser's OO suites
(`rust/tcl-compiler` `analyser`/`oo` tests, `mro_lattice_adversarial.rs`).
Reference results captured from real tclsh 8.4–9.0 are queryable via the
`test-results` skill (`tests/test_reference/<version>/`, written by
`scripts/capture/test_results.sh` on demand — not checked in).

## Key files

| File | Role |
|------|------|
| `rust/tcl-vm/src/cmd_oo.rs` | OO runtime (object/class registry, dispatch, define body parsing) |
| `rust/tcl-vm/src/cmd_info.rs` | `info object` / `info class` introspection |
| `rust/tcl-syntax/src/mro.rs` | MRO linearisation (shared analyser ↔ VM) |
| `rust/tcl-compiler/src/analyser/oo.rs` | Static OO analysis (class/method extraction) |
| `rust/tcl-compiler/src/analyser/class_hierarchy.rs` | Owner-aware hierarchy index + one-hop class resolution |
| `rust/tcl-registry/src/definer.rs` | Definer body grammars (TclOO / snit / itcl as data) |
| `rust/tcl-lsp-core/src/oo_body.rs` | Shared member walker (folding, semantic tokens) |
