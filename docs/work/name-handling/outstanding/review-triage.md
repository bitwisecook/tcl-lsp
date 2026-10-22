# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T13:47:32Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. This agent commits and pushes it after each run and at least every 30 minutes.

## Resume

**Active:** run5 is still running (shell PID 18462; log `../reviews/triage-20261022/run5.log`). The first main command exited 101 because the selective checkout omitted compile-time fixture includes. The scaling command also exited 101 for missing includes while the compiler continued building. The compiler library is now built, and the e2e step is building its remaining dependencies. All three fixture trees embedded by `tcl-test-support` are now restored: `native_encoding_utf8_object306`, `native_encoding_utf8_offset307`, `native_ensemble_map_prefix312`. No new named probe result is available yet; do not count these checkout errors as product failures. The saved temporary probe was deleted before this checkpoint: both probe commands already failed in a dependency, and the remaining e2e step does not need it. Exact launched source: `run5-source.rs.txt`.

**Next:** let run5 finish, record all three exit codes and any e2e result, then record and commit/push its final result. The partial log and independent oracle/static runs are checkpointed while code generation continues. Fetch/rebase as required. Run `source /tmp/review-triage-env.sh; bash docs/work/name-handling/reviews/triage-20261022/run6.sh` for the full replay (fresh `out/run6/`, six-profile rename, completed-world presence, N01 navigation, N17 direct/pure effect control, sibling collision from both cursor origins, all findings and scaling). Then run `bash .../replay-oracles.sh run6` and `python3 .../source-audit.py > .../run6-source-audit.log`. `run6.sh` restores and removes the temporary probe and deletes each executable in the same command. Its exit status reports real test failures, including the live-fold e2e test.

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
| N01 | REPRODUCES (slot collapse); empty tail FIXED |
| N02 | CHANGED (Jim identities distinct; definition abstains) |
| N03 | FIXED |
| N08 | FIXED |
| N09 | constructor FIXED; call graph CHANGED (no summary for `::n::c`) |
| N10 | CHANGED; `namespace path` class selection REPRODUCES |
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
1. N17 live hook. No O129 appears for any hook, including the `mylib::strlen` control. Run the existing `pack_source_e2e::a_pack_file_folds_a_call_site_in_the_optimiser` to confirm whether live const-fold hooks fold at all. compiler-codegen's `zz_ccg_fold_probe.rs` may already cover this.
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

- Starting run5: pending seam checks, live-fold control/e2e, rename with registry, and small scaling rows.
- Required release gates `make rust-check` and `make prep-pr` are not run; this is evidence-only work under the active `work` development-push exception.

## Outstanding (in priority order)

1. Run `run5.sh` in the cloud session and settle the pending items listed above.
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
