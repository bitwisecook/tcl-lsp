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

### LSP analysis layer (`rust/tcl-compiler/src/analyser/`)

The analyser (`oo.rs`, `class_hierarchy.rs`) recognises `oo::class create` /
`oo::define` / `oo::objdefine` during static analysis, building `ClassDef`
entries in the semantic model.  These feed the `rust/tcl-lsp-core` providers:

- **Hover** (`hover.rs`) — class hierarchy, method signatures, inherited
  methods.
- **Go-to-definition** (`definition.rs`) — method bodies and class
  definitions, including the cross-file method paths (plan M6).
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
resolve_class`, mirroring C's `GetClassInOuterContext` — the plan-M4 one-hop
rule), not the `::oo::define` evaluation namespace.

## Test conformance

The behavioural suites live in `rust/tcl-vm/tests/cmd_oo_e2e.rs` (tclsh-pinned
end-to-end vectors) and the analyser's OO suites
(`rust/tcl-compiler` `analyser`/`oo` tests, `mro_lattice_adversarial.rs`).
Reference results captured from real tclsh 8.4–9.0 are queryable via the
`test-results` skill (`tests/test_reference/`, populated on demand).

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
