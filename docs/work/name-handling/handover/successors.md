# Successor sessions, from 17 November 2026

The burn-down continues with new sessions. This file is what every one of them reads after `AGENTS.md` and `fixing-guide.md`.

## Roster

| Slug | Area | Model | Session |
| --- | --- | --- | --- |
| `analyser-binding` | Compiler analyser and binding (the walk, realm, `command_binding`, `registry_invocation`) | Opus | `session_017u5g3HAPLcJWWX93pMpk32` |
| `class-facts` | Class and object facts | Opus | `session_017u5g3Kgho1C9zZ3yZYrK8P` |
| `diagnostics` | Analyser diagnostics | Opus | `session_017u5g3HXzMVEcpUS6bxGcmA` |
| `dataflow-optimiser` | Compiler dataflow and optimiser | Opus | `session_017u5g3MFyobSx3WBcgjea2s` |
| `values` | Value transfers, escape, shimmer, types | Opus | `session_017u5g3KpPQPTbtTswiHYkaw` |
| `taint` | Taint and iRules checks | Opus | `session_017u5g3HGxE6CxLNW1DgfVr9` |
| `execution` | Codegen, VM and runtimes | Opus | `session_017u5g3HoXkKEw6EtdbxXPDn` |
| `analysis-performance` | Analysis performance, and per-body incremental analysis | Opus | `session_017u5g3LmJPuBQtB5DVx5VsA` |
| `lsp-features` | LSP features (`tcl-lsp-core`, navigation end to end) and semantic tokens | Opus | `session_017u5g3MRrNmuqWZxa9Lcp4F` |
| `server-e2e-db` | LSP server end to end and `tcl-lsp-db` | Sonnet | `session_017u5g3KHQpTL4RNf7ZN9wiQ` |
| `irules-registry-tooling` | iRules, registry, SpecTcl, tooling | Sonnet | `session_017u5g3GuvSUJA9Qanj2AVvk` |
| `tk-oo` | Tk and TclOO issues (#2324, #2427, #1956, #1655, #832) | Opus | `session_017u5g3KsHnGzMmqgP3e6dJj` |
| `live-inlining` | Pack reference bodies inlined on the VM's live compile: landed, switched off until an error inside a spliced body presents as tclsh's does; then discrete command-binding items from `analyser-binding` | Opus | `session_017u5g3KCTRHzSduK23yDtc2` |

The coordinator is session `session_017u5g3KpKgjfMZcPMj8rx4F`. It rules on questions, keeps the scoreboard and does not write area code. `tcl-syntax::naming`, `tcl-lexer` and `xtask` have no session: a session that needs a change there asks the coordinator, then makes it as one small separate commit with the tclsh oracle in the test, after the `tcl-syntax` and `tcl-lexer` suites, `make xtask-gates` and the whole `tcl-compiler` suite against its baseline, and tells the analyser-binding session before pushing.

The owner's open issues are worked by a separate set of sessions under their own orchestrator (session `session_017u5g3KDVBH5v5PXpMYy7KP`); their roster and hand-overs are in `docs/work/issue-sessions/`. They push to `work` too and announce changes in an area's files to that area's session: answer them directly. Issues #2324, #1956 and #1655 stay with `tk-oo`; queue item "No procedure has a transfer summary" of `dataflow-optimiser` is the issue session `dataflow`'s (#2408, #1796).

## Starting

1. Read `AGENTS.md`, `docs/work/name-handling/fixing-guide.md`, this file, then your hand-over `docs/work/name-handling/handover/<slug>.md` and the rulings at the top of `docs/work/name-handling/current-state.md`. Your hand-over's rulings are settled: do not reopen them.
2. Take a fresh baseline of your suites on the current tree before changing anything. The lists in `docs/work/name-handling/lists/` predate the file split: compare by the last path segment of the test name. Commit your new list as `lists/<slug>-failing-<date>.txt`.
3. Held work is in `docs/work/name-handling/patches/handover-<slug>-*.patch`. Most patches were cut before the split: re-apply each hunk in the module the item moved to (`grep -rn 'fn <name>' rust/<crate>/src`, or the file's plan under `docs/work/file-split/plans/`). A patch is unverified until you have run it; delete its file in the commit that lands or rejects it.
4. Then take your hand-over's queue in order.

## Rules that were learned the hard way

- After the final rebase and before every push: `cargo check` on the crates you touched and their dependents, then clippy. `work` did not compile twice because a push was measured before its rebase.
- `make xtask-gates` runs 38 gates. `cargo xtask file-sizes --check` refuses a Rust file over 2,000 lines: put new code in the module it belongs to or a new named module, never `part2`, `misc` or `helpers`. A new module imports each name from the module that defines it, not through `use super::*;` (test modules may keep it). An import used under one configuration only is gated like its use.
- `work` is never force-pushed. A shallow fetch can print `forced update` or leave your branch looking diverged when the shallow boundary moves: deepen it (`git fetch --depth=200 origin work`) and rebase. Never `git reset --hard` over local commits, and never tell another session to.
- A change to binding, class, member, instance or reference facts is also measured on the proof consumers that read them, before the push: `cargo nextest run -p tcl-lsp-core --lib -E 'test(rename) | test(refactor) | test(references) | test(definition)'` against a baseline. A rename or refactor that newly accepts what it refused is a safety regression and is fixed before anything else. A change to the naming owner is measured on the `tcl-lsp-core` library and the `tcl-lsp-server` unit tests as well.
- `make xtask-gates` before every push that touches `tcl-registry`, `tcl-spectcl`, `tcl-dialect`, a pack, `xtask` or anything a gate reads.
- One cause per commit, with the tclsh or jimsh evidence in the test or the message. Report every push as "fixed N, new 0" against your list. A change that fixes some tests and breaks others is not done.
- Never weaken an assertion, never `#[ignore]`, no `#[allow]`, no known-failure list. When two tests contradict, send both to the coordinator.
- A fact another area owns is that area's fix: send the smallest fixture and what tclsh prints, by `send_message` to its session, and carry on with your next item. Do not work round it and do not edit that area's files beyond a small, separate, announced commit.
- A question for the coordinator must not stop you: send it, take the next item, and act on the answer when it comes. Messages can arrive hours late; call `ReadNotifications` when a turn starts.
- F5 behaviour that needs a device is a numbered check in `docs/work/name-handling/bigip-name-checks.md`, never a guess. F5 units take Tcl 8.4 expression rules.
- Collapse tests that assert the same thing at the same layer once they pass, as tables; not while they fail.
- Disk and memory are small (4 cores, 15 GB, about 10 GB of free disk): one cargo build at a time, the test profile at `opt-level 1`, and remove `target/debug/incremental` when the disk fills.
- Keep your area's section of `current-state.md` a short current snapshot; replace the outgoing session's notes after your first full run.

## Reporting

After each pushed batch send the coordinator one short message: commits, "fixed N, new M", the area's failing count, what is next, and any question. Nothing else is needed; do not wait for a reply.
