# docs/archive — historical snapshots

This folder holds point-in-time tracking documents from completed
projects.  Each entry is a snapshot of how things stood at a specific
date / branch — **not** a description of how things work now.  When
you're reading current architecture, start from
[`docs/design/`](../design/) or [`AGENTS.md`](../../AGENTS.md)
instead.

Files here are kept for:

- citing decisions in retrospect ("why did we do X?")
- providing historical context to readers who land here from old
  commits or PRs
- preserving perf baselines so we can chart drift over time

They are NOT updated when the codebase moves.  Stale paths and
references to old branch names are expected.

## Contents

### `wasm-tcl9-parity-2026-04/`

WASM runtime vs Tcl 9.0 perf + correctness report from
2026-04-25, branch `claude/tcl-wasm-performance-profile-QP0yH`.  Eleven
sub-reports covering end-to-end perf, stress runs, micro-benchmarks,
correctness gaps, hot-spot analysis, recommendations, the tcltest
suite results, and an after-action note describing what shipped from
phases 0–6.

Use this when:

- You're looking at a current WASM perf regression and want a
  baseline to compare against.
- You're picking up the "tier-2 specialisation" or "long-tail
  TclOO/coroutines" deferred items — the recommendations doc is
  still the best starting point for the design space.
- You want to see what the runtime looked like the day before the
  Phase 0–6 push.

### `zig-runtime-roadmap-2026-04.md`

Companion to the perf report — "work that didn't ship in the
unattended Phase 0–4 push".  Phase status table + concrete first
steps for each deferred item.

### `kcs-completeness-plan-2026.md`

Phased plan to take the knowledge base (KCS) to 100% coverage of
diagnostic, warning, security, taint, iRule, and optimisation codes.
References a specific PR's todo list for live state; the PR has long
since landed, so this is the "what we set out to do" snapshot rather
than a current planning doc.
