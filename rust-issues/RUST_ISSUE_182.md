# RUST_ISSUE_182: `binary_field_bytes` computes `unit * count` without checked arithmetic; a large parseable count overflows u32

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/hover.rs:931` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-lsp-core/src/hover.rs:931 — `binary_field_bytes` computes `unit * count` without checked arithmetic; a large parseable count overflows u32.
Hover on `binary format {d600000000} v`: 600000000 × 8 overflows (debug panic; release wrapped bogus size). Confidence: medium
