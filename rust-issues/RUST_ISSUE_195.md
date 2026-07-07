# RUST_ISSUE_195: `references_to` always uses the single-root graph (`ctx.root.graph()`) even in merge mode, unlike `refs`/`referenced_by` which switch to `ctx.merged_graph()`, so under `--merge` it misses cross-file referrers

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/tcl-bigip-query/src/builtins/graph.rs:213-215` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-bigip-query/src/builtins/graph.rs:213-215 — `references_to` always uses the single-root graph (`ctx.root.graph()`) even in merge mode, unlike `refs`/`referenced_by` which switch to `ctx.merged_graph()`, so under `--merge` it misses cross-file referrers.
With `--merge` and a virtual in file A referencing a pool defined in file B, `references_to("/Common/poolB")` evaluated against file A's root omits/duplicates results per-root instead of walking the merged namespace. Quote: `let root = &ctx.root; let graph = root.graph();`.
Confidence: medium
