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

Source: `compiler/ssa.py` (`build_ssa` at line 1012, `SSAPhi` at line 606, `SSAFunction` at line 648)

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

### Def-use chain derivation

After SSA construction, def-use chains can be built with
`build_def_use_chains(ssa)` from `compiler/def_use.py`.
The chains map each `SSAValueKey` to its definition site and all
use sites, enabling precise dead-store detection and copy propagation.

Phi nodes participate in def-use chains in two roles:
- **As definitions**: The phi's `(name, version)` is a `DefKind.PHI` def.
- **As uses**: Each incoming edge `(pred_block, incoming_ver)` is a
  `UseKind.PHI_INCOMING` use of the incoming version.

### Memory-SSA for aliases

Variables aliased via `upvar`, `global`, or `variable` are tracked by
memory-SSA (`compiler/memory_ssa.py`).  Memory-SSA versions each
store/load to aliased locations and builds alias sets, enabling
alias-aware optimisation and taint tracking.

### The dynamic-name barrier

Name-level SSA can only answer "is `x` defined here?" and "is this store to
`x` ever read?" while every access in the function *spells its target out*.
Tcl lets a program compute the name (`set $switch {}`, `lappend out [set
$name]`, `unset $n`), and the lowering deliberately declines to guess: such a
call stays a generic `Call` with an empty `defs` list.  Nothing in the SSA
graph then records that the name space was touched.

`rust/tcl-compiler/src/dynamic_names.rs` supplies the missing fact as a
per-function summary, `DynamicNameBarrier`, carried on `FunctionUnit`:

| flag | set by | what stops being provable |
|---|---|---|
| `writes` | a `VarWrite`-role argument whose name substitutes (`set $v 1`, `array set $a {…}`, `global $n`) | "`x` was never defined in this function" |
| `destroys` | the same, on a `DESTROYS_VARIABLE` command (`unset $n`) | "this parameter certainly exists" |
| `reads` | a `VarRead`-role argument whose name substitutes (`[set $v]`, `parray $a`), or a `PERFORMS_SUBSTITUTION` command over a non-braced template (`subst $tmpl`) | "this store is never read" |

Three points of design matter:

- **Flags, not a name set.**  A computed name can land anywhere, so
  enumerating candidates would be both unsound and unbounded.  The whole
  lattice is three bits, computed in one flow-insensitive walk, so each
  consumer pays `O(1)`.
- **The roles come from the registry.**  `ArgRole::VarWrite` /
  `ArgRole::VarRead` / the two traits are the only membership tests; the
  module names no command.
- **What is *not* a dynamic name.**  `a($k)` is a run-time element of the
  statically named array `a` (the [place model](../compiler/memory-ssa.md)
  already tracks that); `${ns}::tail` binds its static tail whatever the
  namespace resolves to; and a **brace-quoted** word is a literal name however
  many `$`s it holds — `set {$n} v` creates a variable *called* `$n` and
  leaves `n` alone (tclsh 9.0.4 / 8.6.14: `info exists {$n}` → 1 while
  `info exists n` → 0).  Treating any of them as a whole-name clobber would
  silence array, namespace, and ordinary diagnostics wholesale.  The
  brace-quoted case cannot be told from a substitution on the word's text
  alone, so the scan reads the per-word `braced_literal` / `argv_kinds` +
  `single_token_word` quoting facts rather than looking for a `$`.
- **A computed command *head* is not a computed name.**  `$cmd length foo`
  names an unknown command, so no argument of it has a knowable role — the
  walk skips such a call and raises no flag.  Skipping is load-bearing: a head
  word's text is its lexical content, so `$set` reads back as `set`, and
  resolving that would answer for a command that never runs.  Raising a flag
  instead would be wildly over-broad (`$obj method` and every TclOO dispatch
  would blind their whole function); unknown-command blindness is already
  `Statement::Barrier`'s job.

Consumers and the direction each abstains in:

| consumer | flag | abstention |
|---|---|---|
| `sccp::existence_constant_branches` → I230, O101 | `writes` (absent fold), `destroys` (present fold) | do not fold |
| read-before-set → W210 | `writes` | stay silent |
| dead store / unused → W211, W220 | `reads` | stay silent |
| `optimiser::elimination` → O109, O126 | `reads` | do not eliminate |

`eval $body` / `uplevel 1 $body` are deliberately **out of scope**: they run
arbitrary code, a strictly larger blindness than a computed name, and lower to
`Statement::Barrier` where the per-consumer barrier rules already apply.

### Known limitation — a brace-quoted `$`-bearing name is mis-keyed

A variable whose literal name contains a `$` (`set {$n} 1`) is recognised as
*not dynamic* by the barrier, but the SSA naming layer below still mis-keys
it. `is_dynamic_write_target` reads the name text without consulting the IR's
`name_braced` flag, so it calls `$n` a dynamic target: the assignment produces
no def, and `uses_of` records a read of `n` instead — surfacing as
`W210 Variable 'n' is read before it is set` on code that never touches `n`.

This is pre-existing and independent of the barrier. Making the target
braced-aware is *not* a local fix: `element_var_name_braced` normalises a
braced `$n` down to `n`, so the def would be recorded against the wrong
variable — measured, that trades the false positive for a *missed* one
(`set {$n} 1; puts $n` stops warning, though tclsh errors `can't read "n"`)
plus two fresh false `W220`s. A correct fix has to teach the shared naming
helper that a braced literal keeps its `$`, which reaches defs, reads, rename,
and highlighting together.

The shape is rare and the current verdict is a false positive rather than a
missed error, so it is recorded here rather than half-fixed.

## Related docs

- [Examples 5–9 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-5-if-x--set-y-10-)
- [GLOSSARY.md — SSA, Phi node, Dominator](../../GLOSSARY.md#ssa)
- [kcs-cfg-ssa-fact-model.md](../../../docs/design/compiler/cfg-ssa-fact-model.md)
- [kcs-cfg-construction.md](../../../docs/design/compiler/cfg-construction.md)
- [kcs-def-use-chains.md](../../../docs/design/compiler/def-use-chains.md)
- [kcs-memory-ssa.md](../../../docs/design/compiler/memory-ssa.md)
