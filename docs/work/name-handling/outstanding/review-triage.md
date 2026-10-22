# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T10:49Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress

- Building the probe batch for the transformation-safety findings (N17 template + live hook, N08, N05, N07, N15) in the temporary file `rust/tcl-spectcl/tests/zz_review_probe.rs`. The build has been queued on the shared target lock since 10:40Z, behind other agents' builds.

## Outstanding (in priority order)

1. Run the safety batch (N17, N08, N05, N07, N15), apply the returned edits to scratch source, run the edited source on tclsh 8.4–9.1 and Jim, then send the results to the coordinator.
2. Second probe batch, drafted in scratch and added to the same probe file: N01, N02, N03, N04, N06, N09, N10, N11, N12, N13 (seeded child), N14 (semantic tokens), N16, plus the nested-passthrough, extract/inline and linked-editing seams and the section 4.1 scaling rows (flat index, existence queries, source-order chain, code lenses).
3. Write `docs/work/name-handling/reviews/naming-migration-review-20261022-triage.md`, mapping each finding to its status, evidence, owner, files and fix direction.
4. Delete the probe file, the test executables and the scratch files.

## Blocked (on whom, and what is needed)

- Nothing hard-blocked. Builds wait on the shared cargo lock, and the disk is at about 2.2 GB. A guard script refuses to start a build below 2150 MB.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Oracle answers confirmed on tclsh 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim 0.84 for N01, N02, N03 (including the nested-brace case), N04, N05, N06, N08, N09, N10, N13, N14, N15 and the N17 capture driver. Every one matches the review's expected outcome.
- Statuses settled by static inspection, each still to be cross-checked by probe where noted:
  - `insert_rebound_candidates`: NOT-APPLICABLE. Removed in cf8f1bc85b.
  - `method_barrier`: FIXED. The tail-based `class_head_component` was removed in 944d156922.
  - `WorkspaceIndex::registry_for`: FIXED. It now borrows the document's retained registry (8aa4dfea79).
  - `ns_var_global_fallback`: FIXED. It prefers the resolved profile (944d156922).
  - N14 callback arity: FIXED. The tail-bucketed `project_command_arities` was removed in 3e29714009.
  - N13 caps: still present (`ROUND_CAP = 5` in `imported_constants`).
  - N11 `merge_library_files`: the early return on `workspace_command_exists` is still present.
