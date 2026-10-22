# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T12:30Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

**Status per finding:** see [../reviews/naming-migration-review-20261022-triage.md](../reviews/naming-migration-review-20261022-triage.md). It is updated after every run.

**Settled:**

| Finding | Status |
|---|---|
| N01 | REPRODUCES; the empty-tail part is FIXED |
| N02 | CHANGED |
| N03 | FIXED |
| N08 | FIXED |
| N09 | constructor FIXED; call graph CHANGED |
| N10 | CHANGED; `namespace path` class selection REPRODUCES |
| N11 | CHANGED; merge early return REPRODUCES |
| N12 | FIXED |
| N13 | REPRODUCES |
| N14 | FIXED |
| N15 | CHANGED (refuses) |
| N16 | FIXED |
| N17 template | FIXED |
| `ns_var_global_fallback` | FIXED |
| `registry_for` | FIXED |
| `analyse_with_workspace_classes` | CHANGED |
| `insert_rebound_candidates` | NOT-APPLICABLE |
| `method_barrier` | FIXED |

**Refused, pending R1** (re-check once compiler-names' completed-command-world fix lets rename produce edits): N04, N05, N06, N07.

**Pending run 4:**
- N17 live hook, including the existing `pack_source_e2e` fold test;
- the nested passthrough lowering, extract/inline and linked-editing probes;
- the section 4.1 scaling rows.

**Re-run the current batch:**

1. Restore the probe if it is absent: `cp docs/work/name-handling/reviews/triage-20261022/zz_review_probe.rs.txt rust/tcl-spectcl/tests/zz_review_probe.rs`.
2. Wait until `df -h /` shows more than 2.3 GB free.
3. Run `bash docs/work/name-handling/reviews/triage-20261022/run4.sh`. It builds, runs and deletes in one command, and writes its log to the scratch `run4.log`; copy that log into `triage-20261022/`.
4. To re-check R1 later, run the test filter `rename_with_registry` in the same way:

   ```sh
   CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 \
     cargo test -p tcl-spectcl --test zz_review_probe -- --nocapture --test-threads=1 rename_with_registry
   rm -f target/debug/deps/zz_review_probe-*[0-9a-f]
   ```

   Run both lines in one command. Then apply the edits it saves under `ZZ_SCRATCH` with `oracle.sh`.

**Next steps after run 4:**

1. Fill the PENDING rows in the triage report.
2. Refresh `triage-20261022/zz_review_probe.rs.txt` from the probe file.
3. Delete `rust/tcl-spectcl/tests/zz_review_probe.rs`.
4. Hand back.

## In progress

- Run 4, started 12:13Z, is waiting on the shared cargo lock. It runs the seams, the scaling rows (P = 25–200) and the existing `pack_source_e2e` fold test.

## Outstanding (in priority order)

1. Interpret run 4 and fill the PENDING rows in the triage report.
2. Refresh the evidence copy of the probe, then delete `rust/tcl-spectcl/tests/zz_review_probe.rs` and the scratch files.
3. Later, after R1 lands: re-run `rename_with_registry` and settle N04, N05, N06 and N07.

## Blocked (on whom, and what is needed)

- N04, N05, N06 and N07 are blocked on R1 (compiler-names): rename refuses plain static cases such as `proc foo; puts [foo]`.

## Done since the last update (with evidence: test counts, commit or snapshot)

- At 12:30Z, the triage report was rewritten with every settled row, both in the summary and in per-finding detail: evidence, oracle result, owner, files and fix direction.
- Run 3 rename results (with registry and program view):
  - N04 and N05 refuse with "unknown-completed-command-world".
  - N06 refuses with specific collision reasons.
  - N07 refuses with "unreferenced-edit".
  - The plain-rename controls `puts [foo]` and `set x [foo]` refuse with "incomplete-original-command-coverage".
- N11: p2 is classified `Conditional` and still selected.
- N17: the `mylib::strlen` control also emits no O129.
