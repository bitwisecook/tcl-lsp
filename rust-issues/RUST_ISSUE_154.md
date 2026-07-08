# RUST_ISSUE_154: a memory-`Use` is annotated with the current global `version_counter`, which the same statement's `Def` loop (639-649) already incremented, so a self-referential aliased statement records the post-write version as the read's `reaching_version`; the counter is also a single DFS-preorder monotonic value, so reaching_version isn't a genuine reaching def across merges

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Optimiser passes (inlining/taint/expr-simplify) |
| **Location** | `rust/tcl-compiler/src/memory_ssa.rs:652` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/memory_ssa.rs:652 — a memory-`Use` is annotated with the current global `version_counter`, which the same statement's `Def` loop (639-649) already incremented, so a self-referential aliased statement records the post-write version as the read's `reaching_version`; the counter is also a single DFS-preorder monotonic value, so reaching_version isn't a genuine reaching def across merges.
`upvar c x; set x [expr {$x + 1}]` — the use of `x` is tagged with the def's new version. Consumers are dataflow/graph visualisations + interval_bounds, so impact bounded. Confidence: medium
