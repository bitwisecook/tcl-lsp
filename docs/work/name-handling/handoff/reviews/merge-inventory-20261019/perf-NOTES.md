# perf agent notes: Analyser and SpecTcl

Last updated: 2026-10-23 11:04 UTC

**Latest proof refresh in progress; no production code landed.** The independent
0322d9cd checkout contains the full switched batch. Clean six/two-file patches
and routed residual drafts are refreshed against that exact source; formatting
and byte-exact switch reconstruction pass. This is applicability evidence,
not semantic acceptance.

The coordinator-recorded user2256 contract accepts the distinct authenticated
complete-report API. The new harness requires all three old/new/old comparisons
to be Equal and records strict runtime equality independently. Current compiler
controls and smoke cases are running; see perf/evidence/current-0322d9cd/.
Earlier failed results below remain pinned to their historical source commits.
No latest full-corpus verdict or compiler-patch readiness is claimed.

## Current0322 progress

Lexer library: 586/586 pass in both modes, same complete inventory, zero
warnings, default test opt0/debug0, executable removed. Byte audit retains all
102 text Tcl +15 iRules inputs unchanged from the baseline/added-class roster.
SpecTcl tracked input census is complete (72 present /0 missing). Runtime
providers are pinned Tcl9.0.4/Tcl8.6.18/Jim0.84-9-g5bac7c9.

The initial f149597f build executed zero tests because propagation::run lacked
documentation. The existing upstream f89bd6f6 repair is included in0322; no
perf edit to that optimiser source was made. Failed build/source receipts
remain in evidence/current-f149597f/. The current compiler controls build is
still running; no report availability/equality result is claimed yet.

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

Main artifacts are refreshed against 39b39296 and apply byte for byte;
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
