# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T15:24:20Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. This agent commits and pushes it after each run and at least every 30 minutes.

## Resume

**Run5 finished.** Main and scaling both exited 101 for omitted compile-time fixture includes; all three embedded fixture trees are restored. The e2e step exited 101, but rebasing its tracked active log replaced the file while cargo still wrote the old inode, so that step's diagnostic output is incomplete. Do not infer its failure reason from `run5.log`.

**Executed repeat:** `run5-e2e-repeat.log` records the existing `pack_source_e2e::a_pack_file_folds_a_call_site_in_the_optimiser` on `53ec43d1`: **0 passed / 1 failed**, DSL 1 expected `["5"]`, got `[]`. No O129 replacement is emitted. Runtime 16.17 s. Its executable and the temporary probe are deleted. All shared libraries are now built.

**Run6 finished:** 24 probe functions completed; the combined main command hit its 3600 s cap during scaling (`exit-main=124`). Flat P=25/50/100/200 and all source-chain rows completed; only code-lens P=25 completed before the cap. The three seam functions were not reached. The later e2e step rebuilt against the rebased `90fe96e2` tree and again failed with expected `["5"]`, actual `[]` (`exit-e2e=101`). The temporary probe/executables are deleted.

**Oracle replay complete:** `run6-oracles.log` and `run6-oracle-summary.json`: 96 files, 576 engine executions, 76 generated files, 126 matching-profile checks, 0 unexpected matching results. C Unicode-prefix edits are intentionally not portable to Jim's different bare-variable grammar. All eight N17 drivers print `good` on all six engines. The added first-close indirect-name input is valid on C 8.x/Jim and invalid on C 9.x, as recorded in the replay.

**Next command after this checkpoint is pushed:** `source /tmp/review-triage-env.sh; CARGO_NET_OFFLINE=true bash docs/work/name-handling/reviews/triage-20261022/run-pending.sh`. It freshly builds once from `pending-probe.rs.txt`, deletes the temporary source, then runs seams FIRST, the N17 direct/pure contrast, R1 realm/result and seeding controls, N03 parser-debt controls, source chains and separately budgeted lens sizes. It only reruns the six-profile server rename matrix if a seeded `puts [foo]` control actually has a completed world. An uncompleted P=200 case would get its own budget, but run6 already completed it. Every executable is deleted in that one command.

**After that:** replay the new `out/run7-pending/` on the six oracles, update the report and this file, and push. The R1 post-fix requirement and final latest-work recheck are still outstanding. Run6's main executable was built at `5bd5a503`; incoming compiler/minifier changes were rebased while that immutable executable ran, and its later e2e cargo step rebuilt at `90fe96e2`. Do not describe the main observations as a complete latest-code recheck. Source audit is `run6-source-audit.log` on `90fe96e2`.

**Keep active build logs outside git mutations:** do not rebase a tracked log while its writer is running. Wait for the run to finish, then update/commit/push. During long runs a separate completed snapshot can be committed, but never replace the file an active command has open.

The run5 temporary `.rs` is the earlier source copied at launch; the evidence `.rs.txt` now contains the expanded follow-up. Run5's tested source is recoverable from checkpoint `81a911af`. The latest fetched remote is `f2e58ee7`; its changes after `81a911af` are documentation only. N17 `const_subst.rs` is now owned by compiler-codegen per the user's 13:17Z recorded decision.

Resumed on the separate cloud disk at 2026-10-22T13:17:02Z. Active checkout: `/workspace/tcl-lsp-triage`, a shallow selective checkout of `work`. Rust 1.99.1 and all six pinned oracle engines are ready. No new triage test has run yet.

**Next command:** source `/tmp/review-triage-env.sh`, then run `bash docs/work/name-handling/reviews/triage-20261022/run5.sh` from this checkout. The oracle trees are reused via the ignored `tmp` symlink. Setup evidence: `../reviews/triage-20261022/cloud-setup.log`.

The complete original checkout was too large to recover promptly; the user authorised a shallow selective clone. `experiments/`, unrelated documentation and two unrelated fixture directories are omitted. Fetch/rebase before each push; deepen the shallow clone only if a later rebase needs history.

- **Status per finding:** see [../reviews/naming-migration-review-20261022-triage.md](../reviews/naming-migration-review-20261022-triage.md). It has a summary table and per-finding detail, and is up to date through run 4.
- **Evidence:** [../reviews/triage-20261022/](../reviews/triage-20261022/README.md), containing the probe source `zz_review_probe.rs.txt`, logs `run1.log`…`run4.log`, edited outputs in `out/`, repros in `tcl/`, and `oracle.sh`.
- **Probe file:** `rust/tcl-spectcl/tests/zz_review_probe.rs` has been deleted from the tree. `run5.sh` restores it from the evidence copy and deletes it again when it finishes.

**Settled:**

| Finding | Status |
|---|---|
| N01 | CHANGED: metadata slot collapse remains; navigation now selects both correct declarations; empty tail FIXED |
| N02 | CHANGED (Jim identities distinct; definition abstains) |
| N03 | C Tcl FIXED; Jim CHANGED (definition/references abstain; rename refuses) |
| N08 | FIXED |
| N09 | FIXED in current constructor/call-graph repros (`::n::c` → `::lib::p`); taint not independently executed |
| N10 | CHANGED: path definition now correct; hover/type hierarchy remain empty |
| N11 | CHANGED (`if {0}` is `Conditional`, still selected); merge early return REPRODUCES (static) |
| N12 | FIXED |
| N13 | REPRODUCES |
| N14 | FIXED |
| N15 | CHANGED (refuses `MissingAlphaRenameContract`) |
| N16 | FIXED |
| N17 template | FIXED |
| `ns_var_global_fallback` | FIXED (static) |
| `registry_for` | FIXED (static) |
| `analyse_with_workspace_classes` | CHANGED (static) |
| `insert_rebound_candidates` | NOT-APPLICABLE |
| `method_barrier` | FIXED (static) |
| linked editing | FIXED (static and executed) |

**Refused, pending R1** (compiler-names' completed-command-world fix): N04, N05, N06, N07. Rename refuses even `proc foo; puts [foo]`.

**Pending:**
1. N17 is now settled: direct callbacks return `good` in all 8 DSL/reference/effects combinations. Explicit `world_effects none` gives O129 `good`; undeclared effects give no fold. Existing control e2e FAILS. Compiler-codegen owns the compatibility gate fix; triage only needs final latest-work confirmation and output replay.
2. Nested passthrough lowering for iRules: run 4's probe was inconclusive and has been reworked.
3. Extract/inline: print `Refactoring::disabled` to confirm these are refusals.
4. Section 4.1 scaling rows at P = 25–200. Run 3 measured 248 s to analyse a 250-proc file.
5. Re-check N04, N05, N06 and N07 via `rename_with_registry` once R1 lands.

**Command to re-run the pending batch** (from the repository root, with `df -h /` above 2.4 GB):

```sh
bash docs/work/name-handling/reviews/triage-20261022/run5.sh
```

It restores the probe, builds, runs and deletes the executables in one command, writes `triage-20261022/run5.log` and `out/`, then removes the probe file. Run edited outputs with `bash docs/work/name-handling/reviews/triage-20261022/oracle.sh <abs path>`.

**Then:**
1. Read `run5.log`.
2. Fill the PENDING rows in the triage report: the summary table and the per-finding detail.
3. Keep `triage-20261022/` under 5 MB.

## In progress

- Run6 active: current prefix evidence is in `run6-progress.log`. The direct/pure live-hook contrast is complete. Pending seam checks, completion of server-shaped controls, scaling and output replay.
- Required release gates `make rust-check` and `make prep-pr` are not run; this is evidence-only work under the active `work` development-push exception.

## Outstanding (in priority order)

1. Run `run-pending.sh` to settle the unexecuted seams, diagnose the still-failing R1 positive control, and complete the lens rows with individual budgets.
2. After R1 lands, re-check N04, N05, N06 and N07 on all six profiles, apply returned edits and execute them on the matching engines.
3. Fetch latest `work` and re-check every finding and static seam before the final summary.

## Blocked (on whom, and what is needed)

- N04, N05, N06 and N07 are blocked on R1 (compiler-names).

## Done since the last update (with evidence: test counts, commit or snapshot)

- Run 4 (12:13Z–12:39Z) ran 3 seam tests, all passing:
  - Linked editing returns only the declaration and its resolved self-call.
  - The inline-proc and extract-variable actions were offered, but applying their edits left the source unchanged, so they are probably disabled refusals. Both outputs run correctly on 6 engines.
  - The nested-passthrough probe was inconclusive.
- Run 4 was stopped before scaling and the e2e step when the triage was moved.
- The evidence directory was refreshed with the probe copy, `run3.log`, `run4.log`, `out/`, `run4.sh`, `oracle.sh` and the new `run5.sh`. The probe file was deleted from the tree, and the triage executables were deleted from `target/`.

- Cloud setup completed: Rust 1.99.1; C Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0; Jim 0.84 at `5bac7c99ad65864c87da513e22e2f01703fa4e03`. The standard setup hook stopped at missing sudo; its Rust and Tcl build helpers were reused with writable local paths.

- 2026-10-22T13:45:08Z: cloud baseline replay completed: 19 preserved Tcl files, 114 engine executions, no timeouts (`cloud-baseline-oracles.log`). Feature-unavailable rows retain their nonzero exit codes. N17 driver registers `good` on all six engines. Static source excerpts: `cloud-source-audit.log`; pending current-API rows remain pending.

- 2026-10-22T13:59:37Z: run5 finished; repeat of the existing fold test confirms a real failure (0 passed / 1 failed; DSL 1 expected ["5"], got []). Evidence: `run5-e2e-repeat.log`. No product files or tests were edited. Required release gates remain unrun.

- 2026-10-22T14:16:15Z: checkpoint of the first completed run6 observations; the active log is preserved outside the index. Snapshot: `run6-progress.log`. No new edit output has yet been validated on the oracles; those outputs remain uncommitted until replay.

- 2026-10-22T14:42:43Z: run6 confirms N09 call graph fixed, N10 path definition fixed (hover/hierarchy still missing), and N17 direct scanner results correct. Both DSLs fold with declared pure effects and fail to fold without that declaration. Server-shaped C controls so far: bare `foo` renames with a world; `puts [foo]` has no world; `set x [foo]` has a world but refuses coverage. R1 remains incomplete for the required positive case. A small prepared `run-r1-transport.sh` / `r1-transport-probe.rs.txt` can compare realm/result worlds before/after core seeding after the full run; it also checks N03 parser-debt helpers. These diagnostics are not yet executed.

- 2026-10-22T15:24:20Z: full run6 evidence and output replay recorded. Flat analysis (P25/50/100/200) = 23.490 / 73.748 / 237.256 / 630.311 s; index min-of-three = 1.043 / 3.332 / 5.019 / 10.881 s. Source chain F250/500/1000/2000 = 134.775 / 318.125 / 1396.401 / 5055.848 ms. P25 lens query = 6049.759 ms after 61798.1 ms analysis, 25 lenses. All are unoptimised wall-time measurements, not shipping latency.
