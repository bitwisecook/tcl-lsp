# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load performance.

Last updated: 2026-10-24 01:35 UTC

## Resume

**No six-file perf commit or complete batch acceptance yet. Continue the active
proofs below. Never commit realm/interprocedural, switches or zz_*probe*.rs.
Preserve every failed/incomplete result; no assertion or budget weakening.**

Active FROZEN proof source:83774d43464c7d177bcd505ef22fe898ffee9a25.
- /workspace/tcl-lsp-diagnostic: full8 batch + temporary switch (9 Rust files).
- /workspace/tcl-lsp-owned: exactly six owned files, no switch/no routed2.
- /workspace/tcl-lsp-publish: publisher, latest own push211f7020 at01:10UTC; current checkpoint follows.
- /workspace/tcl-lsp-work writes evidence but has old product source; never
  compile new probes there. /workspace/tcl-lsp-perf-latest is retired b6 evidence.
Keep both active sources unchanged until every active child has finished and
artifacts are removed. Targets are separate and never copied/shared.

Current strict API proof: ALL22 authenticated-report/actual-diagnostic-realm
controls PASS BOTH,128.571/117.964s,0Rust warnings, complete source seal unchanged,
unit executable removed. Ordinary/global/runtime/cache Eq remains strict and
separately recorded; only complete report Equal passes. Necessary empty
ALLOCATED body API repair ce6facfb is canonical (2 API files only). Necessary
upstream WordExpr source-span typing correction83774d43 is canonical (1 line).
Earlier E0609/E0599/resource-failed builds remain in evidence.

Current execution:
- Full driver85973: /tmp/perf-83774d43-api22-repaired-workflow.log;
  evidence current-83774d43-repaired-workflow-api22/. Optimised normal1 corpus
  build FINISHED0warnings in29m02s. Full corpus COMPLETE/PASS: all102 Tcl,
  15iRules and7synthetic sizes in BOTH forced-old and mixed modes; all3 complete
  report pairs Equal for every input, zero Different/Unavailable/child failures.
  Complete ordered PackSet484936lines,8packs/0notices/errors, deterministic
  old endpoints/all3 comparisons equal. Source seal unchanged, both probe
  executables and temporary sources removed. Compiler original8 filters plus
  native0xff+extra ReadFlow NONDEEP COMPLETE:232/232 actual outcomes BOTH,
  identical143pass/89fail, zero P2F;684.068/626.219s, peak408412/408764KiB.
  All failed assertions retained; no whole compiler green claim. Unchanged
  depth1000 now running ALONE new mode, then old; no current deep result yet. Raw corpus/final.json
  and current-83774d43-complete-report-summary.json pin complete coverage.

- Full switched lexer586/586 PASS BOTH,3.194/2.353s,0warnings/ELF removed;
  current full/client-lexer/. /tmp/perf-837-api22-switched-lexer.py is finished.
- Clean6 driver86059: /tmp/perf-83774d43-api22-clean6-workflow.log;
  evidence current-83774d43-clean6-workflow-api22/. Lexer586/586 BOTH PASS,
  2.716/2.197s,0warnings/removed. Old ENV is ignored. Compiler0 build FINISHED
  0warnings in1454.055s; SHA-checked private ELF retained, hash
  eeaadccf8ca99cab1e24877b186068e776630d7740e4539999786c2ea1210c23. It waits full driver cleanup before runtime.
- Fresh SpecTcl COMPLETE/PASS at SAME837 source: eval_loader28/28 BOTH
  319.619/257.425s; golden_packs3/3 BOTH314.814/308.789s. All4 suite outcomes
  pass,0warnings, exact source/hash unchanged, both executables deleted.
  full/spectcl-final-verification.json pins actual cleanup/unchanged source.
  Evidence full/client-spectcl/ and full/spectcl-run-*.json. Original28+3 BOTH,
  original assertions/threads/budgets. After its actual cleanup, full driver
  runs original compiler8 filters +native0xff control +extra ReadFlow test BOTH,
  with unchanged1000-depth case ALONE BOTH. Current full nondeep232 outcomes
  are complete/identical143pass89fail BOTH; deep remains active. Missing
  inventory remains FAILED.
- Clean6 then runs original8 filters +extra ReadFlow BOTH, unchanged1000-depth
  case ALONE BOTH. No runtime is paused, skipped or relabelled a pass.
- Final SAME-source server helper waits both compiler drivers' cleanup, then
  repeats original2 cases/BOTH at180s on all4CPUs after other resource work.
  This quiet repeat addresses the unresolved absolute timeout; first current
  comparison is retained below. Finally /tmp/perf-837-clippy-after-clients.py
  runs scoped fmt and compiler all-target Clippy, preserving/classifying every
  baseline/new diagnostic without suppressions or source formatting edits.

Prepared scripts also live in perf/tools/*.py.txt. Check actual progress.json
PIDs before restarting anything. Current five-helper source suffix is-api22;
the earlier directories without-api22 are FAILED/superseded evidence. Original
idle clean6 waiter was restarted BEFORE any build/test to use cargo-j1, so its
normal/unit rustc do not overlap. Resource guard temporarily let its compiler
use idle CPUs0,1 during full normal1 build; it is now restored to2,3. Fresh
SpecTcl uses0,1; corpus/builds2,3. Quota4CPU/16GiB, memory.events last oom9 /
oom_kill8, no new OOMs or ENOSPC. Environment/terminal reconnections preserved
all build PIDs. Always source /tmp/perf-env.sh and agent-build-env.sh; Tcl9.0.4,
8.6.18/Jim0.84 pinned via tmp links; debug/overflow assertions enabled.

ReadFlow conformance issue: unchanged ordinary compiler test
original_read_card_requires_its_actual_vector_origin_and_snapshot FAILS BOTH
in the first23-case current run at the foreign-card rejection; later mutated
snapshot negative is UNREACHED. All22 actual API controls pass in that run.
Its failure is retained and it reruns with compiler filters; no assertion is
changed or ignored. It is outside the report API inventory. Compiler owner
DELIVERED0f4b8e19, retaining issuing-analysis Arc and requiring actual pointer
correspondence before existing checks. This later repair is canonical but NOT
in frozen837; preserve it at final rebase and record later checks separately.
See readflow-owner-failure.md. The original strict22 API-only controller gate
remains ALL PASS; Different/Unavailable still fail complete corpus acceptance.

First current837 server proof COMPLETE:0Rust warnings/zeroP2F but FAILED gates.
Startup first role-count assertion0vs1 BOTH (6.026/5.468s), later factory reindex
UNREACHED. workspace_symbol timeout180 BOTH (180.103/180.215s). Source/hash
unchanged, artifacts removed. current-83774d43-repaired-workflow/client-server/.
Final quiet repeat cannot erase these original outcomes.

Frozen b6 interim, not current-source substitutes: switched/clean6 lexer586
BOTH; full selected228 actual outcomes142pass86fail identical BOTH, deep OOM
BOTH. Clean6 isolated227 actual outcomes141pass86fail BOTH; original1000-depth
OOM even ALONE BOTH at16423836/16368268KiB. Original incomplete15 resource P2Fs
are retained alongside exact-ELF positive-case recovery. Tcl mixed101Equal/
1Native35Unavailable before API repair, original old run killed after77 then
exact-ELF remaining25 Equal; no corrected full acceptance. Both15iRules and
7synthetics Equal; ordered PackSet484936lines deterministic/all3 equal.
SpecTcl b6 COMPLETE/PASS0warnings/zeroP2F: eval_loader28 BOTH263.054/301.019s,
goldens3 BOTH227.942/430.240s; source exact, both executables deleted. All
historical error/timeout/assertion/resource receipts remain untouched.

Origin/rust fcb7927f same-input audits COMPLETE: actual baseline probe read
absolute work-checkout fixture paths; all117 current text inputs match those
paths and recorded hashes, all8 .tclspec files match, and executing actual
extracted Rust generators proves all7 emitted synthetic Tcl inputs identical.
The19 absent origin-tree migration files were not skipped. Raw timing eq=true
is a placeholder, never equivalence proof. Origin analyse excludes report
capture; current report timings include it. Full provenance: origin-*-audit
artifacts and timings.md. Historical b6 contended opt1 calls80 old26815/new2721/
old15968ms; upvar30 old73450/new15071/old46401ms; same load_embedded API
old14514/new2964/old16003ms, complete PackSet equal; origin4665ms separate.

Next after genuine required proofs: assess all old baseline passes, zero P2F,
strict complete inventories and complete report/PackSet equality. Keep all
failures explicit; never fabricate passing Rust outcomes for missing tests.
Run final scoped fmt/Clippy (b6 baseline62 failures, six owned0; switch-only
Default lint fixed). Land ONLY six owned clean no-switch files as ONE perf
code commit, rebased latestwork. Refresh the compiler-owned2-file patch on
latestwork and record exact requested proven-at-commit readiness ONLY after
true proof. Never commit realm.rs/interprocedural.rs. Later canonical owner
changes must be retained, with source-scope validation recorded honestly.

Residual image/segment/LSP inventory drafts refresh to211f7020; empty-region
draft remains837. All are
Blocked/routed, uncompiled and semantically unproven. Image draft still scans
complete bytes per reuse; LSP draft still has per-lens outer captures/walks.
Neither establishes linear review4.1 scaling or fixes deep OOM. Owners must
prove complete results before acceptance. See residual-routing.md. Final
perf-NOTES.md with final current before/after timings remains required before
stopping. No PR/force; footer Co-Authored-By: Codex <noreply@openai.com>.

Publish own evidence/NOTES/outstanding at most30min apart, Last updated from
date-u. Own checkpoint in this update at2026-10-24 01:35 UTC; next push within30min. Fetch origin
work and rebase before every normal push, retry rejection, never force.

## Completed evidence and current failures

- Switched a8 lexer library:586/586 PASS BOTH, zero warnings, executable
  deleted. Current94 clean six-file no-switch lexer:586/586 PASS in both
  environment executions, zero warnings, deleted. The variable is ignored
  in the no-switch build. The canonical typing repairs do not touch lexer.
- Current94 clean6 metadata check:FAILED,6 upstream type errors/3 unused
  imports, zero tests. Full JSON/stderr/summary retained. The selected-filter
  build was interrupted before tests, not a full comparison. Separate6e729fe9
  restores the missing state import, mutable accumulators, command borrow,
  qualified compilation context and removes exactly3 unused imports. Its
  retry build completed without warnings, but runtime was retired on the
  predecessor; no complete current comparison claimed.
- All117 text fixtures match baseline/added-class hashes on94. Six tracked
  registry oracle captures excluded by that older sparse pattern were restored
  byte-exactly; failed sparse builds are retained, zero tests/warnings.
- Historical0322: report controls12 PASS/3 FAIL BOTH; lexer586 PASS BOTH.
  Partial compiler/server/SpecTcl workers were stopped after the functional
  a8 repair; exact superseded receipts/cleanup remain, no complete verdict.
- Historical e4:102 text Tcl100 equal/2 unequal, zero panics;15 iRules fail
  mixed AND all-old. Calls10/20/40/80, procs41 and upvar10/30 pass complete
  old/new/old comparisons and deterministic endpoints. Ordered PackSet Debug
  agrees in all3 directions,484936 lines. These are not latest-source gates.
- Historical selected compiler226:151 pass/same75 fail BOTH; zero P2F.
  SpecTcl opt1 eval_loader28/28 and golden_packs3/3 BOTH pass. Original
  default eval_loader retained ONE P2F (eda_xilinx wall budget). Quiet exact
  repeat passes BOTH with unchanged assertions/budgets, new1150.068s wall/
  1141.846s CPU, old1429.759s wall/1409.186s CPU. This does not erase the
  original full-suite result. Separate five-second budget-axis fails BOTH.
- origin/rust fcb7927f timings cover the same102 text+15 iRules, synthetics,
  index prefixes10/40/100/253, load_embedded and positive ordinary LSP shapes.
  The original native0xff fixture remains a separate passing native-codegen
  control; no original positive input was dropped/lossily decoded.
- Historical tclIndex253: old5108792ms/new189404ms, origin1596ms; complete
  equality/determinism on that8a base. e4 new-only253=255893ms is timing
  only. CPU calls80 old35450/new3760/old21660ms; upvar30 old71930/new18670/
  old258020ms. Full per-base values/limits are in evidence/timings.md.
- Positive ordinary LSP inputs10/20/40/80 retain every proc/index/lens and
  one real reference per lens. At80, origin/work index0.497/1557.801ms and
  lenses106.465/25542.691ms. Source chains250/500/1000/2000 pass. Still slow.

## Remaining, in order

1. COMPLETE at frozen837: all22 report/realm controls PASS BOTH; full
   all-old/mixed102+15+7 report corpus and complete ordered PackSet PASS.
   Necessary repairs are separately published. Preserve source-scope receipts.
2. Finish original compiler filters BOTH and no-switch clean6 checks; run
   SpecTcl eval_loader/golden_packs BOTH with unchanged budgets. Preserve all
   failed/resource outcomes; zero warnings and zero pass-to-fail required.
3. Land exactly six clean owned source files as ONE perf commit, no switch:
   lexer parse_cut.rs/source_map.rs; compiler original_word.rs,
   original_name_value.rs, formal_call_advice.rs, incomplete_body_metadata.rs.
4. ONLY after full proof, refresh perf-realm-interprocedural.patch against
   latest work and record the requested exact proven-at-commit readiness
   statement. Keep these two compiler-owned files out of the perf commit.
5. Residual image/segment drafts remain Blocked/routed to compiler-names,
   unapplied/uncompiled/unproven. Currentness-only LSP draft routes to LSP;
   it retains per-lens outer rebuilds and is not a linear-scaling fix. Route
   remaining review4.1 costs and current default0 analysis/role/timeout evidence.
6. Update perf-NOTES.md with final scoped before/after timings and stop.

## Reproduction and publication

Use source /tmp/perf-env.sh and the active scripts/dev/agent-build-env.sh:
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6
TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh. Pinned Tcl9.0.4/8.6.18/Jim0.84
are preserved through tmp symlinks. Never share targets across worktrees.
Temporary Rust probes are zz_*probe*.rs, uncommitted, removed by their runner.

/workspace/tcl-lsp-work writes evidence but has historical e4 source; do not
link new probes against its stale targets. /workspace/tcl-lsp-rust is the exact
origin baseline. Old /workspace/tcl-lsp-current is39b evidence only. Active
proof/clean6/publisher roots are above. Persistent status is
/workspace/perf-active-state.json. No automatic publisher is running.
Publish own evidence/tracking and authorised necessary fixes, fetching origin
work and rebasing before every normal push; retry rejection, never force.
Push at most30min apart. No PR. Footer:
Co-Authored-By: Codex <noreply@openai.com>.

### Coordinator routing — 2026-10-22T22:56:21.361198+00:00 (owner header unchanged)

The user authorized **authenticated full-report comparison** as the new perf equivalence acceptance contract. Decision: `../handoff/reviews/resume-20261017-diagnosis/user-decisions-20261022T2256Z.json`. Compiler owns the distinct API and authentic native/hosted carriers; global/runtime equality and cache/Salsa cutoff stay strict. Every field and typed subject requires complete unique current actual correspondence; Unavailable fails the gate. TEMP3 current-source attribution is in `../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/report-first-refusal-20261022T221949Z/`. Resume harness/full old+mixed corpus work when the reviewed API/carriers arrive. No perf patch, including realm/interprocedural, is accepted or forwarded by this decision alone. No duplicate perf role started.

### Coordinator dependency delivery — 2026-10-23T01:05:06.334470+00:00 (owner header unchanged)

Compiler has frozen the user2256 report-only API proposal v7, patch `2351e4cf48adf9d66ddfaa75661e01e4ffc8bc405b4c61e6f1bac4e3dc5cbc8b`, with bounded naming peer review `b52aea24`. [Exact API and 14-control handoff](../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/authenticated-report-v7-20261023T0101Z/) and [guarded execution preparation](../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/authenticated-report-focused14-v1-20261023T0106Z/) are preparation only. Canonical product does not yet include this API; no typing or control/corpus run is claimed. Existing perf session can prepare the harness off-tree from the frozen signatures; wait for root publication and successful controls before actual full all-old/mixed proof. Equal alone passes; Different and Unavailable fail the proof. Keep strict global/runtime equality and cache cutoffs separately recorded, complete source/input/registry/body/hosted authentication and count guards unchanged. Realm/interprocedural patch remains withheld until this owner records it proven. Root has not duplicated or taken over perf. Filename times are labels; see actual receipt timestamps.

Current procs41 timing follow-up: same opt1 source/input, two further full
old/new/old comparisons Equal:2831/5324/2268ms and2184/1379/2538ms. Retain
the original6311/11869/2645ms too; variable timings do not support a universal
speedup claim.0warnings/source exact/probe source+ELFs removed. Evidence
current-83774d43-procs-timing-repeat/. Original origin probes are removed; exact
sources/hashes preserved in origin-temporary-probe-cleanup/. No live job touched.
