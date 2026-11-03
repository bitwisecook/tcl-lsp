# Current LSP naming migration state

Updated 03 November 2026, 22:43 UTC. This is the active snapshot. Earlier results remain historical in git and their original receipts; they do not certify the current release candidate.

## Scope and release verdict

**The LSP is not release-ready.** The user requires the LSP and its transitive libraries to be complete. Separate VM, BIG-IP and ancillary feature development is deferred. Linked SpecTcl/VM/offline parsing dependencies remain in scope. Required gates are not waived, and conditional source advice supplies no Native/provider execution authority.

The checkout is `/workspace/tcl-lsp-partial`, branch `work`, in a shallow clone of `https://github.com/bitwisecook/tcl-lsp.git`. The user explicitly approved pushing to that configured origin. Reviewed changes and truthful status are published before green tests, as requested. No PR, merge, tag or release exists.

The five-minute publication process fetches remote `work`, checks ancestry, commits only the status heartbeat and pushes reviewed HEAD commits. It does not automatically commit product WIP. Last observed success: 3 November, 22:02:53 UTC, `4cfd9e6fca2d344bf94e2010dd1a587644595c3f`, no remote divergence. [repository-sync.json](repository-sync.json) records actual subsequent checkpoints and the separately timestamped semantic snapshot hash; it certifies no tests. The earlier 18:30–19:03 interruption remains a real publication gap, not a continuous cadence claim.

## Validation actually observed

| Scope | Result and practical limit |
| --- | --- |
| Latest completed owned Core cohort | **30/31 pass**, fresh binary `7db7caba01bb279a3f7cdf90d7ae3d90ad9ba4c9d30f4f326afb7a3aed559788`. All 31 selectors executed with 6,730 source/input paths, 44 generated files and binary unchanged. Ordinary Logical literal-name rename is the remaining failure. The prior 29/31 and 27/31 cohorts remain historical. |
| Additional current Core controls | **8/8 hover, 3/3 package-prefilter, 2/2 extra math pass**, same binary and 6,730 input/44 generated seals unchanged. The authentic mixed Global/Local fixture passes, as do all Unicode, original-root, declaration/index and expansion-child controls. These are selected controls, not the whole suite. |
| Latest completed native hover protocol cohort | **11/15 pass**, server `efb07a09b1f71c1338da2f3e67a46ee35116e30a84f1571041bc7effb9af07f3`, fresh all-features build in 356 seconds with unchanged inputs. Exact ASCII SpecTcl and scoped negatives pass; Unicode-before/same-line UTF-16 and global/rooted special-variable cases fail. Predates the latest repairs; not browser/WASI acceptance. |
| SpecTcl evaluator fixture correction | Adopted as `a413f3111`. Exact current baseline reproduces both failures. Candidate full `pack_source_e2e` target **10/10 pass**, retaining twelve derived-body shapes, release rows, unknown/frame refusals, no-host controls and four native ASCII/Unicode lifecycle tests. Its original dependency graph predates the later Unicode compiler patch; not final merged compatibility acceptance. |
| Unsuppressed SpecTcl target Clippy | The earlier SpecTcl target attempt failed on Core lints. After import/renderer/wrapping fixes and one local similar-name fix, current Core/Registry **all-target Clippy passes `-D warnings` with no suppression**, unchanged source/generated seals. This is not full SpecTcl/workspace lint or the release gate. |
| Current merged source-card cohort | Fresh Core build passed in 4m32s with Cargo `fresh:false`, 6,730 inputs/44 generated unchanged, including external 4bfb and adopted package prefilter. Results above bind this cohort; current native-server protocol and performance measurements remain pending. |
| Registry controls and lint | Ten exact Registry controls passed before the later test/hover changes. Registry/Core all-target Clippy and formatting passed after the 93-file Rust1.99 test-assertion cleanup, before subsequent source changes. Four corrected Native compiler fixture tests still need exact execution. No full Registry-suite pass. |
| Loop completion / O101 / other adopted features | Scoped producer, unit and protocol controls pass within their recorded cohorts. Loop dict-for/foreach k/v candidates produce four correct applied edits. O101 actual quick-fix passes with fixed settings/factory intent. These are bounded results, not complete E2E or host acceptance. |
| Full native E2E | Earlier six-thread run stopped after 251 results with 58 marked failures under competing compile load. Neither a complete suite nor 58 confirmed semantic regressions. Complete uncontended current-source E2E remains open. |
| Required release gates | No final-current passing `make rust-check`, `make prep-pr`, global proof/drift, browser/WASI/editor, performance or packaging acceptance. |

Current Core receipts: `/workspace/tmp/lsp-merged-cohort/package-current-{before,after}.json.gz`, `package-current-core31.json.gz`; extras `/workspace/tmp/integration-hover-acceptance-20261103/core-package-current/results.json` and `core-math-package-current/results.json`; prefilter `/workspace/tmp/package-index-negative-prefilter/core-product-current-7db7caba/results.json`; Clippy `package-clippy-fixed-result.json`. The earlier 29/31 source-card receipts and binary are separately held in `/workspace/tmp/proof-lsp-scope/core31-source-card-held-20261103/`. The earlier 27/31 receipts are `hover-{before,after}.json.gz` and `hover-core31.json.gz`. Failed chain and all three extra logs are preserved in `/workspace/tmp/proof-lsp-scope/core31-hover-held-20261103/`; exact binary gzip is `/tmp/lsp-core31-hover-held-3ec62c3f.gz`. The raw 252 MB test executable and inactive 533 MB Core libraries were retired only after verified compressed preservation and restoration receipts. The older24/31 failed cohort is separately retained; neither failed cohort's proof pins were refreshed as passing.

Native protocol and extra-control receipts: `/workspace/tmp/integration-hover-acceptance-20261103/`. SpecTcl baseline/candidate/lint/Native-probe receipts: `/workspace/tmp/spectcl-current-cohort/RESULTS.md`. Reused toolchain/dependency caches and untraced indirect runtime inputs remain outside the bounded build custody.

## External-reporter issue sweep

The 18 open issues not opened by the maintainer (#2021 is excluded and handled separately) were reproduced on base `18e009bbc`; the commits were then rebased onto the later `work` tip without conflicts in code. The results below are from that base and have only been re-checked after the rebase where stated. Per-issue results are in [issues.md § External-reporter issue sweep](issues.md#external-reporter-issue-sweep--3-november-2026).

- **Fixed with regressions:** #2506 (T100 on a comparison result), #2503 (W114 inside a quoted operand), #2499 (W212 on `[set $name]`), #2497 (`my` from a superclass now references a subclass method), #2289 (W140 after a refused creation proof) and #2507 (W100 `scan` suggestion).
- **Guards only:** #2288 (W105 `+[list …]`) is guarded but not observable, because W105 is not emitted on `work`. #2181 changes the semantic-token refresh guard; its e2e tests pass, but it has not been checked in a VS Code host.
- **Already correct on `work`, regression added:** #2265, #2242, #2501, #2502, #2498, #2500 and #2504 (on the production shimmer path), and #2286 (its three tcltest tests pass here).
- **#2290:** the current upstream file is clean; no fixture.
- **Residual (still open):** #2505 inside a proc body. A list-built `uplevel` script whose brackets contain a backslash-newline still draws W301, because the nested `list` invocation has no recorded original binding. This is open.
- **Shared-owner changes:** `BinOp`/`UnaryOp::yields_boolean` (`tcl-syntax`), `OriginalExpressionCommandRegion::whole_operand` (`tcl-syntax`), and set-literal typing through the shared numeral parser (`Inf`, `-Inf` and `1e3` are `Double`).

Validation for this sweep:

- **Targeted compiler tests:** 14 issue tests plus the `tcltest` controls pass.
- **Compiler suite comparison:** the `fp::sh`, `type_infer`, `compilation_unit::tests::return_type`, `fp_sty_14` and `w212` suites were compared against a pristine stash of the same tree. There are no new failures. These tests already failed before the sweep: 52 in `fp::sh`/`type_infer`/`return_type`; four in `fp_sty_14` (W105 never fires); `w212_covers_registry_name_positions`; `nested_expression_advice_uses_actual_renamed_descriptor…`; and `lens_matches_peek_for_inherited_method_with_subclass_my_dispatch`.
- **tcl-lsp-core:** both new tests pass.
- **Server e2e `spec_packs::`:** 23 pass and 12 fail with `--test-threads=2`. The pristine baseline has the same 11 failures (outline, folding, trust, UPF hooks, notices and others). The extra failure, `a_pack_added_after_startup_reloads_and_re_analyses_open_documents`, passes twice when run alone, so it is load-flaky. The new #2242 test passes.
- **S1xx shimmer diagnostics are emitted:** the earlier claim that the CLI emits none was wrong; the cases I used as positive controls are not shimmers under the current rules. With the CLI, `set s hello; expr {$s + 1}` and `set s [string trim "5"]; expr {$s + 1}` each draw S100, and a loop that alternates a string and a list draws S101 and S102. The #2498 and #2500 repros draw no S1xx through the CLI (#2504's repro produces one non-shimmer diagnostic).
- **W105 is still not emitted** for quoted, composite or `bind` bodies (four `fp_sty_14` true-positive tests fail). Not yet diagnosed: the debug tracing in `dispatch_body_arguments` was stopped before it ran.
- **Find References:** results from a debug server are intermittently empty for TclOO methods, in the same-class control too.
- **Gates run for this sweep:**
  - `cargo fmt --check` passes.
  - `cargo clippy --all-targets -D warnings` passes for `tcl-syntax`, `tcl-core-types`, `tcl-compiler`, `tcl-lsp-core` and `tcl-lsp-server`.
  - The `diag-tables`, `gen-ai-diagnostics`, `gen-vscode-package`, `gen-editor-catalogs`, `gen-editor-settings` and `gen-jetbrains-catalog` drift checks pass after regenerating for the W212 catalogue text.
  - `cargo xtask owner-resolution` fails with the same 83 lines before and after this change.
  - `make rust-check` and `make prep-pr` were not run in full.
- **Re-checked after rebasing onto the newer `work` tip:**
  - The 16 targeted compiler tests, both tcl-lsp-core tests and the three spec-pack e2e tests (#2242, plus the two semantic-token tests #2181 touches) pass.
  - All six drift generators are in sync, and rustfmt is clean.
  - Clippy finds nothing in `tcl-syntax`, `tcl-core-types` or `tcl-compiler`. The rebased tip's own `tcl-lsp-core/src/hover.rs:897` fails `needless_pass_by_value`; that code is not part of this change.

## Adopted repairs awaiting merged acceptance

- `5a3cdeaa1` retains a typed Logical root-source reference for special-variable documentation. It requires the genuine current occurrence/root coordinate and available Registry row, and has empty declaration/reference projections.
- `f0e0abc36` permits a positively authenticated conditional Registry card after an unavailable method lookup. A selected method remains terminal; known custom/deleted command barriers and visible runtime-applicability obligations remain. The exact public probe established that the old method terminal blocked an otherwise valid retained `string length` schema. Current execution also exposes a new test-premise issue: after an unknown call, the later `proc`/`rename` itself has only Conditional schema applicability. Authenticated mutations before the unknown call still block the card. Independent review confirmed the fixture premise: `proc`/`rename` after an unknown call is itself conditional. The negative fixtures now place those retained mutations before the unknown call, and the post-unknown cases explicitly require conditional-schema provenance and unresolved-availability text. Rerun pending.
- `3d22a13e9` preserves static Unicode bytes only on DefinitionMember arguments, retaining ASCII heads, Scoped behavior, counted zero/dynamic/expanded facets and the independent source-body geometry owner. Its Native root-card factory requires a genuine original read. Unqualified names require every nonempty original frame alternative to be Global/known-root; authentic absolute qualification has its own input owner. A peer-found merged-frame error was fixed before adoption. Current positive/adversarial source-card controls pass except the separately identified mixed-frame fixture, which fails before reaching its intended guard.
- `a413f3111` corrects the SpecTcl test claim to the installed declared Evaluate implementation. An unentered original procedure's execution SCCP remains Overdefined. It supplies no executable store, erasure receipt or restored LSP computed-value advice. The initial const-fold-only rewrite was invalid and remains unadopted.

Other adopted work includes incomplete-expression math completion, bounded literal expression operator actions, variable display labels with exact edit/filter keys, O101 producer settings/factory intent, genuine factory source advice and fixed-positional Logical member bindings, Registry fixed-prefix/role corrections, parser expectation fixes and a pure immutable release-ladder cache. Their individual contracts and retained receipts remain the scope of their passes. Variable/nonempty-body operator edits and Jim recursive-factorial/regexp-list compaction remain unsupported where observer/effect closure is absent. No speed claim follows from the release-ladder cache.

## Open product blockers and active ownership

| Owner | Actual open work |
| --- | --- |
| Root | Controlled integration/builds, state/publication, required CI, full E2E, host/editor, performance, packaging and release PR preparation. |
| integration_resume — Sol 6.1 | Current hover checks; independent review of conditional source rename and closed-source byte custody. |
| logical_rename_resume — Terra 6.1 | Ordinary Logical rename still refuses complete coverage. Actual probe observes uncertainty from both initial `set` and later `puts` substitution. A separate compiler/Core/server conditional source-edit draft with confirmation-required annotations is unbuilt and under review. It must not silently grant ordinary runtime-complete rename. |
| factory_resume — Terra 6.1 | Foreign factory declaration-site source hover through authentic caller/provider/member parents and lazy census. Frozen private draft is uncompiled/unrun. Dispatch hover, cross-file definition and chained provenance remain unresolved. No foreign header geometry is treated as provider selection or Normal completion. |
| recovery_resume — Sol 6.1 | Closed workspace-symbol exact saved-byte custody and joint mapping/source leases; peer follow-ups preserve raw-read pause controls and deduplicate only identical admission/byte allocations. Draft and factory composition are unbuilt. Durable index recovery/controller and full publisher/reader lifecycle remain unfinished. |
| compiler_resume — Sol 6.1 | Authentic Native body/store custody. A bounded real-entry probe stopped before selecting an Implementation route and establishes no Native positive. Old synthetic source-procedure normal-store fixtures and the actual compiled-body adapter need correction without manufacturing a declaration card. |
| proof_release_resume — Sol 6.1 | Reviewed proof metadata and negative-only package-index preflight. Six math source-slot repairs are held until new matching-source controls; Registry catalogue control is unexecuted. Prefilter first diagnostic correctly refused a malformed fixture; corrected draft is not validated. |

The current server independently reproduces slow startup package discovery: `/usr/lib/blt2.5/pkgIndex.tcl` takes 9845 ms for one Tcl profile and is repeated across profiles. A 45 second probe did not reach scan readiness; this is not attribution of the browser failures or evidence of deadlock. Partial-analysis and singleflight prototypes remain held for incomplete custody/progress semantics. No latency benefit is claimed.

Computed derived SpecTcl values remain unsupported by the existing LSP source-value consumer, which rejects command substitutions. A hypothetical evaluator answer cannot be inserted into its bare-string path while discarding observer/completion/effect obligations.

## CI and proof gates

Latest inspected manual [CI run 41701183374](https://github.com/bitwisecook/tcl-lsp/actions/runs/41701183374) targets exact `3296d2e53a7d7616d47c1c746a79a5fb259feaf8`, before subsequent product fixes. At the 22:08 UTC reinspection: channel, cargo-deny, doctests, native build, E2E archive, web-extension, web-frontends and Python jobs pass. Rust-check, heavy tests, Rust shards 2/5, 3/5 and 4/5, browser/WASI functionality, SpecTcl compatibility, runtime, real-Wasm-link and aggregate PR gate fail. Native E2E partitions 1/3 and 3/3 and desktop partitions 1 and 2 also fail. Rust shards 1/5 and 5/5, E2E partition 2/3, desktop partition 3 and multi-folder were unfinished at that inspection. No green result or required PR-event acceptance follows. Earlier [run 41669761498](https://github.com/bitwisecook/tcl-lsp/actions/runs/41669761498) targets `e52d4795c6748d378adb09e37a661651957201c1` and is historical.

Browser failures include property hover, closed symbols, tclIndex definition and incomplete rename coverage; WASI cross-file/rename cases fail. Native A/B rename demonstrates owner sensitivity but returns a different error from CI, so it is not the same-cause proof. Multi-folder diagnostics can return `CurrentDiagnosticsUnavailable: context`; the earlier server-wide liveness loss remains unattributed. Real-Wasm-link/runtime checks remain failed and cannot be labelled passed because separate backend development is deferred.

Six reviewed math source-role snapshots were adopted as `7a24af898`, supported only by seven exact sealed Core software controls and preserving the complete failed cohort context. Native/provider statuses remain not-tested. The later synced external semantic-token source change and hover borrowing cleanup require renewed snapshots where hashes changed; this is not a current global-gate pass.

The proof gate has **no accepted reduced LSP release profile**. Assessment `/workspace/tmp/proof-lsp-scope/README.md` counted 816 mutable source-evidence slots, 425 immutable appliance mismatches and 973 census mismatches; these are assessment-snapshot counts, not a new recount. Mutable pins need inspected contracts and relevant passing controls. Immutable evidence requires authenticated recovery/new capture or explicit reviewed disposition. No blind digest refresh, unknown-record waiver or source-only-to-Native/provider transfer is permitted. The last complete audit failed; focused metadata fixes do not close the global gate.


The reviewed negative-only package-index prefilter is now adopted before merged execution. It skips the unchanged original advertisement builder only when no canonical Registry target can supply its required static head/name/version/loader shape on the same original vector; malformed/expanded/uncertain regions and budgets preserve the full path. The v6 frozen-extern diagnostic passes all five releases, preserving literal/alias/cooked-head/version cases and matching the unchanged full consumer for cooked script, dynamic-shaped index and nonempty literal prerequisites. Actual retained BLT was excluded by the prefilter; its full-builder result and startup timing are not yet measured. Evidence: `/workspace/tmp/package-index-negative-prefilter/attempt9-catalogue-v6-pass/`. These diagnostic dependencies predate the newly synced external commits, and product wrapper/unit/lint acceptance remains pending.

Remote commits `4bfb8be40` and `b7336c76e` were fetched and fast-forwarded by the publication process at 22:38 UTC. Their issue sweep and scoped validation limits are retained above. The pending merged build must include those changes. Its reported `hover.rs` needless-pass-by-value lint is addressed by borrowing the unchanged navigation selection. No final-current lint result is claimed.

Two overbroad private scratch-copy attempts temporarily exhausted storage after the environment restart. Those duplicate copies were removed, and inactive diagnostic binaries were preserved with verified compressed hashes before raw retirement. No Cargo build or test was running during the exhaustion; all earlier results retain their original pre-incident custody. Available space was rechecked before new builds.

## Next acceptance work

1. Execute the fresh Core cohort and new hover controls, then unsuppressed lint and current native applied-edit/protocol checks. Keep every failure explicit and repair it at its authentic owner.
2. Review and test coherent rename/factory/source-currency changes; complete source/index recovery and required cross-file/editor behavior. Independent partial drafts are not feature acceptance.
3. Re-run required CI on the candidate; complete native E2E, browser/WASI/editor scenarios, cancellation/latency/memory, packaging, proof/drift, `rust-check` and `prep-pr`.
4. Prepare the reduced PR to `rust` only after the required gates. Remove temporary state/process ledgers from the shipping tree per AGENTS; preserve reproducible source/oracle evidence. No merge/tag/publication is performed by these development pushes.

Unadopted drafts stay local. Automatic review rejected exporting private draft archives earlier; that separate rejection was not bypassed. Reviewed code/status pushes remain authorised. Canonical proof/provider outcomes remain in the canonical corpus. [Outstanding work](outstanding/README.md) and this snapshot supersede stale role handoffs and old STOPPED headers.

Publication paused after 22:42 UTC because remote `work` diverged from local product commit `ff22beef9`. The daemon recorded refusals at 22:47, 22:52 and 22:57 and did not force-push. Root is merging the concurrent documentation correction and publishing both histories. This is a real gap in the requested cadence.
