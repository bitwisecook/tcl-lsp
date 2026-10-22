# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-22T17:14Z

Maintained by the bigip-lsp agent. Keep this snapshot and Resume current, and
push at least every 30 minutes and after each finished item. Full unit table:
`../handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## In progress

The broad owned-root lint pass reached every selected crate and found 16
further owned assertion-style findings (editor generators, XC, CLI), all
changed to equivalent typed equality/inequality assertions. Total test-only
conditions preserved: 66. Cached verification is running; 53 non-editor
xtask and 40 registry integration findings are indexed below. The missing
registry include! fixture was restored unchanged from HEAD.

Comparison/generator build has cleared Compiler, VM, core and dependencies
and reached the xtask test target. It then runs the three generated-region
tests and the unchanged plain-Tcl rewrite test WITHOUT only the F5 pull
change. `/tmp/bigip-without-f5.py` restores both server files in its finally
block. No comparison result yet; with-F5 failed at the applicable-rewrite
assertion (55.84s). Server scoped lint must wait for restoration.

## Outstanding (in priority order)

1. Restore ILX variable-handle source navigation after the compiler supplies
   the constructor/setter/read relation described under Blocked. Keep the
   current negative assertions and separate source candidates from live handles.
2. Confirm `current_diagnostics::tests::actual_direct_and_push_rewrites_retain_their_captured_document_source`
   fails without the F5 pull change, or fix an attributable regression. The
   exact F5 change is in `c9dc3c2892` (current_diagnostics.rs and only its F5
   lib.rs hunks). Temporary reverse-and-restore testing must retain assertions.
3. Clippy `-D warnings` on the owned crates and editor generators, including
   tcl-bigip, tcl-bigip-io, tcl-bigip-query, bigip-query-wasm, tcl-irules, f5-xc,
   f5-cli, tcl-f5mku, bigip-report-gen, tcl-lsp-core, tcl-lsp-server, xtask and
   the previously changed tcl-registry lines. Fix owned findings; route other
   owners' blockers without editing their files.
4. Re-run the three `xtask gen_editor_configs` tests. Its generated Emacs README
   was previously regenerated and `gen-editor-configs --check` passed.

## Blocked (on whom, and what is needed)

- Non-editor xtask/registry owners: **53 + 40** assertion-style Clippy
  findings from the broad scoped pass, outside this agent's explicit editor
  generator/previous-registry-line scope. This includes the BIG-IP schema and
  F5-query documentation gates (not editor generators). Exact primary
  locations/command: `../handoff/reviews/merge-inventory-20261019/bigip-lsp/clippy-consumer-blockers-20261022.json`.
  No allowance or assertion changes in those files. Library-test registry
  checking initially hit a missing sparse fixture; restored, rerun pending.

- Generic LSP owner: fresh normal-library core Clippy has **54 errors**,
  zero in owned BIG-IP/F5 modules. The six owned production lint findings
  are cleared without an allowance. Exact command/primary locations:
  `../handoff/reviews/merge-inventory-20261019/bigip-lsp/clippy-core-blockers-20261022.json`.
  Test lint for core is still gated by its library errors; this is not a
  whole-crate green result.

- Full strict Clippy: **789 compiler + 379 VM errors**, exit 101, source
  `aeecf397` with Rust 1.99.1. BIG-IP/LSP/editor consumers were not reached.
  Compiler and VM owners must clear these causes; no new lint allowances.
  Exact command, environment, primary file/line/message inventory and raw-log
  hash: `../handoff/reviews/merge-inventory-20261019/bigip-lsp/clippy-dependency-blockers-20261022.json`.
  The selected-package `--no-deps` pass has reached core (60 errors);
  six owned findings will be fixed, while generic core findings need their owner; it cannot replace this failed full gate.

- ILX: the compiler owner must supply an authenticated original source
  constructor/setter/later-read relation for remote handles. The current
  tcl-irules producer deliberately writes `target: None`; this is not a failed
  compiler lookup. Its contract and tests explicitly forbid the former local
  `HandleScope` inference. Repro, exact cause, existing-owner analogy, F5
  provenance and required contract:
  `../handoff/reviews/merge-inventory-20261019/bigip-lsp/ilx-handle-owner-repro.md`.
  Baseline `origin/rust` (`fcb7927f02`) literal-handle test: 1/1 PASS; current
  tests retaining the refusal: 2/2 PASS. No assertion was weakened. BIG-IP owns
  wiring the new source relation into navigation once available.
- Current ILX e2e: **0/16**, all first `publishDiagnostics` barriers exceed
  30 s on this isolated tree, with healthy harness scheduling probes. The
  plain-Tcl control also fails before any navigation assertion. Exact existing
  test repro, source shape and diagnostic/performance contract:
  `../handoff/reviews/merge-inventory-20261019/bigip-lsp/diagnostics-latency-repro.md`.
  The hot function is not yet attributed; route to generic diagnostic/compiler
  and performance owners. Do not raise budgets. Perf batch not admitted.

## Done since the last update

- XC condition source ownership FIXED in f5-xc. Expression AST ends are
  inclusive; `expression_operand` incorrectly treated them as half-open and
  clipped `]`. It now calls the shared `tcl_syntax::expr::expression_span`.
  The saved probe confirmed current source/input/configuration/Registry and
  retained bindings, then `operand false`. The new ASCII/Unicode/nested-getter
  regression fails before the fix and passes after. Full f5-xc lib: **31/31
  PASS**, including all previous source/registry refusal assertions (no edits
  to their conditions; later Clippy-only spelling changes retain them). Evidence in `bigip-lsp/`: `xc-condition-owner-probe-result-20261022.txt`,
  `xc-range-before-20261022.txt`, `xc-lib-after-20261022.txt`. Probe removed;
  every test executable was built, run and deleted in one command.
- Server: XC e2e **5/5 PASS**, BIG-IP/APL W109 e2e **1/1 PASS**, focused F5
  pull-report unit **1/1 PASS**. Receipts `server-e2e-20261022.txt` and
  `server-f5-pull-20261022.txt`. ILX failures are the separate blocker above.
- `current_diagnostics.rs handed back`: F5 pull behavior is confirmed by both
  e2e and focused unit. Only the authorised temporary reverse-and-restore
  comparison remains; no further production edit to this loaned file planned.
- Standalone `bigip-query-wasm` Clippy on `wasm32-unknown-unknown`:
  **PASS**, zero warnings, Rust 1.99.1 and workspace-local Clang 19.1.7.
  Exact command/environment/hash receipt: `wasm-clippy-20261022.json`.
  Six native owned production-code lint findings have mechanical fixes in
  data-group rendering/ILX hover; compilation confirmed by the WASM graph,
  normal-library native Clippy verification is complete: zero owned F5
  findings, with 54 generic blockers. No new lint allowance.
- ILX comparison: `ilx-baseline-20261022.txt` (baseline 1/1) and
  `ilx-current-20261022.txt` (current 2/2). This establishes the deliberate
  contract change, not restored navigation or a passing e2e run.
- Earlier F5 pull implementation and all 35 BIG-IP units are in the notes.
  Historical F5 unit filter was 32/33; its plain-Tcl rewrite failure still
  needs the attribution above. ILX and strict clippy remain blocked; F5 pull and XC are green.

## Resume

- Validation checkout: `/workspace/bigip-lsp`, source `aeecf397` plus the two
  owned production lint-only files pushed in `f26f6a06` and the 50 test-only
  assertion-style changes pushed in `e4a01bbe`, plus 16 further test-only
  assertion spelling changes in the latest owned commit. Main branch HEAD stays
  frozen during builds. A separate coordination-only worktree at
  `/workspace/bigip-coordination` fetches/rebases and pushes tracking snapshots.
  Its latest pushed HEAD includes newer compiler/core changes; these builds
  do not validate those later product changes.
- `/tmp/bigip-env.sh` exports CARGO_INCREMENTAL=0, test/dev debug=0,
  TCL_LSP_REQUIRE_TCLSH=9.0,8.6 and the verified pinned Jim/Tcl paths, then
  sources agent-build-env.sh. `/tmp/bigip-test.py` builds, runs and deletes
  every test executable within one command. Independent baseline checkout
  `/workspace/bigip-baseline` remains at `fcb7927f02` with its own target.
- Server session 68675 is DONE: selected e2e 6 pass/16 first-diagnostic
  timeouts, F5 unit 1 pass, plain rewrite unit 1 failure. Both executables
  were deleted. With-F5 receipt: `bigip-lsp/rewrite-with-f5-20261022.txt`.
- Active comparison/generator session **24095** uses
  `/tmp/bigip-without-f5.py`; it has TEMPORARILY REVERSED only the two-file F5
  change `c9dc3c2892`. Its finally block restores both exact originals.
  **Do not commit the current temporary server diffs or move main HEAD.**
  The plan runs the three generated-region comparison tests first, then
  the existing generic rewrite assertion without F5; build/run/delete in
  one command. Log `/tmp/bigip-without-f5.log`.
- Native scoped lint **46036** finished exit 101 (50 owned assertion-style
  findings, now changed equivalently, plus the old core check). The core
  check had encountered an intermediate adapter lifetime error; that owned
  error is fixed and compilation confirmed by WASM. Finished **23728** has finished fresh core lint (54 generic errors)
  and completed owned roots (16 additional owned fixes now applied) with core as a dependency (all `-D
  warnings --no-deps`). Logs `/tmp/bigip-clippy-{owned-after,core-final,roots-final}*`.
  Active cached verification **25657**; comparison/generators **24095**.
  Fresh cached root verification logs: `/tmp/bigip-clippy-roots-after*`.
  Standalone WASM **30313** finished PASS with zero warnings.
- Next: finish the above; fix owned lint findings, route generic ones;
  after the comparison restores intact F5 source, run server scoped lint.
  Capture attribution/three generator results, update NOTES and this file,
  sync main only after verified restoration, fetch/rebase/push, then stop.
  `current_diagnostics.rs handed back`; no further production change planned.
- Original `/workspace/tcl-lsp` setup rebase was aborted, tracked files clean;
  its ignored oracle trees remain reachable via the active `tmp` symlink.
  Sparse checkout materialised 218 literal fixture inputs from HEAD without
  changing assertions. No temporary source probe remains.
- Required repository/host gates remain pending under the authorised
  development-push exception. Never open a PR; the PR to `rust` remains gated
  on external-review section F, including integrated-tip validation.
