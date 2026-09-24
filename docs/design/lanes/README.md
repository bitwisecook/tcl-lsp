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
  proven values) have landed; slice 8, the existence rung, is in
  progress item by item in the tracking document's § *Slice 8* ›
  *Record (2026-09-24): the opus items of slice 8*.
- [consumer-contracts.md](consumer-contracts.md) — steps 1 to 3 of
  [registry-consumer-contracts.md](../compiler/registry-consumer-contracts.md)
  § *Build order*: step 1 (the four rulings taken as decided, and the
  documents whose stated rule they replace repaired), step 2 (the
  description contract — `ClauseGrammarSpec`, `MemberEffect`, and
  `OptionEffect`, each behind one derived query, and every consumer this
  build order names moved onto them), and step 3 (trust gates execution —
  `WorkspaceTrust` plumbed from the LSP client through discovery, pack
  hook-body execution gated on it with the dormant-hook notice, and the
  one `untrusted` predicate; stub flags reach their fields — the six
  `StubFlags` on their catalogue fields with nearest-wins role
  resolution, and `spectcl_check`'s `tier`/`trust` preview) have landed,
  item by item in the tracking document's § *Step 2 — progress* (with the
  review's fixes applied after it) and § *Step 3 — progress*; step 4
  (identity: `alias_of`, the alias target's identity, the stamp rule,
  `SiteClaim`), planned in the tracking document's § *Plan for steps
  2–10*, is in progress item by item in the tracking document's § *Step
  4 — progress*: CC4.1 (`alias_of` as a `CommandSpec` field), CC4.2
  (the loader's stamp rejection rule: a codegen-axis stamp survives only
  as a bundled pack's `alias_of` target's own) and CC4.3 (codegen records
  that target's identity, which the VM admits through its alias hop) have
  landed.
