# Outstanding work: bigip-lsp agent

## Resume — coordinator pause 2026-10-27T20:38:21.853504+00:00

USER PAUSED: no implementation, test execution, dispatch or timer restart until the user resumes. Read [the current handoff](../pause-handoff-20261027.md) first.

External role; no duplicate or takeover. Prior owner header is stale and user has been notified. Preserve its implementation report and route generic compiler/LSP blockers to owners on resume. This coordinator pause entry is not a new external owner validation.

The older owner timestamp and history below are preserved; this entry does not assert a fresh run, gate pass or owner activity.


Area: editors/, BIG-IP/F5 crates and LSP modules.

Last updated: 2026-10-23T13:17Z

## Active state

All authorised, unblocked owned implementation and checks are complete. The
remaining failures below require other owners; original positive/refusal
assertions and native admission guards are retained. No PR was opened.
Final tested product snapshot: `73c43bdb`; final push rebases the other owners'
subsequent commits. Those later changes and full shipping gates are not covered
by this snapshot's results. Full implementation table: `../handoff/reviews/merge-inventory-20261019/bigip-lsp-NOTES.md`.
Evidence paths below are relative to `../handoff/reviews/merge-inventory-20261019/`.

## Done

- XC condition owner uses shared inclusive `expression_span`; the regression
  fails without that fix. Latest library: 31/31 PASS; protocol: 5/5 PASS.
- BIG-IP/APL W109 protocol: 1/1 PASS; F5 pull-report unit: 1/1 PASS.
  **current_diagnostics.rs handed back**; no further production edit there.
- Compiler-owned ILX association adapter implemented: exact original invocation
  join, all five obligations, retained analysis owner revalidation, readonly
  source candidate projection through server references. Runtime target stays
  unavailable. Seven later-notify expectations follow the user's stop-after-
  opaque decision; every first-site/JS and foreign/stale refusal assertion stays.
  Core ILX: 19/19 PASS; focused iRules: 23 PASS / 1 compiler source-card FAIL.
- User decision: **Expand explicit file reads to support lossy previews**.
  `file(path)` now reads an exact original archive path beyond forensic discovery,
  returns original metadata and lossy content, and retains raw bytes for the
  unchanged mutation guard. No leaf/lossy path aliases; duplicate exact paths
  refuse. Prefix stripping preserves real dot-names. IO 2/2, DSL 6/6, provider 1/1
  and all three original CLI byte-fidelity/refusal regressions PASS.
- Narrow fixture corrections only: full-path/scalar first for rename, and a
  multiline description with a writable field slot. Donor bytes, both preview
  controls and every refusal/artifact assertion remain. Rationale and actual
  before/after evidence: `bigip-lsp/cli-fixture-corrections-20261023.md`.
- Strict Clippy for all eight native F5 crates and standalone WASM **PASS, zero warnings** on
  73c43bdb, Rust1.99.1, `-D warnings --no-deps`. No new lint allowance.
  Receipts: `bigip-lsp/native-latest-clippy-20261023.json` and
  `bigip-lsp/wasm-current-clippy-20261023.json`.
- Exact three gen_editor_configs comparisons: 3/3 PASS, no expected-file changes;
  receipt `bigip-lsp/xc-editor-final-20261023.json`. Generator/registry source
  is unchanged by the later compiler geometry repair. Owned rustfmt is clean;
  this continuation changes no TypeScript.
- The unchanged plain-Tcl actual rewrite test was physically rerun without
  the F5 change and fails at the same applicable-rewrite assertion. Both files
  restored byte for byte: `bigip-lsp/rewrite-without-f5-repro.md` (Oct22 snapshot).

All issued test executables and temporary Rust probe executables were built,
run and deleted within their wrapping commands. No temporary probe source was
committed. Functional receipts are not guarded shipping/audit-unit admission.

## Blocked (on whom, and what is needed)

1. **Compiler original event operands**: on 73c43bdb, including d1a0fe02,
   all 12 authentic ingress/config combinations still yield2 event bodies but
   zero event words, without native entry. Complete source vectors own both
   original event arguments. CLI explain 2, source-event unit 1 and trace 2 fail
   unchanged. Exact new-compiler-constructor repro, full raw output and contract:
   `bigip-lsp/source-owner-blockers-20261023.md` and
   `bigip-lsp/latest-event-ingress-matrix-20261023.txt.gz`.
   Issue source geometry under its actual owner; do not grant activation or
   fabricate a consumer receipt. Published repair does not clear this result.
2. **Compiler ILX source relations/catalogue**: current flat call has 1 genuine
   association/exact consumer join; `set reply [ILX::call $h m [HTTP::uri]]`
   has 0 despite genuine method vectors. Own procedure constructor/read has 0;
   a procedure prefix also removes the later handler's independent same-frame
   if positive. Same-frame positive works alone and after an outer constructor.
   Minimal current four-case/procedure probes and required chronology/frame
   contract are in the same source-owner blocker file. No local handle walker.
   Last original 16 protocol result: **4 PASS / 11 assertion FAIL / 1 diagnostic TIMEOUT**
   on d392eb63. Latest73c direct prerequisites still fail; no fresh16 pass claim.
3. **VM/compiler native host startup**: both original current simulation tests
   naturally report `NativePreflightRequired`; explain-flow 3/5, including two
   tshark early returns (no tshark coverage),243.26s. Exact session/CLI repro and
   actual C9/F5 grammar/provider contract: `bigip-lsp/live-startup-repro-20261023.md`.
   Supply the genuine provider; never disable admission or substitute a logical
   runtime to make the tests pass. This agent does not own tcl-irule-test/VM.
4. **Generic LSP diagnostics/rewrite**: one original ILX JS-command selector
   still exceeds its unchanged 30s publish barrier with a healthy scheduler
   probe. The older all-16-timeout observation is superseded by the 4/12 run.
   `bigip-lsp/native-combined-20261023.json` and
   `bigip-lsp/diagnostics-latency-repro.md`. The unrelated rewrite comparison
   above also remains with the generic owner; no F5 production workaround.
5. **Generic minifier/compiler/performance**: original depth500 minifier child
   did not return for 634s and was externally stopped; count incomplete, not a
   natural semantic failure/pass. Latest identical-source production probe is
   still incomplete after 180s. Preserve original depth and clean 0/refusal 2
   assertion. Input, hashes and contract: `bigip-lsp/minify-repro-20261023.md`.
6. **Shared strict/release gates**: last scoped generic Core 10 / server 32 and
   non-editor xtask fp_sweep trait-depth findings are indexed in the Oct23
   clippy blocker JSONs. Counts describe those earlier snapshots; later owner
   refactors need their own validation. Zero owned findings; full crate/gate
   green is not claimed. Repository/host gates and external-review section F
   remain open; no PR authorised.

## Resume

- Branch `work`, checkout `/workspace/bigip-lsp`; no running build/test jobs.
  Own implementation is committed and published; final notes updated.
- Definitive latest affected runtime results: `bigip-lsp/published-event-repair-20261023.json`.
  Byte provider 1/1; CLI 15/17 (all byte guards pass); event 0/1; trace 0/2;
  XC 31/31; simulations 3/5 with native-preflight refusal.
- Other completed requested protocol/editor/Core controls and exact snapshots:
  `bigip-lsp/native-combined-20261023.json`,
  `bigip-lsp/xc-editor-final-20261023.json`,
  `bigip-lsp/explicit-file-20261023.json`.
- Next dependent actions: Compiler repairs the three authentic source gaps;
  VM restores genuine native preflight. Then rerun the unchanged original 16,
  six CLI regressions and source-event unit under those delivered owners.
  Generic owners handle diagnostic latency, rewrite, minifier and shared lint.
  Keep every current positive/refusal assertion and all source/native guards.
- Source/helper/log scratch remains `/workspace/scratch/bigip-20261023/`;
  replay requires fresh build/run/delete. Evidence is committed under bigip-lsp/.
  This agent stops after the final push; no further unblocked owned work remains.

## Historical coordinator routes

Preserved decision/provenance records; the active state above supersedes their
old freshness/count statements.

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
