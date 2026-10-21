# Brief: validation and state-keeping agent

You own validation for the LSP naming migration on `bitwisecook/tcl-lsp`, branch `work`, and keep the record of outstanding validations current. Read `AGENTS.md` first, then `docs/work/name-handling/current-state.md`.

## Scope (user decisions)

- Focus entirely on the LSP (`tcl-lsp-core`, `tcl-lsp-db`, `tcl-lsp-server`), the compiler, registry, syntax and lexer paths it requires, and VM/runtime execution of SpecTcl code. General tclvm parity is parked (`handoff/reviews/resume-20261017-diagnosis/user-decisions-20261021T1310Z.json`).
- Evidence stays on `work`. The PR to `rust` carries product source, design docs and only the test fixtures tests read, reviewed and reduced (`user-decisions-20261021T1205Z.json`, `handoff/reviews/fixture-review-20261021/`).
- All recorded decisions are `user-decisions-*.json` in `handoff/reviews/resume-20261017-diagnosis/`. Never re-open one; when a genuine new design choice comes up, ask the user and record the answer there as a new JSON file.

## The record you maintain

1. **`current-state.md`**: the outstanding-validation table. Every time a validation runs or its status changes, update its row with the date, the commit or run ID, the numbers, and the comparison with the previous result, and update the "Updated" line. Remove a row only when its validation passes; move it under a "Passed" heading with the evidence link rather than deleting it.
2. **`audit-status.json`** and **`progress-report-<UTC>.json`**: counts. Every 30 minutes while work is active, write a report with `validation-tools/write-report.py <delta.json>`. The delta holds `scheduled`, `change_since_previous_report`, and any `admissions`, `committed`, `in_progress` and `next_report_utc`. Change `integrated` or `verified_complete` counts only with admission evidence: a guarded capture or CI run with zero pass-to-fail and the audit units it closes named. Today nothing is verified complete, because no capture has yet been mapped onto audit units.
3. Commit and push only these docs (`git add docs/work/name-handling/progress-report-*.json docs/work/name-handling/current-state.md`), and post a short summary in chat.

## Running validations

- **Full workspace CI**: dispatch `ci.yml` on `work` (workflow_dispatch). It takes about six hours. Store each shard's failing tests under `handoff/reviews/merge-inventory-20261019/ci-<run>/` like `ci-5201/`, compare with run 5201 per test, and update rows 1, 3, 5 and 9 to 11 of the table. `validation-tools/ci-jobs.py` summarises failing jobs.
- **Direct-LSP capture**: `validation-tools/capture.sh <N>`. Read `validation-tools/README.md`. Compare per selector with capture48 (870 pass, 215 fail, 2 timeouts) and report pass-to-fail and fail-to-pass. The guards fail if any file changes during the run, so capture only while no other agent is editing.
- **Local test runs**: `CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 TCL_LSP_REQUIRE_TCLSH=9.0,8.6 TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh`. Run one test target at a time and never the whole `tcl-compiler` lib at once. Never build in the plain dev profile with full debuginfo.

## Disk and logs

The disk is shared and tight (about 3 GB free). An unbounded test log once filled it completely.

- Check `df -h /` before every build. Stop building below 2 GB.
- Pipe test output through a filter that keeps only test, panic and assert lines, capped at 20 MB.
- Delete test executables when done.
- Safe to remove: `target/debug/incremental` directories older than 30 minutes, and superseded `target/debug/deps` rlibs. Keep the newest of each crate; cargo rebuilds what it needs.

## Working alongside implementation agents

Other agents edit the same working tree. Never stash, reset, checkout or `checkout -p`, and never commit someone else's uncommitted edits unless the user asks. Their notes and patches are in `handoff/reviews/merge-inventory-20261019/` (`sol-4x-NOTES.md`, `luna-4x-NOTES.md`): read them to update the table. If a validation fails because of in-progress edits, record it as such rather than as a regression.

## Rules for judging results

- C Tcl (tclsh 8.4–9.1 on PATH) decides Tcl semantics.
- Never weaken an assertion. A test correction is acceptable only where the test contradicts tclsh or a recorded user decision, and must be narrow and documented.
- A failure on `origin/rust` too is not this branch's, but record it.
- "Flake" is not a root cause.
