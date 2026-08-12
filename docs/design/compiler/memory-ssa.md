# KCS: Memory-SSA

## Overview

Memory-SSA extends scalar SSA to track versioned memory operations for
variables that may be aliased through `upvar`, `global`, `variable`,
or `namespace upvar`.  It enables alias-aware optimisation and
diagnostic passes.

## Problem

Tcl's `upvar` command creates aliases between stack frames:

```tcl
proc foo {varName} {
    upvar 1 $varName local
    set local 42  ;# writes through to caller's variable
}
```

Without alias analysis, the compiler must conservatively assume any
store through an aliased variable might affect other variables.
Memory-SSA makes these relationships explicit.

## Data Structure

```rust
pub struct MemorySsaFunction {
    /// Alias sets covering this function's aliased variables.
    pub alias_sets: Vec<AliasSet>,
    /// Memory operations in emission order.
    pub memory_ops: Vec<MemoryOp>,
    /// Block-indexed memory phi nodes.
    pub memory_phis: HashMap<String, Vec<MemoryOp>>,
    /// Number of `MemoryOpKind::Def` ops.
    pub count_defs: usize,
    /// Number of `MemoryOpKind::Use` ops.
    pub count_uses: usize,
    /// Number of `MemoryOpKind::Clobber` ops.
    pub count_clobbers: usize,
    /// Whether any invocation could change an unenumerated variable-cell
    /// identity through an unresolved or widened registry transition.
    pub has_wildcard_aliasing: bool,
}

pub struct AliasSet {
    /// Locations merged into this set. Ordered for stable output.
    pub locations: BTreeSet<MemoryLocation>,
    /// Why the set was formed — e.g. `"upvar"`, `"global-cell"`,
    /// `"namespace-cell"`, or a comma-separated sorted combination when
    /// several detection paths merged into the same set.
    pub reason: String,
}

pub struct MemoryLocation {
    /// Local | Upvar | Global | NamespaceVar | ArrayElement | InstanceVar | Unknown
    pub kind: MemoryLocationKind,
    /// Variable name.
    pub name: String,
    /// Location-specific context: the namespace (NamespaceVar), the
    /// caller-side variable name (Upvar), or the array index text
    /// (ArrayElement).
    pub qualifier: String,
}

pub struct MemoryOp {
    /// Def | Use | Phi | Clobber
    pub kind: MemoryOpKind,
    /// Location being read or written.
    pub location: MemoryLocation,
    /// New version assigned by this op (for Def/Phi/Clobber);
    /// for a Use, matches the reaching version.
    pub version: Version,
    /// Version of memory state reaching this read (Use only).
    pub reaching_version: Version,
    /// Block containing the op.
    pub block: String,
    /// Statement index within the block (`-1` for phi).
    pub statement_index: i32,
}
```

## Alias Detection

Alias detection is **registry-driven**: the module never infers a
variable-cell alias from a command spelling or a flattened argument layout.
It resolves each statement's command tokens
(`registry_invocation::resolve_command_tokens`), reads the declared
`StateTransition::VariableCellAlias` facts, and turns each one into a
`(local, target)` pair:

| `VariableAliasTarget` | Target location | Reason string |
|---|---|---|
| `Global { variable }` | `Global(variable)` | `"global-cell"` |
| `CurrentNamespace { variable }` | `NamespaceVar(variable)` | `"current-namespace-cell"` |
| `Namespace { namespace, variable }` | `NamespaceVar(variable)` qualified by `namespace` | `"namespace-cell"` |
| `CallerSelectedFrame { frame, variable }` | `Global(variable)` when the frame level parses as the global frame, otherwise the caller-frame alias | `"global-frame-cell"` / `"upvar"` |

So `global v`, `variable v`, `namespace upvar ns other local`, `upvar 1
other local`, and `upvar #0 g l` are all handled through the same path,
selected by the registry's declared transition rather than by name.

Only **literal** subjects produce a pair (`TransitionSubject::literal`):
a dynamic target such as `upvar 1 $param local` or `global $name` yields no
alias set. Where a transition is unresolved or widened, the function-level
`has_wildcard_aliasing` flag is raised instead.

## Memory Versioning

Memory versions increment at:
1. **Store to aliased variable**: a `Def` operation
2. **Clobber point**: a `Statement::Barrier`, or a command that may modify
   any aliased memory — `MemoryOp::new_clobber` uses the wildcard location
   `Unknown("*")`
3. **Phi merge**: where control flow merges and aliased variables have
   different versions on incoming edges

**Limitation — single version counter**: the implementation uses one
`version_counter` for all memory locations within a function.  When a `Def`
to variable `x` bumps the counter, a subsequent `Use` of unrelated variable
`y` records that bumped counter as its `reaching_version`.  This is correct
for consumers that use `may_alias` and alias sets, but any consumer relying
on `reaching_version` for per-variable reaching-def analysis gets an
over-approximation.

## Key Methods

| Method | Meaning |
|--------|---------|
| `MemorySsaFunction::may_alias(a, b)` | True if two variable *names* may refer to the same storage |
| `MemorySsaFunction::aliases_for(name)` | `Vec<&AliasSet>` — all alias sets containing the given variable name |
| `MemorySsaFunction::aliased_names()` | `BTreeSet<String>` of every variable name involved in aliasing |
| `AliasSet::may_alias(loc)` | True when a specific `MemoryLocation` is in the set |
| `count_defs` / `count_uses` / `count_clobbers` | Pre-computed O(1) summary counts (fields, not methods) |

## Consumer Contracts

| Consumer | What it reads | Benefit |
|----------|---------------|---------|
| GVN (O105) | `may_alias` | CSE across alias groups |
| DSE (O109) | `alias_sets`, `memory_ops` | Dead stores through aliases |
| LICM (O106) | `may_alias` | Hoist loads when no aliased store in loop |
| Taint analysis | `alias_sets` | Track taint through upvar/global |
| Data-flow graph | `alias_sets` | Alias edges in visualisation |

## Module Location

- **Source**: `rust/tcl-compiler/src/memory_ssa.rs`, with storage places in
  `rust/tcl-compiler/src/place.rs`.
- **Registry facts**: `StateTransition::VariableCellAlias` and
  `VariableAliasTarget` from `rust/tcl-registry/`, resolved through
  `rust/tcl-compiler/src/registry_invocation.rs`.
- **Integration**: `rust/tcl-compiler/src/compilation_unit.rs` and the common
  semantic analysis sidecar.
- **Key APIs**: `build_memory_ssa`, `MemorySsaFunction`, alias-set
  construction, and the `FunctionUnit::with_memory_ssa` /
  `CompilationUnit::with_memory_ssa` consumer path used by the compiler
  explorer and the diagnostic passes.

## Example

```tcl
proc update_counter {} {
    global counter
    set counter [expr {$counter + 1}]
}
```

Memory-SSA output:
```
alias_sets:
  {Local(counter), Global(counter)} reason="global-cell"
memory_ops:
  Def counter v1  (global declaration)
  Def counter v2  (set counter …)
```

`may_alias("counter", "counter")` returns `true` — both the local binding
and the global storage refer to the same variable.
