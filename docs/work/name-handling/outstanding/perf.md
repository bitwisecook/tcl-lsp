# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load performance.

Last updated: 2026-10-23 11:50 UTC

## Resume

**BLOCKED on current compiler report controls; regression checks RUNNING. No production commit.**
Latest frozen proof source is 0322d9cd1af50e28157b7c3693f53b77cc92adf7 in
/workspace/tcl-lsp-perf-latest, an independent shallow checkout. AGENTS.md is
byte-identical to the earlier proof guide. The FULL batch is applied in nine
experimental Rust files, including temporary switches; both clean patches are
refreshed against this exact commit. The residual images/segments drafts are
also refreshed, unapplied and unproven, routed to compiler-names.

The user2256 acceptance contract is authenticated complete-report comparison,
separate from strict runtime/cache Eq. tools/authenticated_perfprobe.rs.txt
requires Equal for old1/old2, old1/new AND new/old2. Different and Unavailable
fail; strict runtime equality is recorded separately without alteration.
Current authenticated-report controls are COMPLETE: 15 each, **12 PASS /3 FAIL
in BOTH modes**, zero warnings, identical inventory, zero pass-to-fail.
report-controls-summary.json records the two unchanged alias-role assertions
and hosted CurrentOwnerUnavailable failure. E003 positive/foreign controls
pass. The separate direct-artifact smoke returns complete Equal triples for
E003 and aliases in forced-old AND mixed modes; strict runtime Eq remains
false and was not altered. Direct hosted all-old AND mixed each fail all3 pairs with
Unavailable(CurrentOwnerUnavailable); executable removed.

Full corpus acceptance remains BLOCKED until the original report controls and
the separate diagnostic-output realm control pass in both modes. Different
and Unavailable fail. No field, assertion, subject or budget is weakened;
neither perf patch is proven/ready. The new preflight explicitly checks both
report and actual-realm controls, exact source/batch and all117 fixture bytes.
Evidence is under perf/evidence/current-0322d9cd/.

Runner PID/status: inspect /tmp/perf-run-retry.log and evidence progress.json.
Build/run/delete happen in the same command. Targets are isolated under /workspace/tcl-lsp-perf-latest. The first target
started empty; retries reuse matching dependencies from this same checkout.
Cargo receipts record freshness. No cross-worktree build artifacts are reused. Source-before/after and artifact hashes will be retained. Use
source /tmp/perf-env.sh and scripts/dev/agent-build-env.sh; preserved oracles
are available through the proof checkout's tmp symlink.

The initial f149597f current build stopped at the upstream propagation::run
missing_docs error (0 warnings, no tests), fixed upstream in f89bd6f6. Its
complete failed build/source receipts remain in evidence/current-f149597f/.
The retry uses that repair without any perf edit to optimiser source. The
main clean patches are refreshed against0322; no semantic proof is implied.

Prior confirmed comparison failures at 39b39296 and all historical corpus,
crate, timing and timeout outcomes below remain evidence of those exact
sources. Do not infer a latest-source verdict from them. Compiler changes on
new work include the distinct report API and owner repairs; their actual
latest controls determine whether the full proof can resume. Never normalize
owners, omit fields, change global/runtime/cache Eq or weaken assertions.
Do not commit realm.rs or interprocedural.rs or mark their patch proven
without the required complete latest-source proof.

## Completed evidence

Current0322 lexer library: **586/586 PASS in BOTH modes**, identical complete
inventories, no warnings, test executable removed. Exact default-profile
build/outputs: evidence/current-0322d9cd/crate-tests-default/. No other current
suite verdict yet. All 117 text corpus bytes match the recorded baseline and
added-class roster; all 72 tracked SpecTcl source/test inputs are present.


- Broad frozen e4a01bbe corpus: 102 text Tcl, 100 equal / 2 unequal (E003 and
  aliases), zero panics; every successful row deterministic. Both failing
  fixtures also fail isolated all-old. All 15 iRules fail mixed AND forced-old.
- Calls 10/20/40/80, procs 41, upvar 10/30: complete old/new/old equality and
  deterministic endpoints. Additional process-CPU calls/upvar controls pass.
- Full ordered PackSet Debug equal, 484936 lines, deterministic old endpoints.
- Lexer lib: 583 pass / same 3 fail; selected compiler filters all 226:
  151 pass / same 75 fail. Complete identical inventories, zero pass-to-fail.
- Latest optimized SpecTcl: eval_loader 28/28 PASS each mode; golden_packs
  3/3 PASS each mode. Default original eval_loader: new 26/2, old 27/1, ONE
  observed pass-to-fail on every_shipped_pack's eda_xilinx wall budget.
- Controlled repeat of ONLY that exact default case on quiet host: BOTH PASS,
  same assertions/budgets; new 1150.068 s wall / 1141.846 s CPU, old
  1429.759 s wall / 1409.186 s CPU. Preserve the original full-suite result;
  neither this repeat nor optimized green runs erase it. The separate
  five-second budget-axis failure matches in both default modes.
- Both named server cases compared with 180-second limits: startup fails the
  same FIRST retained-read-count assertion 0 vs 1 (later reindex unreached);
  workspace_symbol times out180 in BOTH. Zero observed pass-to-fail but failed
  gates. Raw timeout states are not fabricated Rust assertion outcomes.
- Historical base8a tclIndex 10/40/100/253: all equal/deterministic. 253 old2
  5108792 ms / new 189404 ms. Only old100 timing was debugger sampled.
  Latest e4 new-only253 255893 ms is timing only, not equality proof.
- origin/rust fcb7927f timings cover same 102 text +15 iRules and synthetics,
  all four index prefixes, load_embedded and positive ordinary LSP shapes.
  Original roster had 101 UTF-8 + one intentional 0xff source. Added class
  fixture gives 102 text; native byte fixture retains actual native-codegen
  control (passes both modes), never lossily decoded or silently dropped.
- Positive ordinary editing-path LSP controls preserve every proc/index/lens
  plus one real reference per lens, sizes10/20/40/80. At80 lenses work25542.691
  versus origin106.465 ms; source-chain250/500/1000/2000 passes. Still slow.

## Remaining, in order

1. Compiler owner resolves the current two alias-role assertion failures and
   hosted CurrentOwnerUnavailable. Then complete actual-realm controls and
   full all-old/mixed 102+15+synthetic report equality/PackSet proof. Never
   treat the native two-fixture smoke as complete corpus acceptance.
2. Finish current-source crate/no-switch gates with zero pass-to-fail and zero
   new warnings. Preserve known failure/resource limits. Older six-file-only
   Clippy snapshot: zero owned /783 foreign; not current workspace green.
3. Land exactly six clean perf-owned files as ONE commit, no switch:
   lexer parse_cut.rs/source_map.rs; compiler original_word.rs,
   original_name_value.rs, formal_call_advice.rs, incomplete_body_metadata.rs.
4. ONLY after genuine full proof, record the requested exact proven-at-commit
   readiness statement for perf-realm-interprocedural.patch. Never commit
   those two compiler-owned files; coordinator routes the patch.
5. Residual images/segments drafts remain BLOCKED/ROUTED to compiler-names,
   unapplied/unproven. Ordinary inventory/index work routes to LSP. Sampled
   source-transition body walk routes compiler/names. Details/controls in
   residual-routing.md. These are not claims of improved behavior.

## Files, trees and reproduction

Main patches are refreshed against0322d9cd; byte-exact clean application only,
no semantic proof implied. /tmp/perf-0322-clean holds clean snapshots.
refresh_patches.py ROOT --base REF --snapshot DEST refreshes clean6/two split;
use --output-dir to avoid overwriting main artifacts during historical checks.
Older e4 proof patches/inputs are archived in evidence/latest-e4a01bbe/proof-patches/.
Never reapply a batch over an already modified tree.

/workspace/tcl-lsp-work is the frozen e4 source and evidence publisher; its
nine experimental Rust changes are uncommitted. /workspace/tcl-lsp-current
contains the fresh39b proof. /workspace/tcl-lsp-rust is exact fcb7927f with own
target. Never link a new probe against an older/mutated target and call it
that old base. Old target/ compiler rlib was invalidated earlier; do not use it.
Current target is isolated and local cache invalidation is recorded. Temporary
probe sources/executables were removed; recorded .rs.txt reproducer sources stay.

Active publication uses /workspace/tcl-lsp-publish, docs-only cherry-picks,
fetch origin work && rebase origin/work before every push, retry/no force.
The checkpoint controller is STOPPED (stop flag exists and log confirms it).
All proof workers completed and removed their executables; no background
build/test/publisher remains. Final evidence publication uses a manual
docs-only cherry-pick; the final response supplies its published commit.
No PR. Footer: Co-Authored-By: Codex <noreply@openai.com>.

Use source /tmp/perf-env.sh then active scripts/dev/agent-build-env.sh.
CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6
TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh. Tcl9.0.4/8.6.18 and pinned Jim
are built; tmp symlinks retain oracles. Keep worktrees' targets separate.
All requested test executables were built/run/deleted in their own command.

### Coordinator routing — 2026-10-22T22:56:21.361198+00:00 (owner header unchanged)

The user authorized **authenticated full-report comparison** as the new perf equivalence acceptance contract. Decision: `../handoff/reviews/resume-20261017-diagnosis/user-decisions-20261022T2256Z.json`. Compiler owns the distinct API and authentic native/hosted carriers; global/runtime equality and cache/Salsa cutoff stay strict. Every field and typed subject requires complete unique current actual correspondence; Unavailable fails the gate. TEMP3 current-source attribution is in `../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/report-first-refusal-20261022T221949Z/`. Resume harness/full old+mixed corpus work when the reviewed API/carriers arrive. No perf patch, including realm/interprocedural, is accepted or forwarded by this decision alone. No duplicate perf role started.

### Coordinator dependency delivery — 2026-10-23T01:05:06.334470+00:00 (owner header unchanged)

Compiler has frozen the user2256 report-only API proposal v7, patch `2351e4cf48adf9d66ddfaa75661e01e4ffc8bc405b4c61e6f1bac4e3dc5cbc8b`, with bounded naming peer review `b52aea24`. [Exact API and 14-control handoff](../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/authenticated-report-v7-20261023T0101Z/) and [guarded execution preparation](../handoff/reviews/merge-inventory-20261019/compiler/resume-20261022/authenticated-report-focused14-v1-20261023T0106Z/) are preparation only. Canonical product does not yet include this API; no typing or control/corpus run is claimed. Existing perf session can prepare the harness off-tree from the frozen signatures; wait for root publication and successful controls before actual full all-old/mixed proof. Equal alone passes; Different and Unavailable fail the proof. Keep strict global/runtime equality and cache cutoffs separately recorded, complete source/input/registry/body/hosted authentication and count guards unchanged. Realm/interprocedural patch remains withheld until this owner records it proven. Root has not duplicated or taken over perf. Filename times are labels; see actual receipt timestamps.

## Active regression pipeline

- /tmp/perf-run-retry.log: main default-profile control/smoke/PackSet command;
  controls complete, Cargo probe build queued behind the server build.
- /tmp/perf-direct-smoke.log: public API smoke linked from exact completed
  normal Cargo artifacts while unit-test build finished; source/library/binary
  hashes and cleanup recorded in evidence/current-0322d9cd/direct-smoke/.
- /tmp/perf-compiler-current.log: original selected compiler filters in BOTH
  default-profile modes, private byte-identical executable copies. No final
  inventory comparison yet; all test assertions unchanged.
- /tmp/perf-server-current.log: default-profile named server build. It waits
  for /tmp/perf-heavy-tests-finished before running the unchanged180s cases.
  Release that resource flag ONLY after compiler/main/direct jobs AND SpecTcl
  compilation stop using CPU, so the server tests run quietly.
- /tmp/perf-spectcl-current.log: separate target-spectcl-opt1 compilation with
  debug/overflow assertions on; eval_loader/golden_packs wait for
  /tmp/perf-server-tests-finished. Release only after server tests finish.
  Opt1 results do not erase historical default-profile budget outcomes.

No background publisher/controller is running. Manually checkpoint own docs
with fetch/rebase before each normal push, at most30minutes apart. All active
commands build/run/delete their executables in the same command. Temporary
probe .rs files must be removed after the main command; never commit them.
The LSP inventory draft is an additional uncompiled/unproven route, not part
of either perf batch patch and not a fix for all review4.1 scaling shapes.

## Current hosted refusal diagnostic

Three original failing controls were rerun unchanged with their existing trace
flag, using a byte-identical private copy of the known control binary. All3
fail, copy removed; this is diagnostic only. report-refusal-trace/ retains full
raw compressed stdout and observed stage/body inventory, not latency evidence.

The raw event NativeWord and hosted metadata exist, but lowering returns before
hosted-event-units: the unlogged original-word-vector / selected-command OR
check in Lowerer::original_when_event_word refuses. The failing subcomparison
is not observed. Later bodies.event-word None is downstream; do not claim a
retained event word existed. perf-hosted-event-refusal-trace.patch is a narrow
OFF-TREE/uncompiled/unrun compiler-owned diagnostic proposal that logs each
original boolean exactly once and preserves OR short-circuit/order/returns.
It is not a functional fix, part of the batch, or proof of readiness.

Logical alias observations: named0/unit2, NamespaceScript and OriginalDeclaration
share source site24 but have distinct declaration extents24..55 and42..54.
The unchanged Named+NamespaceScript same-declaration expectation is unmet. No
producer cause or permission to weaken it is inferred from these observations.
