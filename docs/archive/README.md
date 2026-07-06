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

### `kcs-completeness-plan-2026.md`

Phased plan to take the knowledge base (KCS) to 100% coverage of
diagnostic, warning, security, taint, iRule, and optimisation codes.
References a specific PR's todo list for live state; the PR has long
since landed, so this is the "what we set out to do" snapshot rather
than a current planning doc.
