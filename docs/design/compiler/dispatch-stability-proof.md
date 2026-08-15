# Dispatch-stability proofs — the world-state contents lattice behind stable-call CSE

The contract for `rust/tcl-compiler/src/dispatch_proof.rs`: a forward dataflow
analysis over the executable semantic control-flow graph that decides, per
invocation site, whether Tcl's mutable dispatch machinery can observe the call
being executed. It is the fact that global value numbering
([GVN](../../GLOSSARY.md#gvn)) requires before it may report a repeated
registry-stable call as a common subexpression ([CSE](../../GLOSSARY.md#cse),
reported as `O105`).

Issue #1364 is the requirement this document records the shape of. The
user-facing half is [the O105 note](../../kcs/codes/kcs-optimisation-o105-constant-var-ref-propagation.md).

## §1 — Why result stability is not observational removability

World-state SSA (`rust/tcl-compiler/src/world_state_ssa.rs`) versions the
mutable interpreter world, but it deliberately does not model what any version
*contains*. A registry-resolved call can therefore be referentially transparent
with respect to its result and still be observable through Tcl's dispatch
machinery:

```tcl
proc observe {command operation} {
    puts "$command $operation"
}
trace add execution llength {enter leave} observe

set first  [llength $items]
set second [llength $items]
```

`llength` returns the same value both times. Eliminating the second call
nonetheless deletes two `observe` callbacks and changes what the program
prints, so the rewrite is unsound. Equivalent hazards exist around `rename`,
`unknown`, `interp alias`, namespace imports and ensembles, safe- and
child-interpreter policy, `TclOO` dispatch reconfiguration, and dynamically
installed commands.

The registry splits the two questions:

| Question | Registry fact | Type |
|---|---|---|
| Can replaying this call with the same operand values reproduce its result? | `result_stability` | `ResultStability` (`tcl-registry/src/result_stability.rs`) |
| Which mutable domains decide *which command runs* and *who watches it*? | `dispatch_dependencies` | `DispatchDependencies` (`tcl-registry/src/dispatch_stability.rs`) |

`ResultStability` answers the first and is a static property of the command.
`DispatchDependencies` names the second but cannot answer it: a static command
specification can never prove that the live interpreter left those domains
alone. That proof is this pass's job.

## §2 — Registry inputs

### Dispatch dependencies

`DispatchDependencyDomain` has six members: `CommandBinding`, `NamespaceLookup`,
`CommandTraces`, `InterpreterPolicy`, `ObjectDispatch`, and `UnknownHandling`.
`ResolvedDispatchDependencies::resolve` walks the command → subcommand → form
descriptor chain and then restores the floor:

- `DispatchDependencies::CONSERVATIVE` (all six) is the default for an
  unstamped command, because it has made no narrower claim;
- `DispatchDependencies::BASE` (`CommandBinding`, `NamespaceLookup`,
  `CommandTraces`, and `InterpreterPolicy`) is irreducible — a `Replace`
  descriptor may refine `ObjectDispatch` and `UnknownHandling` away, but never
  the four domains that no static specification can settle;
- `CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT` — the reusable base for a
  command with no mutable-world effects, no state transitions, and an
  argument-only result — declares
  `DispatchDependencyDescriptor::replace(DispatchDependencies::NONE)`, which
  resolves to exactly `BASE`. That is the accurate claim: such a command is not
  an object method and, being resolvable, never reaches `unknown` fallback, but
  its binding, namespace lookup, traces, and interpreter policy still have to
  be proven by flow analysis.

### State transitions

`tcl-registry/src/state_transition.rs` owns the transition vocabulary this pass
consumes: `CommandBindingTransition` (define, move, delete, alias, unknown),
`InterpreterTransition` (create, delete, mark-trusted, recursion limit,
background error, hide, expose), `VariableCellAliasTransition`,
`NamespaceTransition` (ensure, delete, import, forget, export, ensemble,
set-path, set-unknown), `TraceTransition` (add and remove, over
`TraceTarget::{Variable, Command, Execution}`), `ObjectDispatchTransition`
(create, copy, configure, destroy), and an explicit `Widen` for dynamic
operands. Every operand is a `TransitionSubject`, which is either a literal Tcl
value or a typed unknown carrying its argument index and word kind.

Each fact also carries `StateTransitionCommit`: `OnOkOnly`, or
`MayCommitBeforeAbruptCompletion` for pair-wise mutation and re-entrant
callbacks that can expose an intermediate state before an error, return, break,
or custom completion.

**No command names.** The transfer functions below consume only these typed
facts, ordinary effect-footprint intents, and structured word shapes. A
command, trace, namespace, interpreter, or object mutation reaches this pass
exclusively as registry data — adding a new mutating command is a registry
descriptor, never a branch here.

## §3 — The contents/absence lattice

### Tracks

`WorldTrack` is the pass's partition of the world. Its thirteen members mirror
`WorldRegionKind` (minus the per-resource split of external state) so that
world-state SSA, effect footprints, and this pass name the same partitions:

`CommandBindings`, `NamespaceLookup`, `NamespaceUnknown`, `ExecutionTraces`,
`CommandTraces`, `VariableTraces`, `ObjectDispatch`, `InterpreterPolicy`,
`InterpreterTopology`, `VariableStore`, `PackageState`, `HostCapabilities`, and
`ExternalState`.

`WorldTrack::for_dispatch_dependency` maps each registry dependency domain onto
the tracks whose versions key it:

| Dependency domain | Tracks |
|---|---|
| `CommandBinding` | `CommandBindings` |
| `NamespaceLookup` | `NamespaceLookup` |
| `CommandTraces` | `ExecutionTraces`, `CommandTraces` |
| `InterpreterPolicy` | `InterpreterPolicy`, `InterpreterTopology` |
| `ObjectDispatch` | `ObjectDispatch` |
| `UnknownHandling` | `NamespaceUnknown`, `CommandBindings` |

### The abstract state

`WorldContents` is one program point's abstract world: a per-track version
array plus the proof-bearing ledgers.

| Component | Shape | Proves |
|---|---|---|
| `bindings` | identity ledger of changed-subject patterns | a named command's binding is untouched |
| `object_dispatch` | identity ledger | a named object's dispatch is unreconfigured |
| `execution_traces`, `command_traces`, `variable_traces` | trace ledgers of per-target registration counts | no live observer for a target |
| `namespace_lookup_stable`, `namespace_unknown_stable`, `interpreter_policy_stable` | booleans | the whole domain is untouched |
| `aliases` | variable-alias ledger (`local → target cell`) | which cell a local name reaches |

**Identity ledgers** record *changed* subjects, not surviving ones — the sound
direction, because entry contents are asserted by the entry contract and every
subsequent change is observed. A `SubjectPattern` is either `Exact(name)` or
`NamespacePrefix(prefix)`; the prefix form covers a namespace deletion or
import, and the empty prefix (Tcl's global namespace, and therefore the current
namespace of an analysed unit) matches every candidate. Names are normalised
(`::llength` and `llength` compare equal) before comparison.

**Trace ledgers** are the interesting case, because trace liveness is a
counting question rather than a set-membership one. C Tcl appends a fresh trace
record for every `trace add`, duplicates included, and `trace remove` deletes
the most recent exact match or succeeds as a no-op. So each `(operations,
command-prefix)` registration on a target carries a saturating interval
`CountRange { lo, hi }`:

| Operation | Effect on the interval |
|---|---|
| literal add | `lo + 1`, `hi + 1` (`hi` becomes unbounded at saturation) |
| literal remove with a provably matching key | `lo - 1`, `hi - 1` |
| remove whose match cannot be identified | `lo - 1`, `hi` unchanged |
| join | `min(lo)`, `max(hi)` |

A target may be observed while `hi != Some(0)`. The interval stays sound under
either duplicate policy: were Tcl's `trace add` idempotent instead, the upper
bound would merely be conservative. A registration whose command prefix was a
dynamic Tcl value stores `prefix: None` — it still observes, and no literal
removal can be proven to match it, so any removal against such a target only
weakens lower bounds.

Two further trace-ledger facts matter. `step_hazard` records that a live
registration may include `enterstep` or `leavestep`, which observes *every*
nested command rather than only its own target, so it fails the `CommandTraces`
domain for all subjects. Variable-trace cells additionally model array
elements: a cell named `base(element)` fires for a whole-variable access, for
an exactly matching literal element, or for any computed element.

### Bounds and widening on overflow

Every ledger is bounded by a small constant, and overflow widens to lattice top
rather than growing:

| Bound | Value | Overflow behaviour |
|---|---|---|
| `MAX_TRACKED_SUBJECTS` | 16 changed patterns per identity ledger | ledger widens — no subject is stable |
| `MAX_TRACE_CELLS` | 8 traced targets per trace ledger | ledger widens, `step_hazard` set |
| `MAX_TRACE_REGISTRATIONS` | 8 registrations per target | cell becomes `unknown` |
| `MAX_ALIAS_LINKS` | 8 links | alias ledger becomes `unknown` |
| `COUNT_SATURATION` | 8 | upper bound becomes unbounded |

`widen_all` is the top element: every ledger widened, every stability flag
false, every track version bumped to the widening site.

## §4 — Transfer functions

The five transfer sources below are listed in decreasing order of precision. A
resolved invocation uses the first two together — its ordinary effect footprint
first, then its declared transitions on the completion edges — while every other
instruction shape uses exactly one.

**1. Typed registry transitions, per completion edge.** A resolved invocation
whose facts are `StateTransitionKnowledge::Declared` holds its
`StateTransitionFact`s until the block's outgoing edges are known. When the
block's terminator is a `CompletionSwitch`, each edge is transferred by
`EdgeCompletion { ok, abrupt }`:

- an `OK`-only edge commits every fact exactly;
- an abrupt edge commits only the facts declared
  `MayCommitBeforeAbruptCompletion`, and joins the result with the unchanged
  incoming state;
- an edge that carries both `OK` and abrupt arms to the same block commits
  every fact and likewise joins with the unchanged state.

This mirrors the world-state renamer's contract exactly. When the transitions
are *not* dispatched by the block's terminator — another instruction follows,
or the terminator is not a completion switch — edge precision is unavailable
and both outcomes are folded together unconditionally.

**2. Ordinary effect-footprint region writes.** Before the transitions, the
invocation's `world_state_effects()` footprint is projected by
`project_effect_footprint`; every non-`Use` intent damages the ledger for its
`WorldRegion`. `Any` and a wildcard over an interpreter that may be the current
one widen everything; a namespace subtree invalidates lookup, bindings, and
object dispatch; a namespace lineage bumps the lookup version without damaging
contents; a scoped write damages exactly its `WorldRegionKind`, keyed by its
named or wildcard subject. A projection error widens everything.

Two footprint shapes deserve naming. A footprint that declares a re-entrant
callback and supplies no closed callback summary projects an all-world use
followed by an all-world clobber, so every script-running command — `eval`,
`source`, `package require`, `namespace eval`, `uplevel` — widens everything.
And a scoped `VariableStore` write is special: writing a cell fires its write
traces, which run arbitrary re-entrant Tcl, so it widens everything unless the
target is provably untraced.

**3. Word-evaluation hazards.** An `EvaluateWord` instruction is inert for a
literal or braced literal; a variable read is a hazard exactly when a live
variable trace may fire on it (following one alias hop, and honouring the
array-element relation); a command substitution or opaque fragment is always a
hazard and widens everything. World-state SSA carries the matching barrier —
`word_evaluation_runs_commands` makes such a word an all-world clobber there —
so a closed outer invocation such as a pure list query cannot hide a nested
`rename` from either graph.

**4. Lowered statements.** `ExecuteLowered` operations are transferred by
statement shape — `AssignConst`, `AssignValue`, `AssignExpr`, `Incr`,
`ExprEval`, and `Return` — and every other shape widens. The value is checked
first: a retained value word is judged by the same word rule as source 3, and a
value with no retained word is judged by its raw text, which is inert without
bracket, dollar, or backslash syntax, unsafe with a bracket or backslash, and
otherwise safe only while no variable trace can fire at all. An expression is a
hazard when it contains a command substitution, a math-function application
(which dispatches through `tcl::mathfunc`), or unparseable `Raw` text; a braced
`return` value is inert. A hazardous value or expression widens everything;
otherwise the statement's variable write is applied, which itself widens unless
the target cell is provably untraced.

**5. Opaque regions and unknown invocations.** `ExecuteOpaqueRegion`, an
`InvocationResolution::Unresolved` head, and
`StateTransitionKnowledge::UnknownInvocation` all widen everything. This is the
fail-closed default: an unresolved head, an undeclared invocation, an opaque
region, a lowered statement whose operands cannot be proven inert, and every
dynamic transition operand lose all precision rather than assert absence.

## §5 — Joins, loops, and the fixpoint

`WorldContents::join` is pointwise: identity ledgers union their changed
patterns, stability flags conjoin, trace cells join their count intervals per
registration key (a key absent on one side joins against zero), the alias
ledger keeps a link only where both sides agree on its target, and any widened
input widens the result. A trace or interceptor present on either predecessor
therefore survives the join and prevents reuse afterwards.

Track versions join differently: a track whose incoming versions differ becomes
`TrackVersion::Phi { block }`, which is what makes the join visible to value
numbering even when the joined *contents* are equally stable.

The driver is a worklist over CFG blocks with precomputed predecessor edges and
per-edge outgoing states. A block whose predecessor edge state has not yet been
computed — a cycle edge before its first visit — is skipped this round rather
than being given fabricated contents; at fixpoint every reachable predecessor
edge contributes. Unreachable blocks never contribute and never receive proofs.

The fixpoint is bounded at `MAX_BLOCK_APPLICATIONS_PER_BLOCK` (64) block
applications per block. The lattice has constant height, so genuine convergence
needs far fewer; exhausting the budget **fails closed** by returning no site
proofs at all, for the whole function.

Site proofs are captured in a second, separate replay: each reachable block is
re-walked from its fixpoint entry state, and each resolved invocation's proof
is captured from the state as it stands *when that invocation's dispatch
begins* — before its own transfer is applied.

## §6 — Track versions and value keys

`TrackVersion` is a value-numbering token for one track: `Entry` (unchanged
since function entry), `Site { block, instruction }` (last changed by that
instruction), or `Phi { block }` (joined from differing predecessors). Equal
tokens prove equal contents.

`SiteDispatchProof::world_key` renders the fragments that must enter the GVN
expression key, as `world:<track-label>@<version>` (for example
`world:cmdbind@e`, `world:exectrace@b0i3`). The selected tracks are the union
of:

- the tracks of every registry-declared dispatch-dependency domain for the
  site's command; and
- the track of every domain named by `ResultStability::ReadsVersionedWorld`.

The second is what admits a call whose *result* reads versioned world state —
package, host-capability, or variable-store state, say. Such a call is reusable
only when its key pins those versions, so two occurrences separated by a write
to the domain no longer match. A consumer that does not number world state must
abstain, and `ResultStability::Volatile` and `Unknown` are never candidates.

## §7 — The site proof and GVN's eligibility rule

```rust
pub struct SiteDispatchProof {
    pub covers: DispatchDependencies,
    pub versions: [TrackVersion; WorldTrack::ALL.len()],
    pub operand_words_admissible: bool,
}
```

`covers` is computed by testing all six domains against the state at the site,
for the subject `normalise_name(&resolved.canonical_command)`:

| Domain | Proven when |
|---|---|
| `CommandBinding` | the subject appears in no changed-binding pattern |
| `NamespaceLookup` | namespace lookup is untouched |
| `CommandTraces` | no step hazard, and neither the execution- nor the command-trace ledger may observe the subject |
| `InterpreterPolicy` | interpreter policy is untouched |
| `ObjectDispatch` | the subject appears in no changed object-dispatch pattern |
| `UnknownHandling` | namespace `unknown` is untouched **and** no binding changed at all |

`UnknownHandling` is deliberately the strictest: a command that resolves today
can only enter missing-command fallback if some binding moved, so the ledger
must be entirely empty rather than merely free of this subject.

`operand_words_admissible` requires every operand word (word 0, the head, is
excluded) to be free of world observation — no command substitution, no opaque
fragment, and no variable read a live variable trace may observe — and declines
expansion outright, because `{*}` changes the argv shape the registry facts
were resolved for.

`gvn_site_eligibility` (`rust/tcl-compiler/src/gvn.rs`) is the seam. A site is
eligible only when all of the following hold:

1. the static registry gates pass — `PURE` and `CSE_CANDIDATE` traits, an empty
   effect access set, no world barrier, empty legacy command-table/frame/side
   effects, and a *closed and empty* `StateTransitionKnowledge::Declared` set
   (`resolved_invocation_static_gvn_gates`);
2. the result is reusable — `resolved_invocation_is_gvn_candidate`
   (`ReferentiallyTransparent`) or
   `resolved_invocation_is_versioned_world_gvn_candidate`
   (`ReadsVersionedWorld`);
3. a site proof exists at all; and
4. `SiteDispatchProof::satisfies(resolved.dispatch_dependencies)` — every
   declared dependency domain is covered and the operand words are admissible.

The eligibility then carries the canonical command identity and the world-key
fragments, which `statement_occurrences_for_gvn` appends to the value key so
two occurrences match only when the world their dispatch and result depend on
carries the same versions.

A missing proof is a decline, not a fallback. `GvnSemanticFacts::from_function`
runs the analysis only when the world-state graph itself was buildable; a
world-state decline, an unavailable semantic sidecar, an unresolved head, an
ambiguous source-span match, or an unreached site all fail closed, and the
production path never re-classifies from command text.

## §8 — The entry contract

```rust
pub enum DispatchEntryAssumption {
    PristineRegistryWorld,
    SealedLoadGraph(SealedLoadGraphEntry),
    UnknownWorld,
}
```

This is the typed replacement for any "no trace was seen in this file"
heuristic. It is an explicit input chosen by the analysis driver, never
inferred from source text.

`PristineRegistryWorld` asserts that dispatch state at entry is the registry's
baseline for the selected dialect: registry command bindings are intact and no
trace, alias, ensemble, or interpreter-policy observer is live.
`UnknownWorld` starts every domain widened, so every site proof fails closed.

`SealedLoadGraph` is the procedure-only strong contract. Its constructor makes
the host present six typed facts together: an interned `DialectProfile`, a
`FreshSealedInterpreter`, a `CompleteOrderedLoadGraph` containing the exact
ordered identities of ordinary workspace, statically sourced, and statically
selected package units, a `CompleteCallerSet`, a `CompleteExposureSet`, and a
`RegistryDispatchBaseline`. The last two facts mean every host/native command
exposure is represented and the completed load leaves no moved registry
binding, execution/command trace, alias, namespace interceptor, or
interpreter-policy change relevant to the registry baseline. The selected
profile's exact availability mask is checked again when the semantic bundle is
built; a mismatch becomes `UnknownWorld`.

These are host assertions, not conclusions drawn from source absence. In real
Tcl, a procedure remains callable after an embedding application or later Tcl
code installs a trace, renames a command, or changes namespace resolution.
Consequently the ordinary compiler, CLI, and LSP build paths do not manufacture
this contract and remain `UnknownWorld`. A closed-program host which genuinely
owns the complete load and interpreter lifetime may call
`CompilationUnit::build_with_sealed_procedure_entry`.

Where each is chosen (`rust/tcl-compiler/src/compilation_unit.rs`, threaded
through `FunctionUnit::with_semantic_analysis` into
`SemanticAnalysisBundle::dispatch_entry_assumption`):

| Unit | Assumption | Why |
|---|---|---|
| the compilation unit's top-level script | `PristineRegistryWorld` | modelled as evaluating the file in a fresh interpreter |
| procedure units, ordinary hosted build | `UnknownWorld` | a body runs only after arbitrary interposed top-level, host, and cross-file history |
| procedure units, explicit sealed build | `SealedLoadGraph` | the host owns a fresh sealed interpreter, complete ordered workspace/source/package load, complete callers, and registry-baseline dispatch |
| `TclOO` method units | `UnknownWorld` | same, plus an unmodelled receiver |
| synthetic body units (`apply` lambdas, `namespace eval` bodies) | `UnknownWorld` | same |

Because `PristineRegistryWorld` is an assumption about history the compiler has
not verified, the `O105` findings it enables are advisory: hint severity, no
auto-fix payload. An action tier — one that offers to perform the rewrite —
needs a stronger contract that additionally justifies the assumption from
workspace facts.

The enum is the extension point. A workspace-aware driver can add a variant
carrying aggregated cross-file evidence (workspace index, `package require`
graph, `source` graph, a per-file dispatch summary) without touching a single
transfer function: only `WorldContents::entry` interprets it.

## §9 — Conservative abstentions

These are real, current limitations, not incidental gaps.

- **Structured statements are opaque.** Under the linear compatibility
  executable IR, `Block`, `UpFrame`, `If`, `For`, `While`, `Foreach`, `Catch`,
  `Try`, and `Switch` become `ExecuteOpaqueRegion` — an all-world barrier. A
  call inside an `if` body therefore has no exact invocation mapping and cannot
  be proved, and everything after the region starts from top.
- **Expansion.** `{*}` in any operand word makes the site inadmissible.
- **Command substitution in operands.** A `[...]` operand widens the world at
  the `EvaluateWord` that evaluates it, and makes the site inadmissible.
- **PRE and LICM are not proof-gated.** `find_partial_redundancies` and
  `find_loop_invariants` still call `statement_occurrences`, which uses the
  legacy `is_pure_command` string classifier over the legacy CFG/SSA. Only
  `find_redundancies_for_function` (and therefore `find_redundancies_for_cu`)
  consumes the proof. This is tracked debt: the partial-redundancy and
  loop-invariant halves of the reports need the same seam.
- **Hosted procedure bodies do not prove.** Ordinary proc, method, and body
  units run under `UnknownWorld`. An explicitly sealed procedure build can
  prove straight-line stable calls. Methods and synthetic bodies still
  abstain: receiver/object state and callback-frame entry are not covered by
  the procedure contract.
- **`namespace eval`'s body barrier.** Lowering registers the body as its own
  body unit *and* leaves the enclosing statement as a runtime barrier. From the
  enclosing unit the command is an ordinary invocation, but its registry
  footprint declares a script callback with no closed summary, so it widens
  everything; the body unit itself is analysed under `UnknownWorld`. Nothing on
  either side of that seam is proved.
- **Dynamic transition operands.** A computed trace target, operation list,
  command prefix, alias name, interpreter path, or namespace widens its
  domains. Literal removal of the last applicable trace can restore
  eligibility; dynamic removal only weakens lower bounds and cannot.
- **No cross-file evidence.** `eval`, `source`, and `package require` all carry
  the same script-callback barrier, and nothing outside the analysed unit
  contributes a narrowing fact today. Closing this needs a callback summary or
  a workspace-aware entry contract (§8), not a change to the lattice.

## §10 — Complexity and memory

The fixpoint is a worklist over CFG blocks with precomputed predecessor edges:
`O(edges × lattice-height)` block applications, over the one-pass linear
compatibility CFG. Every ledger is bounded by a small constant (§3), so one
abstract state is O(1) to clone, join, and compare, and the lattice height is
constant.

Per-edge states are `Arc`-shared copy-on-write: an instruction that changes
nothing forwards its incoming state without allocating, and equality is checked
by pointer first. `WorldContents::fully_widened` short-circuits further
widening once every proof-bearing component is already at top. The analysis is
deterministic for an isomorphic CFG, and the budget in §5 bounds pathological
inputs.

## §11 — Code and test anchors

| Surface | Location |
|---|---|
| the pass | `rust/tcl-compiler/src/dispatch_proof.rs` |
| world versioning and the word-evaluation barrier | `rust/tcl-compiler/src/world_state_ssa.rs` |
| the GVN seam | `rust/tcl-compiler/src/gvn.rs` (`GvnSemanticFacts::from_function`, `gvn_site_eligibility`) |
| the entry contract's carrier | `rust/tcl-compiler/src/semantic_analysis.rs` |
| where the assumption is chosen | `rust/tcl-compiler/src/compilation_unit.rs` |
| dispatch dependencies | `rust/tcl-registry/src/dispatch_stability.rs` |
| result stability | `rust/tcl-registry/src/result_stability.rs` |
| transition vocabulary and commit contract | `rust/tcl-registry/src/state_transition.rs` |
| the closed-command base | `rust/tcl-registry/src/spec.rs` (`CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT`) |

Behavioural tests live in `rust/tcl-compiler/src/gvn.rs`, covering the pristine
top-level proof, the live-execution-trace suppression, the dependency-coverage
and operand-admissibility gates, volatile and versioned-world results, and the
decline paths (no selected dialect, opaque structured source).
`rust/tcl-compiler/src/compiler_checks.rs` pins the advisory tier — a
proof-gated `O105` is a hint with no replacement and no fixes.
`rust/tcl-registry/tests/registry_sweep.rs` sweeps the registry itself: a
referentially transparent result claim must travel with closed world-effect and
state-transition declarations, every CSE candidate must declare its result
stability, and `CLOSED_REFERENTIALLY_TRANSPARENT` must resolve to exactly
`BASE`.

## Related

- [common-semantic-compiler.md](common-semantic-compiler.md) — the wider
  contract this pass completes: world/effect SSA, the shared versioning
  substrate, and completion/callback flow.
- [optimisation-passes.md](optimisation-passes.md) — where GVN sits in the pass
  order.
- [side-effects-system.md](side-effects-system.md) — the effect footprints the
  region-write transfer consumes.
- [command-registry.md](command-registry.md) — the `CommandSpec` field
  reference.
- [O105 KCS note](../../kcs/codes/kcs-optimisation-o105-constant-var-ref-propagation.md)
  — the plain-English answer for users.
