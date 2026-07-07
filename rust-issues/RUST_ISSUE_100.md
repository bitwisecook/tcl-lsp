# RUST_ISSUE_100: `selection_range` drops positions that yield no chain, breaking the LSP requirement that `result[i]` answers `positions[i]`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:6933` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-server/src/lib.rs:6933 — `selection_range` drops positions that yield no chain, breaking the LSP requirement that `result[i]` answers `positions[i]`.
With multi-cursor Expand Selection where one cursor sits on empty space (`materialise_selection_range` → `None`), `let lifted: Vec<SelectionRange> = result.into_iter().flatten().collect();` returns a shorter array, so the client pairs the remaining ranges with the wrong cursors. Confidence: high
