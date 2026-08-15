# Memory-SSA

## Overview

Memory-SSA extends scalar SSA to track versioned memory operations for
variables that may be aliased through `upvar`, `global`, `variable`,
or `namespace upvar`.  It enables alias-aware optimisation and
diagnostic passes.

## Problem

Tcl's `upvar` command creates aliases between stack frames:

```tcl
proc foo {} {
    upvar 1 count local
    set local 42  ;# writes through to the caller's `count`
}
```

Without alias analysis, the compiler must conservatively assume any
store through an aliased variable might affect other variables.
Memory-SSA makes these relationships explicit.

The literal spelling is the one it can model.  A computed target
(`upvar 1 $varName local`) names a cell no static set can enumerate, and it
is handled by widening — see `has_wildcard_aliasing` below — rather than by
an alias set.

## Data Structure

```rust
pub struct MemorySsaFunction {
    pub alias_sets: Vec<AliasSet>,
    pub memory_ops: Vec<MemoryOp>,
    pub memory_phis: HashMap<String, Vec<MemoryOp>>,
    pub count_defs: usize,
    pub count_uses: usize,
    pub count_clobbers: usize,
    pub has_wildcard_aliasing: bool,
}

pub struct AliasSet {
    pub locations: BTreeSet<MemoryLocation>,
    pub reason: String,          // e.g. "caller-frame-cell"; comma-joined
                                 // and sorted when several paths merged
}

pub struct MemoryLocation {
    pub kind: MemoryLocationKind, // Local | Upvar | Global | NamespaceVar
                                  // | ArrayElement | InstanceVar | Unknown
    pub name: String,
    pub qualifier: String,        // namespace, caller var, or array index
}

pub struct MemoryOp {
    pub kind: MemoryOpKind,       // Def | Use | Phi | Clobber
    pub location: MemoryLocation,
    pub version: Version,
    pub reaching_version: Version, // for uses: which version reaches here
    pub block: String,
    pub statement_index: i32,
}
```

## Alias Detection

`compute_aliases` names no command.  It walks every SSA statement in
`BlockId` order, resolves the statement's `CommandTokens` through the
registry (`resolve_command_tokens`), and materialises an alias pair from
each declared `StateTransition::VariableCellAlias` fact.  The
`VariableAliasTarget` variant decides the location kinds and the `reason`
string:

| `VariableAliasTarget` | Written as | Alias created | `reason` |
|---|---|---|---|
| `Global { variable }` | `global varName` | `{Local(varName), Global(varName)}` | `global-cell` |
| `CurrentNamespace { variable }` | `variable varName` | `{Local(varName), NamespaceVar(varName)}` | `current-namespace-cell` |
| `Namespace { namespace, variable }` | `namespace upvar ns other my` | `{Local(my), NamespaceVar(other) qualified by ns}` | `namespace-cell` |
| `CallerSelectedFrame` with a global level | `upvar #0 g l` | `{Local(l), Global(g)}` | `global-frame-cell` |
| `CallerSelectedFrame` otherwise | `upvar ?level? other my` | `{Upvar(my) qualified by other, Upvar(other)}` | `caller-frame-cell` |

Overlapping pairs are merged with union-find (`AliasUnionFind`), so
`upvar 1 x a; upvar 1 x b` puts `a` and `b` in one set and the set's
`reason` becomes the sorted, comma-joined union of the contributing reasons.

Only **literal** subjects produce a pair: `TransitionSubject::literal`
returns `None` for a computed word, so `upvar 1 $param local` contributes no
alias set.  That loss is covered by `has_wildcard_aliasing` below rather than
by a guess.

## Memory Versioning

`build_memory_ssa` walks blocks in dominator-tree order with an explicit
stack and a single `version_counter`.  Versions increment at:

1. **Phi merge**: one memory phi per aliased variable that already has a
   scalar phi in that block.
2. **Clobber point**: a statement for which `is_clobber` or
   `transition_requires_wildcard` holds.
3. **Store to an aliased variable**: a `DEF` for each of the statement's
   `defs` whose name is in the alias sets.

Uses are recorded last, tagged with a `reaching_version` **snapshotted before
the statement's own defs bumped the counter** — a self-referential aliased
write (`upvar c x; set x [expr {$x + 1}]`) reads the incoming version, not
the one it is defining.

`is_clobber` is registry-driven too: a `Statement::Barrier` or
`Statement::UpFrame` always clobbers; a `Statement::Call` clobbers when its
resolved facts carry `Traits::EVALUATES_CODE` or `Traits::CREATES_BARRIER`,
or when its head is unresolved or its subcommand indeterminate.  No raw
command spelling is inspected as a fallback.

**Limitation — global version counter**: The implementation uses a single
`version_counter` for all memory locations within a function.  When a `DEF`
to variable `x` bumps the counter, a subsequent `USE` of unrelated variable
`y` records that bumped counter as its `reaching_version`.  This is correct
for consumers that use alias sets, but any consumer relying
on `reaching_version` for per-variable reaching-def analysis will get an
over-approximation.

## Key Methods and Properties

| Method / Property | Meaning |
|-------------------|---------|
| `aliased_names()` | `BTreeSet<String>` of every variable name involved in aliasing |
| `count_defs` | Count of `MemoryOpKind::Def` memory operations |
| `count_uses` | Count of `MemoryOpKind::Use` memory operations |
| `count_clobbers` | Count of `MemoryOpKind::Clobber` memory operations |
| `has_wildcard_aliasing` | Whether some statement's transition is unresolved or widened, so every location is potentially aliased |

## Consumer Contracts

Memory-SSA is **optional**: `FunctionUnit::memory_ssa` is `None` unless a
caller runs `with_memory_ssa`, so a consumer either asks for it or falls back
to a direct `compute_aliases` call.

| Consumer | What it reads | Why |
|----------|---------------|-----|
| O127 store-to-load forwarding (`optimiser/propagation.rs`) | `aliased_names()`, falling back to `compute_aliases` when the unit has none | Refuse to forward a store whose name — or any name the expression reads — may be visible through an alias |
| O127's intervening-effect gate | `statement_has_wildcard_aliasing` | An unresolved or widened transition between the store and the load kills the forward |
| Data-flow graph (`dataflow_graph.rs`) | `alias_sets` | `AliasInfo` rows in the `dataflow` view |
| Explorer / LSP graph payloads | `with_memory_ssa` then the above | `tcl-explorer/src/lib.rs`, `tcl-lsp-core/src/graphs.rs` |

## Module Location

- **Source**: `rust/tcl-compiler/src/memory_ssa.rs`, which resolves each
  statement's registry facts through
  `rust/tcl-compiler/src/registry_invocation.rs`.
- **Integration**: `FunctionUnit::with_memory_ssa` /
  `CompilationUnit::with_memory_ssa`
  (`rust/tcl-compiler/src/compilation_unit.rs`), called by
  `rust/tcl-explorer/src/lib.rs` and `rust/tcl-lsp-core/src/graphs.rs`.
- **Key APIs**: `build_memory_ssa`, `compute_aliases`, `has_wildcard_aliasing`,
  `is_clobber`, `statement_has_wildcard_aliasing`, and `MemorySsaFunction`.

## Example

```tcl
proc update_counter {} {
    global counter
    set counter [expr {$counter + 1}]
}
```

`global counter` carries a `VariableAliasTarget::Global` transition, so:

```
alias_sets:
  {counter, global(counter)}  reason=global-cell
memory_ops:
  DEF counter v1              ← the `global counter` statement's def
  DEF counter v2              ← the `set counter …` def
  USE counter reaching=v1     ← the `$counter` read, snapshotted pre-def
```

The `counter` locations are members of the same alias set, so consumers that
need a conservative alias guard retain the assignment.
