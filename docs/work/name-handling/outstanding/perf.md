# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load cost.

Last updated: 2026-10-22 20:12 UTC

## Resume

ADDITIONAL GATE: completed default-profile eval_loader has ONE observed
pass-to-fail: every_shipped_pack_loads_identically_with_and_without_the_static_fast_path
(new hits interpreted eda_xilinx wall budget; old passes). New 26/2 in
4418.488 s, old 27/1 in 2712.566 s; both fail the separate 5-second budget-axis
assertion. Preserve these outcomes. A targeted controlled repeat of ONLY that
case is active, both modes on the now-idle host with identical unmodified
30-minute budget/assertions. /tmp/perf-controlled-eval-proof.py, log
/tmp/perf-controlled-eval-proof.log; outputs latest-e4a01bbe/controlled-default-eval/.
Build succeeded in 22.78 s with zero warnings. No other heavy job remains.
Do not replace the failed comparison with optimized green results or claim
zero pass-to-fail until the controlled result is assessed and the initial
observation remains explicit.

Next after the controlled case finishes: verify the all-old equality blockers
on a FRESH latest-work build before any final hand-back or production commit.
The completed broad proof is frozen at e4a01bbe, whereas upstream has since
landed R1/C6/Jim and lint/API batches. Do not assume an older failure still
occurs on current source. Use another shallow/sparse checkout
(/workspace/tcl-lsp-current), the existing sparse patterns and oracle tmp
symlink. Apply the full batch with --zz there. Keep a unique target directory;
reuse only cached third-party artifacts, invalidate all copied local-crate
fingerprints/rlibs so no sibling-worktree artifact can masquerade as current.
Build only the compiler probe first (default opt=0/debug=0 is sufficient for
the small aliases/E003/hosted all-old controls). Do not run another heavy build
while the controlled wall-budget case is active. If current blockers persist,
route the exact new-base evidence and keep the six-file commit held. If they
clear, resume all required proofs and land only the clean six files.

Active private checkout: `/workspace/tcl-lsp-work`, branch `work`, shallow/sparse.
Production proof base is `e4a01bbe`; later local commits are evidence only.
The FULL batch is applied with temporary switches in nine Rust files. No
production code is committed. Do not stage Rust during evidence checkpoints.
Do not reapply the batch or interrupt the active builds for documentation.

The main perf/*.patch artifacts are refreshed against origin/work 6df7bbea;
patch-inputs.json and patch-refresh-validation.json record the exact base and
byte-exact clean application. These refreshed patches are UNPROVEN on that
base. e4a01bbe proof patches are preserved in
perf/evidence/latest-e4a01bbe/proof-patches/. The active Rust files are STILL
the frozen e4 source; do not apply refreshed patches over the running tree.
For frozen-tree reproduction, use refresh_patches.py ROOT --snapshot
/tmp/perf-clean-batch --check-toggled --output-dir /tmp/perf-proof-patches.
For current-origin artifacts, use --base origin/work --snapshot
/tmp/perf-latest-clean-batch (no --check-toggled). The helper accepts the exact
upstream lint changes and preserves the new empty-token parse guard.

The old `8a01a718` binaries have separate outputs in perf/evidence. Latest
outputs are in perf/evidence/latest-e4a01bbe. Frozen old binaries remain valid
for their recorded base even though the source tree advanced. The first
latest build in target/ was stopped and its orphan compiler terminated;
latest optimized builds now use target-latest exclusively. Do not link new
base-8a diagnostics against target/: its compiler rlib was invalidated.

BLOCKER: full AnalysisResult equality on aliases.tcl fails even with every
optimization forced off, with either ordinary ingress or one retained complete
input. Public fields and retained realm/world compare equal. A separate
machine-level trace locates the first false helper precisely at
ObjectHandleFacts::analysis_report_eq (object_types/report_comparison.rs).
See perf/analysis-equality-routing.md and latest-e4a01bbe/aliases-private-eq-trace.out.
The latest corpus also fails E003_proc_call_arity. Its isolated all-old control
ALSO fails, specifically with diagnostics PartialEq unequal despite identical
Debug strings (FIELD_DIFF diagnostics). Existing probe logs' diagnostics_equal
label means Debug equality only; next-build recorded source fixes the label.
See e003-old-only-report-control.out / e003-old-only-inputs.json and the routing
note. Do not discard typed diagnostic subjects or assume their equality.
This is outside perf ownership. Do not weaken PartialEq, drop the object facts,
retry until equality happens, or call diagnostics-only equality a proof.
The six-file production commit remains held until the required full-result
proof is genuinely satisfied or the coordinator resolves this blocker.

## Current commands and controls

- Deeper object-report trace confirms both admitted-source options are None,
  followed by raw runtime fact inequality. No coordinate correspondence was
  admitted. See aliases-object-report-trace.out and the owner routing note.

- A continued-proof worker copied the latest executable privately and
  independently executes all 15 irules and calls/procs/upvar plus PackSet.
  All 15 irules failed strict equality; calls 10/20/40/80 and procs 41 PASSED
  complete old/new/old equality. Upvar 10/30 both PASSED. Latest PackSet
  PASSED full ordered Debug (484936 lines), deterministic old endpoints:
  old1 44471 ms / new 10673 ms / old2 82646 ms. The continuation command
  exits 1 because it retains/asserts the failing irule verdict; it did not
  stop or hide the independently successful shapes.
  It preserves every failing exit code and finally asserts none failed, but
  continues independent shapes even if the text corpus is blocked. Outputs:
  latest-e4a01bbe/continued-proof/. This cannot manufacture a corpus pass.
- Hosted old-only report control also failed on cookbook_http_collect.irul;
  see irule-old-only-report-control.out and analysis-equality-routing.md.

- `/tmp/perf-run-latest.sh`: optimized complete corpus/irules/synthetic/PackSet
  command, target-latest, source zz_latest_perfprobe.rs and zz_latest_spectclprobe.rs.
  Build log /tmp/perf-latest-probe-build.log; run log /tmp/perf-latest-run.log.
  Optimized build completed (122m48s); full 102-file text corpus FINISHED
  100 equal / 2 unequal (E003 and aliases), zero panics. Both mismatches also
  fail isolated all-old controls. Main command exits 1 at the unchanged full
  equality assertion and deleted its executables; independent shapes/PackSet
  already completed in continued-proof/. All 100 successful rows have det=true.
- `/tmp/perf-latest-crate-lexer-compiler.log`: default test-profile lexer then
  compiler. Lexer completed 583 pass / the same 3 fail in both modes, zero
  pass-to-fail. Compiler build completed and its 226 selected tests are now
  BOTH modes completed all 226: identical 151 pass / 75 fail, zero
  pass-to-fail and identical complete test inventories. The executable was
  deleted by its build/run command. Outputs/summary in latest-e4a01bbe.
- Native byte control copies that completed compiler executable privately and
  runs native_array_indices_emit_iteratively_from_the_full_original_arena in
  both modes; existing assertions require 2000 array instructions and x-ff.
  BOTH PASSED (1/1 each). Outputs: tests-compiler-native-array-bytes-{new,old}.out in latest-e4a01bbe.
- Alias diagnostics: direct test-profile binary outputs aliases-direct-retained-*
  show all-old and mixed runs fail with both retained-input settings. The Cargo-built diagnostic
  also completed all four modes with exit 101 (aliases-retained-*.out), log
  /tmp/perf-alias-diagnostic-build.log. The shared-input setting does not
  remove the object-report mismatch. No complete-graph Debug formatting: it
  duplicates shared graphs and is too costly. Verdict remains full PartialEq.
- `/tmp/perf-crate-rest.log`: old-base unoptimized SpecTcl golden, then the two
  server tests. The golden new run completed 2 pass / 1 fail in 3537.662 s:
  eda_xilinx's interpreted upgrade reached the unchanged 30-minute wall budget.
  Old golden completed 2 pass / the same 1 fail in 5717.749 s,
  zero pass-to-fail. The server build is now active on e4a01bbe plus full batch,
  with recorded HEAD/diff inputs.
  Startup completed the same first assertion failure (0 vs 1 at line 618)
  in both modes, new 24.451 s / old 23.893 s. It does not reach factory reindex.
  Workspace-symbol timed out at 180 s in BOTH modes (180.085 / 180.224 s).
  No old test passed and no pass-to-fail occurred; both server gates fail.
  Exact outputs and validated source inputs are copied into latest-e4a01bbe. Both modes have
  the requested 180-second limit, exact declared test
  inventory checks, and explicit timeout states separate from assertion results.
- `/tmp/perf-spectcl-complete-input-retry.log`: complete-input eval_loader
  new mode completed 26 pass / 2 fail (4418.488 s), both budget assertions:
  the 5-second axis test and eda_xilinx interpreted 30-minute exhaustion.
  Old completed 27 pass / 1 fail: ONE pass-to-fail on every_shipped_pack;
  controlled repeat above is active. No missing sparse input in this retry. It preserves other summaries.
  Do not start another default-profile SpecTcl harness over the same filenames
  while this or the old golden process is active.
- Supplemental base optimized eval_loader: 28 PASS in both modes (1482.833 /
  2614.057 s), zero pass-to-fail. Its later golden BUILD failed because the base
  compiler rlib was invalidated; that is not a semantic regression or proof.
- `/tmp/perf-run-spec-latest-opt1.py`, log /tmp/perf-latest-spectcl-opt1.log:
  latest optimized eval_loader completed all 28 PASS in BOTH modes
  (new 1488.967 s / old 1568.082 s), zero pass-to-fail. Latest optimized
  golden_packs completed all 3 PASS in BOTH modes
  (new 651.764 s / old 864.494 s), zero pass-to-fail. Harness debug=0, optimization=1, debug/overflow
  assertions ON; exact library artifacts are recorded. The default-profile
  budget comparisons above remain separately recorded and are not called green.
- Latest ordinary LSP positive controls completed on the new libraries.
  They use the actual retained editing-input path (as document_analyser does)
  and require one positive reference per lens, plus all proc/index/lens counts.
  BOTH origin and latest positive controls PASSED all sizes; source chain also
  completed. Final positive-control table is in evidence/timings.md and
  latest-e4a01bbe/lsp-positive-medians-ms.json. Older work rows remain historical.
- Full base-8a 10/40/100/253 tclIndex old/new/old FINISHED exactly equal
  and deterministic. 253 new = 189404 ms / old2 = 5108792 ms; raw
  work-tclIndex-threeway.out. Its executable was deleted by its command.
  10/40/100 rows were exactly equal; old 100 timing is instrumented by a live
  debugger pause and must not be used as an uninstrumented ratio. A separate
  new-only base-8a 253 timing completed at 326554 ms. Latest e4 new-only
  10/40/100/253 timings completed at 1319/11962/53223/255893 ms; their eq=true
  fields are timing-only placeholders, NOT equality results.

- Supplemental process-CPU calls 1/10/20/40/80 completed, all exact and
  deterministic. 80 calls CPU old1/new/old2 = 35450/3760/21660 ms; n=1 warmup.
  synthetic-cpu-summary.json records every sample, including the mixed n=20
  result. CPU upvar 1/10/30 also completed exact/deterministic; n30 CPU
  old1/new/old2 = 71930/18670/58020 ms. These
  probes link frozen libraries directly and delete their private executables.
- All-old control for ALL 15 irules FINISHED: all 15 fail full equality,
  all old endpoints differ (irules-all-old-control.out),
  copied from the CPU probe's identical frozen libraries; its middle old=false
  argument is overridden by ZZ_OLD_ENV=1. No assertion or subject is stripped.

## Corpus and completed evidence

Original corpus-original-102.txt contains 101 UTF-8 sources and one intentional
byte-native fixture (native_deep_array_word.tcl contains 0xff). The old baseline
silently skipped that fixture; the former claim of 102 timed analyses was wrong.
Current corpus.txt retains every original text input and adds classes.tcl:
102 UTF-8 AnalysisResult inputs. corpus-native-bytes.txt preserves the original
byte control under its existing compiler test. No lossy decoding is used.
corpus-roster-audit.json records counts, channels, SHA-256 and exact control name.
All 15 irules are present. Origin timings cover all 102 text cases (the added
class fixture measured 740 ms), all 15 irules, synthetic shapes, exact Tk index
prefixes, embedded packs and LSP query/graph shapes. These are timing-only, not
cross-branch AnalysisResult equality claims.

The base corpus stopped on the aliases mismatch then strict UTF-8 rejection;
IRules and synthetics in that command did not run. Do not mark the corpus passed.
The complete ordered PackSet Debug control DID PASS at base 8a01a718:
old1 62828 ms / new 25493 ms / old2 120495 ms, deterministic=true,
equal_complete_debug=true. No sorted-line normalization was used.

Owned-only Clippy snapshot: zero diagnostics in the six owned files, 783
elsewhere. No new allowance; no workspace-green claim. Earlier 791-diagnostic
full-batch check found seven owned lints, all fixed, plus one routed initializer
lint (clean routed patch uses explicit OriginalRealmQueryMemo::default()).
The LF line-index fixed control passes with both modes; full compiler tests
include it. Owned rustfmt is clean. No existing assertion was weakened.

## Remaining work, in order

1. Resolve/routable owner fix for the object-report comparison blocker; finish
   strict old/new/old equality for all 102 text Tcl + 15 irules + synthetics on
   latest source. Do not substitute diagnostic equality or strip owner evidence.
2. Complete selected compiler filters (spectcl_source, original_name_value,
   formal_call_advice, realm, interprocedural, w216, w143, incomplete_body),
   both SpecTcl suites and both named 180-second server cases with zero
   pass-to-fail. Keep matching baseline failures and warning limitations explicit.
3. Land ONLY the six perf-owned clean files as ONE commit, without the switch:
   lexer parse_cut.rs/source_map.rs; compiler original_word.rs,
   original_name_value.rs, formal_call_advice.rs, incomplete_body_metadata.rs.
   `/tmp/perf-clean-batch` holds clean snapshots. refresh_patches.py ROOT
   --snapshot /tmp/perf-clean-batch --check-toggled
   --output-dir /tmp/perf-proof-patches reproduces the nine-file
   experimental tree and six/two split. Never commit realm/interprocedural.
4. Refresh perf-realm-interprocedural.patch against latest work and, only once
   truly proved, record: "perf-realm-interprocedural.patch proven at <commit>,
   ready for the compiler agent". Coordinator routes it.
5. Residual tclIndex candidates perf-analyser-images.patch then
   perf-analyser-segments.patch are UNPROVEN, BLOCKED/ROUTED to compiler-names.
   They are not applied. Full byte guard in the image draft still costs O(N);
   removing that requires an admitted immutable context for the whole operation.
   Required mutation/channel/config/partial-source controls are in residual-routing.md.
6. Review 4.1 ordinary Logical lenses rebuild the call inventory/index repeatedly,
   including inside is_current_in. Record final positive-control timings and
   route request-scoped inventory reuse to LSP without weakening owner checks.
   Other review scaling shapes remain with their semantic owners.
7. Update perf-NOTES.md with final before/after timings and stop. Never open a PR.

## Checkpoint and environment rules

A docs-only publisher worktree /workspace/tcl-lsp-publish keeps rebases isolated
from active proof files. The approved checkpoint controller uses DOCS-ONLY
CHERRY-PICKS, never resets: /tmp/perf-auto-checkpoint.py, log
/tmp/perf-auto-checkpoint.log. It snapshots every 25 minutes, fetches/rebases
before every push, retries rejections, and refuses staged Rust. STOP it before
any production commit by creating /tmp/perf-auto-checkpoint-stop; confirm the
controller stopped. Manual documentation edits must hold
/tmp/perf-checkpoint.lock with flock to avoid races. Automatic approval rejected
an earlier reset-based controller; no such controller was executed.

Use source /tmp/perf-env.sh then the active scripts/dev/agent-build-env.sh.
CARGO_INCREMENTAL=0, CARGO_PROFILE_TEST_DEBUG=0,
TCL_LSP_REQUIRE_TCLSH=9.0,8.6, TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh.
Tcl 9.0.4, Tcl 8.6.18 and pinned Jim are built. Tk 8.6.18 index is 253 lines,
SHA-256 e7ea83cd5df322fb760c9787c9eda92bad3d048737be28d2c9606b9243cf0bbb.
Official crates sparse index is configured in session Cargo home through GitHub
because the normal endpoint returned 502. Keep proxy configuration.

Separate origin/rust tree: /workspace/tcl-lsp-rust at fcb7927f, own target.
/workspace/tcl-lsp has an interrupted checkout; use only its ignored tmp/
oracles through the active tree's symlink. Temporary probes are zz_*probe*.rs,
never committed. Build/run/delete executables within the same command; clean
Cargo executable aliases as well. Commit footer follows the repository norm:
Co-Authored-By: Codex <noreply@openai.com>. No fabricated session attribution.
