# RUST_ISSUE_110: the enclosing-body link is appended after the full-width line link, so a body that starts/ends mid-line doesn't contain the line → LSP parent-contains-child invariant broken

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/selection_range.rs:150` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/selection_range.rs:150 — the enclosing-body link is appended after the full-width line link, so a body that starts/ends mid-line doesn't contain the line → LSP parent-contains-child invariant broken.
`proc foo {} {\n    set x 1 }`, cursor at (1,5): line link (1,0)..(1,13) but parent body link (0,13)..(1,12); line end col 13 exceeds body end col 12. VS Code rejects the chain; single-line procs break identically. Confidence: high
