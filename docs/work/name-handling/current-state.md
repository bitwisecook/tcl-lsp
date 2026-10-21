# Current LSP naming migration state

Updated 21 October 2026, 14:48 UTC, on `work` after the full origin/rust (`fcb7927f02`) merge. This file is the single list of outstanding validations; it is refreshed with each 30-minute progress report. The previous version (17 October, pre-merge) is in git history.

## Scope

User decision 2026-10-21T13:10Z ([record](handoff/reviews/resume-20261017-diagnosis/user-decisions-20261021T1310Z.json)): focus entirely on

1. the LSP (`tcl-lsp-core`, `tcl-lsp-db`, `tcl-lsp-server`);
2. the compiler, registry, syntax and lexer paths the LSP requires;
3. VM and runtime execution of SpecTcl code.

General tclvm parity is parked: upstream ports not needed for SpecTcl, TclOO per-call recompile and frame sizes, `command_mutation_deopt` cluster P, the remaining native `string`/`format`/`regsub`/`encoding` compilers (unless SpecTcl needs them) and compiler lowering cost (unless on an LSP path).

Merge contents (decision 2026-10-21T12:05Z): evidence stays on `work`; the PR to `rust` carries product source, design docs and the test fixtures tests actually read, reviewed and reduced.

## Counts

| Area | Integrated | Verified complete |
| --- | --- | --- |
| Original LSP area | 290 / 325 (35 dedicated BIG-IP deferred) | 0 |
| Baseline units | 570 / 2,158 | 0 |
| Effective (with annex) | 640 / 2,229 | 0 |

Counts live in [audit-status.json](audit-status.json); per-unit sites in [repository-consumer-audit-final.json](repository-consumer-audit-final.json). No unit is marked verified: captures have not yet been mapped onto audit units.

## Outstanding validations

| # | Validation | Last result | Needed | Owner |
| --- | --- | --- | --- | --- |
| 1 | Full workspace CI (`ci.yml`) | 21 Oct, [run 39616304947](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947) on `f6c4694f14` is active and **FAILS**. All five Rust shards, doctests and standalone runtime tests stop at E0004: missing `NativeInstructionPlan::StringOperand(_)` arm, `runtime/rust/src/interp/native_body_artifact.rs:1081`. No shard test cases execute, so comparison with run 5201's 3,634 failures is blocked, not a zero-failure improvement. WASM real-link reports 1 pass / 24 fail, with the same runtime-build blocker in failed tests. Formatting, SpecTcl compatibility, Python typechecking and four cargo-deny advisories also fail; editor/LSP execution jobs remain active. | Collect remaining completed jobs; keep build-blocked test sets separate from executed failures. Local decoded job receipts: `handoff/reviews/merge-inventory-20261019/ci-39616304947/`; each GitHub job log remains remote authority. No fixes in this validation pass. | Validation agent; existing reporter owns scheduled reports |
| 2 | Direct-LSP 1,085-selector capture | 21 Oct on `1e910cd3d3` (product source `f6c4694f14`): build `20261021T1431Z` **PASS**, six library test targets compiled, exit 0; 103,065 source files and all six C/Jim providers match byte-exact before/after build. Artifacts SHA-256 `4bc4d82423f6635288deb30a8779ffe80bd7d98073323dd70c469f31f6277d74`. Capture `lsp-validation-20261021T1431Z` is running; no test acceptance yet. Previous capture48: 870 pass, 215 fail, 2 timeouts. | Finish the unchanged 1,085 selectors at 180 s each, require final source/provider/artifact guards, then compare every selector with capture48. Local receipts: `tmp/name-handling-validation/lsp-build-20261021T1431Z/`. | Validation agent; existing reporter retains scheduled reports |
| 3 | LSP crate tests against CI 5201 | 21 Oct on `1e910cd3d3`: the retained direct-LSP DB group completes **7 pass / 4 fail / 11**, versus capture48's **8 / 3 / 11**. One observed pass-to-fail: `original_optimiser_memo_transport_tests::body_optimiser_memo_retains_the_actual_optional_worker_dialect`, missing body entry unwrapped at `rust/tcl-lsp-db/src/lib.rs:11927`. Source, all six providers and all six artifacts match the guarded build after this group. This is a committed WIP checkpoint observation, not full-crate or root-cause acceptance. | Finish Core/Server selectors and whole-capture guards; compare full CI test results when they execute. Implementation owners must investigate the DB loss; no test/assertion changes made. | Validation agent; implementation owners retain fixes |
| 4 | 18 October user decisions (DB921 merged unit, survey option a, forced-import offsets, test-only helpers, Server102/103, Server108) | DB921, survey and workspace-symbol snapshot landed in integration45 (`aff99dc7e`, `82617ee15`); the rest unconfirmed. | Confirm each on HEAD. | Luna-41 |
| 5 | Clippy `-D warnings` | **FAIL**, 21 Oct on `1e910cd3d3`: `cargo clippy --locked -p tcl-lsp-core --lib --message-format=json -- -D warnings` exits 101 in the necessary Compiler dependency with **776 error diagnostics**. Core itself is not reached; DB and Server were not redundantly rerun through the same blocker. CI rust-check separately stops at formatting before Clippy. Older Sol-43 per-crate counts are not current measurements. | Implementation owners must clear the Compiler dependency before whole LSP Clippy acceptance can be measured. Receipt: local `/tmp/name-validation-clippy-core.jsonl` (bounded diagnostic records, no fixes). | Luna-41, Sol-49; validation agent measures only |
| 6 | Fresh-entry analysis and C6 | Sol-47 fixed false S100/T100/T105 and `if {0}` removal. C6 unapplied with 6 blockers (4 uncalled procedure bodies, `fp_sh_21`, `fp_tnt_03`). | Implement "world after load", clear the blockers, apply C6 with zero pass-to-fail. | Sol-49 |
| 7 | iRules LSP diagnostics | 23 `irules_event_checks` tests fail; iRules documents get unknown runtime history. | Realm-aware command world (decision 2026-10-21T00:10Z). | Sol-49 |
| 8 | Miscompile | `string length "{}$z"` gives 4; tclsh gives 3 (`word_value_resolution_e2e`). | Root-cause fix plus regression test. | Sol-49 |
| 9 | SpecTcl execution | **FAIL**, 21 Oct on `f6c4694f14`: [compatibility job 119280405629](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405629), `eval_loader` **29 pass / 1 fail / 30**. `every_shipped_pack_loads_identically_with_and_without_the_static_fast_path` diverges for `specs/eda_microchip.tclspec`: `add_library` source line 67 versus 65 (`eval_loader.rs:194`). The gate stops here; later compatibility targets and current `codegen_stamps` are unmeasured. Prior Sol-44 24/28 is a different-sized selection, not a per-selector comparison. | Implementation owner must investigate the fast/slow source-line mismatch; rerun compatibility and the remaining execution targets after its fix. No expected values or tests changed. | Sol-48; validation agent records only |
| 10 | `resolution-oracles` workflow | **FAIL**, confirmed 21 Oct on `f6c4694f14`. [Run 39615734200](https://github.com/bitwisecook/tcl-lsp/actions/runs/39615734200) has zero jobs. Explicit `gh workflow run resolution-oracles.yml --repo bitwisecook/tcl-lsp --ref work` is rejected with HTTP 422: `Unrecognized named-value: 'runner'` for `runner.temp` at line 33 column 28 and line 34 column 25. This confirms the previously suspected workflow-file rejection; no oracle tests executed. | Implementation owner must repair the two job-level environment expressions and revalidate before a PR. No workflow changes made by the validation agent. | Unassigned implementation owner; diagnosis complete |
| 12 | Test fixtures for the PR | [Fixture review](handoff/reviews/fixture-review-20261021/REVIEW.md): 2,514 of 29,092 added test files are read by tests. 208 captured per-release expectations are not asserted. | Carry `keep-paths.txt`; merge duplicate sets; convert 51 sets to tables; add the 208 assertions. | After Sol-48/49 land |
| 13 | Repository gates on the merged tree | **Partial validation, FAIL**: 21 Oct on `1e910cd3d3` (product source `f6c4694f14`), `cargo fmt --all -- --check` exits 1: 36 formatting hunks in 11 files (10 Compiler source files and `tcl-spectcl/tests/eval_loader.rs`). This checkpoint includes committed in-progress implementation work; no formatter was applied and no migration regression is inferred. Integration targets, actual server/editor hosts, proofs, generated-file drift, `make rust-check` and `make prep-pr` remain unexecuted. | Implementation owners must address formatting; validate the remaining gates after CI is close to green. The validation agent will not run mutating format/codegen recipes. | Coordinator; validation-only measurement |
| 14 | Python typechecking and native binding tests | **FAIL**, 21 Oct on `f6c4694f14`: [Python job 119280405813](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405813) reports **26 `ty` errors** in executable tooling, including proof-checker/import and BIG-IP probe scripts. `pyright` and native engine binding tests are skipped after the failing command. Python lint independently passes (row 11). | Implementation owners must address type errors before the remaining Python checks execute; no tool code changes in this validation pass. | Unassigned implementation owner |
| 15 | Browser WASM and web extension-host LSP validation | **FAIL**, 21 Oct on `f6c4694f14`: [WASM smoke job 119280405748](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405748) reaches the server and reports six failed checks: pack-property hover, workspace scan finding files, sourced-sibling definition, closed-file workspace symbols, tclIndex autoload definition and shipped-pack notice count. Separately, [web extension job 119280405797](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405797) stops at TS6133 in `extension.ts:927`, `extension.ts:977` and `currentDiagnostics.test.ts:970`; editor-host assertions do not execute there. | Investigate actual server/store/provider failures separately from the extension compilation blocker. Keep original assertions; no fixes applied. | Implementation owners; validation agent records only |

### Carried from the 17 October state (not re-checked since the merge)

- Core `nextto` selector 256 fails.
- Reference completeness is partial for external receivers and unknown bare calls.
- Two server tests time out at 180 s: `startup_index_and_factory_reindex_keep_checked_document_roles` and `workspace_symbol_caps_its_answer`.
- Optimiser proof holds O101, O127 and the Logical quoting O100 path.
- Document-link disagreements 124, 136 and 140.

## Passed

| # | Validation | Evidence and scope |
| --- | --- | --- |
| 11 | Python lint (`lint-py`) | **PASS**, 21 Oct on `f6c4694f14`: `make lint-py` in [Python job 119280405813](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405813), completed `ruff (format --check + check)` step confirms 81 files formatted and all lint checks passed. Current Makefile excludes exact evidence under `experiments/`, `docs/` and authored/native fixtures; the earlier 650-file / 1,937-error evidence-script result does not describe this tooling gate. Python typechecking fails separately (row 14), and native-binding tests are skipped. |
| 13a | Native LSP build and e2e archive preparation | **PASS**, 21 Oct on `f6c4694f14`: [native build job 119280405679](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405679) and [e2e archive job 119280405766](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405766) complete successfully. This is build/archive preparation only; actual e2e partitions are still active. |
| 13b | Shared report and Spec Studio web frontends | **PASS**, 21 Oct on `f6c4694f14`: [web-frontends job 119280405827](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405827) completes its checks. This grants no browser LSP or editor-host acceptance. |

## Where results live

- Agent notes and patches since the merge: [handoff/reviews/merge-inventory-20261019/](handoff/reviews/merge-inventory-20261019/) (`sol-4x-NOTES.md`, `luna-4x-NOTES.md`).
- User decisions: [handoff/reviews/resume-20261017-diagnosis/](handoff/reviews/resume-20261017-diagnosis/) (`user-decisions-*.json`).
- Progress reports: `progress-report-<UTC>.json` in this directory.
- Latest LSP capture: `experiments/name-handling/results/owner-adoption-20261018/lsp-generic-consumers-14/capture.json`.
