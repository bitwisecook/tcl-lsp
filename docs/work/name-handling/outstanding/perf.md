# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load performance.

Last updated: 2026-10-23 22:24 UTC

## Resume

**No perf production commit or full acceptance. Full/clean6 roots remain frozen
b6ea6dc5ebaa9589f6e1c1747875127fecdbb573. Do not commit realm/interprocedural,
switches or temporary zz_*probe*.rs. Existing failures must remain visible.**

Completed full-batch proof at b6:
- Original16 report/actual-realm controls PASS BOTH, source unchanged,0warnings.
- Switched and clean6 lexer586/586 PASS BOTH,0warnings, removed. Clean6 ignores
  old variable; it has six production files only, no realm/interprocedural.
- Mixed Tcl102 COMPLETE:101Equal/1 native35Unavailable. Old main process killed
  after77 reports; exact-ELF tail25/25 Equal restores102 original input coverage
  with the same one refusal. Original interrupted suite remains failed.
- Both15 iRules and all7 synthetic sizes have all3 authenticated Equal pairs.
- Full ordered PackSet Debug all3 Equal/deterministic,484936lines,8packs,
  0notices/errors. Main Rust diff unchanged, all probes/executables removed.
- Full selected229:228 actual outcomes identical142pass86fail BOTH; same1000-depth
  scan missing/resource-killed both. No complete229-case proof.

Native35 report defect is diagnosed, not normalized away:
Exact original-unit cfg(test) trace fails ReportBodyInventory::push for an
empty ALLOCATED OriginalDeclaration span. Existing empty-span admission accepts
only INSTALLED purpose. Public retained-request rebuild independently shows the
same allocated shape. Source-entry currentness still true for failing report.
Small genuine input: recursive fib + unused empty {} + fib20 refuses; fib0,
no call, return-only body pass. Declaration order does not fix it. Original
full fixture and all102+15 inputs remain byte-exact.
Evidence: current-b6ea6dc5-native35-unit-trace/, -capture-empty-order/,
-capture-owner/, -capture-ir/, -no-comment-repro/; strict reproduction-test
patch perf-report-empty-recursive-call-repro.patch is a draft, not a fix.

Candidate report-only repair is ONLY /workspace/tcl-lsp-diagnostic (separate
b6 full batch, source seal/diff recorded). It adds a distinct empty ALLOCATED
admission using complete current original-declaration inventory, exact actual
allocation/site/root/source/namespace/span/mapping and proper role/purpose.
Existing Installed helper, every old assertion, global/runtime/cache Eq and
ordinary AnalysisResult production remain unchanged. Three positive/negative
controls added. Candidate is unproven: initial unit build failed2 E0277 test
assert_eq Debug requirements, corrected to same plain equality assertion; retry
was resource-killed with0Rust warnings. Preserve both failed builds. No repair
production commit yet. Scope/fmt candidate files are recorded under
current-b6ea6dc5-empty-allocated-fix/. First/retry controls are
-empty-allocated-controls/ and -empty-allocated-controls-retry/.

Active memory ordering (no runtime paused):
1. Original clean6 both compiler modes are COMPLETE/FAILED, both exit-9.
   NEW208/228 (126pass82fail),20missing; OLD227/228 (141pass86fail),1missing.
   Original strict comparison retains15 positive-OLD/missing-NEW resource P2Fs.
2. Exact-ELF isolation recovery is FINISHED. Original227 NONDEEP cases COMPLETE
   BOTH, exact141pass86fail, zero P2F, peak351180/352356KiB. Unchanged1000-depth
   case ALONE was OOM-killed BOTH, peak16423836/16368268KiB. Zero completed
   assertions for that case; full inventory gate remains incomplete. Private
   executable removed. current-b6ea6dc5-clean6-isolated-recovery/final.json.
3. Server build has finished0Rust warnings and waits for quiet resources:
   /tmp/perf-b6-server-after-memory.log, current-b6ea6dc5/
   crate-tests-default-after-memory. Original2 tests/BOTH,180s unchanged.
4. Candidate19 report/realm controls QUIET retry BUILT0warnings; runtime active:
   /tmp/perf-b6-retry-controls-after-isolation.py/.log; evidence
   current-b6ea6dc5-empty-allocated-controls-quiet/. No runtime verdict yet.
5. /tmp/perf-b6-capture-allocated-acceptance.py waits for successful controls,
   then builds/runs/deletes the real triple probe for exact original Native35
   AND the compact reproduction, forced-old and mixed. Failed initial mini
   preparation remains recorded separately. No source mutation during controls.
6. Both SpecTcl executables built0warnings, queued for server thenSpec quiet
   window. Original28 eval_loader +3 golden_packs each mode; budgets unchanged.
7. Waiting-only guard was restarted to include the ENTIRE report-control helper
   and acceptance-probe lifetime before releasing server. Original guard56929
   replaced; receipt current-b6ea6dc5/quiet-guard-control-ordering.json.
   Current guard /tmp/perf-b6-quiet-tests-all-workers.py, session89176;
   /tmp/perf-b6-quiet-tests-all-workers-retry.log. Both flags remain absent.

memory.events: oom9/oom_kill8. No Rust compiler is deliberately held. Normal
and diagnostic targets never copied/shared. Tcl9.0.4/8.6.18/Jim0.84 remain
pinned via tmp symlinks. Preserve all failed/resource outcomes.

The corpus runner now supports --isolate-inputs: each original file gets a
process containing ALL three analyses/comparisons, with complete child receipts.
Every original input/hash, assertion and Equal-only requirement is retained;
this prevents cross-input memory accumulation. Python syntax/diff checks pass;
the new mode has not yet run. The latest fetched work0a8b8da6 contains the same
six owned files and realm; interprocedural has a separate helper refactor.
The report repair applies cleanly to latest work, but is NOT yet published.

Next: finish candidate19 controls and the original Native35 triple. If candidate succeeds,
check full original native35 old/new/old and all strict negatives, scoped fmt /
Clippy(no new diagnostics), land necessary report repair separately on latest
work. Then rerun genuine full batch corpus on repaired source: Equal only;
Unavailable/Different fail. No report substitution, source/owner guard waiver,
field dropping, assertion/budget weakening or global equality change. API repair
is not a six-file perf commit. Complete requested crate checks and all inputs.

Only after genuine complete proof, land EXACTLY six owned no-switch files as
ONE perf code commit, rebased latest. Refresh compiler-owned2-file patch and
write exact requested proven-at-commit readiness only then. Six clean files
were byte-identical on29c2f89e off-tree refresh; public patches pin own base.
Clippy b6 failed62, six owned0; the new Default lint was SWITCH-ONLY and clean
patch already qualifies constructor. Two unrelated doc-spacing lints fixed
0f1d32b8; no workspace-green claim/suppression.

Residual image/segment/LSP inventory drafts remain Blocked/routed, uncompiled,
semantically unproven. Additional perf-body-empty-regions.patch avoids cloning
AdviceGraph for existing checked empty coverage (zero old loop iterations),
parser/fmt only, no production edit or proof; routes compiler/compiler-names.
It does NOT fix the1000-depth memory failure. See residual-routing.md.

Current b6 wall timings are recorded in evidence/timings.md (contended opt1,
report capture included, comparisons excluded): calls80 old26815/new2721/
old15968ms; upvar30 old73450/new15071/old46401ms. Ordinary load_embedded
old14514/new2964/old16003ms, complete ordered PackSet equal. Origin same
load API4665ms, separate observation. No release claim/cross-API timing ratio.
Final NOTES with all final timings/limits remains required before stopping.

Publish own evidence, NOTES and this file from /workspace/tcl-lsp-publish at
most30min apart, date-u Last updated and Resume current. Fetch origin work and
rebase before every push, retry rejection; no force/PR. Footer:
Co-Authored-By: Codex <noreply@openai.com>.

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

1. Finish current complete16 report/realm controls and inspect current hosted
   refusal trace. Resolve necessary carrier defects without weakening any
   guard/assertion; publish authorised fixes separately. Then run the complete
   all-old/mixed102+15+synthetics and ordered PackSet on latest canonical work.
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
