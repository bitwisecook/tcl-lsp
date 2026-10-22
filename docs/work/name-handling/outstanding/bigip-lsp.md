# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-22T18:08Z

Maintained by the bigip-lsp agent. Keep this snapshot and Resume current, and
push at least every 30 minutes and after each finished item. Full unit table:
`../handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## Final state

All unblocked owned work and requested checks are complete. No active build,
test, temporary probe or F5 reversal remains. XC, F5 pull diagnostics and the
three editor-generator comparisons pass. The existing plain-Tcl rewrite
assertion was actually rerun without F5 and still fails at the same point.
Scoped checks report zero owned F5/editor findings after the mechanical
cleanups; the full release gate remains blocked below.

`current_diagnostics.rs handed back` — its F5 behavior is confirmed; the
counterfactual reversal is complete and the original file is restored.
Five final lint-only lib.rs hunks were pushed in `d1ebce88` atop the newer
shared file, preserving the coordinator's changes. Total equivalent
empty/non-empty assertions retained: 69. No assertion or budget weakened,
no lint allowance added, no PR opened.

## Outstanding (blocked work only)

1. Compiler owner supplies the ILX constructor/setter/read source relation;
   BIG-IP then wires readonly source navigation and re-runs the 16 ILX e2e.
2. Diagnostic/compiler/performance owners resolve the first-diagnostic latency
   and applicable-rewrite fixture contracts below; re-run the original tests.
3. Generic owners clear the indexed strict lint findings and the coordinator
   completes integrated-tip repository/host gates under external-review F.

## Blocked (on whom, and what is needed)

- Generic LSP server owner: **35 normal-library + 193 test-target diagnostic
  messages** (includes duplicates), zero owned F5 findings. Six messages are
  in the returned current_diagnostics.rs, all in generic functions/tests.
  Exact command, functions/primary locations and completed log hashes:
  `../handoff/reviews/merge-inventory-20261019/bigip-lsp/clippy-server-blockers-20261022.json`.
  Integration/e2e test lint remains gated by generic library failures.

- Generic diagnostic/compiler rewrite owner: the existing plain-Tcl test
  `actual_direct_and_push_rewrites_retain_their_captured_document_source`
  fails at **the same applicable-rewrite assertion without the F5 change**
  (12.43s; with F5 55.84s). Assertions retained; both original server files
  restored byte for byte and both executables deleted. Exact fixture,
  command/configuration caveat and required source-association contract:
  `../handoff/reviews/merge-inventory-20261019/bigip-lsp/rewrite-without-f5-repro.md`.

- Non-editor xtask/registry owners: **53 + 176** assertion-style Clippy
  findings from the broad scoped pass, outside this agent's explicit editor
  generator/previous-registry-line scope. This includes the BIG-IP schema and
  F5-query documentation gates (not editor generators). Exact primary
  locations/command: `../handoff/reviews/merge-inventory-20261019/bigip-lsp/clippy-consumer-blockers-20261022.json`.
  No allowance or assertion changes in those files. Registry library-test input restoration is complete; all 229 remaining
  findings are assertion-style lints outside the owned loaned lines.

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

- Editor generated-region comparisons **3/3 PASS**, 348.52s, no ignored tests.
  Receipt `editor-config-tests-20261022.txt`. The existing plain rewrite test
  was actually run without F5 and still fails; source restored, no production
  edit to the handed-back file remains. Receipts `rewrite-{with,without}-f5-20261022.txt`.

## Resume

- This agent is stopped after completing every unblocked owned item. Resume
  consumer work only when the compiler/latency contracts above are delivered;
  keep current ILX refusal assertions and distinguish readonly source
  candidates from runtime handles. Never rebuild the old local HandleScope.
- Branch `work`, active checkout `/workspace/bigip-lsp`; coordination-only
  worktree `/workspace/bigip-coordination`. All owned code/evidence is pushed.
  Main checkout is synchronised after preserving/confirming its committed
  copies; the shared lib.rs uses only the five owned hunks atop newer upstream.
- Validation source was `aeecf397` plus owned production lint fixes
  `f26f6a06`, assertion-only edits `e4a01bbe`/`9d6804c9`, and the five final
  shared-lib hunks in `d1ebce88`. Later coordinator compiler/provider changes
  need integrated-tip validation. Receipts retain exact commands/source hashes.
- Completed tool sessions: server tests 68675; generator/without-F5 comparison
  24095; native/core/consumer lint 46036/23728/25657/26335; restored-source
  server lint 50998 and final verification 76335; WASM 30313. All finished.
  Every test executable was built, run and deleted within its wrapping command.
- Comparison restored current_diagnostics.rs to
  `7fcf6f6dc3b91a8224b6cec6dee8afd21700c6843c6690cd06cfc1217a92986a`
  and lib.rs to `830fd78baa30f46c2baf9944c6cce51183ecd591064e085027036e2467f663f5`.
  The later five permanent lint-only lib.rs hunks have tested SHA-256
  `92f15c40ea163a30d62570ccb99f43de78998c67e96860c95cd219547e4a9675`.
  `current_diagnostics.rs handed back`; no local reversal remains.
- Rust stable 1.99.1; verified C Tcl 9.0.4/8.6.18 and pinned Jim 0.84.
  `/tmp/bigip-env.sh` exports the requested CARGO/oracle variables and sources
  agent-build-env.sh. Original `/workspace/tcl-lsp` tracked files are clean;
  its ignored oracle trees remain via the active `tmp` symlink. Independent
  baseline `/workspace/bigip-baseline` at `fcb7927f02` uses its own target.
- 232 literal fixture inputs were materialised unchanged from HEAD. No source
  probe remains. Rustfmt passes on all 33 owned changed files and the applied
  shared-file hunks; git diff --check clean. No frontend source changed in
  this continuation; earlier TS/prettier receipts remain historical.
- Full repository/host gates are not green. Never open a PR; the PR to `rust`
  remains gated on external-review section F and integrated-tip validation.
