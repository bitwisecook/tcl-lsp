# KCS: SCCP and core analyses (Stage 6)

## Symptom

A contributor needs to understand how SCCP propagates constants, how
liveness analysis works, how the type lattice infers types, or why a
value is marked `OVERDEFINED` when it seems constant.

## Context

`analyse_function()` in `core_analyses.py` runs SCCP (Sparse Conditional
Constant Propagation) over the SSA graph, producing a `FunctionAnalysis`
with constant values, type information, liveness, dead stores, unreachable
blocks, constant branches, read-before-set, and unused variables.

Source: [`compiler/core_analyses.py`](../../../compiler/core_analyses.py) (`analyse_function` at line 1210, `FunctionAnalysis` at line 176),
[`compiler/types.py`](../../../compiler/types.py)

## Content

### SCCP — constant propagation

The SCCP value lattice:

```
UNKNOWN  ──►  CONST(value)  ──►  OVERDEFINED
 (bottom)      (provably constant)    (top / multiple possible values)
```

SCCP walks the SSA graph and propagates:
- `IRAssignConst(value="42")` → `CONST("42")`
- `IRAssignValue(value="${x}")` where `x₁ = CONST("42")` → `CONST("42")`
- Phi nodes: `phi(CONST("42"), CONST("42"))` → `CONST("42")`
- Phi nodes: `phi(CONST("42"), CONST("99"))` → `OVERDEFINED`
- Loop-carried values: always `OVERDEFINED` (value changes per iteration)

### Constant branch detection

When a `CFGBranch` condition evaluates to a constant:

```python
ConstantBranch(
    block="entry_1",
    condition="$x",
    value=True,
    taken_target="if_then_3",
    not_taken_target="if_next_4",
)
```

- The not-taken target is marked unreachable.
- O112 (constant condition elimination) is triggered.

### Unreachable blocks

Blocks that are never reached (due to constant branches, code after
`return`/`break`, etc.) are collected in `FunctionAnalysis.unreachable_blocks`.
Taint analysis and optimisation passes skip unreachable blocks.

### Type lattice

```
UNKNOWN  ──►  KNOWN(TclType)  ──►  SHIMMERED(from, to)  ──►  OVERDEFINED
```

| TclType | Values |
|---------|--------|
| `INT` | `"42"`, `"0xFF"` |
| `DOUBLE` | `"3.14"` |
| `BOOLEAN` | `"true"`, `"false"`, `"1"`, `"0"` |
| `STRING` | Any non-numeric text |
| `LIST` | Tcl list format |
| `DICT` | Tcl dict format |
| `NUMERIC` | Abstract join of INT and DOUBLE |

`SHIMMERED(from_type, to_type)` tracks forced type conversions — used by
the shimmer detector (S100–S102).

### Liveness analysis

`live_in[block]` / `live_out[block]` — sets of `SSAValueKey` that are
"live" (may still be read) at each block boundary.

A value is dead if it is defined but never appears in any `live_out` set.
Dead values trigger:
- O109 (dead store elimination) — variable set but never read
- O108 (aggressive DCE) — pure statement result never used

### Dead store detection

If `x₁ = "42"` and `x₁` never appears in any `uses` dict, it is a dead
store.  SCCP marks it in `FunctionAnalysis.dead_stores`.

### Read-before-set

If a variable is read at version 0 (never defined before use), it appears
in `FunctionAnalysis.read_before_set` → diagnostic W103.

### Unused variables

Variables that are defined but never read (across all versions) appear in
`FunctionAnalysis.unused_variables` → diagnostic W104.

### Worked example — `set x 5; if {$x < 0} {…} elseif {$x > 0} {…} else {…}`

SCCP determines `x₁ = CONST("5")`:
- `5 < 0` → `CONST(false)` → `if_then_3` unreachable
- `5 > 0` → `CONST(true)` → `if_then_5` taken, `if_next_6` unreachable
- `sign` resolves to `CONST("1")` (only one reachable definition)

### Worked example — `while {$i < 5} { incr i }`

- `i₁ = CONST("0")` (before loop)
- `i₂ = phi(i₁, i₃)` at loop header → `OVERDEFINED` (loop-carried)
- SCCP cannot fold loop induction variables

## Decision rule

- If a value should be constant but is `OVERDEFINED`, check whether a
  loop phi or barrier is widening it.
- Pure commands can be inferred through without invalidating the lattice.
  Impure commands force all potentially affected values to `OVERDEFINED`.
- Liveness is computed backward from uses to definitions — if a new IR
  node reads variables, ensure they appear in `SSAStatement.uses`.
- SCCP runs once per function (no iterative refinement across functions —
  that is interprocedural analysis).

## Related docs

- [Examples 3–7 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-3-expr-2--3)
- [GLOSSARY.md — SCCP, Lattice, Liveness](../../GLOSSARY.md#sccp)
- [kcs-cfg-ssa-fact-model.md](../../../docs/design/compiler/cfg-ssa-fact-model.md)
- [kcs-downstream-pass-contracts.md](../../../docs/design/compiler/downstream-pass-contracts.md)
