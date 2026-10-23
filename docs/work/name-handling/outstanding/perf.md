# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load performance.

Last updated: 2026-10-23 14:44 UTC

## Resume

**Necessary compiler repairs are pushed; compiler proofs are RUNNING. No perf
production commit or full-corpus acceptance yet.** Latest publication uses
/workspace/tcl-lsp-publish. The canonical typing repairs are6c1c9d2c and
6e729fe9; the user expressly authorised necessary shared-file fixes and asked
that they reach work immediately. Both preserve all admission predicates and
assertions. They are separate from the future single six-file perf commit.

The combined proof tree /workspace/tcl-lsp-perf-latest remains frozen at
 a8e7bc7a0d368bbaab5bd1676a0383ad2b837130 plus the exact published6c test-only
byte-slice correction and the FULL switched nine-file batch. Both split
patches were additionally refreshed against94c7f61e, unchanged byte for byte;
ordered apply/snapshot/eight-file scoped fmt checks pass. They remain unproven.
Before final corpus/landing, refresh/rebase the proof to latest canonical work.
Do not commit realm.rs, interprocedural.rs or either temporary switch.

Active commands (build/run/delete executables in one command):

- /tmp/perf-run-a8e7-test-slice.log, session70544, Python21559: current16
  report+actual-diagnostic-realm controls both modes, then Native/hosted smoke,
  synthetics and complete ordered PackSet. Build still running; no control
  verdict yet. Receipts: evidence/current-a8e7bc7a-test-slice/.
- /tmp/perf-watch-compiler-trace-first.log, session90719, Python26478: waits for
  that exact control binary, SHA-verifies a private copy, runs the existing report trace FIRST, then every original
  compiler filter both modes, and deletes it. The earlier waiting watcher
  was stopped before any binary/test existed; watcher-ordering.json records
  the unchanged inventory/assertions and new diagnostic-first ordering.
- /tmp/perf-owned-typing-repair-compiler.log, session68715, Python25478:
  clean six-file no-switch compiler-filter build at canonical6e729fe9 in
  /workspace/tcl-lsp-owned. Receipts: evidence/current-6e729fe9-clean6/.
  It now reaches3 upstream VM typing failures; Cargo waits for its already
  running compiler job to finish, preserving completed dependency codegen.
  The owner repair0747a2a5 is already on work. Automatic retry session28812,
  /tmp/perf-clean6-after-vm-repair.log, validates that errors are exactly the
  three known VM paths, preserves all failed receipts, fetches/checks out
  current work with the identical six-file diff, then rebuilds/runs/cleans the
  original filters. Unexpected errors or a changed perf diff stop that retry.
  No current source is changed while a builder still uses it.
- /tmp/perf-spectcl-a8e7.log, session2241: opt1 eval_loader/golden_packs
  build COMPLETE with zero warnings, debug/overflow assertions on, separate
  target-spectcl-opt1. Runtime is RUNNING on reserved CPU cores2,3; all compiler roots and the
  pending canonical retry use cores0,1 (quota4, allowed5). The same masks,
  original2 test threads, assertions and budgets apply to both modes.
  /tmp/perf-partition-spec-cpus.log, session39125, records every prior mask
  and restores the exact roots after all4 SpecTcl runs and executable cleanup.
  cpu-partition.json is the receipt. New eval_loader has21/28 passing so far,
  no final suite comparison yet.

The quiet server window is COMPLETE. Both original180s cases/modes ran with
compiler builders/watcher stopped; all8 exact owned processes were resumed
in finally and the server executable deleted. Startup fails its FIRST retained
read assertion0 versus1 BOTH: new10.412s, old21.355s; factory reindex remains
UNREACHED. workspace_symbol times out180 BOTH: measured wall181.069/180.329s.
Zero pass-to-fail, both gates FAILED. Raw timeout states are not assertion
outcomes. evidence/current-a8e7bc7a/crate-tests-default/ contains full outputs,
source/build receipts and server-quiet-window.json pause/resume/cleanup proof.

A separate SHA-identical private server copy ran the existing source-phase
trace for the unchanged workspace fixture, then was deleted. The diagnostic
run times out180 after analysis completes at171.372s: walk79.839s and
 diagnostics91.531s (nested/inclusive phase totals, not additive with children).
This trace ran contended after builders resumed; it is not a quiet latency
comparison or proof that indexing/query alone caused the original timeouts.
Raw trace, source/binary hashes and observations: current-a8e7bc7a/
workspace-symbol-source-phase-trace/. The original quiet comparison is retained.

The current public-API hosted smokes still fail all3 comparison pairs with
Unavailable(CurrentOwnerUnavailable), for both cookbook_http_collect.irul and
exact37-byte genuine event source. Strict runtime Eq is separately false.
Fresh normal-library hashes/fresh:false receipts rule out stale artifact reuse.
The next cfg(test) trace must locate the successor refusal; do not attribute
it to the obsolete0322 guard. The a8 upstream repair changed procedure
visitation/hosted event issuance and authentic alias role/purpose/extent
controls. The current expected16 controls must all pass before full corpus.

Use tools/run_authenticated_corpus.py ROOT --controls CURRENT_CONTROLS
--opt-level1 --target-dir ROOT/target-spectcl-opt1 --evidence NEW_DIRECTORY.
Preflight requires exact HEAD/Rust diff, successful complete report AND actual
realm controls, zero build warnings and all117 fixture hashes. Every old1/old2,
old1/new AND new/old2 report comparison must Equal; Different and Unavailable
fail. Strict runtime/cache/global equality remains unchanged and separately
recorded. No normalization, omitted field/subject or weakened assertion/budget.

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
  required compiler retry is still building; no green result claimed.
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
