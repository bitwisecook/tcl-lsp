# SSA construction (Stage 5)

How the SSA builder assigns version numbers, places phi nodes, and computes
the dominator tree. Read this when debugging variable versioning or a missing
phi node.

`build_ssa` in `ssa.rs` transforms a `cfg::Function` into an `SsaFunction`
where every variable definition gets a unique version number.  Phi nodes are
placed at dominance frontiers to merge definitions from different paths.
The dominator tree and dominance frontiers are computed as part of SSA
construction.

Source: `rust/tcl-compiler/src/ssa.rs` (`build_ssa`, `Phi`, `SsaBlock`, `SsaFunction`)

### SSA principles

- Every variable definition gets a unique `(variable, version)` — the
  `ValueKey`, which is `(Symbol, Version)`.  `Symbol(u32)` is a variable
  name interned per `SsaFunction` in first-seen order, so the hot
  per-statement maps key on a copyable id instead of hashing and cloning a
  `String`; `SsaFunction::var_name` resolves a symbol back to its display
  name and `var_symbol` goes the other way.  (`def_use::SsaValueKey` is a
  *different*, name-keyed `(String, Version)` pair used by the def-use
  chains — see below.)
- A variable read resolves to the version currently in scope at that
  program point.
- At merge points (where multiple paths converge), phi nodes select the
  correct version based on which predecessor executed.

### Version numbering

- Version 0 = read-before-set: `RenameWalk::top` returns `0` when the
  variable has no live version, and a use at version 0 is what the
  read-before-set emitter reports as **W210**.
- Version 1, 2, 3, ... = successive definitions in program order, allocated
  by `RenameWalk::push_new` from a per-variable counter.
- Inside loops, the phi node at the header produces a new version that
  merges the initial value with the loop-carried update.

### Phi node placement

Phi nodes are placed at dominance frontier blocks — where a variable's
dominance "ends" — but only for **non-local** names, the semi-pruned rule
(see [algorithms.md](../../../docs/design/compiler/algorithms.md)).  A name
no block reads before redefining it gets no phi at all, because such a phi
would have no reader.

For `set x 1; if {$c} { set x 10 }; puts $x`:

```
  entry_1:  x₁ = "1"
  branch → if_then_3 / if_next_4

  if_then_3:  x₂ = "10"  → if_end_2
  if_next_4:  (no def of x) → if_end_2

  if_end_2:
    phi: x₃ = phi(x₂ from if_then_3, x₁ from if_next_4)
    puts $x   ← reads x₃
```

`if_end_2` is in the dominance frontier of both `if_then_3` and
`if_next_4`, and the trailing `puts $x` is the upward-exposed use that makes
`x` non-local.  Drop that read and `compute_phi_vars` places no phi.

### Dominator tree

Block A dominates block B if every path from entry to B passes through A.
The immediate dominator (`idom`) is the closest dominator.  `build_ssa`
computes it with `compute_idom_fast` (Cooper-Harvey-Kennedy over
reverse-postorder indices), then derives `dominance_frontier` and
`dominator_tree` from it.

`SsaFunction::idom` is a `HashMap<BlockId, Option<BlockId>>`; by block name:

```
entry_1    -> None
if_then_3  -> entry_1
if_next_4  -> entry_1
if_end_2   -> entry_1
exit_5     -> if_end_2
```

### Loop phis

Loops create phi nodes at the loop header.  For
`set i 0; while {$i < 5} { incr i }`:

```
  while_header_2:
    phi: i₂ = phi(i₁ from entry_1, i₃ from while_body_3)
    terminator: Branch { condition: $i < 5, .. }   ← reads i₂

  while_body_3:
    Statement::Incr { name: "i", .. } → i₃ = i₂ + 1
```

A terminator's reads are not stored on the `SsaBlock` — it has no
terminator field.  `build_ssa` only *interns* them
(`intern_terminator_reads`) so `var_symbol` can resolve a name read solely
in a branch condition or `return` value; the versioned use itself is
recovered by `build_def_use_chains`, which is why that function takes the
CFG as well as the SSA function.

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
| `SsaStatement` | `statement: Statement` (the original IR), `uses: HashMap<Symbol, Version>`, `defs: HashMap<Symbol, Version>`, `may_defs: HashSet<Symbol>`, `quoted_uses: HashSet<Symbol>` |
| `Phi` | `name: Symbol`, `version: Version` (produced), `incoming: HashMap<BlockId, Version>` |
| `SsaBlock` | `name: String`, `phis: Vec<Phi>`, `statements: Vec<SsaStatement>`, `entry_versions: HashMap<Symbol, Version>`, `exit_versions` |
| `SsaFunction` | `name: String`, `entry: BlockId`, `blocks: HashMap<BlockId, SsaBlock>`, `idom`, `dominance_frontier`, `dominator_tree`, plus the private block-name and variable-name interners |

`may_defs` is the subset of `defs` that are *synthetic* array-element
writes rather than writes the statement performs itself — the base refresh
alongside an element write (`set arr(k) v` also defs `arr`), and the element
fan of a dynamic-key write.  Type inference **joins** across a may-def;
write-sensitive passes (shimmer oscillation, dead-store) must not count one
as a real write.  `quoted_uses` is the subset of `uses` carried only by a
brace-quoted word the statement does not substitute — see
[def-use-chains.md](../../../docs/design/compiler/def-use-chains.md).

`SsaFunction::trivial` builds an empty shell when the complexity guard
(`is_complexity_guarded`) declines the expensive build for an oversized
body; `FunctionUnit::complexity_guarded` then tells every per-proc pass to
skip that function.

## Decision rule

- If a new IR node defines variables, ensure it appears in `defs` of the
  `SsaStatement` so the SSA builder tracks it.
- Version 0 reads indicate potential read-before-set — check that the
  variable is legitimately undefined (not a cross-event flow in iRules).
- Phi nodes are only placed where needed (at dominance frontiers), not at
  every merge point — this keeps the IR sparse.
- The dominator tree is essential for many analyses (SCCP, liveness) — if
  blocks are reachable but have no idom, check CFG connectivity.

### Def-use chain derivation

After SSA construction, def-use chains are built with
`build_def_use_chains(&ssa, Some(&cfg))` from
`rust/tcl-compiler/src/def_use.rs` and stored as `FunctionUnit::def_use`.
The chains map each `def_use::SsaValueKey` — a name-keyed `(String,
Version)`, since the chains are consumed by name-oriented diagnostics — to
its definition site and all use sites, enabling precise dead-store detection
and copy propagation.

Phi nodes participate in def-use chains in two roles:
- **As definitions**: The phi's `(name, version)` is a `DefKind::Phi` def.
- **As uses**: Each incoming edge `(pred_block, incoming_ver)` is a
  `UseKind::PhiIncoming` use of the incoming version.

### Memory-SSA for aliases

Variables aliased into another frame or namespace — through `upvar`,
`global`, `variable`, or `namespace upvar` — are tracked by memory-SSA
(`rust/tcl-compiler/src/memory_ssa.rs`), which versions each store/load to
an aliased location and builds alias sets.  It is **optional**:
`FunctionUnit::memory_ssa` is `None` unless a caller asks for it with
`with_memory_ssa`.

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
| `caller_frame_opaque_writes` / `caller_frame_opaque_reads` | `uplevel 1 $body`, `upvar 2 …`, `upvar $lvl …` | not resolvable — widen |
| `unnameable_local_aliases` | `upvar 1 x $dst` — the alias has no local name to file it under | structural only: it is what `reaches_caller_frame` counts |
| `frame_reach` / `plain_calls` | how far past this frame the effects land, and the ordinary callees to compose one hop through | bookkeeping for `detect_upvar_procs` |

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

**The cross-document limit.**  `detect_upvar_procs` takes one `Module`, i.e.
one file's parse, so a helper defined in another file is invisible however
the call spells it (issue #923 audit idx 59's real ticklecharts layout:
`setdef` in `utils.tcl`, 1876 call sites in `options.tcl`).  The summary is
a pure function of a procedure's body plus its parameter list and is
`Hash`/`Eq`, so the workspace layer *could* intern one per procedure and
merge the maps before `prepare_cfg_context` runs — no change to the model,
only to who supplies the map.  Nothing does that today: every caller of
`prepare_cfg_context` still passes a single-file `Module`.  The navigation
providers would need the same map plus the call site's scope, which the
analyser's `SignatureCommandInvocation` does not carry.

`eval $body` and `uplevel 1 $body` are in scope for the caller-frame
injection model above.  They still lower to `Statement::Barrier`, so the
per-consumer barrier rules apply as well.

**The navigation view of the same facts.**  Navigation lives in
[`tcl-lsp-core/src/caller_frame.rs`](../../../rust/tcl-lsp-core/src/caller_frame.rs)
and consumes three facts on `ProcDef` rather than this summary, so a
navigation provider reaches them without lowering the document to IR on
every hover:

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
  proc's *source* rather than its lowered IR; if the cross-file interning
  above lands, merging them is the obvious follow-up.
* `ProcDef::caller_frame_literals` — the analyser side of `literal_targets`
  (`upvar 1 name name`, audit idx 22/98), which spells its name nowhere at the
  call site so there is no argument word to key on.  A `name → written-through`
  map computed by `analyser::param_traits::caller_frame_literal_targets`;
  `caller_frame_bindings` answers for those names with the *call-head word* as
  the binding span.  A fully-qualified target (`upvar ::tk::FocusGrab($i)
  data`, idx 98) is not a caller-frame variable at all — it names one fixed
  global cell, which `handle_upvar_command` defines and links directly.

One gap remains: cross-document resolution (idx 59) needs the workspace-level
interning described above, which nothing supplies.  A callee reached by
`next` / `nextto` is also left unanswered — the MRO successor is not named at
the call site, so those reads keep the abstaining answer rather than a wrong
one, and the compiler-side dispatch widening keeps the diagnostics honest for
them.

### A brace-quoted `$`-bearing name is its own variable (issue #1078)

Braces suppress every substitution, so a brace-quoted word in a variable-name
position spells a **literal** name: `set {$n} 1` creates the variable *called*
`$n`, which Tcl keeps entirely separate from `n`. tclsh 9.0.4 and 8.6.14 give
byte-identical transcripts:

```text
set {$n} v ; set {$n}             -> v
info exists {$n} / info exists n  -> 1 / 0     two distinct variables
set n other ; set {$n}            -> v         `n` never affects it
set {$m} 1 ; set m                -> can't read "m": no such variable
set ${$n}                         -> can't read "v"   `${$n}` read `$n`
set {a b} v2 ; set x ${a b}       -> v2        a space is a legal name char
set i 5 ; set {arr($i)} 1
  array names arr                 -> {$i}      the key is literal
  info exists arr(5)              -> 0
  set arr($i) 2 ; array names arr -> {$i} 5    two distinct elements
unset {$n} ; info exists {$n} / n -> 0 / 1
```

The rule lives in the shared naming helpers, once:
`element_var_name_braced` / `normalise_var_name_braced` /
`split_array_name_braced` return the word's content **verbatim** when the
`braced_literal` flag is set — no `$` stripped, no `${…}` unwrapped, the array
key literal whatever it spells (`normalise_var_name_braced` still takes the
`(key)` suffix off, since `{arr($i)}` is element `$i` of the array `arr`).
`is_dynamic_write_target` takes the same flag and answers *not dynamic* for a
braced target. Which words are braced is one question with one answer:
`CommandTokens::arg_is_braced_literal` (and its hook-side twin
`LoweringCommand::arg_is_braced_literal`) — the `Str` representative token
kind, since the IR stores de-braced content that cannot show the difference.

Every consumer of the naming layer reads the same flag, which is what makes
the rule hold end to end: defs (`defs_of_with_registry`,
`collect_defs_from_script`, the
`set`/`incr`/`unset`/`append`/`global`/`variable`/`upvar` lowering hooks,
`lower_default`'s registry role harvest), reads (`var_token_name`,
`scan_var_read_role_names`, `collect_ref_forms` +
`scan_var_ref_forms_braced`, `scan_command_words`'s name-role skip), the
analyser scope layer (`define_var` from the name word's token kind,
`record_var_read_braced`), rename, and semantic highlighting.

The diagnostic behaviour that follows:

| shape | diagnostics |
| --- | --- |
| `set {$n} 1` | silent for `n`; `W211` names `$n` |
| `set {$n} 1; puts $n` | `W210` on `n` — tclsh errors here, so the read is a true positive |
| `set {$n} 1; set {$n} 2; return [set {$n}]` | silent, byte-identical to the plain-named control |
| `unset {$n}` after `set {$n} 1` | silent — no `W213` on `n` |

The last two rows depend on two read scans that also cover the *plain*
spelling: the `[set NAME]` textual read scan recognises a brace-quoted name
word, and the substitution-hidden-read scan walks a `return`'s value word (a
terminator), so `proc f {} {set n 1; set n 2; return [set n]}` reports no
dead store on code tclsh runs to `2`.

All five variable providers resolve their `$ref` cursor through one gate,
`definition::substituting_var_at_position`. The character scan under it
reports `n` for a cursor on the `n` of `set {$n} 1` — that word is
brace-quoted, so the `$n` is not a reference at all but part of a different
variable's literal *name*. The gate answers `None` there (it also folds in
the two `inert_text` proofs, issue #923 idx 24, so no caller re-derives
them), which lets each provider fall through to its declaration-span
search — the same behaviour the `{` column gets. Document-highlight has no
declaration-span search, so it abstains for a braced cursor exactly as it
already does for a plain bareword declaration cursor.

Rename **refuses** rather than guesses for these cells
(`rename_safety::literal_name_variable_rename_refusal`, the #1091 typed-refusal
precedent): the recorded spans cover a word's content, not its delimiters, and
the delimiters a *new* name needs are a property of that new name — rewriting
`{$n}`'s span with `q` would produce `set q} 1`. Renaming the plain `n` is
unaffected and never touches the `{$n}` word.

## Related docs

- [Examples 5–9 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-5-if-x--set-y-10-)
- [GLOSSARY.md — SSA, Phi node, Dominator](../../GLOSSARY.md#ssa)
- [kcs-cfg-ssa-fact-model.md](../../../docs/design/compiler/cfg-ssa-fact-model.md)
- [kcs-cfg-construction.md](../../../docs/design/compiler/cfg-construction.md)
- [kcs-def-use-chains.md](../../../docs/design/compiler/def-use-chains.md)
- [kcs-memory-ssa.md](../../../docs/design/compiler/memory-ssa.md)
