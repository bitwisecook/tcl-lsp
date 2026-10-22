# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T17:15:37Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. This agent commits and pushes it after each run and at least every 30 minutes.

## Resume

**Run7 finished** at 2026-10-22T15:55:08Z on compiled snapshot `0434e7e7`. Every group exited 0: three seams, N17 effect contrast, R1 realm/result/seeding, N03 parser/scalar controls, source chains and P25/50/100 lenses. Temporary source and executable are deleted. Full log: `../reviews/triage-20261022/run7-pending.log`; earlier closed prefix: `run7-progress.log`. Fresh output replay is complete (30 files / 180 engine executions, 10 generated files / 60 matched checks, zero unexpected results). No post-fix rename matrix ran because seeded `puts [foo]` still lacks a completed world.

**Run10 finished** at 2026-10-22T16:55:29Z. Main executable compiled on `60ce7009` completed 24 filter groups / 27 observational tests, all group exits 0. Source is `run10-source.rs.txt` (SHA 0950f3fb…). All findings/seams match reported behavior. Oracle replay: 98 files / 588 engine executions, 78 generated files / 138 matching checks, zero unexpected result. Full closed evidence is `run10-current.log`, `run10-current-oracles.log`, `run10-current-oracle-summary.json` and `out/run10-current/`.

**Critical N17 after-fix result:** the later Cargo e2e rebuilt compiler/dependants on `13050f6a` (includes `fd5c12bf` narrow effect-gate fix), took 13m37s to build, then **0 passed / 1 failed** in 11.70s: DSL 1 expected `["5"]`, got `[]`. All executables/temporary source deleted. This is a fresh post-fix failure, not the old immutable main. No Git HEAD/Rust source mutation during that build. Static after-fix audit is on `6ebc499c`, same const-subst product bytes.

**Run11 finished:** fresh live-hook matrix on `0e21cada` passed 2 observational tests (75.36s): all five original controls fold good/5; all eight DSL/reference/effects combinations fold good, including undeclared effects. Existing `variable_reference_boundaries` passed 4/4 tests in 251.08s, including declared-callback/unclosed-argument negative barriers and templates. All executables/temporary source deleted. The lost tool session handle did not stop the command; process/log monitoring verified completion and the durable `exit-boundaries=0` cleanup footer.

**Run11 replay:** 28 files / 168 engine executions; eight generated folded drivers / 48 matching checks; zero unexpected result. Logs/source are `run11-n17*`, `run11-variable-boundaries.log`, and exact compiled `run11-source.rs.txt` (SHA c94ea9b0…). Evidence disk use is 3.5 MB (keep below 5 MB).

**Run12 finished:** `run12-n17-profile` on `b3c8ea5b` passed one observational test in 73.12s. Exact existing guarded hook body, DSL1/2 × dialect None/Some(tcl8.6) × return/no-return: all eight direct API answers are None and O129 is empty. Dialect/default context is NOT the cause demonstrated by this contrast. C 8.4–9.1 and pinned Jim all evaluate the same guarded callback as `ascii=1|hook=5` (`tcl/n17-ascii-hook.tcl`). Output replay: 29 files / 174 executions, eight generated scripts / 48 matched checks, no unexpected result. Temporary source/executable deleted.

**Next:** commit/push run12, then `source /tmp/review-triage-env.sh; CARGO_NET_OFFLINE=true bash docs/work/name-handling/reviews/triage-20261022/run-focused.sh run13-n17-install n17_control_mylib`. Prepared final-probe adds plan/notices/installation-decline/retained-profile fields to identify whether the guarded callback is declined before invocation; API None alone does not settle that. Run11 proves ordinary explicit-dialect hooks and absent-effect hooks now fold correctly and declared callback barriers remain enforced (4/4 product regressions). No Cargo process is active. Keep evidence under 5 MB (currently about 3.6 MB).

**After run13:** replay generated outputs, update report, commit/push. All independent seam/scaling items are settled. Await R1, then run complete latest-work semantic/rename replay and static/oracle audits before writing the final summary. R1 remains blocked in the latest compiler-names report; no final summary yet.

**N09 retry:** `run9-n09` on `128e1bb4` passed its one observational test (64.04s). Type/call graph correct; taint dependencies retain all three source ProcedureIds conservatively, rather than a wrong singleton. Taint lattices/warnings are unverified. Initial failed run8 used a private helper and executed zero tests; exact source/logs preserved. Run10 repeats the corrected result.

**Remaining finish condition:** after compiler-names' R1 fix lands, rerun N04/N05/N06/N07 on all six profiles, apply edits and execute them on all six oracles (judge profile-specific edits against the matching grammar). Then recheck every finding/static seam on latest `work`; `run-final.sh` and `final-probe.rs.txt` are prepared for that semantic replay, including a direct N09 taint dependency projection. Run source audit and output replay for that snapshot. No final summary until those requirements are met.

**Completed run6:** 24 probe functions completed on `5bd5a503`; a 3600 s cap was reached during scaling before the three seams. Flat P25/50/100/200 and chain F250/500/1000/2000 completed; lens P25 completed. Later existing-fold e2e rebuilt on `90fe96e2` and failed (DSL 1 expected `["5"]`, actual `[]`). All executables and temporary source were deleted. `run6-oracles.log` covers 96 files / 576 engine executions, 76 generated files / 126 matching-profile checks, zero unexpected matching results. All eight N17 drivers print `good` on all engines. `run6-source-audit.log` audits `90fe96e2`. These are different explicit snapshots; a complete final latest-work check is not yet claimed.

**N17 settled:** direct hooks return `good`; hooks with `world_effects none` fold to `good`, undeclared effects suppress folding, and the existing strlen control fails. Compiler-codegen owns `const_subst.rs` compatibility gate correction. Triage must recheck latest, not implement it.

**Checkout/environment:** `/workspace/tcl-lsp-triage`, shallow selective `work`, user-authorised. Source `/tmp/review-triage-env.sh`, set `CARGO_NET_OFFLINE=true` for warmed builds. Rust 1.99.1 and pinned C 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 plus Jim 0.84 are ready. The three required embedded fixture trees are restored; old run5 failed before its main/scaling probes for missing includes. Stable existing-fold failure is in `run5-e2e-repeat.log`; do not infer its cause from the incomplete initial run5 e2e log. No product/test edits are authorised. `make rust-check` and `make prep-pr` remain unrun under the active development-push exception.

Before every push fetch/rebase `origin/work`, retry rejection, never force or open a PR. Commit only this file, the triage report and evidence, using Codex attribution. Temporary probe is never staged. Compiler-names and compiler-codegen tracking files are read-only to this role.

## In progress

- Run11 verifies corrected explicit-dialect hooks and all 4 existing variable-boundary regressions. Exact guarded body fails with both None/explicit contexts; hook installation diagnostics are next. R1 remains blocked; final post-R1 replay remains outstanding.
- Required release gates `make rust-check` and `make prep-pr` are not run; this is evidence-only work under the active `work` development-push exception.

## Outstanding (in priority order)

1. Wait for/fetch compiler-names R1; the independent N09 taint projection and all requested seam/scaling cases are settled.
2. After R1 lands, re-check N04, N05, N06 and N07 on all six profiles, apply returned edits and execute them on the matching engines.
3. Fetch latest `work` and re-check every finding and static seam before the final summary.

## Blocked (on whom, and what is needed)

- N04, N05, N06 and N07 are blocked on R1 (compiler-names).

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
