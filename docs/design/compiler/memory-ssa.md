# Memory-SSA

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
    pub reason: String,          // "upvar", "global", "variable"
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

The module scans SSA statements for aliasing commands:

| Command | Detection | Alias Created |
|---------|-----------|---------------|
| `upvar ?level? otherVar myVar` | Literal args only | `{myVar, otherVar}` |
| `namespace upvar ns otherVar myVar` | Literal args only | `{myVar, otherVar}` |
| `global varName` | All args | `{varName(local), varName(global)}` |
| `variable varName` | All args | `{varName(local), varName(namespace)}` |

Dynamic `upvar` targets (e.g. `upvar 1 $param local`) are not tracked —
the variable scan in `_detect_upvar` skips `$`-prefixed names.

## Memory Versioning

Memory versions increment at:
1. **Store to aliased variable**: `DEF` operation
2. **Clobber point**: `IRBarrier` or commands like `eval`/`uplevel`
3. **Phi merge**: Where control flow merges and aliased variables
   have different versions on incoming edges

**Limitation — global version counter**: The implementation uses a single
`version_counter` for all memory locations within a function.  When a `DEF`
to variable `x` bumps the counter, a subsequent `USE` of unrelated variable
`y` records that bumped counter as its `reaching_version`.  This is correct
for consumers that use `may_alias()` and alias sets, but any consumer relying
on `reaching_version` for per-variable reaching-def analysis will get an
over-approximation.

## Key Methods and Properties

| Method / Property | Meaning |
|-------------------|---------|
| `may_alias(a, b)` | `true` if two variable names may refer to the same storage |
| `aliases_for(name)` | All alias sets containing the given variable name |
| `aliased_names()` | `BTreeSet<String>` of every variable name involved in aliasing |
| `count_defs` | Count of `MemoryOpKind::Def` memory operations |
| `count_uses` | Count of `MemoryOpKind::Use` memory operations |
| `count_clobbers` | Count of `MemoryOpKind::Clobber` memory operations |
| `has_wildcard_aliasing` | Whether a statement made every location potentially aliased |

## Consumer Contracts

| Consumer | What it reads | Benefit |
|----------|---------------|---------|
| GVN (O105) | `may_alias()` | CSE across alias groups |
| DSE (O109) | `alias_sets`, `memory_ops` | Dead stores through aliases |
| LICM (O106) | `may_alias()` | Hoist loads when no aliased store in loop |
| Taint analysis | `alias_sets` | Track taint through upvar/global |
| Data-flow graph | `alias_sets` | Alias edges in visualisation |

## Module Location

- **Source**: `rust/tcl-compiler/src/memory_ssa.rs`, with storage places in
  `rust/tcl-compiler/src/place.rs`.
- **Integration**: `rust/tcl-compiler/src/compilation_unit.rs` and the common
  semantic analysis sidecar.
- **Key APIs**: `MemorySsaFunction`, alias-set construction, and the
  `with_memory_ssa()` consumer path used by Explorer and diagnostics.

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
  {counter(local), counter(global)} reason=global
memory_ops:
  DEF counter v1  (global declaration)
  DEF counter v2  (set counter ...)
```

The `may_alias("counter", "counter")` query returns `true` — both the
local binding and global storage refer to the same variable.
