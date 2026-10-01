# Lane tracking documents

A **lane** is a substantial piece of work handed to a background agent — a
consumer port, a model re-type, a surface conversion. A lane runs for an hour
or more, and a container restart destroys everything it has not committed.
This protocol is what makes that survivable.

## The tracking document

One file per in-flight lane, named for the lane. It is the lane's crash
insurance and handover note: the goal, the design decisions taken and why, the
site inventory with done/remaining status, behavioural deltas accepted so far,
and open uncertainties. The test of whether it says enough: a fresh agent could
resume the lane cold from the file alone.

It is updated in the same commit as the code it describes. When a lane lands,
its content is folded into the final commit message and the file is removed;
git history keeps it. A file sitting in this directory means a lane is in
flight or was interrupted — read the log for its `wip` commits before starting
related work.

## Checkpoint commits

Commit at each coherent point — roughly whenever you would say "that part is
done" — rather than accumulating one large uncommitted change. Three hard rules
make this safe on a shared branch:

- **The tree compiles before every commit.** `cargo check --workspace` passes,
  or there is no commit. A shared head that does not build blocks everyone.
- **Stage only the lane's own files, by explicit path.** Never `git add -A`,
  `git add .`, or a whole shared directory: a concurrent lane may be mid-edit
  in the same worktree.
- **Prefix the message `wip(<lane>):`** so the final tidy commit is
  distinguishable from the checkpoints behind it.

If `.git/index.lock` exists another lane is committing — wait and retry, never
delete the lock.

**Lanes commit locally; the orchestrator pushes.** Two lanes pushing
concurrently race on a non-fast-forward, and local commits already survive a
restart, so pushing buys a lane nothing.

**Checkpointing does not weaken a coherence ruling.** Where a design says a
change is "one change or none" — a coordinated re-type, say — that governs what
ships as *complete*, not whether intermediate states may be recorded. Honestly
labelled `wip` commits that compile keep the state legible. A lane that
concludes it cannot finish says so in its tracking document and leaves its last
checkpoint compiling.

## Resuming

[handoff.md](handoff.md) is the orchestrator's resume point: where each lane stands, what is queued on the running implementers, what follows the lanes, and how the work is run. It is rewritten at every push.

## In flight

- [value-transfers.md](value-transfers.md) — slices 2 to 13 of
  [value-transfers-migration.md](../compiler/value-transfers-migration.md)
  § *The slices*, planned item by item in the tracking document's § *Plan
  for slices 2–13*. Slice 1 (the interface shapes, the exact value
  ingress, the corrected derivation, the analysis context, the SCCP
  dispatcher behind the interface, the ledger, and the `cargo xtask
  value-transfers` gate), slice 2 (the direct routes for `incr`,
  `append`, `lappend`, `set`, the `dict` keyed updates, `string range`,
  `list`, `llength`, and `string length`, with both analysis paths under
  one context), slice 3 (registry-owned `expr` argument assembly over
  the shared expression engine, per-member finite-set branch decisions,
  `format`'s registry-owned route, and the Explorer's per-family route
  tally), and slice 4 (a private SpecTcl command through the same
  interface: the `semantics` / `evaluate` / `facts` loader statements, a
  declared implementation's bounded host with per-evaluation store and
  host-environment confinement, the memoised path's evaluator generation
  and content-keyed cache, the workspace overlay reaching every
  per-procedure query, the request and iteration budgets, `-native`
  resolution for every family, and the four-surface round trip), and
  slice 5 (destructuring and structured bodies: ordered `Write` /
  `Preserve` / `Unbind` / `MayWrite` / `WriteElement` storage outcomes
  applied per place in execution order for `regexp`, `regsub`, `scan`,
  `binary scan`, `lassign` and `array set`; the regexp owner's typed
  match / no-match / decline result, so an exhausted search raises rather
  than folding to no-match; `folded_types`' representation evidence;
  `dict with`, `dict update` and a loop header's structural plan; the
  `subst` template-word plan, which its folders, extract-proc and the
  dynamic-name barrier read instead of walking; the stored existence
  branch fact and W210's preserve-outcome reads; W100's proven produced
  set; and hover, inlay hints, semantic tokens and document links reading
  proven values), and slice 6 (branch integration: the whole-variable
  `switch` subject read from the lattice, so the flattened form decides per
  arm; one selection fact for each opaque form — `-glob`, `-regexp`,
  `-nocase`, a fall-through arm, `case` — from the shared `switch` core,
  which O112, the analyser's selected-body test and the loop simulator read
  in place of three private matchers, and of which I231 reports the
  unselected arms without dropping a block; W240 and W241 reading the loop
  header's branch fact, either replacing W242; and the iRules flow checks
  reading applied reachability, with the review's fixes — the lattice
  states what an opaque `switch`, a callback script and a call the module
  cannot see write; the flattened chain compares the values of its words,
  and a subject the option scan may read — on a release before 8.5, or with
  the arms as words — stays to the selection record; and the comments, the
  design pages, the Explorer's rendering and I231's wording were repaired
  beside them; two later rounds made an opaque `catch`, a deferred `try`, a
  computed head and a callback the scan cannot read state what they may write,
  a `[…]` substitution mark the unseen code in every body it runs, and a read
  after unseen code no read before it is set), and slice 8
  (the existence rung: a
  flow-sensitive bound / unbound / may-bound fact per place and per SSA
  version, owned by the solver and fed by storage outcomes, the entry
  states, the join,
  the absent-cell release rule, `[info exists]` and `[array exists]`
  deciding inside the fixed point through the expression route, and the
  guard narrowing as an edge refinement; W210, W211, W213, W214, O108,
  O109, I230, O101 and S100 consume the one fact; `const`, `array unset`
  and `array default` have semantics; a fast-tier request reads
  `Unavailable`), and slice 9 (nested writes in expressions: the ordered
  evaluation state at `LocalWrites` applies a nested write to a place it can
  own in the expression's order, so `set r [expr {$x + [incr x] + $x}]` gives
  `r` 5 and leaves `x` at 2 for every consumer; a statement's substitutions are
  evaluated once, as the synthetic call that carries their writes and the host
  that takes the result, and a command's own substituting words run before it,
  in order, under one state; a statement's reads beside a write its own
  substitutions make are read by name, so O102 forwards no earlier value and
  O109 keeps the store they read; O100 and O102 forward the nested store's
  value; a nested write to a place the state cannot own declines as
  `StatefulNested`; and the seven ordered-state witnesses run through the direct
  unit, the memoised unit, `tcl opt` and the registry's routes) have landed; the
  slices after them are planned item by item in the tracking document's §
  *Plan for slices 2–13*.
- [consumer-contracts.md](consumer-contracts.md) — steps 1 to 7 of
  [registry-consumer-contracts.md](../compiler/registry-consumer-contracts.md)
  § *Build order*: step 1 (the four rulings taken as decided, and the
  documents whose stated rule they replace repaired), step 2 (the
  description contract — `ClauseGrammarSpec`, `MemberEffect`, and
  `OptionEffect`, each behind one derived query, and every consumer this
  build order names moved onto them), step 3 (trust gates execution —
  `WorkspaceTrust` plumbed from the LSP client through discovery, pack
  hook-body execution gated on it with the dormant-hook notice, and the
  one `untrusted` predicate; stub flags reach their fields — the six
  `StubFlags` on their catalogue fields with nearest-wins role
  resolution, and `spectcl_check`'s `tier`/`trust` preview), step 4
  (identity — `alias_of` as the one source of a pack command's builtin
  identity; the loader's stamp rejection rule, under which a codegen-axis
  stamp survives only as a bundled pack's `alias_of` target's own; codegen
  recording that target's identity, which the VM admits through its alias
  hop; and `SiteClaim` with its `PackFactStamp`, so a site resting on a
  pack's facts claims them and the VM admits it only while it holds the
  same facts), step 5 (persisted guard identities, per-member
  semantics keys, the Explorer record — one `guard_semantics_key` per
  `IntrinsicId` member; both runtimes' guard identities held under the
  command's token generation and read through the guarded name's current
  binding, so a definition, rename, alias or profile pin of another command
  no longer costs every guarded intrinsic its fast path; and every premise
  the sealed native i64 addition rejects recorded on the plan and shown by
  the Explorer's new `aot` view), step 6 (the take-shipped floor over a
  shipped command's whole codegen and dispatch axis; the dependency-tier
  capability matrix, under which a pack a transitive or development
  dependency ships loses its `alias_of`, its `runtime_backing` and its
  codegen stamps, with a warning naming the tier; and the workspace overlay
  reaching the compile service, with an overlay nothing installed an error
  each consumer answers for itself — the compile service declines, the
  compilation unit is not built — and no longer the plain registry under
  another name) and step 7 (identities attached from the pinned generation
  and the runtimes' backing query — `runtime_backing` on the spec, declared on
  every core command and held by one gate to what the WASM runtime registers;
  the intrinsic table split by family, with a guard request that must cover
  its member's family's domains; `ArtefactIdentityManifest` on every compiled
  module and as a WASM custom section, the runtime pinned to a
  `RuntimeContext` resolved through the ingress the compiler uses, and the VM
  refusing, per rung, the functions that rest on a field that disagrees; and a
  fuzz campaign over the two runtimes as the exit) have landed, item by item
  in the tracking document's § *Step 2 — progress* (with the review's fixes
  applied after it), § *Step 3 — progress*, § *Step 4 — progress*, § *Step 5
  — progress*, § *Step 6 — progress* and § *Step 7 — progress*, planned in its
  § *Plan for steps 2–10*. Step 8 (reference bodies, `tcl spec test`, the
  manifest `spec` directive) is next.
