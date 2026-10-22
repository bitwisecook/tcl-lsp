# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-22T15:46Z

Maintained by the bigip-lsp agent. Keep this snapshot and Resume current, and
push at least every 30 minutes and after each finished item. Full unit table:
`../handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## In progress

WASM Clippy PASS (zero warnings); native scoped Clippy continues, followed
by a fresh core pass and owned roots checked with core as a dependency.
The F5-reversed rewrite comparison and three editor-generator tests are
building in one command (`/tmp/bigip-without-f5.log`). Its finally block
restores both loaned server files byte for byte. The with-F5 rewrite case
failed at the existing "actual worker must retain an applicable rewrite"
assertion; attribution awaits the reversed result.

## Outstanding (in priority order)

1. Restore ILX variable-handle source navigation after the compiler supplies
   the constructor/setter/read relation described under Blocked. Keep the
   current negative assertions and separate source candidates from live handles.
3. Confirm `current_diagnostics::tests::actual_direct_and_push_rewrites_retain_their_captured_document_source`
   fails without the F5 pull change, or fix an attributable regression. The
   exact F5 change is in `c9dc3c2892` (current_diagnostics.rs and only its F5
   lib.rs hunks). Temporary reverse-and-restore testing must retain assertions.
4. Clippy `-D warnings` on the owned crates and editor generators, including
   tcl-bigip, tcl-bigip-io, tcl-bigip-query, bigip-query-wasm, tcl-irules, f5-xc,
   f5-cli, tcl-f5mku, bigip-report-gen, tcl-lsp-core, tcl-lsp-server, xtask and
   the previously changed tcl-registry lines. Fix owned findings; route other
   owners' blockers without editing their files.
5. Re-run the three `xtask gen_editor_configs` tests. Its generated Emacs README
   was previously regenerated and `gen-editor-configs --check` passed.

## Blocked (on whom, and what is needed)

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
  to those assertions). Evidence in `bigip-lsp/`: `xc-condition-owner-probe-result-20261022.txt`,
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
  Six native owned lint findings have corresponding mechanical fixes in
  data-group rendering/ILX hover; compilation confirmed by the WASM graph,
  native lint verification remains pending. No new lint allowance.
- ILX comparison: `ilx-baseline-20261022.txt` (baseline 1/1) and
  `ilx-current-20261022.txt` (current 2/2). This establishes the deliberate
  contract change, not restored navigation or a passing e2e run.
- Earlier F5 pull implementation and all 35 BIG-IP units are in the notes.
  Historical F5 unit filter was 32/33; its plain-Tcl rewrite failure still
  needs the attribution above. ILX and strict clippy remain blocked; F5 pull and XC are green.

## Resume

- Active shallow `work` checkout: `/workspace/bigip-lsp`. Independent
  `origin/rust` baseline: `/workspace/bigip-baseline` at `fcb7927f02`.
  Rust 1.99.1, pinned C Tcl 9.0.4/8.6.18 and Jim 0.84 are built and verified.
  No temporary probe is left in the source tree.
- Temporary `/tmp/bigip-env.sh` exports the required oracle variables and
  CARGO_INCREMENTAL=0 plus test/dev debug=0, then sources agent-build-env.sh.
  `/tmp/bigip-test.py` builds, runs and deletes each test executable within one
  command. Build targets are independent between checkouts.
- Active server run uses `BIGIP_TEST_PLAN` from `/tmp/bigip-server-plan.json`
  with `/tmp/bigip-test.py --locked -p tcl-lsp-server --lib --test e2e`. The
  e2e/F5 runs above are done; only the plain-Tcl rewrite process remains. Do not restart it as an unfiltered full suite.
  Original server session 68675 finished and deleted both test executables.
  Active sessions: negative comparison/generators 24095; native scoped
  lint 46036; queued core/owned-root lint 23728. WASM 30313 finished PASS.
  Logs `/tmp/bigip-{without-f5,clippy-owned-after,clippy-core-final,clippy-roots-final,wasm-clippy-after}*`.
  `/tmp/bigip-without-f5.py` reverses only the F5 change and restores in a
  finally block. Do not commit the temporary server diffs.
 The original `/workspace/tcl-lsp` setup rebase was
  aborted and its tracked files are clean; its ignored oracle directory is
  retained through the active checkout's `tmp` symlink.
- Next: finish the running server and clippy checks; rewrite attribution
  with/without the F5 change; standalone bigip-query-wasm clippy; three generator
  tests; hand the diagnostics file back.
  Keep unresolved/compiler-owned items explicit. Never open a PR; the PR to
  rust remains gated on external review section F.
- The first server build needed 76 literal oracle snapshots excluded by the
  sparse checkout. They and 141 further literal fixture inputs are now
  materialised from HEAD; no code or assertion was changed. The restarted
  server build has completed and execution has started. The selected e2e aggregate is 6 pass / 16 diagnostic-timeout failures.
- Validation above used the initial `9cd5a75b` product source plus the XC fix.
  Remote includes newer compiler/core changes in `d1a5bf95`; running checks
  remain pinned to `aeecf397`, so they do not validate those later changes. Required
  repository/host gates remain pending under the authorised development-push
  exception; these results do not grant release sign-off.
