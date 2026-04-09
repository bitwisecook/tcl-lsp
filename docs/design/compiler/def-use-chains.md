# KCS: Def-Use Chains

## Overview

Def-use chains link each SSA variable definition to all statements that
read (use) it.  They are built in two passes over the SSA function
after SSA construction, and stored in `FunctionAnalysis.def_use_chains`.

## Data Structure

```
DefUseResult
  chains: dict[SSAValueKey, DefUseChain]

DefUseChain
  key: SSAValueKey          # (variable_name, ssa_version)
  definition: DefSite       # where the value is defined
  uses: list[UseSite]       # all sites that read it

DefSite
  block: BlockName
  kind: DefKind             # STATEMENT | PHI | PARAMETER
  statement_index: int      # -1 for phi/parameter

UseSite
  block: BlockName
  kind: UseKind             # OPERAND | PHI_INCOMING | TERMINATOR
  statement_index: int
  variable: str             # for phi: the phi variable
  phi_version: int          # for phi: the phi's version
```

## Derivation from SSA

1. **Pass 1 — definitions**: Walk every block.  For each phi node,
   record `(name, phi.version)` with `DefKind.PHI`.  For each
   statement, record each `(name, ver)` in `stmt.defs` with
   `DefKind.STATEMENT`.

2. **Pass 2 — uses**: Walk every block again.  For each phi incoming
   edge `(pred_block, incoming_ver)`, record a `UseKind.PHI_INCOMING`
   use.  For each statement operand `(name, ver)` in `stmt.uses`,
   record a `UseKind.OPERAND` use.

3. **Version 0**: If a use references version 0 (read-before-set), the
   chain is lazily created with `DefKind.PARAMETER` in the entry block.

## Key Properties

| Property / Method | Meaning |
|-------------------|---------|
| `chain.is_dead` | No uses at all — candidate for dead-store elimination |
| `chain.use_count` | Number of use sites |
| `chain.has_phi_use` | At least one use is a phi incoming edge |
| `result.is_dead(name, ver)` | True if the SSA value has no uses |
| `result.uses_of(name, ver)` | All use sites for a given SSA value (or empty list) |
| `result.reaching_defs(name)` | All SSA definitions of a variable across the function |

## Consumer Contracts

| Consumer | What it reads | What it produces |
|----------|---------------|-----------------|
| Dead store detection | `chain.is_dead` | Improved `DeadStore` precision |
| Unused variable detection | `chain.use_count == 0` | Improved W213/W214 |
| Copy propagation (O127) | Single-def chains | Optimisation suggestions |
| Data-flow graph | All chains | Visualisation nodes and edges |
| Compiler explorer | Per-function chains | JSON for Data Flow tab |

## Module Location

- **Source**: `core/compiler/def_use.py`
- **Integration**: `core/compiler/core_analyses.py` (built in `analyse_function`)
- **Graph export**: `core/compiler/dataflow_graph.py`

## Example

Given:
```tcl
set x 1
set y [expr {$x + 1}]
```

Chains:
- `(x, 1)`: def=STATEMENT in entry, uses=[(OPERAND in entry, stmt 1)]
- `(y, 1)`: def=STATEMENT in entry, uses=[] → **DEAD**

With branching:
```tcl
if {$cond} { set a 1 } else { set a 2 }
set b $a
```

Chains:
- `(a, 1)`: def=STATEMENT in if_true, uses=[(PHI_INCOMING → a#3)]
- `(a, 2)`: def=STATEMENT in if_else, uses=[(PHI_INCOMING → a#3)]
- `(a, 3)`: def=PHI in if_next, uses=[(OPERAND in if_next)]
- `(b, 1)`: def=STATEMENT in if_next, uses=[] → **DEAD**
