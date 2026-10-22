# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-22T17:39Z

Maintained by the bigip-lsp agent. Keep this snapshot and Resume current, and
push at least every 30 minutes and after each finished item. Full unit table:
`../handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## In progress

Only server scoped lint and final handoff/sync remain. Server lint session
**50998** uses the restored F5 source and the same owned-root package graph;
logs `/tmp/bigip-clippy-server{.jsonl,-build.log}`. No temporary server diff
remains. Main HEAD is frozen until this last source check finishes.

## Outstanding (in priority order)

1. Finish server scoped lint: fix findings in owned F5 modules; route generic
   provider/current-diagnostics findings to the returned owner. No allowances.
2. Update final NOTES/tracking, push after fetch/rebase, sync the main checkout
   without losing the tested source snapshot, then stop. Never open a PR.
3. ILX source navigation and the 16 e2e cases remain blocked on the contracts
   below. BIG-IP owns consumer wiring/re-run when those owners deliver.

## Blocked (on whom, and what is needed)

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

- Validation checkout `/workspace/bigip-lsp` remains at `aeecf397` plus owned
  production lint fixes `f26f6a06`, 50 assertion spellings `e4a01bbe` and 16
  more in `9d6804c9`. Coordination-only worktree
  `/workspace/bigip-coordination` fetches/rebases/pushes newer tracking and
  upstream snapshots; these source-pinned runs do not validate later
  coordinator compiler/provider changes.
- Server e2e **68675** finished: XC 5/5, W109 1/1, F5 unit 1/1 PASS;
  ILX 0/16 first-diagnostic timeouts; plain rewrite fails with F5. Executables
  deleted. Comparison/generators **24095** finished: **3/3 editor tests PASS**,
  rewrite fails at the same assertion without F5. Both executables deleted;
  both original server files restored and their `git diff` is empty.
- Restoration hashes: current_diagnostics.rs
  `7fcf6f6dc3b91a8224b6cec6dee8afd21700c6843c6690cd06cfc1217a92986a`;
  lib.rs `830fd78baa30f46c2baf9944c6cce51183ecd591064e085027036e2467f663f5`.
- Native scoped lint **26335** finished after restoring all literal fixture
  inputs: zero owned F5/editor findings, 53 off-scope xtask and 176 generic
  registry findings. Earlier **25657**, **23728**, **46036** finished; core
  normal-library has 54 generic errors, zero owned. WASM **30313** PASS.
  Receipts `clippy-owned-scoped-20261022.json` and blocker indexes above.
- Only active check: server Clippy **50998**, full owned-root feature graph
  plus tcl-lsp-server, `--lib --bins --tests --no-deps -- -D warnings`.
  **Keep main source/HEAD frozen until it finishes.** Afterwards update all
  final states and push via the coordination worktree, then sync main only
  after verifying every local owned copy matches the pushed content.
- `/tmp/bigip-env.sh` exports the required CARGO and oracle variables and
  sources agent-build-env.sh. C Tcl 9.0.4/8.6.18 and pinned Jim 0.84 are built
  and verified; Rust 1.99.1 stable. `/tmp/bigip-test.py` builds/runs/deletes in
  one command. Baseline `/workspace/bigip-baseline` at `fcb7927f02` has its own
  target; original `/workspace/tcl-lsp` tracked files are clean and its ignored
  oracle trees are reached via the active `tmp` symlink.
- 232 literal fixture inputs were materialised from HEAD, with no test data
  changes. No temporary source probe remains. `current_diagnostics.rs handed back`;
  the authorised temporary reversal is complete and restored.
- Required full repository/host gates remain pending under the authorised
  development-push exception. The PR to `rust` remains gated on external
  review section F, including integrated-tip validation. Never open a PR.
