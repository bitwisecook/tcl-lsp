# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T11:04Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## In progress

- Second probe batch: build and run are queued. It covers N01–N04, N06, N09–N14 and N16, the seams and the section 4.1 scaling rows, plus controls for the N17 live hook (constant fold and braced form) and N05 (plain rename), and the N08 compilation-unit inference check.

## Outstanding (in priority order)

1. Second probe batch, drafted in scratch and added to the same probe file: N01, N02, N03, N04, N06, N09, N10, N11, N12, N13 (seeded child), N14 (semantic tokens), N16, plus the nested-passthrough, extract/inline and linked-editing seams and the section 4.1 scaling rows (flat index, existence queries, source-order chain, code lenses).
2. Write `docs/work/name-handling/reviews/naming-migration-review-20261022-triage.md`, mapping each finding to its status, evidence, owner, files and fix direction.
3. Delete the probe file, the test executables and the scratch files.

## Blocked (on whom, and what is needed)

- Nothing hard-blocked. Builds wait on the shared cargo lock, and the disk is at about 2.2 GB. A guard script refuses to start a build below 2150 MB.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Safety batch run at 11:03Z (6 probe tests passed) and results sent to the coordinator:
  - N17 template: FIXED. `good` is registered in all 16 variants.
  - N17 live hook: CHANGED. No O129 is emitted any more; the control is pending.
  - N08: FIXED for edits. No optimisations are returned, and the output is `LOCAL` on all 6 engines.
  - N05: CHANGED. Rename refuses with "unknown-completed-command-world"; the plain-rename control is pending.
  - N07: FIXED. Rename edits only the proc-local `$g`, and definition is empty.
  - N15: CHANGED. Both tiers refuse with `MissingAlphaRenameContract`, and all 48 outputs run correctly on 6 engines.

- Oracle answers confirmed on tclsh 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim 0.84 for N01, N02, N03 (including the nested-brace case), N04, N05, N06, N08, N09, N10, N13, N14, N15 and the N17 capture driver. Every one matches the review's expected outcome.
- Statuses settled by static inspection, each still to be cross-checked by probe where noted:
  - `insert_rebound_candidates`: NOT-APPLICABLE. Removed in cf8f1bc85b.
  - `method_barrier`: FIXED. The tail-based `class_head_component` was removed in 944d156922.
  - `WorkspaceIndex::registry_for`: FIXED. It now borrows the document's retained registry (8aa4dfea79).
  - `ns_var_global_fallback`: FIXED. It prefers the resolved profile (944d156922).
  - N14 callback arity: FIXED. The tail-bucketed `project_command_arities` was removed in 3e29714009.
  - N13 caps: still present (`ROUND_CAP = 5` in `imported_constants`).
  - N11 `merge_library_files`: the early return on `workspace_command_exists` is still present.
