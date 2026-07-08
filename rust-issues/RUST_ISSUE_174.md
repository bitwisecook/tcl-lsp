# RUST_ISSUE_174: `semantic_tokens_range` is not gated on the `semanticTokens` feature toggle, unlike `semantic_tokens_full`/`full_delta`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:6240-6246` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-server/src/lib.rs:6240-6246 — `semantic_tokens_range` is not gated on the `semanticTokens` feature toggle, unlike `semantic_tokens_full`/`full_delta`.
After `tclLsp.features.semanticTokens = false`, `full` returns `None` but a viewport `semanticTokens/range` request still computes and returns tokens, so highlighting a user disabled keeps rendering in clients that use range requests. Confidence: high
