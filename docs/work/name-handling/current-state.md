# Current LSP naming migration state

Updated 4 November 2026, 18:56 UTC. Results apply only to recorded source, inputs and purpose. Git and receipts retain history; this is the current snapshot.

## Scope, ownership and release verdict

**The LSP is not release-ready.** Scope includes the LSP, its transitive naming dependencies, and actual linked Engine/VM/SpecTcl execution and hook confinement. Separate VM, BIG-IP and ancillary feature development remains deferred. Source advice grants no ordinary lookup, Native execution, completed-world or physical-edit authority. Source, owner, ancestor, input, collision and reference-inventory guards remain mandatory.

Main is the shallow `/workspace/tcl-lsp-partial` checkout on `work`, origin `https://github.com/bitwisecook/tcl-lsp.git`. Root owns Main, scratch `/tmp/optimiser-settings-work`, shared target and builds. Five-minute development syncs may precede green tests: fetch, normally merge clean changes, push without force or WIP adoption; conflicts require review. Issue-sweep integration is not naming acceptance.

Publication last succeeded at 2026-11-04T18:53:02.027592+00:00, head `fcf507632253a1ae86a898a995e3988e09d45086`; earlier interruptions remain in `/workspace/tmp/lsp-publication/{last-success.json,checkpoint.log}`. No PR, merge to `rust`, tag or release has been created.

## Current execution and failures

Normal-source Compiler34 v2 completes after a successful fresh build: **30/34 controls pass**. All autoload/prefix/channel controls, W214 used-parameter, body/card ownership, memo and Boolean controls pass. Four Logical namespace controls fail; exact schema observation identifies an inherited coarse interpreter-policy effect rejected by the source transfer. A reviewed source-only coverage correction is adopted but untested, preserving SCRIPT and independent policy effects and ordinary Native facts. Captured source/dependency/generated/executable checks pass; later Main drift makes current-Main acceptance false. `compiler-channel-namespace-current-v2/result.json.gz`. The first attempt's locked-resolution refusal remains recorded; synchronizing the xtask workspace manifest corrected it without changing Cargo.lock or disabling --locked.

Core27 is compiling current normal source with the original 23 controls plus three regex-hover controls and variable definition. Known unknown-writer and all workspace manifests are presealed; no overlays. The regex fallback is adopted after a public probe proved current pattern metadata existed but method abstention stopped hover; actual new Core behavior is not yet verified. `/tmp/lsp-regex-current-public-probe-v1/receipt.json`; `core-current-normal-27/`.

Receipt paths below are relative to `/workspace/tmp/lsp-merged-cohort/` unless absolute. Captured pins do not seal topology or undiscovered runtime/toolchain inputs.

| Scope | Actual result and limits | Receipt |
| --- | --- | --- |
| Compiler diagnostic V3, 21 controls | Fresh build succeeds; **16/21 pass**. Failures: Browser-cut autoload, Logical namespace shadowing, used-parameter W214, and two Logical-card controls. Cold/warm and shifted memo comparisons, direct for-condition references and three Boolean completion/short-circuit controls pass. Both overlays restore; captured pins match, later Main drift rejects current-source acceptance. Tracing excludes a performance claim. | `autoload-first-refusal-diagnostic-current-v3/result.json.gz` |
| Core V13 parent | Fresh build succeeds, then **refuses before all tests**: actual dep-info discovers `experiments/name-handling/patterns/composed-callback-prefix/probes/unknown-writer.tcl`, absent from the preseal. Overlay restores exactly. Zero tests run. | `autoload-purpose-core-current-v13/{result,restoration}.json.gz` |
| Separate Core13 post-build V2 diagnostic | Same fresh ELF with historical diagnostic overlay; normal scratch is restored. **21/23 semantic controls and the observation pass**. All 15 source/reference/rename controls and four directory controls pass. Two autoload-header controls fail at canonical recipe/provider prerequisites. Frozen post-build pins remain unchanged. The late file was captured after compilation: complete prebuild custody, current-Main and parent acceptance remain permanently false. Main-only drift is observed; scratch/artifact drift aborts. | `autoload-core13-postbuild-diagnostic-v2/result.json.gz`; frozen scope `/tmp/autoload-core13-postbuild-diagnostic-v2-private/frozen-postbuild.json.gz` |
| Linked RMW v19 | Three fresh builds and all 19 jobs complete: **31/32 pass** (Engine 7/7, hooks 14/14, VM 10/11). Jim cache/unset passes; the dictionary-member wrapper control fails because local d changes before a foreign-alias refusal. All captured pins remain unchanged, Main drift prevents current-source acceptance. The reviewed early owner check and direct local set/unset control are now adopted but untested. | `linked-rmw-current-v19/` |
| Last completed linked cohort, RMW v17 | Three builds succeed; all 19 jobs run; **30/32 pass**: Engine 7/7, containment 14/14, VM 9/11. Jim retained-member and cache-before-unset assertions fail. Captured pins remain unchanged; sticky Main drift prevents current-Main acceptance. | `linked-rmw-current-v17/result.json` |
| Server V11 | Fresh build; **26/26 mandatory controls pass**, including original 17, eight symbol V3 controls and normalization regression. Probe exits `[101,0,0,0,0,0]`; symbol succeeds, autoload is null, root references stop at incomplete scope. Overlays restore and captured pins match; later Main drift rejects current-Main acceptance. Five unchanged existing-library repros fail separately. | `server-current-controls-probe-v11-after-provenance-sync/{prevalidated,result}.json.gz`; `server-v11-existing-repros/receipt.json` |

## Adopted changes and open semantic work

Publication/recapture, immutable-survey reuse, conditional-purpose bridge, symbol ownership and normalization repairs are adopted; prior passes do not validate later source. Require/unknown-call preludes lose lookup authority; bare autoload/package observations differ. Original-sequence autoload remains unresolved.

Named-body binding, assertion-preserving card-fixture and deferred-autoload prefix transport repairs are adopted **but untested**. Transport copies only exact parent prefix inventory under source-advice purpose, no ordinary effects/execution receipts. Whole/cold/warm W214 and earlier channel uncertainty remain open. Probes distinguish the original Named fixture from a genuinely issued Logical card: `/tmp/terra-logical-body-{public,card-public}-actual-v1/receipt.json`. The reviewed Logical namespace source-flow/activation and narrow Return continuation repair is now adopted but uncompiled and unexecuted. The channel-observer extension is also adopted: its four residual conditions remain unresolved, and its readonly definition Location is only a conditional SOURCE-header association. It supplies no runtime binding, ordinary lookup, symbol publication, reference census or edit authority.

Names-only parameter metadata (`5ca4b7dde`) grants no live-frame/strict body authority. Current UI/provider and native two-file edit validation remain open. Preserve captured bindings, authored-random guards and genuine C8.4/F5/Jim issuers.

Budgets stay unchanged: loop 50 needs 305 commands against cap 200; default 250 ms refuses loop 300. A changed budget proves no original pass. Test-purpose correction preserves the negative; no performance acceptance follows. `/workspace/tmp/linked-hook-budget-diagnosis/current-v3-0724/receipt.json`.

## Hosts and packaging

New host run [41877790468](https://github.com/bitwisecook/tcl-lsp/actions/runs/41877790468), selected by push `5c0ba57f8075a153750bfc4ee038c5676d4ac491`, now has terminal Browser and WASI jobs while Native remains active in the last observed snapshot. Browser passes 40/41 twice, with only autoload null. WASI fails before runtime on the trace’s deprecated atomic API under warnings-as-errors; no new module or lifecycle/CPU observation was produced. The reviewed trace API fix is adopted but awaits a real WASI build; missing module/receipt now records no-runtime explicitly and cannot execute stale artifacts. `/tmp/lsp-ci-5c0b-job-126741986{682,734}.raw.json` and `/tmp/wasi-5c0b-126741986682/artifact/`. Prior run 41871981409 on `b0ff26539172f34f9590916f1e77e8d02d4a1a45` fails all three builds before tests on Explorer's Arc/realm mismatch. Integrated `33cf8caf26` needs the new host run. Old logs: `/tmp/lsp-ci-b0ff-job-126721330{423,490,785}.log`.

Completed Browser/WASI results remain **old-source evidence**, manual run [41859243999](https://github.com/bitwisecook/tcl-lsp/actions/runs/41859243999), exact `b06734318768fdef9dcfef7b0109beb7db97544c`:

- Browser **40/41** in original and diagnostic runs; only original autoload request id 4 returns null. Symbol, sourced sibling, empty-directory, rename/UTF-16/revision and opaque/incomplete-coverage controls pass. `/tmp/autoload-baseline-alias-trace-combined/browser-job-126676253361.raw.json`.
- WASI **32/33** in both runs. Busy 4,000-call exit plus stdin FIN times out in all 12 attempts per run at the unchanged 30-second budget; ordinary EOF and abrupt exit pass. `/tmp/wasi-job-126676253571-{logs,steps}.json` and `/tmp/wasi-job-126676253571-artifact/`. Adopted lifecycle tracing is no demonstrated fix; The cancelled Native job recorded 1,657 successful, 411 failed and 3 ignored terminals; 612 selected tests have no terminal. The 3 ignored terminals are outside the 2,680 selected listing, no run-end was captured, and no aggregate verdict is established. `/tmp/lsp-b067-native-terminal-triage.json` retains counts and the census discrepancy; causes remain unassigned.

Host V7 (`887920c189`) adds acknowledged admission and sticky incomplete census. Its 12 private controls, extension lint and TypeScript checking pass: `/tmp/lsp-vscode-{lint,types}-current-1641.log`. Private scope: `/tmp/browser-empty-root-host-v7/receipt.json`. Real VS Code/Wasm host integration remains pending; b067 predates V7.

Four no-build packaging contracts pass in `/tmp/lsp-packaging-contracts-current-1258.log`. No current module, VSIX, editor package or native-release acceptance follows from source/output custody and no-build checks.

## Historical evidence

Git and receipts retain every earlier attempt; later fixes never relabel failures. Under `/workspace/tmp/lsp-merged-cohort/`:

- `autoload-reuse-vectors-v3/{result.json.gz,actual-index-vectors/result.json.gz}`: joint 15/20; all 11 autoload and 15/15 installed vectors pass.
- `closed-cut-core-results-current-v3/{result,restoration}.json.gz`: 15 semantics plus observation pass, final custody fails; `restored:false` remains despite later normal-source equality. `closed-cut-core-focused-current/result.json.gz`: 7/11.
- `linked-rmw-current-v11/result.json`: Engine 4/6, containment 14/14, VM 7/8 (8/8 corrected). V14 diagnostic: 30/31; v16 runs no controls. Leakage/error-global/dict-cleanup failures stay recorded.
- `autoload-purpose-compiler-current-v3-after-cache-restore/result.json.gz`: earlier 2/4, separate memo 6/9. `autoload-purpose-core-current-v12/{result,restoration}.json.gz` and `server-current-controls-probe-v8/result.json.gz`: ENOSPC before controls, affected outputs later cleaned. `server-v10-diagnostic-after-main-proposal-drift/receipt.json`: 16/17 mandatory, 3/5 probes, five repro failures; parent refusal unchanged.

Earlier native passes are source-bound; strict startup still fails (null autoload/package timeout): `/workspace/tmp/package-autoload-startup-contention-hover-20261104/receipt.json`. Earlier Browser/WASI and compile/membership/custody failures remain failures. Cancelled/partial CI leaves unknown coverage: `/tmp/ci-native-partial-browser-owner-1223/{receipt.json,TRIAGE.md}`. Skips and missing terminal events cannot close acceptance.

## Gates, performance and resource integrity

Reviewed SpecTcl corrections are adopted but untested: provider/no-provider pack snapshots use distinct cache domains, manifest matching hashes exact source bytes, and hook generation/plan publication is coherent. Standalone LSP eager loading is restored so its first request cannot consume a manifest-only registry; CLI demand and other performance improvements remain. `docs/LSP_RELEASE_PACKS.md` records this release choice.

`make check-tcl-evidence` passes: 1,409 Tcl evidence files and 17 checker tests on current source; `/tmp/lsp-check-tcl-evidence-current-1854.log`. Legacy proof material is historical. Its 2,171-error audit (781 implementation joins, 425 appliance joins, 965 census mismatches) is not a current gate count. Source-only repairs renew no execution/provider/census authority or reduced total. `/workspace/tmp/proof-current-gate-20261104-0750/gate-receipt.json`. No digest renewal or exemption grants acceptance.

Clippy V22 passes eight packages/all targets with `-D warnings` on earlier inputs (`source-cohort-clippy-v22-result.json` under the receipt root); later stack changes require fresh lint. Final `make rust-check`, `make prep-pr`, formatting/generated drift, full native/browser/WASI/editor, packages and performance/cancellation/memory gates remain open. Required section F and zero pass-to-fail against the selected `origin/rust` baseline remain open; [release plan](lsp-release-pr-plan.md) retains the extraction, fixture-reduction and final-SHA requirements. Evidence stays on `work`; the shipping candidate keeps only consumed fixtures and durable contracts.

**Performance remains release-blocking.** Recorded 30-line analysis costs 3.88 G instructions; 20 edits reach 620 MiB RSS. Library cases take tens of seconds. Adopted `d57c2d639`/`232a479dd` reduce recorded cold work to 3.29 G, 20 edits from 20.4 to 16.3 s and RSS to 410 MiB; corpus total falls from 236 to 160 s. These are not final release measurements. Conditional-metadata/enclosing-context fallback, repeated walks, allocation and lookup cost remain open; unmatched timing or changed budgets prove no acceptance.

Historical executable archives/receipts remain; approved normal caches may be retired. Old-graph probes need recovery/rebuild; exact bytes are not promised. An agent wrongly followed a source-tree alias and deleted 13 tracked compressed fixtures. Root restored all 13 from Git with original SHA256 matches before builds/merge; no tests ran with missing fixtures. `/tmp/lsp-failed-merge-recovery-1640/source-fixtures-restored.json` and `/tmp/lsp-cache-archive-retirement-2330.source-fixture-incident.json`. Cleanup now requires literal artifact paths, tracked-source exclusions and Root coordination. Resource recovery is not test acceptance.

## Next actions

1. Retest the Jim ownership fix and complete the new host run; retain the old RMW failure and all confinement guards.
2. Test adopted Named/card/prefix repairs, diagnose Core recipe/provider and Compiler namespace/W214 failures, test adopted Logical flow and review the private channel extension. Obtain a fully presealed current cohort; Core13's diagnostic cannot supply it.
3. Complete current Server/host/edit/lifecycle, full gates, packages and matched release performance. Keep truthful five-minute syncs. Open the PR only after readiness; do not merge or release.

[Outstanding work](outstanding/README.md) mirrors this snapshot. Separate issue-sweep observations remain in [issues.md](issues.md#external-reporter-issue-sweep--3-november-2026).
