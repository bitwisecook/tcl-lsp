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

## Consumer Contracts

| Consumer | What it reads | Benefit |
|----------|---------------|---------|
| GVN (O105) | `may_alias()` | CSE across alias groups |
| DSE (O109) | `alias_sets`, `memory_ops` | Dead stores through aliases |
| LICM (O106) | `may_alias()` | Hoist loads when no aliased store in loop |
| Taint analysis | `alias_sets` | Track taint through upvar/global |
| Data-flow graph | `alias_sets` | Alias edges in visualisation |

## Module Location

- **Source**: `core/compiler/memory_ssa.py`
- **Integration**: `core/compiler/core_analyses.py` (built in `analyse_function`)
- **Key functions**: `compute_aliases()`, `build_memory_ssa()`

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
