# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load performance.

Last updated: 2026-10-23 20:27 UTC

## Resume

**Full batch and clean six-file proofs are running on frozen canonical
b6ea6dc5ebaa9589f6e1c1747875127fecdbb573. No perf production commit or full
corpus acceptance exists. Keep the active Rust sources fixed until the jobs
finish; later work commits are publication/rebase inputs, not this proof base.**

Both normal compiler libraries and combined unit executable built without Rust
warnings. ALL16 report/actual-realm controls PASS BOTH modes (38.705/67.329s),
source seal unchanged, executable removed. Switched lexer586/586 PASS BOTH
(4.427/2.879s); clean6 lexer586/586 PASS BOTH (2.993/4.237s), no warnings,
removed; old variable is ignored in the clean6 no-switch source.

**Current blockers: full corpus is not accepted and compiler run is incomplete.**
NEW selected compiler finished228/229 (142 pass/86 assertion failures). The
remaining1000-level deeply_nested_if_survives_interprocedural_scan grew past
15GiB RSS and the cgroup recorded its first OOM kill at20:06. Original driver
then asserted incomplete inventory before OLD. This is a failed resource gate,
not a passing test, and no original depth/stack/assertion was changed.
Exact SHA-verified ELF was preserved BEFORE cleanup. Recovery49887 finished
OLD with exit-9: the same228 completed outcomes match NEW (142 pass/86 fail),
same1000-level case missing. Both resource failures stay explicit; zero observed
P2F among completed tests is NOT a complete229-case comparison. Private ELF
was removed. Recovery final.json records complete=false.
Recovery evidence: current-b6ea6dc5-filter-resource-recovery/.

Full corpus examples built cleanly and run unchanged. One current all-old
failure: samples/wasm/t3-procs/35_native_dispatch_matrix.tcl returns all3
Unavailable(CurrentOwnerUnavailable), hence fails acceptance. A separate exact
three-report all-old diagnostic confirms capture_available=false immediately
for all3 analyses, not invalidation caused by later analyses (17 procedures).
Its11 fixed component variants, including the original first16 declarations,
all capture and self-compare Equal. Canonical segmenter prefixes locate the
first refusal exactly: first16 original declarations capture/self-compare Equal;
adding the next original command puts [fib 20] makes immediate capture refuse.
Both exact prefix inputs and probe sources are preserved. Route actual carrier to the
compiler owner; preserve every source/input/owner/body guard. Never normalize,
skip the fixture or substitute Debug for authenticated correspondence.
Evidence: current-b6ea6dc5-native35-refusal/, -capture-variants/, -capture-prefix/;
blocking-findings-20261023T2020.json pins fixture bytes and scoped observations.

Memory actions are explicit failed-build receipts, zero warnings/zero tests:
clean6 unit rustc40734 SIGINT at19:57, then server normal-library rustc44343
SIGINT at20:02 as the one runtime kept growing. Both builders were cancelled
to preserve runtime; no runtime was signalled/paused. Completed clean6 normal
library remains cached. Cold-cache advisory eviction changed no file content.
Clean6 retry47490 was held ONLY as an idle script until OLD recovery ended.
The guard finally RESUMED it at20:21; it is now rebuilding the unit executable
from the cached clean normal library, then runs original compiler BOTH modes.
No process remains deliberately held. retry-recovery-ordering.json is complete.
Server retry waits for47490 naturally, then rebuilds/runs/deletes original server
cases with180s limits. Quiet guard is perf-b6-quiet-tests-serialized.py, session42932:
it waits server actual build readiness before releasing server, then SpecTcl.
Neither quiet readiness flag is released. No other process is deliberately held.

The main all-old Tcl process was also OOM-killed (exit-9) after77/102 reports
during the same memory pressure. Preserve that failed one-process run. Exact
ELF old-tail recovery restores the original25 missing inputs, unchanged triples
and hashes; first7 already Equal, including lifecycle fixture78. Evidence is
current-b6ea6dc5-old-corpus-tail-recovery/. This is coverage recovery, not a
passing original suite. Main driver continued: ALL15 old-only iRules pass;
synthetics/mixed/PackSet continue. No original fixture is skipped or decoded.

Both SpecTcl test executables built without warnings and await the quiet window.
The proof runner now retains incomplete-inventory assertions until AFTER both
modes/remaining suites and artifact cleanup, so an OOM cannot silently skip OLD.
Missing outcomes stay missing (not invented assertion results), completeness
and P2F are recorded separately, and the final assertion still fails the gate.
No assertion, source input, test thread count or budget was weakened.

Active roots/logs (terminal handles lost at18:41; use PIDs/files):
- Controls34585 COMPLETE: /tmp/perf-b6-controls.log; current-b6ea6dc5-controls/.
- Original clean634692 finished with the resource-deferred unit build;
  successor47490 rebuilds, as above. Original log /tmp/perf-b6-clean6.log retained.
- Original compiler waiter41020 exited after NEW OOM/incomplete inventory;
  /tmp/perf-watch-compiler-b6.log. Recovery49887 COMPLETE/incomplete inventory:
  /tmp/perf-b6-filter-recovery.log.
  Original native0xff control PASSED NEW and remains in recovered OLD inventory.
- Corpus waiter35120: /tmp/perf-b6-full-corpus.log. Requires every report
  control BOTH modes and unchanged source before starting complete all-old /
  mixed102 text Tcl+15 iRules+synthetics and ordered PackSet. All3 report pairs
  must Equal; Different/Unavailable fail. Strict runtime Eq stays separate.
- Combined lexer36947 COMPLETE: /tmp/perf-b6-lexer.log.
- Remaining-suite builder: /tmp/perf-b6-remaining-suites.log. Waits for the
  control executable, then builds server(default0) and SpecTcl(opt1).
- Quiet guard serialized: /tmp/perf-b6-quiet-tests-serialized.log. Waits for
  all other workers and
  rustc processes to finish naturally, then releases server and SpecTcl in
  order with original assertions/timeouts/budgets. No readiness flag released.

Source/split/toggle reconstruction, ordered application, scoped fmt and exact
clean6 snapshot checks PASS. Current artifacts/driver receipts are in
current-b6ea6dc5/; active driver sources are preserved there for reproduction.
Independent worktrees/targets; opt1 retains debug and overflow assertions.
Pinned Tcl9.0.4/8.6.18/Jim0.84; publisher/tmp resolves to the same oracle tree.
Build/run/delete remain one command per driver, including private ELF cleanup.

Current b6 Clippy FAILED62 diagnostics, zero in the six owned files and no
compiler type errors. Corrected classification: the one new Default lint is
ONLY in the temporary-switch source. The existing no-switch batch path already
qualifies query_memo's constructor, so the deliverable needs no style repair.
The routed interprocedural length lint predates its two call substitutions.
Two unrelated doc-spacing lints were fixed separately0f1d32b8; other baseline
lints route to compiler. No workspace-green claim or suppression.

Off-tree refresh on29c2f89e: clean split ordered application and scoped fmt
PASS; all six files byte-identical to the active clean6 proof snapshot. The
interprocedural call sites moved into an owner-extracted helper; the refreshed
two hunks still perform the same substitutions. latest-patch-context-check.json
records exact hashes and the Clippy correction. No new semantic proof or change
to the frozen Rust sources is implied. Public patches still pin their own base.

Latest necessary hosted repair19936eb9 compares source when and retained
::when through the existing rooted-key helper, preserving all guards; its
original genuine control covers BOTH spellings. All16 current controls pass BOTH modes; full corpus remains pending.
The earlier a3 build failures and retired ccd partial43 new tests remain
preserved under their exact bases. No old run is represented as current proof.

Finish controls, complete corpus/PackSet and every requested crate suite BOTH
modes, zero pass-to-fail and zero Rust warnings. Preserve failed comparisons:
a8 SpecTcl had one actual eda_xilinx wall-budget P2F; golden packs were UNRUN.
a8 server startup failed first retained-read assertion BOTH (reindex unreached)
and workspace-symbol timed out180 BOTH. Quiet retries do not erase these runs.

Only after the full proof, land EXACTLY the six owned no-switch files as ONE
perf code commit rebased onto latest work. Never commit realm/interprocedural.
Refresh their two-file patch against latest work and record the exact requested
proven-at-commit readiness statement only when genuinely proven. Residual image /
segment and ordinary-LSP drafts remain Blocked/routed and semantically unproven;
no production files owned by those agents were applied. Final NOTES needs scoped
before/after timings and remaining limits before stopping.

Publish from /workspace/tcl-lsp-publish with fetch/rebase before every push,
retry rejection, never force, no PR. Push at most30min apart. Footer and pinned
reproduction environment are below. /workspace/perf-active-state.json is local
operational metadata; this Resume is the current published task snapshot.

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
