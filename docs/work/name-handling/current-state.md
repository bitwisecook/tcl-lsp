# Current LSP naming migration state

Updated 21 October 2026, 14:40 UTC, on `work` after the full origin/rust (`fcb7927f02`) merge. This file is the single list of outstanding validations; it is refreshed with each 30-minute progress report. The previous version (17 October, pre-merge) is in git history.

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
| 1 | Full workspace CI (`ci.yml`) | 21 Oct, [run 39616304947](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947) on `f6c4694f14` is active with confirmed failures. Shards 3/4/5, doctests and runtime tests fail to compile: E0004, missing `NativeInstructionPlan::StringOperand(_)` arm at `runtime/rust/src/interp/native_body_artifact.rs:1081`. These jobs execute no test cases, so their failures cannot be compared per test with run 5201's 3,634 failures. Rust formatting and four cargo-deny advisories also fail. | Collect the remaining completed jobs and compare executed tests with run 5201; retain compile blockers separately. Raw job receipts are in local `handoff/reviews/merge-inventory-20261019/ci-39616304947/`; GitHub job logs remain the remote authority. No fixes in this validation pass. | Validation agent; existing reporter owns scheduled reports |
| 2 | Direct-LSP 1,085-selector capture | 21 Oct on `1e910cd3d3` (product source `f6c4694f14`): build `20261021T1431Z` **PASS**, six library test targets compiled, exit 0; 103,065 source files and all six C/Jim providers match byte-exact before/after build. Artifacts SHA-256 `4bc4d82423f6635288deb30a8779ffe80bd7d98073323dd70c469f31f6277d74`. Capture `lsp-validation-20261021T1431Z` is running; no test acceptance yet. Previous capture48: 870 pass, 215 fail, 2 timeouts. | Finish the unchanged 1,085 selectors at 180 s each, require final source/provider/artifact guards, then compare every selector with capture48. Local receipts: `tmp/name-handling-validation/lsp-build-20261021T1431Z/`. | Validation agent; existing reporter retains scheduled reports |
| 3 | LSP crate tests against CI 5201 | 21 Oct on `1e910cd3d3`: the retained direct-LSP DB group completes **7 pass / 4 fail / 11**, versus capture48's **8 / 3 / 11**. One observed pass-to-fail: `original_optimiser_memo_transport_tests::body_optimiser_memo_retains_the_actual_optional_worker_dialect`, missing body entry unwrapped at `rust/tcl-lsp-db/src/lib.rs:11927`. Source, all six providers and all six artifacts match the guarded build after this group. This is a committed WIP checkpoint observation, not full-crate or root-cause acceptance. | Finish Core/Server selectors and whole-capture guards; compare full CI test results when they execute. Implementation owners must investigate the DB loss; no test/assertion changes made. | Validation agent; implementation owners retain fixes |
| 4 | 18 October user decisions (DB921 merged unit, survey option a, forced-import offsets, test-only helpers, Server102/103, Server108) | DB921, survey and workspace-symbol snapshot landed in integration45 (`aff99dc7e`, `82617ee15`); the rest unconfirmed. | Confirm each on HEAD. | Luna-41 |
| 5 | Clippy `-D warnings` | 21 Oct [rust-check job 119280405717](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405717) **FAILS at formatting before Clippy**. New Clippy counts are unmeasured. Prior Sol-43 counts remain compiler 1,177, Server 587, VM 517, Core 414 and DB 58; they are not measurements of this checkpoint. | Measure the LSP and necessary Compiler Clippy paths independently, or after the implementation owners clear the formatting blocker. | Luna-41, Sol-49; validation agent measures only |
| 6 | Fresh-entry analysis and C6 | Sol-47 fixed false S100/T100/T105 and `if {0}` removal. C6 unapplied with 6 blockers (4 uncalled procedure bodies, `fp_sh_21`, `fp_tnt_03`). | Implement "world after load", clear the blockers, apply C6 with zero pass-to-fail. | Sol-49 |
| 7 | iRules LSP diagnostics | 23 `irules_event_checks` tests fail; iRules documents get unknown runtime history. | Realm-aware command world (decision 2026-10-21T00:10Z). | Sol-49 |
| 8 | Miscompile | `string length "{}$z"` gives 4; tclsh gives 3 (`word_value_resolution_e2e`). | Root-cause fix plus regression test. | Sol-49 |
| 9 | SpecTcl execution | `eval_loader` 24/28 and `codegen_stamps` 1/29 (Sol-44); `spectcl-compat` CI job red. | Fix VM/runtime causes; send compiler repros. | Sol-48 |
| 10 | `resolution-oracles` workflow | **FAIL**, confirmed 21 Oct on `f6c4694f14`. [Run 39615734200](https://github.com/bitwisecook/tcl-lsp/actions/runs/39615734200) has zero jobs. Explicit `gh workflow run resolution-oracles.yml --repo bitwisecook/tcl-lsp --ref work` is rejected with HTTP 422: `Unrecognized named-value: 'runner'` for `runner.temp` at line 33 column 28 and line 34 column 25. This confirms the previously suspected workflow-file rejection; no oracle tests executed. | Implementation owner must repair the two job-level environment expressions and revalidate before a PR. No workflow changes made by the validation agent. | Unassigned implementation owner; diagnosis complete |
| 12 | Test fixtures for the PR | [Fixture review](handoff/reviews/fixture-review-20261021/REVIEW.md): 2,514 of 29,092 added test files are read by tests. 208 captured per-release expectations are not asserted. | Carry `keep-paths.txt`; merge duplicate sets; convert 51 sets to tables; add the 208 assertions. | After Sol-48/49 land |
| 13 | Repository gates on the merged tree | **Partial validation, FAIL**: 21 Oct on `1e910cd3d3` (product source `f6c4694f14`), `cargo fmt --all -- --check` exits 1: 36 formatting hunks in 11 files (10 Compiler source files and `tcl-spectcl/tests/eval_loader.rs`). This checkpoint includes committed in-progress implementation work; no formatter was applied and no migration regression is inferred. Integration targets, actual server/editor hosts, proofs, generated-file drift, `make rust-check` and `make prep-pr` remain unexecuted. | Implementation owners must address formatting; validate the remaining gates after CI is close to green. The validation agent will not run mutating format/codegen recipes. | Coordinator; validation-only measurement |

## Passed

| # | Validation | Evidence and scope |
| --- | --- | --- |
| 11 | Python lint (`lint-py`) | **PASS**, 21 Oct on `f6c4694f14`: `make lint-py` in [Python job 119280405813](https://github.com/bitwisecook/tcl-lsp/actions/runs/39616304947/job/119280405813), completed `ruff (format --check + check)` step reports success. Current Makefile excludes exact evidence under `experiments/`, `docs/` and authored/native fixtures; the earlier 650-file / 1,937-error evidence-script result does not describe this tooling gate. Python typechecking and native-binding tests remain separate, unfinished checks. |

### Carried from the 17 October state (not re-checked since the merge)

- Core `nextto` selector 256 fails.
- Reference completeness is partial for external receivers and unknown bare calls.
- Two server tests time out at 180 s: `startup_index_and_factory_reindex_keep_checked_document_roles` and `workspace_symbol_caps_its_answer`.
- Optimiser proof holds O101, O127 and the Logical quoting O100 path.
- Document-link disagreements 124, 136 and 140.

## Where results live

- Agent notes and patches since the merge: [handoff/reviews/merge-inventory-20261019/](handoff/reviews/merge-inventory-20261019/) (`sol-4x-NOTES.md`, `luna-4x-NOTES.md`).
- User decisions: [handoff/reviews/resume-20261017-diagnosis/](handoff/reviews/resume-20261017-diagnosis/) (`user-decisions-*.json`).
- Progress reports: `progress-report-<UTC>.json` in this directory.
- Latest LSP capture: `experiments/name-handling/results/owner-adoption-20261018/lsp-generic-consumers-14/capture.json`.
