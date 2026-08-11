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

```
MemorySSAFunction
  alias_sets: list[AliasSet]
  memory_ops: list[MemoryOp]
  memory_phis: dict[BlockName, list[MemoryOp]]

AliasSet
  locations: frozenset[MemoryLocation]
  reason: str               # "upvar", "global", "variable"

MemoryLocation
  kind: MemoryLocationKind  # LOCAL | UPVAR | GLOBAL | NAMESPACE_VAR | ARRAY_ELEMENT | UNKNOWN
  name: str
  qualifier: str            # namespace, caller var, or array index

MemoryOp
  kind: MemoryOpKind        # DEF | USE | PHI | CLOBBER
  location: MemoryLocation
  version: int              # memory version number
  reaching_version: int     # for uses: which version reaches this point
  block: BlockName
  statement_index: int
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
| `may_alias(a, b)` | True if two variable names may refer to the same storage |
| `aliases_for(name)` | All alias sets containing the given variable name |
| `aliased_names` | Frozenset of all variable names involved in aliasing |
| `total_memory_defs` | Count of `DEF` memory operations |
| `total_memory_uses` | Count of `USE` memory operations |
| `total_clobbers` | Count of `CLOBBER` memory operations |

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

The `may_alias("counter", "counter")` query returns `True` — both the
local binding and global storage refer to the same variable.
