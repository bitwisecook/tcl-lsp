# Issue sessions, from 17 November 2026

These sessions close the open issues filed by the repository owner. They run beside the naming burn-down (`docs/work/name-handling/handover/successors.md`) and are separate from it: an issue session owns its issues, a burn-down session owns its area's failing tests. The sessions that worked on these issues until 16 November were paused, wrote the hand-overs in `handover/`, and were replaced on the owner's instruction.

The orchestrator is session `session_017u5g3KDVBH5v5PXpMYy7KP`. It records the owner's design decisions on the issues, rules on questions, and does not own area code.

## Roster

| Slug | Issues | Model | Session | Hand-over |
| --- | --- | --- | --- | --- |
| `bytecode` | #2490 | Opus | `session_017u5g3KBvRVWYHVoF5FSZmT` | `handover/bytecode.md` |
| `server` | #1657, #2243 | Opus | `session_017u5g3HDHiPN1CimB7QiVSF` | `handover/infra.md` |
| `dataflow` | #2408, #1796 | Opus | `session_017u5g3JkH4kppJ1LJpZQUus` | `handover/dataflow.md`, and the #1796 part of `handover/irules-taint.md` |
| `audit` | #1712, #1710 | Opus | `handover/audit.md` |
| `generations-payloads` | #1797, #2426 | Sonnet | `session_017u5g3KGtKSUFqBEdo1S6kR` | `handover/generations-payloads.md` |

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
