# Outstanding work: review-triage agent

Area: Replay of the external review's findings on the current tree.

Last updated: 2026-10-22T16:36:45Z

Maintained by the review-triage agent. It lists what this agent considers it must still complete. Update it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes while working. This agent commits and pushes it after each run and at least every 30 minutes.

## Resume

**Run7 finished** at 2026-10-22T15:55:08Z on compiled snapshot `0434e7e7`. Every group exited 0: three seams, N17 effect contrast, R1 realm/result/seeding, N03 parser/scalar controls, source chains and P25/50/100 lenses. Temporary source and executable are deleted. Full log: `../reviews/triage-20261022/run7-pending.log`; earlier closed prefix: `run7-progress.log`. Fresh output replay is complete (30 files / 180 engine executions, 10 generated files / 60 matched checks, zero unexpected results). No post-fix rename matrix ran because seeded `puts [foo]` still lacks a completed world.

**N09 retry finished:** `run9-n09` on `128e1bb4` compiled and passed its one observational test (64.04 s). Constructor/call graph remain correct; taint `resolved_callees` for `::n::c` conservatively retains all three ProcedureIds (`::p`, `::lib::p`, `::n::c`); both leaves have no dependencies. This is a widened may-dependency answer, not a wrong singleton or precise taint proof. Temporary source/executable deleted. Initial failed `run8-n09` source/logs retain the E0624 probe error. This result is pushed as `60ce7009`. **Run10 current-tree replay is active:** `run-final.sh run10-current` launched on `60ce7009`, replaying all semantic findings and seams after intervening compiler/LSP changes; it is an interim check while R1 is still blocked, not final sign-off. Logs are `run10-current-build.{json,log}` and `run10-current.log`; keep the live main log untracked. A closed checkpoint is `run10-progress.log`; completed groups through N16 remain unchanged. The first wrapper launch failed before build for a joined setup line; it was corrected, and the actual run built in 11.27 s. `run10-startup.log` preserves that non-product failure. Exact current compiled source (SHA `0950f3fb…`) is saved as `run10-source.rs.txt`. The prepared future `final-probe.rs.txt` now omits the ignored whole-Module Debug occurrence count; the immutable running executable is unchanged. Source/executable cleanup is handled by the runner. At finish, replay generated outputs and audit current source, update report and push. If it exceeds 30 minutes, commit a closed progress snapshot after the .rs is deleted, never the live log. The final complete semantic replay is `source /tmp/review-triage-env.sh; CARGO_NET_OFFLINE=true bash docs/work/name-handling/reviews/triage-20261022/run-final.sh <new-tag>`. It must be followed by source audit, six-engine output replay and a fresh fetch check. Do not call an interim pre-R1 replay final sign-off.

**Remaining finish condition:** after compiler-names' R1 fix lands, rerun N04/N05/N06/N07 on all six profiles, apply edits and execute them on all six oracles (judge profile-specific edits against the matching grammar). Then recheck every finding/static seam on latest `work`; `run-final.sh` and `final-probe.rs.txt` are prepared for that semantic replay, including a direct N09 taint dependency projection. Run source audit and output replay for that snapshot. No final summary until those requirements are met.

**Completed run6:** 24 probe functions completed on `5bd5a503`; a 3600 s cap was reached during scaling before the three seams. Flat P25/50/100/200 and chain F250/500/1000/2000 completed; lens P25 completed. Later existing-fold e2e rebuilt on `90fe96e2` and failed (DSL 1 expected `["5"]`, actual `[]`). All executables and temporary source were deleted. `run6-oracles.log` covers 96 files / 576 engine executions, 76 generated files / 126 matching-profile checks, zero unexpected matching results. All eight N17 drivers print `good` on all engines. `run6-source-audit.log` audits `90fe96e2`. These are different explicit snapshots; a complete final latest-work check is not yet claimed.

**N17 settled:** direct hooks return `good`; hooks with `world_effects none` fold to `good`, undeclared effects suppress folding, and the existing strlen control fails. Compiler-codegen owns `const_subst.rs` compatibility gate correction. Triage must recheck latest, not implement it.

**Checkout/environment:** `/workspace/tcl-lsp-triage`, shallow selective `work`, user-authorised. Source `/tmp/review-triage-env.sh`, set `CARGO_NET_OFFLINE=true` for warmed builds. Rust 1.99.1 and pinned C 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 plus Jim 0.84 are ready. The three required embedded fixture trees are restored; old run5 failed before its main/scaling probes for missing includes. Stable existing-fold failure is in `run5-e2e-repeat.log`; do not infer its cause from the incomplete initial run5 e2e log. No product/test edits are authorised. `make rust-check` and `make prep-pr` remain unrun under the active development-push exception.

Before every push fetch/rebase `origin/work`, retry rejection, never force or open a PR. Commit only this file, the triage report and evidence, using Codex attribution. Temporary probe is never staged. Compiler-names and compiler-codegen tracking files are read-only to this role.

## In progress

- Run7 and N09 retry are complete. Latest fetched `f93058f0` contains compiler-names g4 evidence: the aggregate test still fails ten positive subcases, so R1 remains blocked. An interim all-finding replay is active on `60ce7009`; the required post-R1 final replay remains outstanding.
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
