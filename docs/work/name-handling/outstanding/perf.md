# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load cost.

Last updated: 2026-10-22 14:00 UTC

Maintained by the perf agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Active checkout: `/workspace/tcl-lsp-work`, shallow `work` clone with selected
sources and proof inputs. Base for the current full-batch proof: `8a01a718`.
The original checkout at `/workspace/tcl-lsp` has an interrupted checkout from
an oversized history fetch; do not use it as the active source tree. Its ignored
`tmp/` holds the built Tcl/Jim oracles, shared through the active checkout's
`tmp` symlink. The separate `/workspace/tcl-lsp-rust` worktree is the verified
`origin/rust` baseline `fcb7927f02b5395753247b322fdd7be87ae5c731` and has its own target.

The FULL batch is now applied in the active tree with the temporary old/new
switch. No production code is committed yet. All nine changed source paths
(including the lexer switch and compiler-owned realm/interprocedural files)
must remain outside evidence-only checkpoints. Land only the six perf-owned
files after the proofs; remove the switch before landing. Refresh and route
`perf-realm-interprocedural.patch` separately after proof.

Repository evidence remains in
`docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/`.
The helper applied by exact anchors on the base above. Its parse-cut change
now reuses `image.bytes()` rather than adding an eighth argument or a Clippy
allowance. The corpus probe now fails on unreadable inputs, panics and
mismatches. The PackSet probe compares the entire Debug structure in an
old1/new/old2 run, preserving order and field association.

All **102 `.tcl` + 15 `.irul` inputs are present**. The origin/rust timing run
has completed on all of them, the synthetic cases, exact Tk tclIndex prefixes,
and actual LSP document/index/existence/code-lens and source-chain shapes.
These baseline timing-only rows do not compare AnalysisResult across branches.
The new full-batch corpus verdict is still pending compilation.

The lexer lib proof finished: **583 pass / 3 fail in both modes**, identical
pass/fail inventories and zero pass-to-fail. Captured outputs and summary:
`perf/evidence/tests-lexer-lib-{new,old}.out`, `tests-lexer-summary.json`.
The identical failures are original_jim_syntax_roster, original_parse_descent,
and original_bracket_boundaries; assertions are unchanged. Compiler lib,
SpecTcl and the two server tests are still pending.

A standalone six-file Clippy check is running in `/workspace/tcl-lsp-owned`,
with its own target. The batch API no longer adds a wrapper used only by the
routed realm integration: the existing formal advice entry borrows the realm
memo directly. No new allowance is added. Refreshed clean split patches have
six owned files and two routed files, with no ZZ switch.

Full-batch Clippy has reported failures in foreign files (similar names,
empty doc-comment spacing, unused unit and unnested or-patterns). The complete
check is still running; do not claim zero-warning gates. Diagnostic log:
`/tmp/perf-clippy-full-batch.log`. No foreign fixes are applied.

## In progress

- `/tmp/perf-run-work.sh`: builds the two toggled examples with `-j2`, opt-level
  1, line tables, then runs old1/new/old2 across both corpus lists and synthetic
  calls/procs/upvar, followed by the complete PackSet comparison. Build output:
  `/tmp/perf-work-probe-build.log`; results go to `perf/evidence/*run2_threeway.out`.
  The command deletes example executables on exit. Temporary source probes are
  `rust/tcl-compiler/examples/zz_perfprobe.rs` and `zz_spectclprobe.rs`; never stage.
- Completed baseline build/run at `/workspace/tcl-lsp-rust` (opt-level 1, `-j2`):
  `/tmp/perf-origin-build.log`, `/tmp/perf-origin-calls.out`,
  `/tmp/perf-origin-spectcl.out`. It also deletes executables in its command.
- Tcl 9.0.4, Tcl 8.6.18 and the pinned Jim oracle are built. Tk 8.6.18 is
  fetched for the identical tclIndex prefixes. Logs:
  `/tmp/perf-tcl-setup.log`, `/tmp/perf-jim-setup.log`, `/tmp/perf-tk-setup.log`.
- Full crate proof runner: `perf/tools/run_crate_proofs.py ROOT --target-dir ROOT/target-tests`; log `/tmp/perf-crate-all.log`. Lexer completed; compiler test build is active. Each executable runs new/old and is deleted in the same command.
- Shell build environment: `source /tmp/perf-env.sh`, then source the active
  tree's `scripts/dev/agent-build-env.sh`; export
  `TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`. Incremental and test debug are 0.
  The normal crates.io sparse endpoint returned HTTP 502. The official index
  served from GitHub plus the normal crate downloads works with HTTP
  multiplexing disabled; this is configured only in the session Cargo home.

## Outstanding (in priority order)

1. Three-way corpus equality rerun (resume step 4).
2. Crate tests with and without `ZZ_OLD_ENV` (step 6), aiming for zero pass-to-fail.
3. origin/rust timings for the same inputs: `git worktree add <dir> origin/rust`, copy the probe without the `ZZ_*` toggles, then time `calls 10,20,40,80`, the tclIndex prefixes, and load_embedded.
4. Land only the six perf-owned files as one commit without temporary switches; refresh and route the proven realm/interprocedural patch via the coordinator.
5. tclIndex residual (253 lines, 29 s at opt-level 1, mildly super-linear): per-command whole-document SourceImage/SourceMap/LineIndex rebuilds and re-segmentation. Candidate fix: one cached document SourceImage on the Analyser. Those are compiler-names / unowned files; send a patch.
6. `incomplete_body_operand_layout_advice` still loops over every earlier site per command (now ~7% of tclIndex samples). Index the sites if needed.
7. Review section 4.1/4.2: measure the ordinary LSP path after the batch; cover flat procedure index intake, existence queries, source-order chains and code lenses, and route fixes in other agents' files.

## Blocked (on whom, and what is needed)

- Compiler-names: `perf/perf-analyser-images.patch` is a formatted, apply-checked
  **unproven draft**, deliberately not applied to the proof tree. It reuses the
  genuine document image after a complete byte/channel check, including when
  callers mutate the public source buffer. Other source/input guards remain.
  Re-segmentation still needs a separate routed patch; original_source_segment
  currently reconstructs a projection and segments again although the actual
  source origin already owns the same complete-vector lexical projection.
- Current lint failures and the three matching lexer failures need their
  owning agents; this checkpoint is not a green workspace verdict.

- `realm.rs` / `interprocedural.rs` hunks: compiler agent, via the coordinator.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Resumed on a separate disk; read the requested state and scaling review.
- Shallow clone plus sparse selection avoids downloading old experiment history.
- Applied the full batch and verified all 117 corpus paths exist.
- Strengthened probe failure handling and complete PackSet comparison; no tests weakened.
- Full required crate proof is in progress. Full warning/fmt gates and both named LSP tests remain unresolved; development checkpoints use the work-branch push exception.

- Saved the clean split patches, apply script, probes, tools, timings, corpus outputs and stacks under `perf/`. Wrote `perf-NOTES.md`.
- Deleted /dev/shm/perf-target, /dev/shm/exp and the rust-wt worktree. The `zz_perfprobe.rs` tree probe was already deleted.
