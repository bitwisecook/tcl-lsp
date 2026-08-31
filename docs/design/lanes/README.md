# Lane tracking documents

One file per in-flight agent lane, named for the lane
(`c1-executable-ir-rekey.md`, `o1-o2-option-relations.md`, …).

A lane's file is its crash-insurance and its handover note. It records the
goal, the design decisions taken and why, the site inventory with
done/remaining status, behavioural deltas accepted so far, and open
uncertainties — written so a fresh agent could resume the lane cold from
that file alone. It is updated in the same commit as the code it
describes.

When a lane lands, its content is folded into the final commit message and
the file is removed; the git history keeps it. A file sitting here means a
lane is either in flight or was interrupted — check the log for its `wip`
commits before starting related work.

The protocol these files belong to — checkpoint commits, compile-before-
commit, staging by explicit path, and why lanes commit locally while the
orchestrator pushes — is in [`AGENTS.md`](../../../AGENTS.md) under
"Long-running agent lanes: checkpoint or lose it".

## In flight

- [`tclvm-grammar-conformance.md`](tclvm-grammar-conformance.md) — Tcl 9.0.4
  grammar foundations for #1579/#1580, with the release-aware #1732 follow-up
  contract.
- [`tclvm-array-index.md`](tclvm-array-index.md) — release-aware Tcl 9 array
  index source validation for #1732.
