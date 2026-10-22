# Outstanding work: perf agent

Area: Analysis and SpecTcl pack-load cost.

Last updated: 2026-10-22 18:30 UTC

## Resume

Active private checkout: `/workspace/tcl-lsp-work`, branch `work`, shallow/sparse.
Production proof base is `e4a01bbe`; later local commits are evidence only.
The FULL batch is applied with temporary switches in nine Rust files. No
production code is committed. Do not stage Rust during evidence checkpoints.
Do not reapply the batch or interrupt the active builds for documentation.

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
This is outside perf ownership. Do not weaken PartialEq, drop the object facts,
retry until equality happens, or call diagnostics-only equality a proof.
The six-file production commit remains held until the required full-result
proof is genuinely satisfied or the coordinator resolves this blocker.

## Current commands and controls

- Deeper object-report trace confirms both admitted-source options are None,
  followed by raw runtime fact inequality. No coordinate correspondence was
  admitted. See aliases-object-report-trace.out and the owner routing note.

- A continued-proof worker waits on the latest executable, copies it privately,
  then independently executes all 15 irules and calls/procs/upvar plus PackSet.
  It preserves every failing exit code and finally asserts none failed, but
  continues independent shapes even if the text corpus is blocked. Outputs:
  latest-e4a01bbe/continued-proof/. This cannot manufacture a corpus pass.
- Hosted old-only report control also failed on cookbook_http_collect.irul;
  see irule-old-only-report-control.out and analysis-equality-routing.md.

- `/tmp/perf-run-latest.sh`: optimized complete corpus/irules/synthetic/PackSet
  command, target-latest, source zz_latest_perfprobe.rs and zz_latest_spectclprobe.rs.
  Build log /tmp/perf-latest-probe-build.log; run log /tmp/perf-latest-run.log.
  Production library compilation remains active; do not cancel it for probes.
- `/tmp/perf-latest-crate-lexer-compiler.log`: default test-profile lexer then
  compiler. Lexer completed 583 pass / the same 3 fail in both modes, zero
  pass-to-fail. Compiler build completed and its 226 selected tests are now
  executing new mode, then old mode. Outputs/summary in latest-e4a01bbe.
- Native byte control copies that completed compiler executable privately and
  runs native_array_indices_emit_iteratively_from_the_full_original_arena in
  both modes; existing assertions require 2000 array instructions and x-ff.
  BOTH PASSED (1/1 each). Outputs: tests-compiler-native-array-bytes-{new,old}.out in latest-e4a01bbe.
- Alias diagnostics: direct test-profile binary outputs aliases-direct-retained-*
  show all-old and mixed runs fail with both retained-input settings. Another
  Cargo-built diagnostic is queued after compiler build, log
  /tmp/perf-alias-diagnostic-build.log. All four direct diagnostic modes failed; the shared-input setting does not
  remove the object-report mismatch. No complete-graph Debug formatting: it
  duplicates shared graphs and is too costly. Verdict remains full PartialEq.
- `/tmp/perf-crate-rest.log`: old-base unoptimized SpecTcl golden, then the two
  server tests. The golden new run completed 2 pass / 1 fail in 3537.662 s:
  eda_xilinx's interpreted upgrade reached the unchanged 30-minute wall budget.
  Old golden remains active, so no zero-pass-to-fail verdict for that suite yet.
  The later server build will use current source and records HEAD/diff inputs.
  Both server modes have the requested 180-second limit, exact declared test
  inventory checks, and explicit timeout states separate from assertion results.
- `/tmp/perf-spectcl-complete-input-retry.log`: queued complete-input eval_loader
  retry after that initial SpecTcl stage finishes. It preserves other summaries.
  Do not start another default-profile SpecTcl harness over the same filenames
  while this or the old golden process is active.
- Supplemental base optimized eval_loader: 28 PASS in both modes (1482.833 /
  2614.057 s), zero pass-to-fail. Its later golden BUILD failed because the base
  compiler rlib was invalidated; that is not a semantic regression or proof.
- `/tmp/perf-run-spec-latest-opt1.py`, log /tmp/perf-latest-spectcl-opt1.log:
  waits for completed latest libraries, then runs unchanged eval_loader and
  golden_packs in both modes. Harness debug=0, optimization=1, debug/overflow
  assertions ON; exact library artifacts are recorded. The default-profile
  budget comparisons above remain separately recorded and are not called green.
- Latest ordinary LSP positive controls are queued on the completed libraries.
  They use the actual retained editing-input path (as document_analyser does)
  and require one positive reference per lens, plus all proc/index/lens counts.
  BOTH origin and latest positive controls PASSED all sizes; source chain also
  completed. Final positive-control table is in evidence/timings.md and
  latest-e4a01bbe/lsp-positive-medians-ms.json. Older work rows remain historical.
- Old 253-line tclIndex old/new/old remains active in its private copied binary.
  10/40/100 rows were exactly equal; old 100 timing is instrumented by a live
  debugger pause and must not be used as an uninstrumented ratio. A separate
  new-only 253 timing completed at 326554 ms; its eq=true field is explicitly a
  timing-only placeholder, NOT an equality result.

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
   --snapshot /tmp/perf-clean-batch --check-toggled reproduces the nine-file
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
