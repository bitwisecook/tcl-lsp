# RUST_ISSUE_178: A relative `source` literal matched via a *workspace root* is rewritten relative to the *script's directory*, breaking the path it just matched

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/file_ops.rs:165-188` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-lsp-core/src/file_ops.rs:165-188 — A relative `source` literal matched via a *workspace root* is rewritten relative to the *script's directory*, breaking the path it just matched.
`compute_new_literal` always uses `posix_dirname(dep_path)` — match base and rewrite base disagree. Confidence: medium
