# Outstanding work: bigip-lsp agent

Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-23T12:22Z

Maintained by the bigip-lsp agent. Keep this snapshot and Resume current, and
push at least every 30 minutes and after each finished item. Full unit table:
`../handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.

## Active state

Resumed on 2026-10-23 at the user's request, from published `cbfb4771`.
The compiler first-operand source API is delivered and typed (coordinator
receipt at `5a4182df`); the former missing-API blocker is cleared.
The BIG-IP adapter now retains that association, joins only exact original
invocation cards, preserves all five source obligations and revalidates the
same analysis owner. Runtime target remains unavailable. Focused ILX unit validation: **23 PASS / 1 FAIL**, including both new hosted
association/obligation/barrier tests PASS. The unchanged foreign-frame test
returns no method card after an earlier opaque constructor; assertions remain.
Fresh strict irules library+tests Clippy **PASS, zero warnings** after aligning
its trait recursion limit with Compiler/server256. Core/server/e2e remain unrun on this continuation; input restoration and
the user-approved explicit-file behavior required a new coherent test snapshot.

The user decision `user-decisions-20261022T2231Z.json` stops navigation after
opaque calls. Seven e2e selectors are corrected narrowly for that later-site
expectation, preserving first-site/JS positives and all existing refusals.
Server reference routes project the separate readonly source candidate APIs;
Core identity references and edit authority remain separate.

`current_diagnostics.rs handed back` remains in effect. The prior XC, F5 pull,
editor comparisons and actual without-F5 rewrite comparison are complete;
prior receipts below describe their original source, not the new integrated tip.

## Outstanding

1. Finish focused ILX consumer/Core/compiler controls and original16 e2e;
   rerun XC/F5 pull on the new integrated source and strict owned lint.
2. Reproduce and repair the routed CLI/iRules failures on this checkout.
   Latest exact CI baseline join is stored as
   `coordinator-20261023/ci5206-log-triage/new-completed-0200Z/heavy-exact-baseline-join.json.gz`.
   It reports six F5 CLI regressions plus four baseline-absent failures; the
   coordinator's larger16 routing count includes additional iRules selectors.
   Distinguish current executed outcomes from the historical CI inventory.
3. Route any remaining compiler/generic diagnostic/VM cause with a minimal
   repro and contract; finish owned code/evidence and update final notes.

## Blocked (on whom, and what is needed)

- Generic minifier/compiler/performance owner: unchanged500-level aggressive
  minifier case did not return in634s; child externally stopped, classification
  incomplete, not a natural semantic failure. Original depth/assertion retained.
  Exact input, command and clean0/refusal2 contract: bigip-lsp/minify-repro-20261023.md.

- VM/compiler/performance host startup: current isolated LiveSession::new
  does not return within60s after materialising its embedded framework; actual
  CLI explain-flow simulation yields no report within120s. These finite probes
  do not reproduce CI5206 NativePreflightRequired or identify a hot function.
  Exact minimal repro, raw/hashes and unchanged guard contract:
  `bigip-lsp/live-startup-repro-20261023.md`. Original CLI runs are pending.

- Fresh server strict lint has32generic library errors, zero F5 findings;
  `bigip-lsp/clippy-server-blockers-20261023.json`. The library failure prevents
  e2e lint. This supersedes the prior server count for its tested scope.

- Fresh scoped lint: ten generic Core errors are indexed in
  `bigip-lsp/clippy-core-blockers-20261023.json`. Non-editor xtask
  `fp_sweep.rs:524` also needs the compiler trait-evaluation recursion depth
  aligned (the direct CompilationUnit Send+Sync assertion passes at256 and
  fails with E0275 at128). No allowance or non-editor edit. These current
  results supersede the older snapshots below only for their tested scope.

- Current compiler source owners: original event operands are absent from both
  retained event bodies despite current complete source vectors; procedure-body
  method cards and the promised procedure first-operand positive are absent.
  Exact minimal repro/contracts/raw receipts:
  `../handoff/reviews/merge-inventory-20261019/bigip-lsp/source-owner-blockers-20261023.md`.
  No consumer bypass, local handle tracking or assertion weakening.


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

- ILX (prior blocker CLEARED by the published first-operand API; consumer validation active): the compiler owner supplied an authenticated original source
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

- Active branch `work`, checkout `/workspace/bigip-lsp`, source checkpoint
  `c695061e` (ILX adapter), plus the next explicit-file change below.
- User reply in this chat: **"Expand explicit file reads to support lossy
  previews"**. Original archive-content e2e restored unchanged. `FileOp::Exact`
  now selects an exact original UTF-8 member path outside forensic discovery,
  returns original metadata plus lossy content, and retains raw bytes for the
  unchanged pre-publication mutation guard. No leaf/lossy path alias fallback;
  duplicate exact paths refuse. New IO/DSL/provider tests; source/dialect
  behavior is unchanged by this file-preview product decision.
- Two CLI input-only corrections select `.full-path` rather than leaf `.name`;
  all refusal/output/artifact assertions remain. Rationale in
  `bigip-lsp/cli-fixture-corrections-20261023.md`.
- Focused explicit-file tests PASS: IO2/2 and query6/6; receipt
  `bigip-lsp/explicit-file-20261023.json`. Prefix stripping now preserves real
  dot-names instead of aliasing `.hidden` to `hidden`. Source-name/query
  alias controls pass; all executables deleted. Sessions92295 initial run
  (new assertion JSON rendering mismatch only),62986 corrected8/8 final run.
  Combined native build65063 finished typing/linking0warnings (27m53s),
  and its sequential original tests are active. The compiler same-frame
  procedure positive already failed0-versus1 as predicted by the minimal repro;
  the same-source owner/barrier positive passed. Whole5/Core/CLI/server/e2e
  results remain pending. Strict F5/editor lint52191 finished101
  with only XC and non-editor xtask trait-recursion errors (four messages,
  including lib/test duplicates); owned XC correction awaits the frozen build.
  Raw/index: `bigip-lsp/clippy-f5-editor-initial-20261023.{json,jsonl.gz}`.
  Server lint28044 completed101 with32generic library errors and
  zero owned F5 findings; the e2e lint target is gated. Exact primary locations
  and enclosing functions: `bigip-lsp/clippy-server-blockers-20261023.json`. Standalone WASM lint39088 PASS0warnings
  on the new query API; `bigip-lsp/wasm-clippy-20261023.json`. `/workspace/scratch/bigip-20261023/`
  holds `env.sh`, build/run/delete helper and all plans/logs. This helper now
  runs any available selected artifact even after a sibling build failure,
  labels missing targets not-built, and always deletes every issued test exe.
- The combined native plan is `native-combined.json` (Compiler5 controls,
  Core ILX, CLI/provider/trace/simulation, server F5 pull, ILX16/XC/W109 e2e).
  Previous launch96931 was stopped before artifacts after the new user decision;
  has been restarted as65063 after focused8/8 and restoring16 additional
  literal fixtures unchanged from HEAD. The compiled snapshot is pinned as d392eb63 (exact source patch and all
  11 binary hashes retained). Follow-up source edits below are after that build;
  no whole-run source-seal claim. Runtime fixture files are unchanged.
- Earlier Core test build failed only on three omitted literal experiment
  fixtures; their exact HEAD bytes are restored and sparse patterns retained.
  Scoped broad Clippy completed with10generic Core errors, no owned finding;
  exact index/raw hashes: `bigip-lsp/clippy-core-blockers-20261023.json` and
  `clippy-broad-20261023.jsonl.gz`.
  selecting Core as a strict root blocked dependent targets. Next pass must
  select F5 roots+xtask separately, then server separately, using --no-deps.
  Current irules strict library/tests is green (256trait recursion, no allowance).
- Archive raw receipts with exact commands/hashes. ILX focused23/1 and source
  owner repros are already pushed; current_diagnostics.rs handed back remains.
  Continue all owned fixes/checks and keep compiler/VM causes under Blocked.
  Full repository/host gates and external-review F remain open. Never open a PR.

### Coordinator routing — 2026-10-22T21:19Z (owner header unchanged)

CI5205 heavy on2b934 confirms six exact origin/rust PASS→FAIL and four additional baseline-absent failures in F5 CLI suites. Full named routing: `../handoff/reviews/merge-inventory-20261019/coordinator-20261022/heavy-ci5205-bigip-routing-20261022T2119Z.json`. Original assertions remain. Two explain-flow simulations also depend on the VM-owned tcl9.0/F5 manifest mismatch. Owner session has not been duplicated;18:08 owner freshness remains stale and is flagged to user. These are new release blockers for the owning separate session, not a claim of resumed owner work.

### Coordinator routing — 2026-10-22T22:31:42.102809+00:00 (owner header unchanged)

The user chose **"Stop navigation after opaque calls"** for the new ILX policy question. Decision: `../handoff/reviews/resume-20261017-diagnosis/user-decisions-20261022T2231Z.json`. Compiler owns the genuine first-operand constructor/setter/read relation and withdrawal at opaque earlier operations/calls; no repeated conditional association or runtime/quiet/Normal grant. Seven unchanged ILX16 selectors currently expect later-site navigation across an opaque prefix; correct those expectations narrowly against this decision, retaining exact first-site/JS and all stale/foreign/reassignment/observer refusals. Exact static expectation roster: `../handoff/reviews/merge-inventory-20261019/coordinator-20261022/ilx-external16-association-review-20261022.json`. This is static test review, not a new executed verdict. The separate owner session remains responsible for consumer changes and original16 e2e; no duplicate has started.

### Coordinator routing — 2026-10-23T00:21Z

Compiler has frozen the distinct first-operand source API in [ilx-first-operand-v3-20261023T0020Z](../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/ilx-first-operand-v3-20261023T0020Z/consumer-api.json), proposal `fac70719`, with naming peer review. This is off-tree adapter preparation only: the proposal is not yet applied, typed or executed. Use exact authenticated invocation/constructor/setter/read cards from the same analysis; preserve conditional obligations and keep runtime target unavailable. Do not create a consumer-local walker or stale/standalone fallback. `Some(cards)` proves bounded positives, not catalogue completeness. The user2231 decision still stops navigation after opaque calls; the corrected compiler proposal refuses uncovered resolver effects before selected conditional descent. Original16 tests remain required after coherent integration. Details and hashes: [routing receipt](../handoff/reviews/merge-inventory-20261019/coordinator-20261022/ilx-first-operand-routing-20261023T0021Z.json). This coordinator append does not refresh the separate owner's header or claim its work.

Coordinator follow-up 2026-10-23T00:31Z: the compiler API is now applied in active checkpoint `3b1a750be337`, with its six-package all-target typing run in progress. The latest published `work` snapshot `e10387285` contains the frozen patch/API/review, not this source integration yet. Continue off-tree consumer preparation until the source is published and typed; no original16 or runtime acceptance is claimed. The owner timestamp remains unchanged.

Coordinator follow-up 2026-10-23T00:48:04.062877+00:00: compiler first-operand source API is published on `work` at `5a4182dfefc4` and passes six-package all-target typing with0errors/0warnings. The source is available for the existing role's consumer adapter. Compiler4 runtime controls and BIG-IP original16 remain unrun; preserve conditional obligations/user2231 withdrawal and runtime target unavailable. [CI5206](https://github.com/bitwisecook/tcl-lsp/actions/runs/39879737945) is on this exact published source. This is a coordinator route, not an owner timestamp update.

### Coordinator current-CI routing — 2026-10-23T02:06Z (owner header unchanged)

CI5206 product3b1 heavy job120175508112 completed: 354 exact outcomes,312PASS42FAIL. Matching actual origin/rust CI5203 heavy job119956195910 shows18PASS-to-FAIL, including16 CLI/iRules selectors and2 compiler/BPF selectors. Exact separate ledger: `handoff/reviews/merge-inventory-20261019/coordinator-20261023/ci5206-log-triage/new-completed-0200Z/heavy-exact-baseline-join.json`; byte-exact decoded gzip/log hashes alongside. External BIG-IP retains CLI/iRules ownership; compiler-codegen is triaging the2 BPF cases. Old-owner timestamp remains stale; no takeover or green claim. Compiler49 current-source guarded run active; source batch remains pending closure.

### Follow-up checks — current continuation

- Compiler controls4PASS/1FAIL (procedure body), Core ILX19/19PASS. The
  initial f5-query bin filters selected0tests: provider tests live in f5_cli
  library and are explicitly scheduled in cli-final.json. No provider-test
  pass is claimed from those0-test runs.
- Original CLI17:13PASS/4FAIL. Two explain goldens reproduce the missing
  compiler event-word inventory. The other two failures precede byte admission:
  scalar rename received a stream; inline description had no writable slot.
  Corrected only to first(full-path) and multiline config; donor bytes, both
  preview controls and every refusal/artifact assertion remain. Production
  controls pass; original17+library provider/source-event rerun pending.
- Native eight-root strict Clippy PASS0warnings after owned XC recursion256;
  exact command/source patch: bigip-lsp/f5-clippy-final-20261023.json.
- Original irule14 is still active in the500-level minifier stack regression.
  Remaining trace/simulation/server/e2e results pending. Separate f5-xc31 and
  exact three editor comparison tests build99672 active; no test artifact
  overlaps with the remaining original run. Do not rebuild f5-query until its
  original irule/explain-flow consumers finish.

- Completed original irule14:11PASS/2actual trace FAIL/1externally interrupted
  minifier. Harness says11/3 because interruption fails its original clean-exit
  assertion. Trace failures reproduce the missing event-word blocker. Original
  explain-flow5 now active; do not rebuild its f5-query binary yet. Closed raw
  Compiler/Core/CLI prefixes, all11 artifact hashes and source equivalence are
  archived via bigip-lsp/native-partial-20261023.json.
