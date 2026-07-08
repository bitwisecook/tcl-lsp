# RUST_ISSUE_199: `fakecmp_suggest_sources` allocates a `Vec` of `tmm_count` buckets with only a `>= 2` lower bound, so a huge `tmm_count` aborts the MCP server on allocation

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-mcp/src/fakecmp.rs:211` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-mcp/src/fakecmp.rs:211 — `fakecmp_suggest_sources` allocates a `Vec` of `tmm_count` buckets with only a `>= 2` lower bound, so a huge `tmm_count` aborts the MCP server on allocation.
A client calling `fakecmp_suggest_sources` with `tmm_count: 1000000000000` triggers `vec![Vec::new(); usize::try_from(tmm_u).unwrap_or(0)]` (~24 TB) → OOM abort of the whole server process (no upper bound, and only ≤64516 candidate tuples exist anyway). Confidence: high
