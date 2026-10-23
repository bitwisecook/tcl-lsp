# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load performance.

Last updated: 2026-10-23 19:41 UTC

## Resume

**Full batch and clean six-file proofs are running on frozen canonical
b6ea6dc5ebaa9589f6e1c1747875127fecdbb573. No perf production commit or full
corpus acceptance exists. Keep the active Rust sources fixed until the jobs
finish; later work commits are publication/rebase inputs, not this proof base.**

Both normal compiler libraries and the combined unit-test executable built
with zero Rust warnings. ALL16 report/actual-realm controls PASS BOTH modes
(new38.705s, old67.329s), source seal unchanged and executable deleted. This
proves the19936 hosted control repair on b6, including both when spellings.
The build guard resumed clean6 rustc40734 and exited; no process remains held.
Combined switched lexer586/586 PASS BOTH (4.427/2.879s), zero warnings, removed.
Clean6 lexer586/586 PASS BOTH (2.993/4.237s), zero warnings, removed; the old
variable is ignored in that no-switch source.

Full corpus preflight has passed and its examples are building. Selected
compiler229-case NEW run has started, including the exact original non-UTF8
native-array control; complete BOTH-mode comparison remains pending. Server /
SpecTcl builds have started and wait for the later quiet window. No full corpus,
compiler-filter, server or SpecTcl acceptance is claimed yet.

Active roots/logs (terminal handles lost at18:41; use PIDs/files):
- Controls34585: /tmp/perf-b6-controls.log; current-b6ea6dc5-controls/.
- Clean634692: /tmp/perf-b6-clean6.log; current-b6ea6dc5-clean6/.
- Compiler-filter waiter41020: /tmp/perf-watch-compiler-b6.log. Copies the
  exact SHA-verified control ELF, waits for controls cleanup, runs original8
  filters PLUS the original non-UTF8 native-array control, BOTH modes.
- Corpus waiter35120: /tmp/perf-b6-full-corpus.log. Requires every report
  control BOTH modes and unchanged source before starting complete all-old /
  mixed102 text Tcl+15 iRules+synthetics and ordered PackSet. All3 report pairs
  must Equal; Different/Unavailable fail. Strict runtime Eq stays separate.
- Combined lexer36947: /tmp/perf-b6-lexer.log, queued on the Cargo lock.
- Remaining-suite builder: /tmp/perf-b6-remaining-suites.log. Waits for the
  control executable, then builds server(default0) and SpecTcl(opt1).
- Quiet guard: /tmp/perf-b6-quiet-tests.log. Waits for all other workers and
  rustc processes to finish naturally, then releases server and SpecTcl in
  order with original assertions/timeouts/budgets. No readiness flag released.

Source/split/toggle reconstruction, ordered application, scoped fmt and exact
clean6 snapshot checks PASS. Current artifacts/driver receipts are in
current-b6ea6dc5/; active driver sources are preserved there for reproduction.
Independent worktrees/targets; opt1 retains debug and overflow assertions.
Pinned Tcl9.0.4/8.6.18/Jim0.84; publisher/tmp resolves to the same oracle tree.
Build/run/delete remain one command per driver, including private ELF cleanup.

Current b6 Clippy FAILED62 diagnostics, zero in the six owned files and no
compiler type errors. One NEW routed realm warning needs mechanical cleanup
AFTER frozen proof: spell query_memo's existing resolved constructor as
crate::command_binding::original_name_value::OriginalRealmQueryMemo::default().
Record that spelling-only delta and scoped fmt/typecheck before readiness.
Two unrelated doc-spacing lints were fixed separately0f1d32b8; other baseline
lints route to compiler. No workspace-green claim or suppression.

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
