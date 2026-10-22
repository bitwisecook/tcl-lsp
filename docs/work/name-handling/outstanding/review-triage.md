# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T18:18:26Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. This agent commits and pushes it after each run and at least every 30 minutes.

## Resume

**All independent requested checks are settled.** The nested-lowering path conservatively retains generic iRules calls (no old direct/inline mismatch); extract/inline repros explicitly disable with zero edits; all small scaling rows are measured; live N17 hooks fold correctly after fd5c12bf and declared barriers remain enforced. The named existing guarded strlen e2e still fails, but its hooks install successfully and the published owner trace shows execution refusing for a missing native compiler preflight provider. Do not classify every live hook as broken, and do not bypass that provider guard.

**Run13 complete on fc5aeac1:** both DSL plans nonempty, notices empty, declined None, registry_profile tcl8.6; all eight direct answers None / O129 empty. One observational test in 29.32s after 11m21s rebuild (temporary switches removed in d33ee64c). Eight generated scripts / 48 matched checks pass; 29 files / 174 engine executions. Both targets/temporary source deleted. `owner-n17-public-axes-excerpt.log` is read-only quoted owner evidence with exact commit/blob, not another triage run. Positive same-body provider-installed behavior is unmeasured; no further independent N17 probe is required for the requested triage.

**R1 landed and post-fix matrix is complete:** on 49b067ce all five C ordinary puts-call controls have completed worlds; import cases still refuse UnknownWorld, N06 same-document guards now give specific command/member collisions, all-six sibling cases refuse coverage at both origins, N07 safely refuses. No N04–N07 edit was returned. Early edit replay: 198 engine executions / 12 matching generated checks, zero unexpected result; published checkpoint b7a43ade.

**Run14 intentionally interrupted:** a187c859 integrated a broad metadata batch touching reviewed compiler/registry/LSP consumers while the immutable 49b067ce executable ran. It completed 19 groups through N15, every exit 0, then own runner/test tree was terminated at N16 (tool exit 143). No semantic failure inferred. Partial N16 and later groups/e2e are not counted. All temporary sources/executables are deleted. Completed outputs replay: 89 files / 534 executions / 68 generated scripts / 78 matched checks, zero unexpected result (`run14-partial-*`). All 71 generated files are in byte-verified `out/run14-final.tar.gz` with manifest; raw output directory removed only after archive verification, to keep evidence below 5 MB.

**NEXT after this checkpoint is pushed:** `source /tmp/review-triage-env.sh; CARGO_NET_OFFLINE=true bash docs/work/name-handling/reviews/triage-20261022/run-latest.sh run15-latest`. The prepared grouped serial runner executes all 27 semantic probes on the new integrated tree: server-shaped matrix first, 19 core tests sharing registry startup, isolated 4 hook tests and isolated 3 seams. A separate fresh small scaling probe repeats P25 flat/index/query, source chains 250–2000 and P25 lenses; the full original scaling test stays excluded. Existing named fold e2e runs last. Build/run/delete in one command, df checks; keep active logs untracked. After it finishes, replay all fresh edits on six engines and run a fresh static audit/fetch/affected-case check. Only then write final summary. No Cargo process is currently active.

**Final completion rule:** ordinary C puts-call worlds are confirmed after R1. The new grouped runner replaces the slower per-filter runner; after it finishes, `source /tmp/review-triage-env.sh; CARGO_NET_OFFLINE=true bash docs/work/name-handling/reviews/triage-20261022/run-final.sh <fresh-tag>`. It freshly replays all findings, all-six-profile server rename matrix (N04/N05/N06/N07), refactor/lowering seams and the existing N17 e2e. Apply/execute all returned scratch edits using `replay-oracles.sh <tag>` and `summarize-oracles.py <tag>`. Run a fresh static source audit. Fetch again and recheck any relevant later product changes before final summary. Do not write the final summary while these obligations remain.

**Last complete broad replay:** run10 main compiled at 60ce7009, exact `run10-source.rs.txt`, 24 filter groups / 27 observational probes, all group exits 0. 98 files / 588 engine executions / 78 generated files / 138 matched checks, zero unexpected result. Later e2e rebuilt on 13050f6a with N17 correction and failed [] vs ["5"]. New N17 explicit-hook/negative-barrier checks are run11 (0e21cada; 2 probes + 4/4 existing regressions, 168 oracle executions), exact context comparison run12 (b3c8ea5b), installation distinction run13 (fc5aeac1).

**Small scaling:** run6 flat P25/50/100/200 and chains; run7 bounded lenses P25/50/100 all complete. Lens query ms 6093.848 / 43231.733 / 134609.430, after analysis ms 35242.482 / 150409.102 / 280831.269; counts 25/50/100. Unoptimised wall-time observations, not shipping latency. Report has all rows.

**Environment/ownership:** active shallow selective checkout `/workspace/tcl-lsp-triage`, branch work. Source `/tmp/review-triage-env.sh`; CARGO_NET_OFFLINE=true uses warmed dependencies. Rust 1.99.1; pinned C Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim 0.84 ready. Standard user flags are set; ZZ_CCG_BASELINE was unset in checked command environment. All required embedded fixture trees are restored. Never edit product/test files; restore probes only to `rust/tcl-spectcl/tests/zz_review_probe.rs` from evidence `.rs.txt`, delete before commits, build/run/delete each executable in one command, df check before builds. Do not rebase while Cargo builds. No PR. Commit only report/evidence/this file; Codex trailer; fetch/rebase before every push and retry rejection. Current evidence disk use ~3.9 MB; keep below 5 MB.

## In progress

- R1 matrix checked; run14 interrupted after N15 for the broad a187c859 integration. Next is run15-latest complete grouped replay on new source.
- Required release gates `make rust-check` and `make prep-pr` are not run; this is evidence-only work under the active `work` development-push exception.

## Outstanding (in priority order)

1. Start/complete run15-latest on the integrated metadata batch; R1 matrix is already checked on prior source.
2. After R1 lands, re-check N04, N05, N06 and N07 on all six profiles, apply returned edits and execute them on the matching engines.
3. Fetch latest `work` and re-check every finding and static seam before the final summary.

## Blocked (on whom, and what is needed)

- No external R1 blocker now; final replay is in progress. Earlier ten-positive-mismatch results are historical and superseded by the published 54-subcase diagnosis.

## Done since the last update (with evidence: test counts, commit or snapshot)

- Run 4 (12:13Z–12:39Z) ran 3 seam tests, all passing:
  - Linked editing returns only the declaration and its resolved self-call.
  - The inline-proc and extract-variable actions were offered, but applying their edits left the source unchanged, so they are probably disabled refusals. Both outputs run correctly on 6 engines.
  - The nested-passthrough probe was inconclusive.
- Run 4 was stopped before scaling and the e2e step when the triage was moved.
- The evidence directory was refreshed with the probe copy, `run3.log`, `run4.log`, `out/`, `run4.sh`, `oracle.sh` and the new `run5.sh`. The probe file was deleted from the tree, and the triage executables were deleted from `target/`.

- Cloud setup completed: Rust 1.99.1; C Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0; Jim 0.84 at `5bac7c99ad65864c87da513e22e2f01703fa4e03`. The standard setup hook stopped at missing sudo; its Rust and Tcl build helpers were reused with writable local paths.

- 2026-10-22T13:45:08Z: cloud baseline replay completed: 19 preserved Tcl files, 114 engine executions, no timeouts (`cloud-baseline-oracles.log`). Feature-unavailable rows retain their nonzero exit codes. N17 driver registers `good` on all six engines. Static source excerpts: `cloud-source-audit.log`; pending current-API rows remain pending.

- 2026-10-22T13:59:37Z: run5 finished; repeat of the existing fold test confirms a real failure (0 passed / 1 failed; DSL 1 expected ["5"], got []). Evidence: `run5-e2e-repeat.log`. No product files or tests were edited. Required release gates remain unrun.

- 2026-10-22T14:16:15Z: checkpoint of the first completed run6 observations; the active log is preserved outside the index. Snapshot: `run6-progress.log`. No new edit output has yet been validated on the oracles; those outputs remain uncommitted until replay.

- 2026-10-22T14:42:43Z: run6 confirms N09 call graph fixed, N10 path definition fixed (hover/hierarchy still missing), and N17 direct scanner results correct. Both DSLs fold with declared pure effects and fail to fold without that declaration. Server-shaped C controls so far: bare `foo` renames with a world; `puts [foo]` has no world; `set x [foo]` has a world but refuses coverage. R1 remains incomplete for the required positive case. A small prepared `run-r1-transport.sh` / `r1-transport-probe.rs.txt` can compare realm/result worlds before/after core seeding after the full run; it also checks N03 parser-debt helpers. These diagnostics are not yet executed.

- 2026-10-22T15:24:20Z: full run6 evidence and output replay recorded. Flat analysis (P25/50/100/200) = 23.490 / 73.748 / 237.256 / 630.311 s; index min-of-three = 1.043 / 3.332 / 5.019 / 10.881 s. Source chain F250/500/1000/2000 = 134.775 / 318.125 / 1396.401 / 5055.848 ms. P25 lens query = 6049.759 ms after 61798.1 ms analysis, 25 lenses. All are unoptimised wall-time measurements, not shipping latency.

- 2026-10-22T15:46:08Z: run7 semantic groups complete on `0434e7e7`: 3 seam probes, N17 contrast, R1 realm/result/seeding controls and N03 parser/scalar controls. Both refactors explicitly disabled, zero edits. R1 missing at retained realm for puts before/after seeding. Source-chain F250/500/1000/2000 = 70.989/252.495/1085.463/5692.710 ms; P25 analysis 35242.482 ms, lens 6093.848 ms, 25 lenses. Fresh outputs: 10 files, 60 engine checks, all correct. Larger lenses remain active.

- 2026-10-22T15:56:19Z: run7 finished with every group exit 0. P50 lenses = 43231.733 ms after 150409.102 ms analysis, 50 lenses. P100 = 134609.430 ms after 280831.269 ms analysis, 100 lenses. Small scaling rows complete. Temporary source/executable deleted. Final post-R1 and latest-work requirements remain.

- 2026-10-22T16:00:03Z: run8-n09 executed zero tests: temporary probe used a crate-private helper and failed compilation. Corrected the evidence source to public procedure IDs; next attempt is run9-n09. Product/test sources unchanged.

- 2026-10-22T16:04:03Z: run9-n09 complete, 1 observational test passed in 64.04s; public-ID taint projection is conservatively all three source candidates. Constructor/call graph unchanged and correct. Temporary source/executable deleted. Awaiting R1.

- 2026-10-22T16:12:43Z: closed run10 progress checkpoint: 7 completed filter groups, all exit 0, through N06; runtime observations remain as reported. Fresh static audit is `run10-source-audit.log` on `60ce7009`. Main command remains active; outputs are not yet replayed and remain uncommitted.

- 2026-10-22T16:30:00Z: run10 closed prefix refreshed through N16 (all groups exit 0; N15 generated all 48 outputs, not replayed yet). Remote `fd5c12bf` publishes the N17 declared-effect barrier correction; R1 still fails ten positive g4 subcases. Rebase at this checkpoint leaves the main executable on `60ce7009`; its later e2e Cargo step will build the rebased tree. After run10 finishes, run a fresh focused `n17_live_hook` replay and all-six-engine outputs on the new correction. No final sign-off yet.

- 2026-10-22T16:36:45Z: run10 closed prefix now has 22 completed filter groups, all exit 0, through pre-fix N17; server-shaped rename matrix is active. Fresh source audit on rebased `6ebc499c` confirms the N17 declared-footprint guard (`run10-after-fix-source-audit.log`). Main exe remains pre-fix `60ce7009`; the later Cargo e2e is expected to build the rebased fix. Keep live log/output directory untracked until replay.

- 2026-10-22T16:40:56Z: run10 server-shaped six-profile rename matrix completed (460.09 s, exit 0); all results match the recorded pre-fix R1 refusals, C bare-call and Unicode-prefix edits. Three seam checks are active, then Cargo rebuilds the named existing fold control on the rebased N17 fix. After this command finishes, replay `out/run10-current`, run `run-focused.sh run11-n17 n17_live_hook`, and run existing `variable_reference_boundaries` (includes the declared-callback negative control). Build/run/delete its executable in one command after df check. Do not rebase while the upcoming Cargo build runs. Main log remains untracked; only its closed prefix is committed.

- 2026-10-22T16:46:28Z: all run10 main probes completed: 24 filter groups / 27 observational tests, all exit 0, compiled on `60ce7009`. Oracle replay completed: 98 files / 588 engine executions, 78 generated files / 138 matching checks, zero unexpected result. Existing-fold e2e Cargo rebuild is ACTIVE on `13050f6a` (N17 fix included); do not rebase or start another Cargo command until it finishes. Local commit `13050f6a` has not pushed yet because the build phase started; latest pushed is `13a37436`. After completion update/push full log and all replayed outputs, then run focused new N17 controls and existing variable_reference_boundaries.

- 2026-10-22T16:57:13Z: completed run10 after-fix e2e still fails, 0/1 (DSL 1 [] vs ["5"]). Fresh compile source 13050f6a includes fd5c12bf. Full main + 588-engine replay preserved; all targets deleted.

- 2026-10-22T17:04:56Z: run11 is active on `0e21cada`. Fresh custom live-hook filter completed 2 observational tests (75.36s): all five original controls fold (good/5), and all eight effects/reference combinations now fold good, including undeclared effects. Existing variable_reference_boundaries has three of four tests passed so far, including declared-effect negative control; last template test running. Raw logs durable; tool session handle was unavailable but verified script/test PIDs remain active. Fresh driver oracle replay is active. Prepared future control in final-probe.rs.txt compares the exact existing strlen body with None/Some(tcl8.6), both DSLs and with/without return; tested run11 source is saved separately in run11-source.rs.txt.

- 2026-10-22T17:07:01Z: run11 complete: 2 observational probes, 4/4 existing regressions, 168 oracle executions / 48 matched generated checks, all correct. Explicit dialect now folds undeclared-effect hooks; default None control still needs context comparison. Targets deleted.

- 2026-10-22T17:15:37Z: run12 context/return contrast complete, 1 observational probe / 8 variants all direct None and no O129; C/Jim guarded-body result is 5. 174 engine executions / 48 matched output checks all correct. Next field is installation decline, not another context hypothesis.

- 2026-10-22T17:18:05Z: run12 evidence pushed as fc5aeac1. Run13 installation diagnostic is ACTIVE (`run-focused.sh run13-n17-install n17_control_mylib`) on fc5aeac1. Rebase includes d33ee64c temporary-codegen-switch removal across compiler/registry/install, so fresh libraries are rebuilding; no run13 verdict yet. Keep live build/main logs untracked and do not rebase during build. None of the incoming product paths is R1 command-world completion. After this run inspect installation decline/profile, replay drivers, update/push.

- 2026-10-22T17:33:15Z: run13 completes N17 installation distinction; hook installs successfully, no folded value, consistent with owner missing-provider execution refusal. Six-engine replay 174 executions / 48 matching generated checks, no unexpected result. Next is R1 watch and required final replay; targets deleted.

- 2026-10-22T17:38:52Z: R1 landed in ec80a189/79d4e834. Run14 final replay launched on 49b067ce, rename matrix first; Cargo build active. Published world diagnosis 54/54 subcases, local final behavior still to verify.

- 2026-10-22T18:02:09Z: post-R1 matrix complete on 49b067ce: all C ordinary puts worlds true (coverage still refuses), imports still refuse UnknownWorld, N06 same-document guards now give specific command/member collision reasons on 8.6/9.0/9.1, all-six sibling cases refuse at both origins, N07 safely refuses. Early returned edits replayed: 33 files / 198 engine executions / 12 generated files / 12 matched checks, zero unexpected result. Closed run14-progress.log has 12 completed groups, all exit 0. Remaining final findings still running; live log not committed.

- 2026-10-22T18:18:26Z: run14 completed 19 groups through N15, then deliberately interrupted for a187c859 metadata integration. All 68 generated Tcl scripts executed, 534 engine executions / 78 matched checks / zero unexpected result. Outputs byte-verified into archive to preserve evidence under 5 MB. Grouped latest-tree replay is prepared, not yet run.
