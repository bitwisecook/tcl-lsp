# Lane tracking documents

A **lane** is a substantial piece of work handed to a background agent — a
consumer port, a model re-type, a surface conversion. Lanes run for an hour
or more, and a container restart destroys everything a lane has not
committed. It has happened; this protocol exists because of it.

## The tracking document

One file per in-flight lane, named for the lane
(`c1-executable-ir-rekey.md`, `o1-o2-option-relations.md`, …). It is the
lane's crash insurance and handover note: the goal, the design decisions
taken and why, the site inventory with done/remaining status, behavioural
deltas accepted so far, and open uncertainties. The test of whether it says
enough: a fresh agent could resume the lane cold from the file alone.

It is updated in the same commit as the code it describes. When a lane lands
its content is folded into the final commit message and the file is removed;
git history keeps it. A file sitting here means a lane is in flight or was
interrupted — check the log for its `wip` commits before starting related
work.

## Checkpoint commits

Commit at each coherent milestone — roughly whenever you would say "that
part is done" — rather than accumulating one large uncommitted change. Three
hard rules make this safe on a shared branch:

- **The tree compiles before every commit.** `cargo check --workspace`
  passes, or there is no commit. A shared head that does not build blocks
  everyone.
- **Stage only the lane's own files, by explicit path.** Never `git add -A`,
  `git add .`, or a whole shared directory: a concurrent lane may be mid-edit
  in the same worktree.
- **Prefix the message `wip(<lane>):`** so the final tidy commit is
  distinguishable from the checkpoints behind it.

If `.git/index.lock` exists another lane is committing — wait and retry,
never delete the lock.

**Lanes commit locally; the orchestrator pushes.** Two lanes pushing
concurrently race on a non-fast-forward, and local commits already survive a
restart, so pushing buys a lane nothing.

**Checkpointing does not weaken a coherence ruling.** Where a design says a
change is "one change or none" — a coordinated re-type, say — that governs
what ships as *complete*, not whether intermediate states may be recorded.
Honestly labelled `wip` commits that compile make the state legible. A lane
that concludes it cannot finish says so in its tracking document and leaves
its last checkpoint compiling.

## In flight

None. Every lane of the #1631 programme has landed; what each one decided
lives in its final commit message, and what the programme left open lives in
the redesign's §11 open-questions ledger.
