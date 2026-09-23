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
  one context), and slice 3 (registry-owned `expr` argument assembly over
  the shared expression engine, per-member finite-set branch decisions,
  `format`'s registry-owned route, and the Explorer's per-family route
  tally) have landed; slice 4, a private SpecTcl command through the same
  interface, is next.
- [consumer-contracts.md](consumer-contracts.md) — step 1 of
  [registry-consumer-contracts.md](../compiler/registry-consumer-contracts.md)
  § *Build order*: the four rulings taken as decided, and the documents whose
  stated rule they replace repaired. Documents only.
