# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load performance.

Last updated: 2026-10-22 21:00 UTC

## Resume

**BLOCKED on compiler/source-report ownership. No production commit.**
Latest checked source is 39b39296 in /workspace/tcl-lsp-current, branch work,
shallow/sparse. Full batch with temporary switches is applied in nine Rust
files. Current all-old/mixed controls are complete; exact outcomes are in
perf/evidence/current-39b39296/summary.json. Both E003 and aliases still fail
whole AnalysisResult equality with optimization disabled. Current diagnostics
Eq differs for E003 despite matching Debug; aliases diagnostics Eq agrees.
Fresh build rebuilt all 28 local artifacts, zero warnings; only third-party
caches were reused. Do not mask/normalize owners, drop subjects, compare only
Debug, retry until equal, weaken assertions or change budgets.

The earlier aliases private machine trace at e4a01bbe identifies
ObjectHandleFacts::analysis_report_eq (both admitted report sources None,
raw facts unequal). No private helper trace was taken on 39b39296. Route
through analysis-equality-routing.md; report comparison must preserve actual
input/registry/source/body ownership and memo/Salsa cutoff/currentness semantics.
Do not commit realm.rs or interprocedural.rs. Their patch is NOT proven/ready.

## Completed evidence

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

1. Owner resolution of current all-old full-result comparison blocker. Then
   repeat complete latest-work 102+15+synthetic equality and PackSet proof.
   Fresh controls are a blocker confirmation, not a latest broad-suite verdict.
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

Main patches are refreshed against39b39296; byte-exact clean application only,
no semantic proof implied. /tmp/perf-latest-clean-batch holds clean snapshots.
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
