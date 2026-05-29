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

### Existence-check folding (`info exists` / `array exists`)

`_ExistenceFolder` rewrites `[info exists X]` / `[array exists X]`
sub-expressions in a branch condition to a `0`/`1` literal before the branch
decision is taken, so an existence guard becomes an ordinary constant branch
(feeding `I230` + DCE):

- **absent** — a use at SSA version 0 of a plain local scalar that is not a
  parameter and not frame-escaping (`global`/`variable`/`upvar`) → fold to
  `0`. (`array exists` also folds `0`.)
- **present** — a use whose reaching version is a real assignment, or a
  parameter → `info exists` folds to `1` (a scalar `set` does not prove
  `array exists`, so that stays unfolded).
- **`unset`** version → fold to `0`.

The fold is gated per function: it is disabled whenever any statement could
create or destroy a local invisibly (any barrier/`IRBlock`/`IRUpFrame`, an
`IRCall` with an UNKNOWN-target write, a dynamic barrier, or an inline `BODY`
argument). Array elements (`A(k)`) and qualified names (`::ns::X`) are never
folded. This keeps the rewrite sound for DCE/codegen.

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

Existence checks are excluded: `info exists X` / `array exists X` test a
variable rather than reading its value, so the check reference itself is never
a read-before-set. A check also narrows the region it dominates —
`_existence_narrowed_blocks` records, for each block, the names a dominating
guard proves to exist (the true region of `if {[info exists X]}`, or the false
region of a negated guard), and reads of those names there are suppressed. The
opposite branch keeps version 0, so a read there is still flagged.

`_existence_implications` recognises the membership idioms too, all restricted
to a single exact (non-glob) name: `[info vars X]` / `[info locals X]`
compared with `""` (`ne`/`eq`), `[llength [info vars X]]`, and
`[lsearch [info vars] X] > -1` / `>= 0` / `!= -1`. `info globals` is *not*
narrowed (it proves the global exists, not the bare-`$X` local), and unsound
`lsearch` options (`-regexp`, `-nocase`, …) are rejected. Narrowing is a
runtime fact (the guard passed), so unlike the fold it needs no foldability
gate.

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
