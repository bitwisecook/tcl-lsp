# RUST_ISSUE_180: The quoted-unwrap in `brace_expr` fires whenever the multi-arg span merely *starts and ends* with `"`, producing invalid output

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/refactor/brace_expr.rs:56-60` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/refactor/brace_expr.rs:56-60 — The quoted-unwrap in `brace_expr` fires whenever the multi-arg span merely *starts and ends* with `"`, producing invalid output.
`expr "1" + "2"` → rewritten to `expr {1" + "2}` (stray quotes inside the braced expr → runtime parse error). Confidence: high
