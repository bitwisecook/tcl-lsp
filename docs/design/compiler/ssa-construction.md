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

### The caller-frame injection model

A computed name is one way to lose the name space.  The other is to have
somebody *else* write it.  Tcl's frame-crossing primitives make a callee's
writes land in the **caller's** frame, and the caller's own text shows
nothing:

```tcl
proc setdef {d key args} { upvar 1 $d _dict; dict set _dict $key … }
proc build {} { setdef options name -type str; return [dict get $options name] }
```

`options` is never assigned in `build`, yet it is set twice before the read.
`rust/tcl-compiler/src/cfg_builder/upvar_info.rs` supplies the missing fact
as a per-procedure **frame-effect summary** (`UpvarInfo` — the name is kept
because the salsa interning layer imports it by that path):

| bucket | shape | resolved |
|---|---|---|
| `literal_targets` | `upvar 1 caller_x x` | at summary time |
| `param_targets` | `upvar 1 $param x` | at the call site, from the argument passed for `param` |
| `args_tail_upvar` | `upvar 1 $args x` | at the call site, positionally |
| `uplevel_literal_writes` | `uplevel 1 {set n …}`, `uplevel 1 [list set n …]` | at summary time |
| `uplevel_param_writes` | `uplevel 1 [list set $param …]` | at the call site, like `param_targets` |
| `uplevel_forwarded_calls` | `uplevel 1 [list callee …]` | one hop, in `detect_upvar_procs` |
| `has_unresolvable_caller_target` | `upvar 1 $computed x` | not resolvable — widen |
| `caller_frame_opaque_writes` / `_reads` | `uplevel 1 $body`, `upvar 2 …`, `upvar $lvl …` | not resolvable — widen |

**Complexity.**  One walk per procedure to build the summary; one hash
lookup plus work proportional to the *bound arguments* at each call site.
The forwarded-call hop composes against the **own-body** summaries, never
the composed ones, so a recursive or mutually-recursive forward cannot
diverge — a single level by construction, no fixpoint and no iteration cap.

**Frame targeting is the whole game.**  Only a `Relative(1)` level (or an
omitted one) reaches the direct caller.  Which argument is the level word,
and how to read it, is registry data —
[`FrameEffectSpec`](../../../rust/tcl-registry/src/frame_effect.rs) on
`CommandSpec`, so no consumer names `upvar` or `uplevel`:

| written in the callee | frame written | contributed |
|---|---|---|
| `upvar 1 x y` / `upvar x y` | the caller | a named binding |
| `upvar #0 g l` | the global frame | nothing — `global_write_info` owns it |
| `upvar 0 x y` | the callee's own frame | nothing |
| `upvar 2 far f`, `upvar $lvl a b` | somewhere further out | widen |
| `uplevel 1 $body` | the caller | opaque write **and** read |
| `uplevel 0 $body` / `eval $body` | the callee's own frame | the callee's own barrier |

C Tcl decides whether `upvar`'s level word is present from the **argument
count parity** (`Tcl_UpvarObjCmd` tests `objc`), never from the word's text.
Three consumers had each re-derived that by sniffing for digits or `#`, and
two of them were wrong: `upvar $lvl a b` has three words, so `$lvl` *is* the
level and `(a, b)` is the pair, but a text sniff sees no level and pairs
`($lvl, a)` — losing the commonest by-reference binding of all. Pinned on
tclsh 9.0.4 and 8.6.14, identical:

```tcl
proc t3 {} {upvar 1 b; return [catch {set b} e]:$e}
proc h3 {} {set 1 ONE; return [t3]}
h3                          ;# → 0:ONE   — `1` is the otherVar, not a level
catch {upvar foo bar baz} e ;# → bad level "foo"  — 3 words ⇒ the first IS the level
```

**Where the fact lands.**  A *named* caller-frame write is merged into the
call statement's `defs`, so name-level SSA sees the definition and every
existing consumer works unchanged.  An *opaque* one has no name to merge, so
the CFG builder records it on `cfg::Function::caller_frame_barrier` and
`dynamic_name_barrier` joins it into the same three-bit lattice above.
Deliberately **not** a per-call-site fact: every consumer of that lattice
already reads it once per function and abstains for the whole function, so a
per-site fact would need flow-sensitivity none of them have and would buy
nothing.

**Soundness direction.**  Identical to the dynamic-name barrier's — abstain
toward silence for warnings, toward no-fold for the optimiser.  A level the
summary cannot place at the direct caller is *widened*, not dropped:
dropping an `upvar 2` write would leave it invisible to the frame that
receives it and produce a false `W210`, while widening only ever silences
one.

This also closes the `eval $body` / `uplevel 1 $body` gap that the
dynamic-name barrier left out of scope.  The two are **not** the same
effect, and the difference is load-bearing — tclsh 9.0.4 / 8.6.14,
identical:

```tcl
proc runner {body} {set helper 42; uplevel 1 $body}
runner {set x $helper}      ;# → can't read "helper": no such variable
proc evalrunner {body} {set helper 42; eval $body}
evalrunner {set helper}     ;# → 42
```

So `eval $body` blinds the frame it is written in, while `uplevel 1 $body`
blinds that procedure's *callers* and leaves its own locals provable.

**What cross-document (PR C1b) needs.**  `detect_upvar_procs` takes one
`Module`, i.e. one file's parse, so a helper defined in another file is
invisible however the call spells it (issue #923 audit idx 59's real
ticklecharts layout: `setdef` in `utils.tcl`, 1876 call sites in
`options.tcl`).  The summary is already a pure function of a procedure's
body plus its parameter list and is `Hash`/`Eq`, so the workspace layer can
intern one per procedure and merge the maps before `prepare_cfg_context`
runs — no change to the model, only to who supplies the map.  The navigation
providers need the same map plus the call site's scope, which the analyser's
`SignatureCommandInvocation` does not yet carry.

`eval $body` / `uplevel 1 $body` are no longer out of scope — see the
caller-frame injection model above.  They still lower to
`Statement::Barrier`, so the per-consumer barrier rules apply as well.

**What PR C1b took, and what is left.**  The *navigation* half landed in
[`tcl-lsp-core/src/caller_frame.rs`](../../../rust/tcl-lsp-core/src/caller_frame.rs),
but it consumes two per-parameter facts on `ProcDef` rather than this
summary, so a navigation provider reaches them without lowering the document
to IR on every hover:

* `ProcArgTrait::VarWrite` / `VarRead` — the parameter's value is used as a
  variable name through an `upvar`, and whether the callee writes through the
  alias or only reads it.
* `ProcDef::caller_frame_params` — the alias lands in the **immediate
  caller's** frame.  The traits carry no level at all, so this is the analyser
  side of the very gate `record_upvar_call` applies here
  (`FrameLevel::is_caller_frame`): `upvar 0` aliases the callee's own frame,
  `upvar #0` the global one, `upvar 2` the caller's caller, and none of them
  creates anything at the call site.  Computed by
  `analyser::param_traits::caller_frame_upvar_params`, which shares `upvar`'s
  arity-parity split with the trait scan through the registry's
  `FrameEffectSpec`, so the two views of a proc cannot drift apart.  It is a
  deliberate near-duplicate of `param_targets`, differing only in reading the
  proc's *source* rather than its lowered IR; if the interning below lands,
  merging them is the obvious follow-up.

Two gaps remain, both needing a fact the analyser does not record yet:

* `literal_targets` (`upvar 1 name name`, audit idx 22/98) has no call-site
  word to key on, so navigation abstains.  It needs a per-proc *literal
  caller-frame target* list on `ProcDef`, computed by the same body walk
  `infer_param_traits` already makes.
* Cross-document (idx 59) still needs the interning described above.  Nothing
  in the model changed; the workspace layer still has to merge the maps
  before `prepare_cfg_context` runs.

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
