# Burn-down hand-over, 17 November 2026

The owner is replacing the burn-down sessions with new ones. Each outgoing session writes one hand-over here, `SLUG.md`, that a successor in a fresh container can start from. The successor has no access to the outgoing container, its scratchpad, its stashes or its conversation: what is not in the repository is lost.

An outgoing session stays paused for code work and does exactly this, and nothing else (no fixes, no test-suite runs, no rebuilds beyond what the steps need):

1. `git fetch --depth=50 origin work` and get onto the restructured tree (rebase; `work` is at 7cf404ef or later). Do not push any code change.

2. Export every piece of held work as a patch file under `docs/work/name-handling/patches/`, named `handover-SLUG-<topic>.patch`: local commits (`git format-patch --stdout <base>..<commit>`), stashes, uncommitted edits, side branches and worktrees, patch files outside the repository. Say in the hand-over which commit each patch applies to (most are against the pre-restructure layout; say so) and how far it was verified. If a held commit rebases cleanly onto the restructured tree, export the rebased form instead and say so. Do not apply them to `work`.

3. Write `docs/work/name-handling/handover/SLUG.md` (UK spelling, plain sentences, no padding), with these sections:
   - Scope: the crates, modules and test groups this area owns, and what it explicitly does not.
   - How to measure: the exact commands for your area's suite (environment variables, profile flags, exclusions such as the 1,200-second test), how long they take, and how you compare against a baseline (by last path segment since the file split).
   - Current numbers: failing and total at your last full run, with the commit it ran on and the date.
   - Baseline list: commit your latest failing-test list as `docs/work/name-handling/lists/SLUG-failing-2026-11-16.txt` (one test per line) and name it here.
   - Queue: what to do next, in order; for each item the cause as far as you traced it, the smallest fixture, the files involved (paths on the restructured tree), and the tests it should turn green.
   - Held work: each patch from step 2, what it does, what is unverified.
   - Open questions and rulings: every question you have asked that is unanswered, and every coordinator ruling you are working under (so the successor does not re-litigate them).
   - Hand-offs: what you sent to other areas and are waiting on, and what other areas sent you that you have not started.
   - Hazards: mistakes you made or nearly made that the successor should not repeat (build breaks after rebase, tests that flip between runs, disk limits, slow suites, fixtures that looked wrong but were right).
   - Oracle habits: how you checked expectations against tclsh 8.4 to 9.1 and Jim in this container (paths, scripts).

4. Commit only those docs files (`docs: hand-over for SLUG`), with your usual trailers, rebase onto `origin/work` and push to `work`. Several sessions push at once: on a rejected push, fetch, rebase and retry. Each session writes its own files, so there are no conflicts.

5. Reply to me with the pushed commit id and one line per patch file. Then stop; you will not be resumed.

| Slug | Area |
| --- | --- |
| `analyser-binding` | compiler analyser and binding (the walk) |
| `dataflow-optimiser` | compiler dataflow and optimiser |
| `execution` | codegen, VM and runtimes |
| `lsp-features` | LSP features |
| `irules-registry-tooling` | iRules, registry, SpecTcl, tooling |
| `diagnostics` | analyser diagnostics |
| `taint` | taint and iRules checks |
| `values` | value transfers, escape, shimmer, types |
| `server-e2e-db` | LSP server end to end and database |
| `class-facts` | class and object facts |
| `analysis-performance` | analysis performance |
| `tk-oo` | Tk and TclOO issues |
