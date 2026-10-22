# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load cost.

Last updated: 2026-10-22 17:38 UTC

Maintained by the perf agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. The coordinator commits and pushes it with each WIP snapshot.

## Resume

Active checkout: `/workspace/tcl-lsp-work`, shallow `work` clone with selected
sources and proof inputs. Source HEAD is now `e4a01bbe` (latest fetched work),
with the full toggled batch reproduced exactly against that base. Existing
binaries continue their `8a01a718` runs without changing their input bytes;
new proof outputs are isolated under `perf/evidence/latest-e4a01bbe/`.
The prior local docs graph was already pushed/rebased; it is preserved locally
at refs/perf/base-before-latest-sync. Source adoption preserved every live
output and all nine proof edits; none overlapped the fetched upstream paths.
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

The original 102-path Tcl roster contained **101 UTF-8 sources and one
intentional byte-native input** (native_deep_array_word.tcl contains 0xff).
The old baseline silently skipped it; the original claim of 102 timed analyses
was incorrect. corpus-original-102.txt preserves that exact roster. The current
corpus.txt retains all 101 text inputs and adds name-consumers/classes.tcl,
for 102 genuine AnalysisResult cases. corpus-native-bytes.txt retains the
original byte input, exercised with its existing native codegen test in both
modes. The added origin timing completed (740 ms). corpus-roster-audit.json
records exact counts, hashes and channels; no lossy UTF-8 decoding is used.
All **102 text `.tcl` + 15 `.irul` inputs are present**. The origin/rust timing run
has completed on all of them, the synthetic cases, exact Tk tclIndex prefixes,
and actual LSP document/index/existence/code-lens and source-chain shapes.
These baseline timing-only rows do not compare AnalysisResult across branches.
The optimized base-8a01a718 build finished in 90m 59s. Its full-batch
old/new/old corpus run STOPPED: the aliases.tcl case does not equal either
old result (diagnostics match), then strict UTF-8 reading rejected the byte
fixture. No complete corpus verdict. An all-old control (ZZ_OLD_ENV=1) also
computes old1 != old2 and middle != either endpoint, so this is not yet a batch
regression. The expensive full Debug rendering was stopped only after the
complete PartialEq returned false; aliases-old-only-control-classification.json
records that fact. No equality assertion is weakened. A separate complete ordered PackSet Debug comparison PASSED:
old1=62828 ms, new=25493 ms, old2=120495 ms, deterministic=true,
equal_complete_debug=true. Exact output: packset-base-8a01a718-threeway.out.
10/40/100-line tclIndex results are exactly equal; the 253-line old run is
still active. The 100-line old timing was instrumented by a live debugger
pause and is not an uninstrumented latency comparison.

The lexer lib proof finished: **583 pass / 3 fail in both modes**, identical
pass/fail inventories and zero pass-to-fail. Captured outputs and summary:
`perf/evidence/tests-lexer-lib-{new,old}.out`, `tests-lexer-summary.json`.
The identical failures are original_jim_syntax_roster, original_parse_descent,
and original_bracket_boundaries; assertions are unchanged. Compiler lib,
complete-input SpecTcl and the two server tests are still pending.

The standalone six-file Clippy check completed after cleanup: **zero diagnostics
in the six owned files**, 783 diagnostics elsewhere. Full-batch Clippy before
cleanup had 791 diagnostics, seven in owned files and one for the temporary
realm query-memo initializer. All seven owned diagnostics are fixed; the clean
routed patch uses the explicit memo initializer. No new allowance is added.
The complete structured reports are `perf/evidence/clippy-{full-before,owned-after}-cleanup.json`.
This is not a green workspace lint verdict.

The pack-word line helper now reuses the actual image's native LF index rather
than scanning its prefix; it retains the original bounds check and one-based
line conversion. The probe switch restores the exact old counting path.
The added fixed unit control covers CRLF, bare CR, EOF/out-of-bounds, opaque
bytes and both channels. Existing assertions are unchanged. The formal advice
entry also borrows the realm memo directly, avoiding a wrapper used only by the
unlanded realm integration. Refreshed patches still contain exactly six owned
and two routed files, no switch. `tools/refresh_patches.py ROOT --snapshot
/tmp/perf-clean-batch --check-toggled` reproduces the proof tree byte-for-byte.

## In progress

- Alias field diagnostics: /tmp/perf-alias-diagnostic-build.log, command waiting
  for the latest compiler test build to finish in target-tests, then builds
  zz_diagprobe with the same normal test-profile libraries. It runs old-only
  and old/new/old with both ordinary ingress and one retained complete input
  generation (ZZ_RETAIN_INPUT=1). The latter is diagnostic, not yet the corpus
  verdict. The probe now prints differing public field names plus retained
  realm/world equality rather than formatting the complete shared graph.
  All final verdicts still use the complete AnalysisResult PartialEq.
- Native-byte fixture control waits for that same compiled compiler test binary,
  copies it privately, and runs native_array_indices_emit_iteratively_from_the_full_original_arena
  in both modes. It asserts the original 2000 array instructions and x-ff literal
  through the existing test, with no changed Rust assertions. Results go under
  evidence/latest-e4a01bbe/tests-compiler-native-array-bytes-*.out.
- Latest lexer finished: 583 pass / same 3 fail in both modes; zero pass-to-fail.
  The base golden new run finished 2 pass / 1 fail after 3537.662 s: eda_xilinx's
  interpreted upgrade reached its unchanged 30-minute wall-clock budget. Old
  golden remains active, so no comparison verdict for that suite yet.
- Ordinary LSP lens rows previously asserted declaration/index/lens survival,
  which alone cannot exclude unavailable reference reports. The template now
  ALSO requires each fixture's one positive reference title. Re-run both origin
  and latest document measurements with that stronger control before reporting
  the final ordinary-path table. The older rows are provisional for this reason.

- Latest proof build/run: /tmp/perf-run-latest.sh, target-latest, separate
  zz_latest_perfprobe/zz_latest_spectclprobe sources and binaries. Logs:
  /tmp/perf-latest-probe-build.log and /tmp/perf-latest-run.log. Outputs:
  perf/evidence/latest-e4a01bbe/. The initial latest build was stopped early
  to give it a separate target; it must not share base dependencies that the
  supplemental optimized SpecTcl script may still read. Same-worktree cached
  dependencies were copied; Cargo rebuilds the changed workspace crates.
- Latest lexer/compiler default-profile runs: /tmp/perf-latest-crate-lexer-compiler.log,
  target-tests, with --evidence-dir perf/evidence/latest-e4a01bbe. The base
  SpecTcl executable is statically linked and unaffected by library rebuilds;
  do not rebuild/delete its golden executable before its existing run ends.
- The base optimized SpecTcl eval_loader control passed all 28 new-mode tests
  in 1482.833 s and is running old mode. Its later golden compilation must not
  be accepted as base evidence: the first latest Cargo attempt invalidated the
  base compiler rlib. The required unoptimized base golden run remains valid
  and active; latest complete-input SpecTcl still requires both suites/modes.
  Never treat a missing/mixed dependency build failure as a semantic regression.
- The two server filters now use the explicitly requested 180-second limit in
  BOTH modes. The runner lists each selected test first, records timed_out
  separately from Rust assertion outcomes, and preserves exact declared test
  inventories. No assertion or ignored-test state is changed. The queued
  initial server command will compile current source, records its HEAD/diff
  fingerprint, and therefore belongs to latest source rather than base 8a.

- origin/work advanced to `a5aeb5a7` with source changes in other owners' files.
  Active build results are pinned to production base `8a01a718`; they are not
  final rebased evidence. After preserving these results, rebase on the latest
  work and repeat required proofs before landing. Owned/routed patch paths have
  no overlap with the newly fetched production changes.

- SpecTcl's initial eval_loader run completed with the same 26 passes and two
  failures in both modes, zero pass-to-fail. One failure was the omitted sparse
  shipped-pack inventory directory; that directory is now fetched and the full
  SpecTcl stage is queued to rerun in both modes. The other is the unchanged
  budget-axis assertion (wall-clock budget reached before command-step budget)
  on this contended host. Initial logs will be preserved as incomplete-input
  evidence, not accepted as the final complete-input proof.
- Optimized corpus, tclIndex and ordinary LSP probes are running. The first
  10/40-line tclIndex rows are exactly equal; 100/253 remain pending. Document
  lenses retain all measured rows and remain much slower than origin/rust; the
  memo batch does not fix the request-scoped Logical inventory rebuilds in
  residual-routing.md. No complete equality or fixed-timeout verdict yet.
- An additional optimized SpecTcl integration control links the exact completed
  Cargo rlibs and compiles the unchanged eval_loader/golden_packs test sources,
  with test debuginfo=0 and debug/overflow assertions enabled. Its source/artifact
  provenance and results are separately named tests-spectcl-opt1-*. This does
  not replace the requested unoptimized Cargo test runs. Command and log:
  /tmp/perf-run-spec-opt1.py, /tmp/perf-spectcl-opt1.log.
- The complete-input SpecTcl retry now reruns eval_loader only, retaining the
  already complete-input golden outcomes when they finish. run_crate_proofs.py
  --suite preserves other suite summaries and keeps the same assertions and
  exact pass/fail inventory comparison.

- Current optimized proofs are executing the final owned cleanup. Do not reapply the
  batch or interrupt a build merely to publish evidence. A docs-only worktree
  `/workspace/tcl-lsp-publish` fetches/rebases and pushes snapshots while the
  source tree is building. Its rebased evidence commits can differ from the
  active tree's commit IDs; production Rust is unchanged between those bases.
- Initial compiler lib test build stopped before execution on one omitted
  `include!` fixture. That file and 47 other literal Rust includes are now
  fetched. A second compiler build exposed a macro-composed missing fixture and
  three byte-array-to-Arc type errors in the new cfg(test)-only line control.
  Macro includes are now fetched; the control uses byte slices and passes in
  both modes when the exact helper and test are compiled against the lexer rlib.
  The complete compiler stage still needs rebuilding. `/tmp/perf-crate-rest.log`
  is running SpecTcl then server. Results and
  summaries are written under perf/evidence. No compiler/SpecTcl/server test
  verdict is claimed until those runs finish.

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

- Compiler-names: `perf/perf-analyser-images.patch` and, on top of it,
  `perf/perf-analyser-segments.patch` are **unproven drafts**, deliberately not
  applied to the proof tree. Images retains full byte/channel correspondence;
  segments reuses the actual origin's existing complete-vector projection.
  `perf/residual-routing.md` records the exactness argument and required
  corpus, mutation/channel/config/partial-source and tclIndex controls.
  These artifacts are routed candidates, not a landed or proven speedup.
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
