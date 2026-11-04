# Current LSP naming migration state

Updated 04 November 2026, 01:55 UTC. This snapshot supersedes earlier execution and ownership claims; their original receipts and git history remain available.

## Scope and release verdict

**The LSP is not release-ready.** The goal is the LSP and the libraries it uses, migrated to Tcl name resolution. Separate VM, BIG-IP and ancillary feature development is deferred. Linked dependencies and mandatory release gates remain in scope. Conditional source advice grants no Native/provider execution authority.

The checkout is `/workspace/tcl-lsp-partial`, branch `work`, a shallow clone of the configured `https://github.com/bitwisecook/tcl-lsp.git` origin. The user approved pushing reviewed work before green tests. No PR, merge to `rust`, tag or release has been created.

The active publisher fetches `work`, checks ancestry, fast-forwards safe remote changes, commits only its status heartbeat and pushes reviewed commits every five minutes. It never automatically commits product WIP or force-pushes. [repository-sync.json](repository-sync.json) and `/workspace/tmp/lsp-publication/last-success.json` record actual checkpoints, not test certification. Last inspected success was `2026-11-04T01:50:57.623887+00:00`, `2fd53e6a9a89404928237ea1fd58fc4b1eed24d1`. Real earlier gaps at 18:30–19:03 and 22:42–23:01 on 3 November remain gaps; remote divergence in the latter was reviewed and merged as `ec079dd0666b28f93209b8eea98d1f86e07f6600`.

## Validation actually observed

| Scope | Result and exact limit |
| --- | --- |
| Last completed selected unit cohort, V7 | Build passes, single Cargo job, pre/post source and generated correspondence unchanged. Selected controls **59/62 pass**. Compiler8/8, Core conditional3/3, foreign2/2, public opaque4/4 and producer2/2, server conditional9/9, policy suppression, factory/custody/allocation, disk read/retry and autoload14/14 pass. Three failures in that cohort are described below; subsequent source changes remain unexecuted. Not the full suites. |
| Current combined compiler/Core/server build, V10 | **Build passes**, all three test artifacts fresh, one Cargo job and source/prior-generated pre/post checks matched. Selected72 controls are executing. The original five-case conditional rename now passes; one new certificate adversary has failed and is being diagnosed. No all-pass cohort claimed. |
| Direct Syntax source extents | **5/5 pass**, fresh test artifact `7405df1976491dc027a66b872266e8020e3f012c4994629c942bc2170923e126`; source/prior-generated correspondence unchanged. Exact namespace `backslash::original_literal_extent_tests`. Initial wrong namespace aborted before tests; corrected selectors used the same freshly built artifact. |
| Native stdio hover | **15/15 pass** on freshly built server `8699c7193f32988d0bf7f8ccd50cbd978501b522264b97b5ffd32687a3bc8c27`, with bounded build and protocol source/binary checks. No editor/browser/WASI acceptance. |
| Native stdio conditional rename | **18/18 pass** on that server:17 explicitly configured Logical `tcl` cases and1 ordinary `tcl8.6` case. Checks capabilities, confirmation/versioned annotated edit, exact raw newline/BOM/Unicode application and adverse-source refusals. An earlier unconfigured harness failure remains preserved. Actual editor preview/UI remains open. |
| Latest compiler/Core/DB/server all-target Clippy request | **Fails on8 server lints**, after reaching server checking. `-D warnings`, no suppressions; source/prior-generated unchanged. The first helper build fails on an Arc registry argument mismatch; that plumbing is corrected before the next build. This is not a full workspace lint pass. |
| Fresh startup A/B | **Both baseline and candidate time out at60s before scan.ready.** Navigation requests were never reached. Installed Tcl tclIndex still causes five full analyses of6.34–6.93s; Tk analysis remains incomplete. No startup improvement or readiness accepted. |
| Earlier selected cohorts | Registry15/15; SpecTcl `pack_source_e2e`10/10; earlier Core30/31 and Core/Registry all-target lint passes remain bound to their own artifacts. They do not certify current source. |
| Required release gates | No final-current passing `make rust-check`, `make prep-pr`, global proof/drift, full native E2E, browser/WASI/editor, performance or packaging acceptance. |

V7 receipts: `/workspace/tmp/lsp-merged-cohort/conditional-build-v7/{before.json.gz,result.json.gz,cargo.jsonl,cargo.stderr}` and `conditional-controls-v7/results.json.gz`. The three failures are:

- Five-case original Logical rename control stops at `cash$name` in a legacy quote guard. Commit `9a3a183e7` removes that guard only from the conditional wrapper, retaining mandatory typed whole-container/generated-source checks and both ordinary-route guards. Current V10 unit execution is in progress.
- Closed-disk metadata fixture reads0 rather than1 because it installs an overlay only for `tcl`, while querying `tcl8.6`. Commit `7c2055520` uses the real all-profile installer. Exact V7 public probe demonstrates OverlayMiss before the walk and read1→withdrawal0 with production installation. Current V10 unit execution is in progress.
- Keyed-availability fixture incorrectly expects W150 while changing the primary configured version and declaring no target range. A reviewed purpose correction now explicitly configures15.0.0, requires actual W135, changes to21.1.0 and requires its absence, separately asserting no W150 at either point. Store/input/currentness and formatting assertions remain. Baseline native wire confirms this actual lifecycle route; current unit execution is pending.

The Core redundant borrow and ordered conflict/no-op helper extraction are adopted as `23a74a8dd`. The latest Clippy request now reaches eight server findings: four long functions, one misplaced import, one collapsible condition and two question-mark suggestions. Receipt `/workspace/tmp/lsp-merged-cohort/source-cohort-clippy-v3-result.json`, with complete JSON/stderr logs beside it.

Syntax receipt: `/workspace/tmp/lsp-merged-cohort/syntax-source-controls/result.json.gz`. Native build receipt: `/workspace/tmp/lsp-merged-cohort/server-source-after.json.gz`; hover `/workspace/tmp/integration-hover-acceptance-20261103/native-source-current/results.json`; wire `/tmp/conditional-wire-current/results/receipt.json`. The server predates the latest quote-guard/overlay/style changes and is not the final candidate.

Reused toolchains, dependency caches and undiscovered indirect runtime fixtures are outside the bounded build custody. V5 tried two Cargo jobs and both rustc processes were OOM-killed; no V5 test result was accepted. V6/V7 use one job. No full suite aggregate is inferred from incomplete, cancelled or load-competed runs.

## Open product blockers and active work

| Work | Current action and missing acceptance |
| --- | --- |
| Root integration | Review private changes, own the single build/target, execute current units and lint, maintain state and publication. Rebuild native server after coherent adoption. |
| Startup indexing, Sol6.1 + independent Terra6.1 review | Authentic typed no-contribution certificate for standalone tclIndex structural lookup. Adopted and compiled in V10; focused controls running. Exact unchanged-source comparison shows the fresh and full consumers both lose later nested loader schemas after unresolved `$dir`. No blanket skip-on-None, fake dir value, barrier weakening or partial-analysis shortcut is adopted. |
| Server lint, Sol6.1 | Adopted behavior-preserving helper extraction and trivial fixes for eight findings; execution pending. No new lint suppression. |
| Cross-file and CI triage, Sol6.1 | Closed source child disk ingress is corrected through the existing admitted disk publisher, with five unexecuted deterministic controls. Final-call suffix completion, source-view hover and rename coverage remain separate open blockers. No Native source-link fabrication. |
| Conditional rename | Selected Core/server and stdio checks pass within recorded cohorts; current five-case regression rerun and actual editor-host preview remain open. Ordinary runtime-complete Logical rename still refuses. |
| Foreign factory/source index | Narrowed source hover passes selected Core/server and stdio checks. Dispatch hover, cross-file definitions, controller/index recovery and source-reference coverage remain open. |
| Required gates | Full native E2E, compatibility consumers, browser/WASI/editor, performance, packaging, drift/proof, `rust-check` and `prep-pr` remain open. |

Startup receipt `/workspace/tmp/package-autoload-startup-ab-20261103/receipt.json`; fixed/installed-source comparisons `/tmp/autoload-actual-trace/compare-receipt.json`. The comparison runs identical configuration and unchanged original source across five releases. A no-contribution certificate must account for full-only conditional/scoped-body/catalogue routes and preserve fallback on uncertainty before it can be adopted.

The policy-only diagnostic receipt repair is adopted as `4b8870bc5` and its selected direct/drain suppression/currentness control passes V7. Enabled semantics still requires authentic input. Opaque script authoring is an inert-source purpose, not execution/body/frame authority; its six selected public/producer controls pass V7. The corrected readonly tests distinguish reported text from authentic Native producer values and exercise genuine Logical formal fields. Positive computed Native producer coverage remains open.

## CI and proof gates

Manual [CI41749944578](https://github.com/bitwisecook/tcl-lsp/actions/runs/41749944578) targets exact `8050f4d4c6477af73b42bcf69761805aa653cc6c`, created4November00:51:36UTC, before the latest quote-guard/overlay/style fixes. Latest inspected snapshot at01:50UTC:8success/11failure/9active/2skipped (30jobs). All three native VS Code partitions fail; five Rust shards, Heavy and three native E2E partitions remain active. No final aggregate is available. Rust-check's two Core lints are now corrected in Main but this run does not test those corrections. Web smoke fails ENOSPC extracting Playwright before build/tests. WASM/WASI closed/cross-file/rename cases fail. Runtime2300pass/104fail, not a runtime pass. Multi-folder passes. Triage `/tmp/ci-current-41749944578/triage.md`.

Older manual CI41736891101 on `b263…` finishes9success/15failure/4cancelled/2skipped. Its native partition3 tally616pass/238fail/5ignored combines server units and wire tests; it is not a wire-only tally or full Rust aggregate. Desktop partition2 is11pass/9fail and partition3 is319pass/39fail. Exact panic extraction `/workspace/tmp/ci-triage-current/E2E3-EXACT-PANICS-41736891101.md`. Remaining examples include W150 metadata premises, readonly-field references, callback references and pack-backed optimise behavior. Actual linked failures remain release obligations even when separate backend feature work is deferred.

The b263 global proof gate reports2,178 structural errors:1,204 evidence digest mismatches,973 census mismatches and1 hover conclusion mismatch. Conclusion wording parity is repaired without renewing evidence hashes or provider execution outcomes. The gate has no accepted reduced LSP profile. Historical assessment counts816mutable/425immutable/973census are a different snapshot, not a current classification or recount.

The existing transport-projection disclaimer/checksum file does not implement dual evidence identity. The canonical schema/checker still expects one digest of executed bytes. A representative original/projection pair is authenticated, but restoring it would reverse deliberate transport sanitisation. No private original is restored and no digest is blindly refreshed. Reconciliation needs individually verified provenance and a reviewed contract preserving original execution identity, current projection identity and protected semantic facts. Scoped source/helper results grant no Native/provider result.

## External-reporter issue sweep

The 3November sweep is recorded in [issues.md](issues.md#external-reporter-issue-sweep--3-november-2026). Original base `18e009bbc`, later rebased commits `4bfb8be40`/`b7336c76e`, their targeted controls and inherited failures remain source-specific. #2505's list-built `uplevel` with backslash-newline remains open. VS Code semantic-token guard and intermittent TclOO references require real host acceptance. Earlier local targeted and generator passes do not close the current full gates.

## Next acceptance work

1. Review/adopt the standalone tclIndex certificate and server lint repairs only when their boundaries are concrete. Complete current-source V10 controls and lint; retain the genuine failed adversary. The original five-case rename passes; closed-disk withdrawal is pending in the running cohort.
2. Build current native server, rerun fixed startup A/B through scan.ready and actual package/tclIndex/symbol/navigation requests. Repeat relevant wire checks and exercise editor-host preview.
3. Complete source/index/cross-file recovery and remaining diagnostic/compatibility premises; run required CI on the coherent published candidate. Keep failures and unexecuted checks explicit.
4. Pass required release gates and prepare the reduced PR to `rust`. Before merge, remove temporary state/process ledgers repository-wide per AGENTS, preserving reproducible oracle/source evidence. No release tag or publication is implied by development pushes.

Private drafts stay local. Earlier automatic rejection of exporting private unadopted archives was not bypassed; reviewed source/status pushes are authorised. Temporary duplicate copies and inactive binaries were removed only after scoped preservation checks. Earlier agent writes into Main were restored before scratch resync; their drafts were not compiled. Current builds use regular isolated Rust copies and one owned target. [Outstanding work](outstanding/README.md) mirrors this snapshot; historical continuation prompts do not override the instruction to complete the LSP.

Reviewed standalone schema abstention and server lint helper patches are adopted before green tests. The certificate remains purpose/source/input bound and tests retain absent-schema, altered query/channel, directives/barrier, malformed and Tcl8.4 encoding fallback controls. No startup or current lint pass is claimed. V7 exact executables are preserved in verified compressed archives before replacement; `/workspace/tmp/lsp-merged-cohort/v7-elf-preservation.json` records byte hashes and raw retirement.

V8 combined build stops on one server helper type mismatch (`&CommandRegistry` versus the retained `Arc` needed by its worker). Pre/post source/prior-generated checks match; no test executable/result is accepted. Correction `449b59013` passes the original Arc into that helper. Receipt `/workspace/tmp/lsp-merged-cohort/conditional-build-v8/result.json.gz`. Availability/formatter purpose correction `675d440da` and hosted web SDK-space cleanup `86dfba725` are adopted before execution; no hosted capacity recovery is yet observed. Closed-child ingress uses real disk admission/publication and the inherited cut, independently reviewed; actual final-main and hover failures remain open.

V9 combined build stops only on two new test assertions requiring Debug for SourceFile (reported as four trait errors). Production server compilation passes. Those assertions now retain exact identity equality without Debug. No new unit result is accepted. V9 source/prior-generated checks match; receipt `/workspace/tmp/lsp-merged-cohort/conditional-build-v9/result.json.gz`. Root workspace cargo fmt --all -- --check passes at this source checkpoint; full lint/gates remain open.

Current V10 build receipt `/workspace/tmp/lsp-merged-cohort/conditional-build-v10/result.json.gz` passes with three fresh unit executables and unchanged source/generated correspondence. The running72-control receipt is `conditional-controls-v10/partial-results.json.gz`; its new rename-prefix adversary over global survey availability failed, so no all-pass result is asserted. Actual certificate issuance at the barrier is being checked before deciding whether implementation or fixture purpose needs repair. The complete installed public Compiler probe passes15/15 Tcl/Tk/BLT × release cases with all negative schema sites authenticated and exact schema fingerprints equal; `/tmp/autoload-installed-v8-probe/{summary.json,final-source-artifact-seal.json}`. That uses sealed V8 production rlibs and does not establish Core registration-vector or startup acceptance.

A private definition-only suffix experiment has independent static review but is uncompiled/unadopted. It would require typed cursor ownership in every relevant incomplete scope and would retain no-completion, reference, rename and command-world refusals. A separate completed-source hover design is private. V10 executable bytes are verified in compressed preservation receipt `v10-elf-preservation.json` before any replacement; raw executables remain until controls finish.
