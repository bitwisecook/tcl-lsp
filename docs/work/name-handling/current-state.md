# Current LSP naming migration state

Updated 04 November 2026, 03:17 UTC. This is the active snapshot. Older results retain their original sources and receipts in git; they do not certify the current candidate.

## Scope and release verdict

**The LSP is not release-ready.** The goal is the LSP and the libraries it uses, migrated to Tcl name resolution. Separate VM, BIG-IP and ancillary feature development is deferred; linked dependencies and mandatory gates remain in scope. Source documentation and conditional navigation grant no Native execution, handler, complete command-world or edit authority.

The checkout is `/workspace/tcl-lsp-partial`, branch `work`, a shallow clone of configured origin `https://github.com/bitwisecook/tcl-lsp.git`. The user approved reviewed code/status pushes before green tests. No PR, merge to `rust`, tag or release has been created.

The publisher fetches remote `work`, checks ancestry, safely fast-forwards new remote commits, commits only its status heartbeat and pushes reviewed HEAD every five minutes. It neither commits product WIP nor force-pushes. Last inspected success: `2026-11-04T02:40:57.689118+00:00`, `1c3c766cea255ec4b32f8aa724d4541b68950b82`.636547+00:00`, `29a478cd284860b7f5f3b5ab54bcd2a6dd02185a`. [repository-sync.json](repository-sync.json) records actual subsequent checkpoints, not tests. Real earlier cadence gaps at18:30–19:03 and22:42–23:01 on3November remain gaps; the latter remote divergence was reviewed and merged as `ec079dd0666b28f93209b8eea98d1f86e07f6600`.

## Adopted current source

- Closed source children enter through the existing admitted disk publisher before an inherited interpretation is captured (`dfd609048`, test assertion correction `b605b8eef`). Physical SourceFile/admission identity, current parent cut, changed/deleted withdrawal and unsaved buffers retain their owners.
- Definition-only partial suffix navigation (`18742cb29`) follows a successful private experiment. Every relevant incomplete scope must supply an actually issued cursor-owning region; unique target and uncertainty guards still apply. Scope completion, references, rename and command-world gates are unchanged. Missing normal completion is never manufactured.
- Standalone autoload schema abstention (`fe925d8f7`) retains exact whole source/input/settings/purpose and full-consumer route checks. The new authored command-table-mutation guard (`90782b1ca`) only withdraws that proof. No blanket None-skip, fake dir value, barrier weakening or partial AnalysisResult is used.
- Server lint helper extraction is adopted with the actual retained registry Arc (`3d1b206de`, `449b59013`). Clippy v6 reaches server tests and fails9 test-only findings. Reviewed fixes are adopted as `d90b82b4e`; v7 is running. No complete lint pass is claimed.
- Primary-version fixture purpose is corrected (`675d440da`): explicit15.0.0 requires W135,21.1.0 clears it, neither declares targets or emits W150. Store/input/currentness assertions remain. Formatter fixture now explicitly requests single-command body expansion (`22db36ad1`), preserving its output assertions and production defaults.
- Hosted web CI frees unused Android/.NET SDK space before dependency/cache restore (`86dfba725`). Syntax/structural checks pass; hosted capacity recovery remains unobserved.

## Validation actually observed

| Scope | Result and practical limit |
| --- | --- |
| Last complete Main unit cohort, V10 | All3 compiler/Core/server test executables freshly built, one Cargo job, source/prior-generated pre/post checks matched. Selected **70/72 pass**. Original five-case Logical rename, compiler/Core/server conditional controls, opaque controls, readonly fields, policy suppression, foreign factory/custody, autoload controls, primary-version availability, closed metadata withdrawal and all5 new closed-child lifecycle controls pass. Not full suites; source predates the three latest adopted repairs. |
| V10 failures | Rename-prefix preflight remains available after authored command mutation; generic Registry-effect guard now adopted, rerun pending. Formatter fixture expected expansion despite inline defaults; explicit style now adopted, rerun pending. Neither failed result was relabelled passed. |
| Private suffix experiment | Build passes with **exactly3 intentional scratch/Main source differences**, separately pinned; no false Main-correspondence claim. Selected **12/13 pass**: both new full-fixture/unknown-body controls, all5 closed-child controls and5 existing Core controls pass. Remaining existing Core failure reproduces on restored V10 binary. The definition-only production patch is now adopted; the private host-index module is removed from scratch by normal resync. |
| Actual installed autoload vectors | **15/15 pass**, Tcl/Tk/BLT ×5 releases, exact original-name-input/path vectors.13 cases accept fresh survey and match unchanged full consumer; Tcl/Tk8.4 decline because installed scripts use unsupported source encoding and independently match integrated fallback. Candidate source, external private Rust module, dependencies, binaries, original files and generated outputs unchanged. Diagnostic experiment, no Native startup result. |
| Installed public Compiler schemas | Earlier **15/15 pass**, all negative sites authenticated: Tcl219, Tk731 and BLT7 per profile. V8 production rlibs and794 Main Compiler files unchanged. V8 overall build failure remains explicit. This is schema evidence, separate from actual Core vectors above. |
| Existing Core operand-horizon failure | Restored exact V10 ELF also fails the Unicode prior-puts positive. Public80-case matrix across5 releases confirms retained call/declaration plus genuine earlier puts uncertainty, ASCII and Unicode alike; nested-list/later-operand positives pass, six negative controls refuse,75 foreign-owner/stale checks refuse. Native Unicode cooked CESU-8 versus document UTF-8 is correct. Fixture-purpose repair remains private; no production debt guard is weakened. |
| Syntax source extents | **5/5 pass**, fresh artifact `7405df1976491dc027a66b872266e8020e3f012c4994629c942bc2170923e126`, source/prior-generated checks matched. Initial wrong selector namespace aborted before tests; corrected selectors ran on the same fresh binary. |
| Last native stdio artifact | Server `8699c7193f32988d0bf7f8ccd50cbd978501b522264b97b5ffd32687a3bc8c27`: hover15/15 and rename18/18 pass (17 explicitly configured Logical tcl cases plus ordinary tcl8.6). Exact annotated/versioned/confirmation edits and raw newline/BOM/Unicode application checked. This artifact predates recent repairs; actual editor UI/preview remains open. |
| Startup | Last baseline/candidate native A/B **both time out at60s before scan.ready**, so requested navigation checks were not reached. Installed vector timings show fresh Tcl roughly2–2.3s and Tk7.6–10.2s per admitted profile;8.4 Tk fallback alone takes about25s. These are diagnostic stages, not accepted startup/performance measurements. |
| Earlier selected cohorts | Registry15/15, SpecTcl pack_source_e2e10/10 and earlier Core/Registry all-target lint passes remain source-specific historical results. No full current workspace/gate pass follows. |

Current receipts: `/workspace/tmp/lsp-merged-cohort/conditional-build-v10/result.json.gz`, `conditional-controls-v10/results.json.gz`; `/workspace/tmp/lsp-merged-cohort/suffix-experiment/{plan.json.gz,result.json.gz,controls-v2/results.json.gz,actual-index-vectors/result.json.gz}`; `/workspace/tmp/lsp-merged-cohort/v10-nested-navigation-baseline/receipt.json`; `/workspace/tmp/lsp-merged-cohort/nested-navigation-probe/`; `/tmp/autoload-installed-v8-probe/{summary.json,final-source-artifact-seal.json}`. Current Clippy logs/result prefix: `/workspace/tmp/lsp-merged-cohort/source-cohort-clippy-v4`.

Native receipts: `/workspace/tmp/lsp-merged-cohort/server-source-after.json.gz`, `/workspace/tmp/integration-hover-acceptance-20261103/native-source-current/results.json`, `/tmp/conditional-wire-current/results/receipt.json`; startup `/workspace/tmp/package-autoload-startup-ab-20261103/receipt.json`. Initial unconfigured wire expectation failure remains preserved. Reused toolchains, dependency caches and undiscovered indirect runtime inputs remain outside bounded build custody.

## External-reporter issue sweep

The 18 open issues not opened by the maintainer (#2021 is excluded and handled separately) were reproduced on base `18e009bbc`; the commits were then rebased onto the later `work` tip without conflicts in code. The results below are from that base and have only been re-checked after the rebase where stated. Per-issue results are in [issues.md § External-reporter issue sweep](issues.md#external-reporter-issue-sweep--3-november-2026).

- **Fixed with regressions:** #2506 (T100 on a comparison result), #2503 (W114 inside a quoted operand), #2499 (W212 on `[set $name]`), #2497 (`my` from a superclass now references a subclass method), #2289 (W140 after a refused creation proof) and #2507 (W100 `scan` suggestion).
- **Guards only:** #2288 (W105 `+[list …]`) is now observable, since W105 is emitted again. #2181 changes the semantic-token refresh guard; its e2e tests pass, but it has not been checked in a VS Code host.
- **Already correct on `work`, regression added:** #2265, #2242, #2501, #2502, #2498, #2500 and #2504 (on the production shimmer path), and #2286 (its three tcltest tests pass here).
- **#2290:** the current upstream file is clean; no fixture.
- **#2505 proc-body residual fixed.** A braced body whose value differs from its bytes only by the lexer's backslash-newline fold now keeps a contiguous document mapping (`ExecutedScriptSource::from_word` / `from_original_word_value`, decided by `collapse_braced_word`), instead of becoming a separate materialised origin. Bindings inside such a body are therefore recorded at document offsets, so the nested `list` call is proven and the canonical-list W301 exemption applies.
  - Parsing the authored bytes substitutes each continuation exactly as the fold does, with one exception: a `${…}` name is taken verbatim, so a body containing `${` stays materialised.
  - Changed expectations: the operand-layout test now expects a Native operand, read as `local name`. Three mapped-source tests and the derived-geometry fixture add a `${…}` comment to stay materialised. The `brace_continuation_mapping_obeys_the_actual_word_policy` and `original_body_word_extent…` tests assert the new mapping.
- **Shared-owner changes:** `BinOp`/`UnaryOp::yields_boolean` (`tcl-syntax`), `OriginalExpressionCommandRegion::whole_operand` (`tcl-syntax`), and set-literal typing through the shared numeral parser (`Inf`, `-Inf` and `1e3` are `Double`).

Validation for this sweep:

- **Targeted compiler tests:** 14 issue tests plus the `tcltest` controls pass.
- **Compiler suite comparison:** the `fp::sh`, `type_infer`, `compilation_unit::tests::return_type`, `fp_sty_14` and `w212` suites were compared against a pristine stash of the same tree. There are no new failures. These tests already failed before the sweep: 52 in `fp::sh`/`type_infer`/`return_type`; four in `fp_sty_14` (W105 never fires); `w212_covers_registry_name_positions`; `nested_expression_advice_uses_actual_renamed_descriptor…`; and `lens_matches_peek_for_inherited_method_with_subclass_my_dispatch`.
- **tcl-lsp-core:** both new tests pass.
- **Server e2e `spec_packs::`:** 23 pass and 12 fail with `--test-threads=2`. The pristine baseline has the same 11 failures (outline, folding, trust, UPF hooks, notices and others). The extra failure, `a_pack_added_after_startup_reloads_and_re_analyses_open_documents`, passes twice when run alone, so it is load-flaky. The new #2242 test passes.
- **S1xx shimmer diagnostics are emitted:** the earlier claim that the CLI emits none was wrong; the cases I used as positive controls are not shimmers under the current rules. With the CLI, `set s hello; expr {$s + 1}` and `set s [string trim "5"]; expr {$s + 1}` each draw S100, and a loop that alternates a string and a list draws S101 and S102. The #2498 and #2500 repros draw no S1xx through the CLI (#2504's repro produces one non-shimmer diagnostic).
- **W105 fixed.** The body walk publishes body roles only when the call is proven to execute, so an unset operand (`eval "do $script"`), an unproven package (`bind` before Tk is proven) or a clause-grammar body (`if {$c} "puts $c"`) got no W105. W105 is now also emitted from the original source schema, the owner W100 and W301 use. It covers `authored_source_script_arguments` plus clause-plan bodies (the inert reading when a computed word could be a keyword); a concatenating command is judged by its first word only. The emitter skips a span already reported. The four `fp_sty_14` and `analyse_emits_w105_for_unbraced_if_body_with_substitution` tests now pass, and #2288's guard is observable: its test has true-positive controls. Compared against a pristine baseline over the `fp::sty`, `fp::inj`, W101/W105/W301, `eval_` and `uplevel` filters, there are no new failures. The 102 failures in that subset predate this change.
- **Find References race fixed.** A request arriving right after `didOpen` was answered empty, or with "conditional source advice … is unavailable", in about a third of debug-server runs, the same-class TclOO control included. Two transient refusals were reported as final answers:
  - The source-view settlement reads its table with `try_lock`, so the publication that follows `didOpen` refused it. `refresh_original_source_view_graph` now retries the gated settlement up to 40 times at 25 ms, and the navigation route retries a refused settled-scope read. A refusal that persists is still terminal.
  - `rehomed_index_guard` refused a settlement whose views had not yet reached the workspace index. It now waits within the same bound, releasing the rehoming gate between checks.
  - Result: 16/16 immediate requests return both references, for the #2497 subclass case and the same-class control (previously 2/6 and 1/3 failed).
  - Compared against a pristine baseline over the server's `source_view`, `rehom`, `references` and `readonly` tests: no new failures (36 were already failing).
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

## Active work and remaining product blockers

| Work | Current action and missing acceptance |
| --- | --- |
| Root integration | Current-source unsuppressed all-target lint, native rebuild and actual startup/wire checks, unit reruns, state/publication, required gates and packaging. Single Cargo build job/owned target; V5 two-job OOM is preserved as a resource failure, not a test result. |
| Hover, Sol6.1 with Terra6.1 review | Reviewed successor adopted as `c1cec1199`, correcting earlier private regressions and preserving positive-applicability routing. Fresh native4068 builds; unchanged protocol15/15 and targeted18/19 pass with complete pre/post seals. Exact full final-main closed-child definition fails; procedural hover also remains unavailable. All other baseline cards/custody checks pass. Detailed first-guard diagnosis is private. |
| Autoload, Sol6.1 | Investigating a separate typed no-contribution proof for unavailable filename argument grammar, especially8.4 source encoding. Existing fallback assertion remains until new proof and adversarial controls justify evolution. No source-name string dispatch or runtime/dir fabrication. |
| Core navigation fixture, Sol6.1 | Preserve original prior-puts debt as explicit negative; choose a verified debt-free Unicode positive without changing production uncertainty rules. Fixture/prose correction adopted as `ad4567bd9`; its extracted public-API module passes3 tests across5 releases, while current Main Cargo rerun remains pending. |
| Broader LSP stack | Source-reference coverage, complete cross-file/editor/WASM/WASI behavior, command rename coverage, computed Native producer positives, compatibility hooks and other inherited gate failures remain open. |

The narrowed policy-only diagnostic receipt repair passes its V10 direct/drain currentness control; enabled semantics still requires authentic input. Opaque authoring remains inert-source advice, not execution/body/frame authority. Readonly tests distinguish reported text from actual producer values; positive computed Native producer coverage remains open.

## CI, proof and release gates

Manual [CI41749944578](https://github.com/bitwisecook/tcl-lsp/actions/runs/41749944578) targets exact `8050f4d4c6477af73b42bcf69761805aa653cc6c`, created4November00:51:36UTC, before current repairs. Last inspected01:50UTC:8success/11failure/9active/2skipped (30jobs); no final aggregate. Five Rust shards, Heavy and3 native E2E partitions were still running. All3 native extension partitions fail. Web smoke fails ENOSPC before build/tests; WASM/WASI source/closed/rename checks fail; runtime2300pass/104fail; SpecTcl compatibility and real-link remain failed. Multi-folder passes. Triage `/tmp/ci-current-41749944578/triage.md`. No old CI result verifies the latest fixes.

Older CI41736891101 on b263 finishes9success/15failure/4cancelled/2skipped. Its native partition3 tally616pass/238fail/5ignored combines server units and wire tests, not a full Rust aggregate or wire-only count. Desktop partitions2/3 report11pass/9fail and319pass/39fail. Original exact panic inventory: `/workspace/tmp/ci-triage-current/E2E3-EXACT-PANICS-41736891101.md`.

The b263 global proof gate reported2,178 structural errors:1,204 evidence digest mismatches,973 census mismatches and1 hover conclusion mismatch. Conclusion parity is repaired without changing evidence hashes/provider outcomes. Historical816mutable/425immutable/973census assessment is a different snapshot. No accepted reduced LSP profile exists. Transport-projection prose/checksums do not implement dual execution/projection identity; originals must not be unsanitised or digests blindly renewed. Reconciliation requires individually verified provenance and a reviewed representation contract.

No final-current passing `make rust-check`, `make prep-pr`, global proof/drift, full E2E, browser/WASI/editor, performance, packaging or release acceptance exists. Deferred backend feature work does not waive required linked gates.

## Next acceptance work

1. Finish current lint, adopt only reviewed coherent hover/fixture changes, then build and test the current native server. Repeat all relevant wire controls, including existing factory/schema/special/class cards.
2. Run fresh startup through scan.ready and actual closed-symbol/package/tclIndex/definition/hover requests. Fix remaining producers and8.4 fallback cost under genuine typed proofs; no startup gain is accepted yet.
3. Rerun current units/CI and complete required compatibility, reference/edit coverage, host, performance and packaging gates. Keep baseline failures and missing inventories explicit.
4. Prepare the reduced PR to rust only after required acceptance. Before merge remove temporary state/process ledgers repository-wide per AGENTS while preserving reproducible source/oracle evidence. Development pushes imply no merge/tag/release publication.

The3November external-reporter sweep and source-specific #2505 correction are in [issues.md](issues.md#external-reporter-issue-sweep--3-november-2026). Its targeted results remain bound to their original base/rebased sources. VS Code repaint and intermittent TclOO references still need actual host acceptance.

Private drafts remain local; the earlier rejection of exporting unadopted private archives was not bypassed. Reviewed source/status pushes are authorised. Verified compressed artifact restoration receipts include `v10-elf-preservation.json`, `suffix-experiment-elf-preservation.json` and `inactive-autoload-probe-preservation.json` in `/workspace/tmp/lsp-merged-cohort/`. Earlier agent Main writes were restored before source resync and not compiled. No new full worktree copy or unreviewed archive upload is authorised. [Outstanding work](outstanding/README.md) mirrors this snapshot.

Latest adopted hover `c1cec1199` and Core style `2f6ebdfb1` compile in fresh native server `4068c4bf2c50cdad9b31b1f27d15b75f65984c8adcfad1c0e94ab0dd7510fc77`, Cargo fresh:false/all-features, source/prior-generated unchanged. Receipt `/workspace/tmp/lsp-merged-cohort/server-hover-current-after.json.gz`; no functional/host/startup pass yet. Latest Clippy `/workspace/tmp/lsp-merged-cohort/source-cohort-clippy-v6-result.json` fails9 server test findings with source seals matched. Reviewed test-only patch stays private until the native protocol/source seals finish.

Remote work commits `4d0f25b47`/`240be4501` were reviewed and merged as `a3c1d8a76`, including brace-fold document geometry and #2505 fixture coverage. Their source-specific reported checks are not a merged acceptance result. Publication missed the02:30 checkpoint on divergence, then resumed successfully at02:35:57; this remains a real cadence interruption. No force push occurred. The current native build includes those remote production changes.

Native4068 acceptance `/tmp/source-hover-acceptance-current/receipt.json`:15/15 original controls,18/19 mandatory targeted checks, no harness failures and full source/generated/binary/fixture/runner seals matched. The sole mandatory failure is exact full final-main fixture definition=null; its procedural hover is also still unavailable. Separate five-arm `/tmp/source-hover-first-request-control/receipt.json` preserves the original failure: exact lazy closed child fails twice, identical child-open or folder-preindexed strings succeed, and simpler closed child succeeds. No internal guard or dialect mismatch is yet proven.

Fresh startup `/workspace/tmp/package-autoload-startup-current-20261104/receipt.json`:baseline bbbb1 and candidate4068 both time out at60s before scan.ready, with identical configuration/env/fixtures and matched pre/post seals. Candidate avoids full analysis on4 Tcl profiles, but Tcl8.4 still spends6337ms and Tk8.4 spends26783ms in fallback; Tk8.4 ends49.75s, scan still unfinished60s. Navigation readiness is not accepted.

Reviewed optional invalid-layout proof API is adopted as `9e95e5e41` with contract `436c0de21`, after independent static review. Its7 fixed controls remain unexecuted; default extraction and the existing8.4 fallback assertion are unchanged. It proves only unchanged-consumer non-contribution, not runtime errors or file absence; live source/input/provider leases remain separately required. It has not improved production startup.

The opt-in proof API type wiring is corrected in `bf983a16f`/`057837b2c`; current extraction remains unchanged. Clippy v9 reaches Core tests, v10/v11 reach compiler integration tests. Their remaining empty-value assertion findings are corrected in `daab73ec1`, `957ca8cf5` and `bc8cdab15`, with the affected integration files formatted. These are lint/fixture changes, not a passing full gate; source/prior-generated checks matched each completed attempt.

A controlled four-crate unit build is now running under `/workspace/tmp/lsp-merged-cohort/source-full-first-diagnostic-build/`. Exactly one scratch SourceViews file intentionally differs for cfg(test) first-response observation; both exact WASI fixture files and all other Main/scratch inputs are explicitly pinned. No Main correspondence or diagnostic execution is claimed yet. Its private diagnostic is never committed.

The five-arm native discriminator confirms the exact child-open/preindexed versus lazy-closed difference but not an internal refusal cause. The private unit diagnostic will observe the first public response before any second reconciliation. Earlier dialect-downgrade speculation is not established.

The03:05 GitHub push failed503; explicit retry succeeded03:07:28 and the regular publisher succeeded03:10:57 and03:15:57. These record actual checkpoints, including the interruption; no continuous five-minute success is claimed across the network failure.
