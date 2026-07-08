# RUST_ISSUE_076: the DFS visited/visiting set is path-scoped (`ctx.visiting.remove(cls)`/`visited.remove(cls)` on backtrack), so a shared sub-DAG is re-explored once per reaching path → Θ(2^k) on k stacked diamonds; `has_super_cycle` runs the same way with no depth cap or cross-class memoization. Reached from the shipping W308 path

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Analyser & diagnostics |
| **Location** | `rust/tcl-compiler/src/analyser/mro.rs:139 & :164` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/analyser/mro.rs:139 & :164 — the DFS visited/visiting set is path-scoped (`ctx.visiting.remove(cls)`/`visited.remove(cls)` on backtrack), so a shared sub-DAG is re-explored once per reaching path → Θ(2^k) on k stacked diamonds; `has_super_cycle` runs the same way with no depth cap or cross-class memoization. Reached from the shipping W308 path.
~30 stacked `oo::class` diamonds (legal multiple inheritance, no cycle) hangs the diagnostics pass; a ~20k-deep linear `superclass` chain overflows the stack (no MAX_DEPTH, unlike param_traits.rs:151). Confidence: medium
