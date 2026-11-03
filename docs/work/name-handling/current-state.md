# Current LSP naming migration state

Updated 3 November 2026, 16:32 GMT. This is the active snapshot. Earlier observations and superseded states are in git history and their original evidence records; they are not current acceptance results.

## Release scope and repository

The user requires the LSP and the libraries it depends on to be release-ready. Separate VM, BIG-IP and ancillary feature work is deferred. Required transitive dependencies remain in scope, including SpecTcl execution and the VM/offline BIG-IP libraries linked by the server. This scope does not waive mandatory repository gates or turn failed checks into passes.

The active checkout is `/workspace/tcl-lsp-partial`, branch `work`, a full worktree in a shallow clone. Product checkpoint `5ab3de41ffca55b943c5684b520042569e3b81be` was successfully pushed to the configured origin, `https://github.com/bitwisecook/tcl-lsp.git`, after explicit user approval on 3 November. The earlier automatic-review push rejection is resolved by that approval and successful upload. No PR or release has been created. State checkpoint `e52d4795c` was subsequently pushed. Manual [CI run 41669761498](https://github.com/bitwisecook/tcl-lsp/actions/runs/41669761498) was dispatched for exact `e52d4795c6748d378adb09e37a661651957201c1` (hosted Rust runner, release build proof disabled). Workflow classification, web-frontends, doctests, native server build and E2E archive passed. Cargo-deny, rust-check, browser LSP tests and Wasm real-link failed; other jobs remain running at last inspection. No final run result exists. This manual run does not satisfy required PR-event checks.

**The LSP is not release-ready.** Local focused passes do not establish complete feature, host, performance or release acceptance.

## Current validation

| Check | Observed result and limits |
| --- | --- |
| Clean native server smoke | 14/14 pass from fresh `/tmp/lsp-current-target`, Rust sources at `a8bc0992b`; subsequent changes before this run completed were shell/docs only. Covers CodeLens, diagnostics ordering, completion, definition, hover, symbols and signature help. `/tmp/lsp-current-server-smoke.log`. |
| Full native all-features E2E | **Incomplete and failing.** Built the 1,652-test executable from current Rust sources. Stopped the six-thread run after 251 results with 58 marked failures because four-core CPU saturation plus concurrent compilation made timing material. This is not 58 confirmed semantic regressions or a complete suite result. Partial log `/tmp/lsp-current-server-e2e-6threads.log`; executable hashes `/tmp/lsp-current-e2e-artifact-sha256.txt`. |
| Isolated E2E failures | Missing O101 quick-fix and missing `sin` completion reproduce separately without competing builds. Logs `/tmp/lsp-e2e-optimiser-isolated.log` and `/tmp/lsp-e2e-completion-isolated.log`. E004 dangling-clause failure was an out-of-range fixture; corrected in `0a314e2b1`, with exact JSON-RPC positive and invalid-range negative probes. The corrected E2E binary has not yet been rerun. |
| LSP CI wiring | `3e5cadd66` fixes stale nextest-action syntax in the partition checker. Runner/partition positive checks and four missing-install/verification negatives pass; installer pin and integrity checks preserved. |
| Proof gate | Last complete audit invocation exits 1 with 2,236 structural errors and zero unlinked tests; it began before a proof-doc change, so this count is **not sealed to the current checkpoint**. `/tmp/lsp-current-proof-audit.log`. Later focused fixes are not a full-gate pass. |
| Formatting | Current-source formatter check passed before subsequent shell/docs changes and E004 fixture correction; not a final release-commit formatting verdict. |
| Required aggregate gates | `make rust-check` and `make prep-pr` have no passing final-current-source result. Earlier rust-check stopped at the proof gate. Full CI, browser/WASI host, editor, packaging, performance and shipping-commit checks remain open. |

## CI failures under active triage

These are results for manual CI checkpoint `e52d4795c`, not for the later dependency patch or off-tree candidates.

- cargo-deny: five advisories; patched locally in `c9ae9c87c` with all 12 workspace checks passing, CI rerun still pending.
- rust-check: Core test `original_property_options_keep_opaque_names_and_selected_read_write_roles` exceeds Clippy's line limit (107/100). A reviewed test-only helper extraction is applied **uncommitted** in the worktree; focused test and Core Clippy rerun pending. No lint suppression or assertion removal.
- Browser LSP job `126037059309`: build succeeded; property hover empty, closed-file workspace symbols absent, tclIndex definition absent, then rename refused incomplete original coverage. These are unresolved host functionality, not fixed by the receipt draft.
- Wasm real-link job `126037059076`: 24/25 pass, `emitted_modules_run_against_the_real_runtime` fails because whole-function generic argv selection is unavailable. Required gate remains failed even though separate backend work is deferred.

## Committed changes in the latest continuation

- `c9ae9c87c`: compatible security updates to pulldown-cmark 0.13.6 and Wasmtime 49.0.5 plus coupled Cranelift packages; two report-generator lockfiles updated too. Actual `scripts/dev/cargo-deny-all.sh` passes all 12 locked workspaces with cargo-deny 0.20.2. Independent diff/log review clear; log `/workspace/tmp/lsp-cargo-deny-fix/all-workspaces.log`. Compilation and a CI rerun for these new lockfiles remain pending. Existing scratch candidate binaries use the prior locked graph and must not be described as validating this dependency update.

- `99534469c`: bounded source-only expression-syntax correspondence for Tcl 8.5/9.0/9.1. Focused registry/Core controls and library Clippy pass; original 8.6 API retained. Pinned 8.5/9 runtime oracles were unavailable. Jim residual remains failing.
- `23720877f` and `1d891222a`: file-rename input/sidecar/factory custody and controls. Initial scoped comparison used a common mixed-origin dependency cohort; it was not a current-source gate. The later factory draft's clean current-source cohort also passes all 13 rename controls, without validating the entire factory draft or every server path.
- `ca0c0b10d`, `1a252577e`, `9b5592854`: scoped proof bindings/contract corrections backed by focused validation; no whole-gate acceptance.
- `1c056e331`: three DB arity implementation-contract snapshots revalidated against current source, four DB controls and independent review. Callback wording corrected to match its actual negative fixture. Immutable experiment evidence/provider outcomes unchanged.
- `5ab3de41f`: indexes two existing source-role contracts and corrects overstatements. Two focused unindexed errors removed; all providers remain not-tested, with no execution or edit-authority claim.

Earlier integrated fixes, source inventories, oracle captures and their scoped results remain available in git and the proof/experiment records. Historical audit totals have not been re-admitted as current verified units.

## Active drafts and blockers

These drafts are **off-tree and uncommitted** unless explicitly stated otherwise. Source review, type checking and isolated controls are separate from final adoption.

| Owner | Draft and current evidence | Still required |
| --- | --- | --- |
| review | O101 optimiser edit receipt: measured analyser realm has captured settings while worker realm does not; source/input/origin/registry match. Draft carries actual producer settings without relaxing realm equality. Earlier draft type-checked, then review found source-point and private factory-issuance issues. Revised dedicated settings ingress passes Compiler/Core/DB metadata checks; two actual producer tests pass, but the original quick-fix E2E still fails against the candidate server (`/tmp/optimiser-settings-lsp-quickfix.log`). The remaining server mismatch was measured as project evidence captured after the read-only unit was built (None versus Some(empty)). A combined evidence-before-analysis patch has static review and is awaiting compilation, exact evidence-state controls and protocol rerun. Source-point/factory-issuance and final lint controls are still pending. `/tmp/optimiser-settings-propagation.patch`, `/tmp/optimiser-settings-work`. | Current/stale/foreign-settings tests, source-point/factory-issuance controls, actual O101 E2E, Clippy and final review. Do not adopt the superseded type-check as proof for the revised draft. |
| integration | Partial-expression completion: root-only draft initially passes positive test but fails execution-trace refusal. Revised draft uses a CompletePrefix normal-source outcome query; focused rerun passes 2/2 (direct receipt, public math completion and trace/arity/unknown/shadow/stale/config/terminal negatives), `/tmp/partial-expr-focused-revised.log`; two existing complete-expression cursor/shadow controls also pass. Final independent adoption review and actual nested E2E remain open. Nested bracket extension passes two focused controls, including the exact prior-procedure/partial-expr shape, and four root/existing regressions. Final static review found no further blocker, but Clippy and actual candidate-server E2E are pending. These passes precede the dependency update; final checks will use the updated lockfile. | Trace/arity/unknown-prefix/stale/config negatives; authenticated nested-parent entry; separately authorised procedure candidates; original missing-`sin` E2E and actual server test. `/tmp/partial-expr-work`, `/tmp/nested-expr-source.rs`. |
| lsp_failures | Bounded De Morgan/comparison action proof: literal finite expressions, actual selected handler, one expression role, complete empty bodies and exact source/AST. Metadata type-check passes after API corrections; native system Tcl 8.6 finite witnesses recorded. Fresh isolated Compiler/Core library build and metadata checks pass; public functional/action harness is running, with no pass claimed yet. | Fresh Compiler/Core public harness, actual emitted edit/refusal controls, Clippy and review. Existing variable/nonempty-body E2E cases remain unsupported. `/tmp/lsp-expression-operator-candidate`, `/tmp/lsp-operator-source`. |
| cache | Factory census/live receipt V1: 23 targeted + real-close + 38 existing controls pass with fresh current-source dependencies. Five broader factory/metadata prerequisites still fail. Final review found full analysis currency checked only after possible index mutation and lock-contention concerns. V2 moves checks before mutation/rechecks refreshed seeds and adds contention controls; V2 has not passed final Clippy/native tests. The earlier current-source executable also passes three ownership and sixteen provider-path regressions; these do not validate V2. `/workspace/tmp/cache-factory-live-current`, `/workspace/tmp/cache-factory-live-v2`. | Final V2 tests/review; exact omission baseline; fixes for five prerequisite failures. This is not durable recovery. |
| cache | Closed-index recovery controller prototype has isolated tests, but no complete production integration. Current workspace readiness is a one-shot first-pass signal, and diagnostic refresh errors do not retain/retry all unfinished obligations. | Retained URI obligations, Pending/Unavailable state, bounded owned retry, all publisher fencing, semantic reader handling, close/removal lineage, cancellation, host/lifecycle validation. |
| compiler | Class-factory prerequisite failures: Native superclass source provider absent; genuine class-transition advice exists but current body wrapper uses a different authority. Off-tree producer repair built, but its first Logical-profile control failed before remaining cases ran. The candidate is HOLD and being narrowed to preserve the existing Logical authority. Downstream use of source-only factory metadata as normal-completion authority must also be constrained before adoption. | Positive producer and foreign-publication proof plus shadow/trace/current-source negatives; unchanged-current-source baseline where needed; actual five failed server controls. |

Jim recursive factorial minification remains refused. Pinned Jim proves that replacement `expr` and `xtrace` observe whitespace; no recursive runtime effect/observer closure has been established. Regexp switch-list compaction is also conservatively refused for dynamic traced operands. These are not fixed by the C Tcl expression correspondence.

## Proof gate remediation

The current proof gate has no accepted LSP release profile. The off-tree profile/accounting proposal is not a waiver. Unknown records remain required. Durable assessment: `/workspace/tmp/proof-lsp-scope/README.md`.

- 816 mutable implementation-evidence slots require reviewed current source-contract snapshots plus relevant tests; the DB batch above fixes three selected pins only.
- 425 immutable appliance-evidence mismatches require authenticated recovery/new captures or explicit reviewed scope disposition, not digest refresh.
- 973 census mismatches require artifact closure and scope accounting; appliance paths cannot all be discarded because some feed LSP or unclassified proofs.
- Remaining annotations, owners, selectors and document discrepancies require their own fixes. Counts above describe the assessment snapshot, not a new full-gate result after later repairs.

## Acceptance and next actions

1. Finish and review the actual O101, incomplete-expression completion, factory and recovery fixes; run exact positive/adversarial controls and apply LSP edits through the real server.
2. Repeat complete native E2E without competing compiler load, classify remaining failures and verify fixed cases. A partial run cannot establish zero regression.
3. Clear required proof/drift checks and run `make rust-check` and `make prep-pr` on the release candidate. Preserve failed/unexecuted results explicitly.
4. Validate browser/WASI and shipped editor hosts, cancellation/latency/memory, release packaging and required CI on the shipping source. Manual CI was dispatched for `e52d4795c` as recorded above; no passing result is claimed.
5. Prepare the reduced PR to `rust` with supported product/contracts/tests and remove temporary probes/process documents from the shipping tree per AGENTS. No merge, tag or publication has occurred.

Authoritative semantic proof/provider outcomes remain in the canonical proof corpus. Older progress reports are immutable historical observations. [Outstanding work](outstanding/README.md) lists current role ownership; [release plan](lsp-release-pr-plan.md) defines preparation. Do not treat old STOPPED headers, old unit counts, absent `/tmp` files or agent-reported plans as current validated results. `/tmp` was reset earlier in this session; workspace handoffs are preferred where available.
