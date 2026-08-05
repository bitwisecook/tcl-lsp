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
  class: UseClass           # SUBSTITUTED | QUOTED
```

## Use classification (`UseClass`)

A use is **classified**, not merely present or absent, because the two
families of consumer need opposite conservatism about the same word.

Tcl substitutes `$name` in a bare or `"`-quoted word and never in a
brace-quoted one: `puts {$y}` prints the two characters `$y` and reads
nothing.  A braced word's contents may still be *evaluated* later — by
`expr`, by `if`, by an `after` callback, by an unknown definer — but when,
and in which frame, is the callee's business.

| `UseClass` | Meaning | Who honours it |
|---|---|---|
| `SUBSTITUTED` | Substituted at this site, or evaluated by the callee in this same frame (an `ArgRole::Expr` / `ArgRole::Body` word) — a genuine read here | everyone |
| `QUOTED` | Carried only by a brace-quoted word this site passes through verbatim | liveness / dead-store only |

Liveness, W211, W220 and store elimination must assume a quoted word *may*
be evaluated, so the use has to exist.  Read-before-set (W210 / W213) must
assume it *may not* be, or may be evaluated in a frame that binds the name,
so it skips `QUOTED` uses.  Filtering at either end breaks the other:
dropping the use resurrects `W211 set but never used` on `set a(k) 1; puts
{$a(k)}`, and recording the name as a self-initialising def deletes the
feeding store outright (issues #1142, #1237).

Which roles keep a braced word `SUBSTITUTED` is registry data, not a
command list: `ArgRole::braced_word_evaluated_in_frame` is the single
answer, and an un-roled position — including every position of a command
the registry does not describe — falls to `QUOTED`.

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

- **Source**: `compiler/def_use.py`
- **Integration**: `compiler/core_analyses.py` (built in `analyse_function`)
- **Graph export**: `compiler/dataflow_graph.py`

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
