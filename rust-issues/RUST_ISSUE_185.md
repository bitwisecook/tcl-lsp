# RUST_ISSUE_185: nested symbols dropped for a namespace-qualified proc because the body-scope lookup compares mismatched name forms

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/document_symbols.rs:422` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/document_symbols.rs:422 — nested symbols dropped for a namespace-qualified proc because the body-scope lookup compares mismatched name forms.
`proc ns::outer {} { proc inner {} {} }`: child scope named `ns::outer` while proc_def.name is `outer`, so `find` returns None and `inner` never appears. `child.name == proc_def.name` Confidence: high
