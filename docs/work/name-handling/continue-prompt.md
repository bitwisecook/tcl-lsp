# Brief: validation and state-keeping agent

**USER PAUSED (2026-10-27T20:38:21.853504+00:00).** No implementation, builds, dispatches or reminders until a new resume instruction. Read [the authoritative current checkpoint](pause-handoff-20261027.md) before older instructions below. No PR is open; source cleanup44/51 and all final gates remain outstanding.

You own validation for the LSP naming migration on `bitwisecook/tcl-lsp`, branch `work`, and keep the record of outstanding validations current. Read `AGENTS.md` first, then `docs/work/name-handling/current-state.md`.

## Scope (user decisions)

- The 35 dedicated BIG-IP/F5 LSP units are now in scope and being implemented in parallel (`user-decisions-20261021T1500Z.json`).
- Focus entirely on the LSP (`tcl-lsp-core`, `tcl-lsp-db`, `tcl-lsp-server`), the compiler, registry, syntax and lexer paths it requires, and VM/runtime execution of SpecTcl code. General tclvm parity is parked (`handoff/reviews/resume-20261017-diagnosis/user-decisions-20261021T1310Z.json`).
- Evidence stays on `work`. The PR to `rust` carries product source, design docs and only the test fixtures tests read, reviewed and reduced (`user-decisions-20261021T1205Z.json`, `handoff/reviews/fixture-review-20261021/`).
- All recorded decisions are `user-decisions-*.json` in `handoff/reviews/resume-20261017-diagnosis/`. Never re-open one; when a genuine new design choice comes up, ask the user and record the answer there as a new JSON file.
- **Release gate** (decision 2026-10-22T11:00Z): the PR to `rust` opens only when the external review's section F checklist is met ([reviews/naming-migration-review-20261022.md](reviews/naming-migration-review-20261022.md)) in addition to zero pass-to-fail against origin/rust. Per-finding status is in `reviews/naming-migration-review-20261022-triage.md`; mirror the checklist in `current-state.md`.
- Other decisions since 21 Oct: providers abstain for force-import shadows a sibling exports (2026-10-22T07:18Z); the owner-resolution manifest lists only genuinely shared entries (2026-10-21T19:00Z).

## The record you maintain

1. **`current-state.md`**: the outstanding-validation table, the release-gate checklist and an "agent work in progress" table. Validated rows change only with a recorded validation; the agent table carries agent-reported numbers, clearly labelled as not validated. When an agent reports a change, re-measure it before moving a row. Every time a validation runs or its status changes, update its row with the date, the commit or run ID, the numbers, and the comparison with the previous result, and update the "Updated" line. Remove a row only when its validation passes; move it under a "Passed" heading with the evidence link rather than deleting it.
2. **`audit-status.json`** and **`progress-report-<UTC>.json`**: counts. Every 30 minutes while work is active, write a report with `validation-tools/write-report.py <delta.json>`. The delta holds `scheduled`, `change_since_previous_report`, and any `admissions`, `committed`, `in_progress` and `next_report_utc`. Change `integrated` or `verified_complete` counts only with admission evidence: a guarded capture or CI run with zero pass-to-fail and the audit units it closes named. Today nothing is verified complete, because no capture has yet been mapped onto audit units.
3. **Push progress while a run is in progress** (user decision 2026-10-21T15:00Z): while a CI run or capture is running, update its row and push at least every 30 minutes and after each completed group, shard or target, with the partial counts so far. Do not wait for the run to finish.
4. Commit and push only these docs (`git add docs/work/name-handling/progress-report-*.json docs/work/name-handling/current-state.md`), and post a short summary in chat.

## Running validations

- **Full workspace CI**: dispatch `ci.yml` on `work` (workflow_dispatch). It takes about six hours. Store each shard's failing tests under `handoff/reviews/merge-inventory-20261019/ci-<run>/` like `ci-5201/`, compare with run 5201 per test, and update rows 1, 3, 5 and 9 to 11 of the table. `validation-tools/ci-jobs.py` summarises failing jobs.
- **Direct-LSP capture**: `validation-tools/capture.sh <N>`. Read `validation-tools/README.md`. Compare per selector with capture48 (870 pass, 215 fail, 2 timeouts) and report pass-to-fail and fail-to-pass. The guards fail if any file changes during the run, so capture only while no other agent is editing.
- **Local test runs**: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`. Run one test target at a time and never the whole `tcl-compiler` lib at once. Never build in the plain dev profile with full debuginfo.

## Disk and logs

The disk is shared and tight (about 3 GB free). An unbounded test log once filled it completely.

- Check `df -h /` before every build. Stop building below 2 GB.
- Pipe test output through a filter that keeps only test, panic and assert lines, capped at 20 MB.
- Delete test executables when done. Test executables are shared: every agent's `cargo test -p <crate> --lib` writes the same `target/debug/deps/<crate>-<hash>` file. Build and run in ONE `cargo test` command, and delete the executable only in that same command after your run; never rely on an executable built by an earlier command.
- Safe to remove only when **no build is running** (`[ -n "$(pgrep -x rustc; pgrep -x cargo)" ]`; plain `pgrep -f` matches its own command line): `target/debug/incremental` directories older than 30 minutes, and superseded `target/debug/deps` rlibs, keeping the newest of each crate. Deleting rlibs under a running build breaks it. Never delete a test executable younger than 60 minutes, even if no process is running it: agents run several filters from one binary, and it is idle between runs.
- `/dev/shm` (RAM-backed) can host a small isolated `CARGO_TARGET_DIR`; keep it under 3 GB and watch `free -g`.
- If the container restarts, every agent and build stops; uncommitted edits survive. Snapshot the tree, prune, then resume agents.

## Working alongside implementation agents

Other agents edit the same working tree. Never stash, reset, checkout or `checkout -p`. By user instruction the coordinator commits and pushes the whole tree as a WIP snapshot every 30 minutes (green is only required for the PR); snapshots exclude live temporary probe sources (`**/zz_*probe*.rs`); agents keep `.rs.txt` evidence copies in the repo. Some agents run in their own cloud sessions (see `outstanding/README.md`) and push to `work` themselves, rebasing first. Validate a named snapshot commit, not a moving tree, and say which.

Current implementation agents (by role): **lsp** (tcl-lsp-core/db/server generic), **compiler** (lowering, C6, iRules realm, itcl/snit, OO), **compiler-names** (command_binding.rs except the C6 hunks, source_name_lookup, analyser scope/commands, nested lookups), **compiler-codegen** (compile_service, codegen, compiled_preflight, tcl-spectcl install.rs, review N17), **vm-runtime** (tcl-vm, runtime/rust, tcl-spectcl, tcl-spec-studio, SpecTcl acceptance), **bigip-lsp** (editors/, BIG-IP crates and modules), **perf** (analysis and pack-load cost), **review-triage** (external review replay). Finished: gates, contracts. Each agent keeps its outstanding work in `outstanding/<role>.md` (index `outstanding/README.md`); read those before re-assigning or validating work. Their notes and patches are in `handoff/reviews/merge-inventory-20261019/` (`sol-4x-NOTES.md`, `luna-4x-NOTES.md`): read them to update the table. If a validation fails because of in-progress edits, record it as such rather than as a regression.

## Rules for judging results

- C Tcl (tclsh 8.4–9.1 on PATH) decides Tcl semantics.
- Never weaken an assertion. A test correction is acceptable only where the test contradicts tclsh or a recorded user decision, and must be narrow and documented.
- A failure on `origin/rust` too is not this branch's, but record it.
- "Flake" is not a root cause.
