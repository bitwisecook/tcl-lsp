# Issue sessions, from 17 November 2026

These sessions close the open issues filed by the repository owner. They run beside the naming burn-down (`docs/work/name-handling/handover/successors.md`) and are separate from it: an issue session owns its issues, a burn-down session owns its area's failing tests. The sessions that worked on these issues until 16 November were paused, wrote the hand-overs in `handover/`, and were replaced on the owner's instruction.

The orchestrator is session `session_017u5g3KDVBH5v5PXpMYy7KP`. It records the owner's design decisions on the issues, rules on questions, and does not own area code.

## Paused, 18 November 2026, 12:00 UTC

The owner paused this work and the burn-down together. Every issue session stopped, pushed its work in progress to a branch, and committed a hand-over. Nothing below is running. A resumed or fresh session starts from its hand-over, not from the roster's older notes.

| Slug | Open issues | Hand-over | Work in progress (unverified unless the hand-over says otherwise) | Paused session |
| --- | --- | --- | --- | --- |
| `dataflow` | #2408, #1796 | `handover/dataflow.md` (84f80e862c) | `issues/dataflow-2` at 429ef5dae2 | `session_017u5g3JkH4kppJ1LJpZQUus` |
| `audit` | #1710 | `handover/audit.md` (886f3c683a, e4f6121629) | `issues/audit-22` at 279875ba60 | `session_017u5g3MNKJTTq44wzXNszit` |
| `server` | #2543, #2243 | `handover/server.md` (68ba57b738, 5b952660e2) | `issues/server-2` at 673e7b9403 | `session_017u5g3HDHiPN1CimB7QiVSF` |
| `bytecode` | #2545 | `handover/bytecode.md` (d0e255fed) | `issues/bytecode-3` at 7a7f1434a9 | `session_017u5g3KBvRVWYHVoF5FSZmT` |

Closed since 15 November: #2437, #1755, #1830, #2528, #2246, #2207, #1778, #2427, #2129, #1714, #1734, #2490, #2426, #2542, #1797, #1657, #1712, #2544. Opened on the owner's instruction: #2542 and #2544 (closed), #2543 and #2545 (open).

What waits on a burn-down session, which is paused too (`docs/work/name-handling/handover/pause-2026-11-18/`):

- `dataflow`: `analyser-binding`'s review of the declared-frame view fix and its parked switch patch (the wrong `info exists` fold stays on `work` until the switch lands at new 0); the rename-as-command-mutation and `namespace eval` barrier rows, which the coordinator cleared this session to write with `analyser-binding` reviewing.
- `audit`: `analyser-binding`'s review of the walk half of the callback fix for #1710; its two answers on the fact shape are in the hand-over. `taint` has two replay-side fixes parked behind it.
- `server`: `analysis-performance` on #2543's two closing conditions (the retained-module table clears everything at 512 entries; entries outlive their database) and on #2243 (`lassign` in a nested body ends the walk, with `analyser-binding`).

Settled by the owner on 18 November:

- #2546 is open for document-free requests that take up to 18–20 s while 183 documents are re-analysed. It belongs to the `server` session, after #2543.
- #2547 is open for the `tcl-spectcl-cache-test-*` directories tests leave in the temp directory. No session has it yet; `tcl-spectcl` is the burn-down's `irules-registry-tooling` area.
- F5: the 36 builtins pure under Tcl 8.4 stay impure for `f5-irules` until device check 33 is complete (recorded on #1712).
- Stale `issues/*` branches: the owner deletes them, since no session here can. Every tip as it stood at the pause is in `branch-tips-2026-11-18.txt`. Keep `issues/dataflow-2`, `issues/audit-22`, `issues/server-2` and `issues/bytecode-3`; all others hold only work that has landed.

Still not for these sessions: #2324, #1956, #1655 (the burn-down's `tk-oo`); #2151, #2165 (BIG-IP device checks 11 and 12).

Lessons from this round, for whoever resumes:

- Verify every reported push yourself: `git fetch origin work && git merge-base --is-ancestor <sha> origin/work`, in a clone deepened far enough (`git fetch --shallow-since=<date> origin work`). One session reported two pushes that had been rejected; another read a shallow clone's boundary as rewritten history.
- A session's worker can restart and lose ordinary messages. Send anything urgent with priority `now`, and have sessions call ReadNotifications at the start of each turn.
- `work` moves about a hundred commits an hour while the burn-down runs. Prove a change once on a fixed base, then land with a rebase and targeted checks; a full re-run after every rebase never converges.
- Tests leave `/tmp/tcl-spectcl-cache-test-*` directories behind (about 8 GB in one session). Delete them when disk runs short.

## Roster (as started on 17 November)

| Slug | Issues | Model | Session | Hand-over |
| --- | --- | --- | --- | --- |
| `bytecode` | #2490 and #2542 (closed), then #2544, #2545 | Opus | `session_017u5g3KBvRVWYHVoF5FSZmT` | `handover/bytecode.md` |
| `server` | #1657 (closed), #2243, #2543 | Opus | `session_017u5g3HDHiPN1CimB7QiVSF` | `handover/infra.md` |
| `dataflow` | #2408, #1796 | Opus | `session_017u5g3JkH4kppJ1LJpZQUus` | `handover/dataflow.md`, and the #1796 part of `handover/irules-taint.md` |
| `audit` | #1712 (closed), #1710 | Opus | `session_017u5g3MNKJTTq44wzXNszit` | `handover/audit.md` |
| `generations-payloads` (finished 17 November) | #1797, #2426, both closed | Sonnet | `session_017u5g3KGtKSUFqBEdo1S6kR` | `handover/generations-payloads.md` |

Not in this roster:

- #2324, #1956 and #1655 belong to the burn-down's `tk-oo` session (`docs/work/name-handling/handover/tk-oo.md`).
- The held taint work in `handover/irules-taint.md` (the concatenating-`eval` fix and the fixtures moved to named releases, branch `issues/taint-barriers`) goes to the burn-down's `taint` session through the burn-down coordinator. #1710 cannot close while `taint::tests` fails, so the `audit` session tracks it.
- #2151 and #2165 need a BIG-IP device: checks 11 and 12 in `docs/work/name-handling/bigip-name-checks.md`.

## Starting

1. Read `AGENTS.md`, `docs/work/name-handling/fixing-guide.md`, the section *Rules that were learned the hard way* in `docs/work/name-handling/handover/successors.md`, then your hand-over and the comments on your issues. Decisions recorded on an issue by the owner or the orchestrator are settled.
2. The hand-overs were written before the file split of 16 November (`docs/work/file-split/README.md`): every hand-written Rust file is now at most 2,000 lines, items moved without being renamed, and test paths gained a module. A file a hand-over names may now be a facade over `name/<topic>.rs`: find an item with `grep -rn 'fn <name>' rust/<crate>/src`.
3. Held commits are on the remote branches the hand-overs name. They were cut before the split and do not rebase: re-apply each hunk in the module the item moved to (`git show <sha>` for the diff), then verify it as new work.
4. Take a baseline of your suites on the current `work` before changing anything. The burn-down's lists are in `docs/work/name-handling/lists/`; compare by the last path segment of the test name.

## Rules

- The burn-down's rules apply unchanged: named Tcl releases in tests (8.4 to 9.1), the tclsh oracle in the test or the commit message, one cause per commit, no weakened assertion, no `#[ignore]`, no `#[allow]`, no known-failure list, names read through `tcl_syntax::naming`.
- `work` is never force-pushed. Land by fast-forward after a rebase, then `cargo check` and clippy on what you touched and its dependents, and `make xtask-gates`. Push work in progress to `issues/<slug>-2`.
- A change in a file a burn-down area owns is announced to that area's session first (roster in `successors.md`), and is one small separate commit. A fact another area owns is that area's fix: send the smallest fixture and what tclsh prints.
- An issue closes with state reason `completed` and a short comment naming the commits and the witnesses. A decision that changes what an issue asks for is the owner's: send it to the orchestrator and take your next item.
- F5 behaviour that needs a device is a numbered check in `bigip-name-checks.md`, never a guess.
- Report to the orchestrator after each pushed batch: commits, "fixed N, new 0", what is next.
