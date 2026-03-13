# KCS: SSA construction (Stage 5)

## Symptom

A contributor needs to understand how the SSA builder assigns version numbers,
places phi nodes, or builds the dominator tree, or is debugging incorrect
variable versioning or missing phi nodes.

## Context

`build_ssa()` in `ssa.py` transforms a `CFGFunction` into an `SSAFunction`
where every variable definition gets a unique version number.  Phi nodes are
placed at dominance frontiers to merge definitions from different paths.
The dominator tree and dominance frontiers are computed as part of SSA
construction.

Source: [`core/compiler/ssa.py`](../../../core/compiler/ssa.py) (`build_ssa` at line 359, `SSAPhi` at line 168, `SSAFunction` at line 210)

## Content

### SSA principles

- Every variable definition gets a unique `(name, version)` — the
  `SSAValueKey`.
- A variable read resolves to the version currently in scope at that
  program point.
- At merge points (where multiple paths converge), phi nodes select the
  correct version based on which predecessor executed.

### Version numbering

- Version 0 = read-before-set (the variable was used without being defined).
  This triggers diagnostic W103.
- Version 1, 2, 3, ... = successive definitions in program order.
- Inside loops, the phi node at the header produces a new version that
  merges the initial value with the loop-carried update.

### Phi node placement

Phi nodes are placed at dominance frontier blocks — where a variable's
dominance "ends":

```
  entry_1:  x₁ = "1"
  branch → if_then_3 / if_next_4

  if_then_3:  x₂ = "10"  → if_end_2
  if_next_4:  (no def of x) → if_end_2

  if_end_2:
    phi: x₃ = phi(x₂ from if_then_3, x₁ from if_next_4)
```

`if_end_2` is in the dominance frontier of `if_then_3` for variable `x`.

### Dominator tree

Block A dominates block B if every path from entry to B passes through A.
The immediate dominator (`idom`) is the closest dominator.

```python
SSAFunction.idom = {
    "entry_1": None,
    "if_then_3": "entry_1",
    "if_next_4": "entry_1",
    "if_end_2": "entry_1",
    "exit_5": "if_end_2",
}
```

### Loop phis

Loops create phi nodes at the loop header:

```
  while_header_3:
    phi: i₂ = phi(i₁ from entry_1, i₃ from while_body_4)
    branch uses: {i: 2}

  while_body_4:
    IRIncr(i) → i₃ = i₂ + 1
```

The phi merges the initial value (`i₁ = 0`) with the loop-carried update
(`i₃`).  SCCP cannot fold loop-carried values to constants — they become
`OVERDEFINED`.

### Multi-way merge (if/elseif/else)

```
  if_end_2:
    phi: sign₄ = phi(sign₁ from if_then_3,
                      sign₂ from if_then_5,
                      sign₃ from if_next_6)
```

Three definitions merge — the phi has three incoming edges.

### SSA data structures

| Type | Fields |
|------|--------|
| `SSAStatement` | `statement` (original IR), `uses dict[name→version]`, `defs dict[name→version]` |
| `SSAPhi` | `name`, `version` (produced), `incoming dict[block→version]` |
| `SSABlock` | `phis tuple`, `statements tuple`, `entry_versions`, `exit_versions` |
| `SSAFunction` | `blocks dict`, `idom dict`, `dominance_frontier dict`, `dominator_tree dict` |

## Decision rule

- If a new IR node defines variables, ensure it appears in `defs` of the
  `SSAStatement` so the SSA builder tracks it.
- Version 0 reads indicate potential read-before-set — check that the
  variable is legitimately undefined (not a cross-event flow in iRules).
- Phi nodes are only placed where needed (at dominance frontiers), not at
  every merge point — this keeps the IR sparse.
- The dominator tree is essential for many analyses (SCCP, liveness) — if
  blocks are reachable but have no idom, check CFG connectivity.

## Related docs

- [Examples 5–9 in walkthroughs](../../example-script-walkthroughs.md#example-5-if-x--set-y-10-)
- [GLOSSARY.md — SSA, Phi node, Dominator](../../GLOSSARY.md#ssa)
- [kcs-cfg-ssa-fact-model.md](kcs-cfg-ssa-fact-model.md)
- [kcs-cfg-construction.md](kcs-cfg-construction.md)
