# perf agent notes: Analyser and SpecTcl

Last updated: 2026-10-23 22:34 UTC

**Frozen b6 proof progressed; perf code remains unlanded.** Switched and clean6
lexer586/586 pass BOTH with0Rust warnings. Full selected compiler228 completed
outcomes match BOTH (142pass86fail); the remaining original1000-depth case
is resource-killed. Clean6 isolated227 outcomes match BOTH (141pass86fail);
the unchanged1000-depth case OOMs even alone BOTH at roughly15.7/15.6GiB RSS.
This is not a complete passing compiler inventory; original incomplete runs
and their15 resource P2Fs remain recorded alongside the recovery.

Full-batch b6 mixed Tcl is101Equal/1Unavailable; old main run was interrupted,
then its exact-ELF remaining25 inputs all pass. The sole refusal is original
Native35, caused by a current empty ALLOCATED declaration being admitted only
for Installed bodies. A report-only candidate preserves strict current cards,
source/namespace/geometry/role/purpose checks and adds positive/negative controls.
Its quiet19-control retry now PASSES BOTH with0warnings and unchanged source.
Exact full Native35 plus compact reproduction subsequently give all3 Equal
pairs BOTH forced-old/mixed, with strict runtime Eq separately false. The
necessary API-only repair is published separately in this update. Full
corrected latest-source corpus acceptance is still pending.
Both15 iRules and all7 synthetic sizes give all3 Equal pairs; complete ordered
PackSet Debug is deterministic/equal (484936 lines,8packs,0notices/errors).

Current contended opt1 b6 wall timings (report capture included; comparison
excluded): calls80 old26815/new2721/old15968ms; upvar30 old73450/new15071/
old46401ms. Ordinary load_embedded old14514/new2964/old16003ms with full
PackSet equality. Origin same load API4665ms is a separate observation. Full
provenance and limits: perf/evidence/timings.md. No latest-source/release claim.

Server and SpecTcl binaries built0warnings and await original-budget quiet
runtime. Shared residual drafts remain Blocked/routed and unproven. The next
steps and exact active commands are in outstanding/perf.md. Six-file perf
commit and compiler-owned patch readiness remain withheld. The following
historical results retain their original source/profile scope.

## Current0322 progress

Lexer library: 586/586 pass in both modes, same complete inventory, zero
warnings, default test opt0/debug0, executable removed. Byte audit retains all
102 text Tcl +15 iRules inputs unchanged from the baseline/added-class roster.
SpecTcl tracked input census is complete (72 present /0 missing). Runtime
providers are pinned Tcl9.0.4/Tcl8.6.18/Jim0.84-9-g5bac7c9.

The initial f149597f build executed zero tests because propagation::run lacked
documentation. The existing upstream f89bd6f6 repair is included in0322; no
perf edit to that optimiser source was made. Failed build/source receipts
remain in evidence/current-f149597f/. The current compiler control build passes with zero warnings. Its15 report
controls finish12PASS/3FAIL in BOTH modes: two original alias-role assertions
at body_correspondence.rs813/884 and hosted CurrentOwnerUnavailable at
analyser/authenticated_report.rs500. Equal inventory/zero pass-to-fail does
not make that a usable full-report acceptance gate.

Direct public-API smoke from the exact completed Cargo libraries returns all
three authenticated Equal pairs for E003 and aliases, forced-old AND mixed;
strict runtime comparisons remain false. This is only a two-fixture smoke.
Hosted smoke, full corpus and current regression suites remain pending or
blocked; see the outstanding file for the active resource-separated pipeline.

## Historical current-source confirmation (39b39296)

A separate shallow checkout rebuilt all 28 local artifacts at 39b39296 with
zero warnings; only third-party caches were retained. Exact source, binary,
cache-invalidation and Cargo receipts: `perf/evidence/current-39b39296/`.
The full batch and temporary switch are applied only in private proof trees.

| Current control | Equal | Unequal | Panics |
| --- | ---: | ---: | ---: |
| tcl/old-only | 0 | 2 | 0 |
| tcl/mixed | 0 | 2 | 0 |
| irules/old-only | 0 | 15 | 0 |
| irules/mixed | 0 | 15 | 0 |

E003 and aliases fail all-old as well as mixed runs. E003 diagnostics PartialEq
is false despite identical Debug text; aliases diagnostics PartialEq is true.
The earlier e4a01bbe machine trace locates aliases' first false helper in
ObjectHandleFacts::analysis_report_eq: neither report source is admitted, then
raw fact equality fails. That helper was not privately traced on 39b39296.
See `perf/analysis-equality-routing.md`; report correspondence must preserve
actual owners, input, registry, source and body coordinates. Audit memo/Salsa
cutoffs before changing global equality. Debug equality is not the verdict.

## Completed broad proof at e4a01bbe

The text corpus completed **102 inputs: 100 equal, 2 unequal, zero panics**.
Every equal row has deterministic old endpoints. The two failures are E003
and aliases, both also failing isolated forced-old controls. All 15 iRules
fail mixed and forced-old comparisons. Calls 10/20/40/80, procs 41 and upvar
10/30 pass full old/new/old equality with deterministic endpoints. Complete
ordered PackSet Debug passes (484,936 lines), with deterministic old endpoints.

The original roster contained 101 UTF-8 Tcl files and one intentional byte
fixture. The corrected text roster adds classes.tcl; the original 0xff fixture
remains a separate actual native-codegen control, passing both modes. No
original positive input was dropped or decoded lossily. The origin/rust
baseline fcb7927f covers the same 102 text cases and 15 iRules plus synthetics.

| Suite | New | Old | Comparison |
| --- | --- | --- | --- |
| Lexer lib | 583 pass / 3 fail | same | zero pass-to-fail |
| Compiler selected filters, all 226 | 151 pass / 75 fail | same | zero pass-to-fail |
| SpecTcl opt1 eval_loader | 28 pass | 28 pass | zero pass-to-fail |
| SpecTcl opt1 golden_packs | 3 pass | 3 pass | zero pass-to-fail |
| Default eval_loader, original run | 26 pass / 2 fail | 27 pass / 1 fail | **one observed pass-to-fail** |
| Same default failing case, quiet-host repeat | pass | pass | same assertions/budgets |
| Server startup metadata | first assertion 0 vs 1 | same | later reindex assertions unreached |
| Server workspace-symbol cap | timeout180 | timeout180 | gate fails in both modes |

The default-profile pass-to-fail is every_shipped_pack's interpreted eda_xilinx
wall budget. The targeted quiet-host repeat passed new/old in 1150.068 /
1429.759 s wall, 1141.846 / 1409.186 s CPU. The original full-suite failure is
retained; neither optimized green results nor this repeat overwrite it. The
separate five-second budget-axis assertion fails in both default modes.
Historical base-8a default golden_packs has 2 pass / the same 1 wall-budget
failure in both modes; its provenance is separate from latest tests.

## Before/after measurements

Wall times are contended opt-level-1 observations with debug assertions on,
not release latency. Process-CPU samples below exclude equality/reporting.
All exact inputs, source/artifact receipts and tables are in
`perf/evidence/timings.md`; units below are milliseconds.

| Full-analysis CPU, e4a01bbe | Old1 | New | Old2 |
| --- | ---: | ---: | ---: |
| Calls 10 | 1470 | 810 | 1260 |
| Calls 20 | 3280 | 3600 | 4340 |
| Calls 40 | 10350 | 2460 | 8480 |
| Calls 80 | 35450 | 3760 | 21660 |
| Upvar 10 | 12810 | 3880 | 6070 |
| Upvar 30 | 71930 | 18670 | 58020 |

| Input | origin/rust wall | Work old wall | Work new wall |
| --- | ---: | ---: | ---: |
| load_embedded, e4a01bbe | 4665 | 44471 / 82646 | 10673 |
| tclIndex 253, historical 8a01a718 | 1596 | 5108792 | 189404 |
| tclIndex 253, e4a01bbe timing only | 1596 | not measured on this base | 255893 |

The full historical tclIndex 10/40/100/253 comparisons are equal/deterministic.
Only the 100-line old timing was debugger sampled. Latest new-only timings
10/40/100/253 are 1319/11962/53223/255893 ms; their equality field is a timing
placeholder. Do not treat that row as latest-source equality proof.

Ordinary LSP positive controls pass at 10/20/40/80 procedures, including one
actual reference per lens. At 80 procedures, origin/work index medians are
0.497/1557.801 ms; lens medians 106.465/25542.691 ms. The work harness uses
the actual retained editing-input path. Source-chain controls pass at
250/500/1000/2000. These show remaining ordinary-path cost, not a fix for it.

## Patches and next owner actions

Main artifacts are refreshed against94c7f61e and apply byte for byte;
`patch-inputs.json` / `patch-refresh-validation.json` record that limited check.
`perf-owned.patch` contains exactly six files, with no switch: lexer parse_cut.rs
and source_map.rs; compiler original_word.rs, original_name_value.rs,
formal_call_advice.rs and incomplete_body_metadata.rs. `perf-realm-interprocedural.patch`
contains the two compiler-owned callers. Neither patch is declared proven.
Exact older proof patches are retained under evidence/latest-e4a01bbe/proof-patches/.

The batch hoists script images, adds an immutable-allocation equality shortcut,
checks bound-word ancestry inductively, reuses native LF line indexes, and
memoizes immutable realm geometry, caller-alias surveys and incomplete headers.
Fixed geometry/config/channel/native-byte controls remain intact. The clean
snapshots pass owned-file rustfmt. An older six-file-only Clippy snapshot had
zero owned and 783 foreign diagnostics; this is not a current workspace-green
claim. Clean no-switch validation remains required before landing.

`perf-analyser-images.patch` then `perf-analyser-segments.patch` are unproven,
unapplied compiler-names drafts. Image reuse still scans full bytes per command;
eliminating that scan needs an admitted immutable operation context. Ordinary
Logical lenses repeatedly rebuild inventory/index, including inside currentness
checks. A sampled compiler body walk also recursively revisits regions/clones
AdviceGraph. See `residual-routing.md` for exact limits, controls and owner routes.

Next: resolve the current all-old/report blocker with its owner, then rerun the
complete proof on latest work, preserve zero pass-to-fail and clean-switch
gates, and commit only the six owned production files as one commit. Only after
full proof may the routed compiler patch be marked ready. Never open a PR.

Current hosted public-API smoke is complete: forced-old AND mixed each give
Unavailable(CurrentOwnerUnavailable) for all3 pairs; the private executable
is removed. Existing unchanged-control tracing narrows the first refusal to
the unlogged word-vector/selected-command guard in original_when_event_word;
which comparison fails is not observed. The routed pure diagnostic draft is
uncompiled/unrun, outside the batch. See analysis-equality-routing.md and
current-0322d9cd/report-refusal-trace/; current regression suites continue.

Current a8e7 compiler control build stops before tests at upstream source_graph.rs560
E0277: the byte-array reference does not implement Into<Arc<[u8]>>. Zero
warnings, no tests, source unchanged and cleanup complete. The routed
perf-source-graph-test-slice.patch borrows the same literal bytes as &[u8],
retaining the channel and every assertion. It is off-tree, with no owner source
edited. Independent opt1 SpecTcl compilation continues; full proof pending.

The user authorised necessary shared-file work and immediate work publication.
The source_graph.rs byte-slice test typing correction is pushed separately as
6c1c9d2c, retaining fixture bytes/channel/assertions and clean rustfmt. The
compiler retry now includes this exact test-only diff; its fresh receipts
will be current-a8e7bc7a-test-slice/. Default0 server and opt1 SpecTcl builds
continue independently. Neither patch is semantically proven or accepted.

Current94c7f61e clean-six snapshot: exactly six files, no switch, all117
fixture bytes match baseline; lexer586/586 PASS in both environment executions
with zero warnings and executable cleanup. The variable has no effect in
this no-switch build; the independent a8 switched lexer comparison also
passes586/586 BOTH. Initial missing sparse oracle input failures are archived,
then exact tracked captures restored. Clean-six metadata/compiler-filter
checks continue. Both split patches are unchanged byte for byte on94c7f61e;
ordered apply/snapshot/fmt checks pass. Neither is semantically proven.
The a8 server default0 and SpecTcl opt1 binaries have completed zero-warning
builds and wait for quiet resources; no runtime verdict is claimed.

Current clean6 metadata check94c7f61e failed with6 upstream typing errors
and3 unused imports, zero tests. Full diagnostics are retained. Necessary
minimal compiler repairs are PUSHED separately as6e729fe9, preserving every
admission predicate/assertion. The clean6 compiler retry uses that canonical
head and remains unproven. Its builder and the a8 control builder/watcher
are temporarily paused while all4 named server180s runs execute quietly;
the window resumes exact owned processes in finally after server cleanup.
SpecTcl opt1 tests remain queued; no new runtime result is implied.

Quiet a8 default0 server comparison is COMPLETE, zero build warnings and
zero P2F but FAILED gates: startup first retained-read assertion0 vs1 BOTH
(new10.412s, old21.355s), later factory reindex UNREACHED; workspace-symbol
timeout180 BOTH (wall181.069/180.329s). Compiler builds/watcher were paused
for all4 runs, resumed afterward; server executable removed. Exact receipts
are current-a8e7bc7a/crate-tests-default/.

A separate SHA-identical private server copy, unchanged fixture/assertions,
ran the existing source-phase trace and timed out180, then was removed.
Analysis completed at171.372s: walk79.839s, diagnostics91.531s; child timings
are inclusive and must not be summed again. This trace was contended after
builders resumed, so it is diagnostic only and does not locate the quiet
original timeout in index/query. Full raw trace/source/artifact/cleanup and
observations are current-a8e7bc7a/workspace-symbol-source-phase-trace/.

SpecTcl opt1 runtime is now active on reserved cores2,3; compiler/control
roots and the canonical clean6 retry use cores0,1 (CPU quota4). Both modes
retain the same2 threads, original assertions and budgets. The guard restores
original masks after all4 runs and executable cleanup. New eval_loader has
21/28 completed passes so far; no complete comparison is claimed. Current
clean6 build reaches3 VM typing errors already repaired by owner0747a2a5.
It preserves in-flight compiler codegen, then the checked automatic retry
updates to canonical work with the identical six-file perf diff. Neither
main perf patch is proven/landed; main16 controls still build.

The complete6e clean6 build fails only3 known VM typing errors, zero Rust
warnings/no tests. Checked retry is now on canonicalccd57033 with the exact
same six-file diff. That canonical head also changes Native declaration-span
selection, so final complete corpus acceptance requires canonical source.
Frozen a8 results remain scoped evidence. New SpecTcl eval_loader is27/28
passes so far, last shipped-pack case still running; no full suite verdict.

A fresh-normal-library public diagnostic on the exact37-byte hosted fixture
finds hosted input, realm image, source origin and binding site0 in BOTH modes;
self report comparison remains CurrentOwnerUnavailable. This is the public
realm query, not the private Lowerer guard or an equality acceptance proof.
Exact source/library/binary receipts and output are current-a8e7bc7a-test-slice/
hosted-site-diagnostic/. Its executable and temporary Rust source were deleted.

Complete a8+test-slice controls:16 each,15 PASS/1 FAIL BOTH, identical
 inventories and zero P2F/zero warnings. Native/Logical role/purpose/extent
 controls and actual diagnostic-realm control all pass; genuine hosted capture
 still fails. Exact unchanged single-case trace retains a genuine Event body
 and current hosted domain, then refuses because the event card has no event
 word. Lowering original words/event operand/context are Some; intervening
 branch/guard boolean remains unobserved. The first exact-selector attempt
 ran0 tests and is explicitly INVALID; corrected trace runs exactly1, fails
 unchanged assertion and deletes its SHA-identical executable copy.

Current opt1 eval_loader new finishes27/1 in3336.099s: interpreted eda_xilinx
 hits its unchanged30-minute per-pack wall budget. Old mode is still running;
 no zero-P2F verdict yet. Preserve this run. Resource maintenance evicted cold
 clean build-file cache (~3GiB) without altering source/settings/file contents;
 no OOM kill occurred. The resource receipt is not timing acceptance evidence.

Canonical full switched proof is now rebased toa3a4b181, preserving the upstream
line-index implementation in BOTH old/new modes. Clean six/two split, ordered
patch application, scoped formatting and exact toggle reconstruction PASS; no
semantic acceptance inferred. Current controls/normal libraries compile; clean6
ccd filters run after a zero-warning build. Both residual analyser drafts and
nested LSP currentness draft refresh toa3, remain Blocked/routed and unproven.

Both proof sources are rebased tob6ea6dc5 with fresh independent builds.
Exact six/two split + ordered application, scope fmt, toggle reconstruction
and clean6 match PASS. Clean6 opt1 lexer586/586 BOTH passes without warnings;
compiler/control/corpus acceptance remains pending. No perf production commit.

B6 broad Clippy completed with62 diagnostics, zero in the six owned files.
One is new in the routed realm constructor spelling (Default::default), to
qualify before delivery; the remaining issues are retained/routed. Separate
0f1d32b8 removed2 pre-existing doc-spacing errors, scope fmt passes. No full
workspace lint success or new runtime outcome is claimed. Both compiler
libraries still build; no frozen source was edited to address lint meanwhile.

Current b6 mixed Tcl completes101 Equal/1 native35 Unavailable; recovered old
25/25 tail Equal restores original102 coverage with the same single refusal.
Original77-row resource failure is retained. The exact cfg(test) trace confirms
empty allocated OriginalDeclaration body rejected by Installed-only empty-span
admission. Separate full original-allocation-card admission is a candidate in
isolated diagnostic tree, with all old assertions plus new positive/negative
controls; build underway, no Rust diagnostics yet. No repair acceptance or perf
production commit is claimed. Main iRules/synthetics/PackSet continue.

Frozen b6 opt1/report wall examples (capture included, comparisons excluded):
calls80 old26815/new2721/old15968ms; upvar30 old73450/new15071/old46401ms, all3
Equal. Ordinary load_embedded old14514/new2964/old16003ms; complete ordered
PackSet equal/deterministic,8packs/0notices/0errors,484936 lines. Origin same API
4665ms is a separate contended observation. Full timing/acceptance limits remain
in evidence/timings.md; this does not accept the native35 refusal.

Candidate report repair build retries are not green: first2 test-only Debug
assertion type errors corrected without changing the predicate; subsequent
build resource-killed while deep compiler test ran. Quiet retry now waits for
isolated compiler recovery. Original clean6 suite has resource-based incomplete
comparison/P2Fs; isolated227 nondeep outcomes are identical BOTH, zeroP2F,
peak351180/352356KiB. Unchanged1000-depth case runs alone next. No runtime is
paused and no original failure is removed. No repair/perf production commit.
