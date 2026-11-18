# Coordinator handoff at the pause, 18 November 2026, 11:50 UTC

The owner said "pause here". Every working session was told to stop, save unfinished work as patches,
bring its ledger up to date, write a handoff beside this file and push. The hourly check
(`trig_01PEuFxF1AEeNAVJYTFw4nnC`, "Burn-down session check", cron `52 * * * *`, bound to the
coordinator session `session_017u5g3KpKgjfMZcPMj8rx4F`) is **disabled**; resuming means enabling it
again or creating its equal for a new coordinator session.

## Where to read first

1. `docs/work/name-handling/current-state.md`: scoreboard and every coordinator ruling, newest
   wording in place.
2. `docs/work/test-scrub/README.md`: the scrub's method, rulings made during it, the wrong-answers
   table, the owner-approval table, the status of the ledger rows.
3. `docs/work/name-handling/handover/successors.md`: roster with session ids and models, and the
   standing rules (oracles required, gates, fmt and clippy before a push, clean-head baselines,
   disk).
4. This directory: one handoff per session, and `patches/` for work that was not ready for `work`.

## State in one screen

- Goal, unchanged: the lsp, compiler, runtimes, VMs and tooling on the shared naming
  infrastructure, tests matching C Tcl 8.4 to 9.1 and Jim, custom parsers removed, overlapping
  tests collapsed, BIG-IP checks in `docs/work/name-handling/bigip-name-checks.md`.
- Since 17 November 20:15 UTC the priority is the test scrub. First pass complete: thirteen
  ledgers signed off, 3,393 rows. At the pause: see "Status of the rows" in the scrub README.
- `tcl-compiler`: 1,891 failing of 13,154 at 08:55 on 18 November, the same set with and without
  the oracles required. Other counts are on the scoreboard with their times.
- Wrong answers go before missing facts. Fixed on 18 November among others: the `proc add` to
  `return 7` rewrite, the 8.4 `args` leading-`#` rewrite, a class recorded after a proved error,
  the `switch -mode --` lowering, both brace-span faults, Incoming Calls inside a method.
- `release_optimiser_safety` is 69 of 69. `make xtask-gates` is 39 of 39 on every push reported
  today.

## Open wrong answers, in the order I would resume them

| Wrong answer | Owner | State |
| --- | --- | --- |
| O129 rewrites `set x [split ""]` to `set x {{}}` (#2418 back) | dataflow-optimiser | routed 11:48, not started; witness on `work` |
| Strict local rename past a computed head that may `upvar` the local | diagnostics | routed 11:38; fixtures with lsp-features |
| I230 "always true" after a body-role stub call | diagnostics | routed 11:47; witness on `work` |
| O101 `info exists` fold in a procedure called twice | analyser-binding, behind the `dataflow` issue session | the declared-frame switch fixes it but breaks 66 tests; collector gaps with the `dataflow` issue session (`docs/work/test-scrub/declared-frame-switch.md`) |
| O103 folds a call although an unknown command may rebind the callee | dataflow-optimiser, root with analyser-binding | blocked on analyser-binding |
| After `rename C M`, `C create b` in `catch` still bound to the class | analyser-binding | queued |
| A global stays stale after a call that writes it through `global` | analyser-binding | queued next; tk-oo's `eval` fix waits on it |
| `rename bump bump2` leaves trust and revision unchanged | `dataflow` issue session writes, analyser-binding reviews | handed over 08:10 |
| Two `tcl-compiler` tests fail only in parallel (shared state between analyses) | analysis-performance finds the cache | not started |
| Four bytecode mismatches against tclsh | execution | queued |
| W210 where Jim raises at the call for `{p p(key)}` | diagnostics | witness red |
| 25 `tcl-irule-test` tests: an authored provider seals words with a native 8.4 protocol | execution | ruled: split source escape grammar from value representation, prototype first |

## Rulings made on 18 November that the owner should confirm

Each is recorded where it applies; none is hard to reverse.

1. The iRule simulator keeps its Tcl 9 host engine; 8.4 behaviour comes only through installed
   simulation hooks; source escape grammar and value representation are split
   (`current-state.md`, the simulator bullet).
2. A widget or object instance is published only where its factory's package is proved present
   (scrub README; tk-oo writes it, not landed).
3. A real error through an alias-captured word is reported at the alias definition
   (contract `diagnostic-registry-source-ownership.md` amended; landed in `00e24edd`).
4. Nine `switch` to `dict` refactor tests that were red by design are restated to a disabled action
   with its reason, and the contract sentence that kept them red is amended (lsp-features, not yet
   done).
5. A dialect-only flip does not move the expression language; a release-to-release flip moves engine
   and dialect together; surviving state the new engine cannot represent is refused; a pure cache is
   re-derived.

## Waiting on the owner (each in the session named, because its permission check asked)

- irules-registry-tooling: the test-body edits listed under "Waiting on the owner's approval" in
  the scrub README (five originals, six merges, the seven-test rewrite, eight `tcl-diagram`
  fixtures, the unowned-crate merges and rewrites, one Jim `pkgIndex` fixture).
- live-inlining: switching `LIVE_REFERENCE_INLINING` on and changing the guard test
  `the_live_compile_splices_no_reference_body`; the case is in `docs/work/test-scrub/live-inlining.md`.

## Measurement holes found on 18 November, not yet closed

- `tcl-lsp-core` integration targets `it_2` to `it_5` (about 1,254 tests) were never measured;
  lsp-features was running them when the pause came.
- About 25 workspace crates appear in no failing list and no ledger; irules-registry-tooling was
  sweeping them (`docs/work/name-handling/lists/unmeasured-crates-2026-11-18.md`, as far as it got).
- `tcl-lsp-db` integration: `it_5` does not finish in hours; `interned_gc` takes 948 s for three
  tests. server-e2e-db was timing each test.
- The ledger truth pass (re-running the rows of analyser-binding, class-facts and tk-oo and marking
  the ones that now pass) had not started.

## In flight between sessions

- `dataflow` issue session (orchestrator `session_017u5g3KDVBH5v5PXpMYy7KP`, not a burn-down
  session): declared-frame collector gaps (estimate was five to six hours from 10:33 UTC), then the
  two analyser items for #2408. Its worker drops ordinary messages; go through the orchestrator.
- `audit` issue session: registry facts for when a retained callback can run, to be agreed with
  analyser-binding and tk-oo before it writes the `requires_reentry_guard` walk change; it unblocks
  six `callback_replay` tests and taint's 14 Tk replay rows.
- taint: advisory solver mode (A), prototype on `incr` with a byte-for-byte differential of
  `sccp.values`; dataflow-optimiser must approve, values reviews.
- live-inlining: ClausePlan lowering written, waiting on analyser-binding's review.
- tk-oo: the package-presence rule, to land together with the fixture repairs.
- #2543: remaining 2.7 MB per round being attributed by the server issue session.

## How messages behaved

Cross-session messages were lost or delivered hours late several times (execution received six
at once). A ruling is real when it is in `current-state.md` or the scrub README; tell a session
that is waiting to read the file.

## Resuming

1. Read the thirteen session handoffs here and check each named patch file exists.
2. Enable the hourly check, or create one bound to the new coordinator session with the same prompt.
3. Tell each session to resume from its handoff file, re-apply its patches on a fresh
   `origin/work`, rebuild Jim (`bash scripts/dev/ensure-jim-oracle.sh`), and take a clean-head
   baseline before quoting any count.
4. First work: the open wrong answers above, then the measurement holes.

## Confirmed at 12:05 UTC: every session has stopped and handed off

All thirteen files are on `work` beside this one, and every patch a handoff cites exists (here
under `patches/`, or under `docs/work/name-handling/patches/` for the two older ones).

| Session | Handoff commit | Unfinished work saved as |
| --- | --- | --- |
| analyser-binding | `6d9b4cbc9` | `patches/analyser-binding-declared-frame-switch.patch`, `-namespace-unknown`, `-operator-errors`, `-proc-rename-trust` |
| class-facts | `41b9181d` | `patches/class-facts-parked-lenient-recipe-and-bodiless-receipt.patch`, `-probe-test`, `-rejected-labels-complete`, `-unmarked-switch-subject` |
| diagnostics | `286ce2ebe4` | none, tree clean |
| dataflow-optimiser | `922a28f23d` | `patches/dataflow-optimiser-lifecycle-fixture.patch` |
| values | `dbd1be232` | none, nothing in flight |
| taint | `e0d095be8` | eight `patches/taint-*.patch` (rule-procedure advice, Tk callback replay, `entry_after` probes, others) |
| execution | `e6014fb9` | `patches/execution-c86-optimizer-and-else-jump.patch` |
| analysis-performance | `961a7f9a3` | `docs/work/name-handling/patches/handover-analysis-performance-body-wrapper-realm.patch` |
| lsp-features | `f499969f` | none; the `it_2` to `it_5` run had not started |
| server-e2e-db | `87cfef554` | none, tree clean |
| irules-registry-tooling | `e6651283` | `patches/irules-registry-tooling-spectcl-test-scratch-guards.patch`, `unmeasured_sweep.py.txt`, `ledger_truth_pass.py.txt` |
| tk-oo | `8aeda0817` | `patches/tk-oo-package-presence-rule.patch`, `patches/tk-oo-eval-proved-constant.patch` |
| live-inlining | `938b2c9a` | `patches/live-inlining-clauseplan-lowering.patch`, `-clauseplan-release-surface` |

Corrections to what is written above, learned after it was written:

- dataflow-optimiser says it never received two messages: the `split ""` wrong rewrite (O129,
  #2418 back; witness `procedure_summaries::split_of_the_empty_string_is_the_empty_list`) and the
  request to approve taint's advisory solver mode. Its handoff says so. Give it both again at resume.
- lsp-features had not started measuring `tcl-lsp-core` `it_2` to `it_5`. That hole is wholly open.
- The issue sessions (`dataflow`, `audit`, `server`, `bytecode`) were paused by the owner at the
  same time; their handovers are in `docs/work/issue-sessions/handover/`. The declared-frame
  collector work is on branch `issues/dataflow-2` at `429ef5da`, unverified: its view fix passes 6
  of the 65 regressions with the switch applied, the other 59 need a merge that is built and not
  measured. The orchestrator offers it to analyser-binding to take over. The rename-trust and
  `namespace eval` barrier items it was cleared to write were not started and are free again.
- #2543's two closing conditions (per-database table, evict one entry at the cap) did land, in
  `2a026629`; what remains is the 2.7 MB per round residue.
- The hourly check `trig_01PEuFxF1AEeNAVJYTFw4nnC` is disabled.
