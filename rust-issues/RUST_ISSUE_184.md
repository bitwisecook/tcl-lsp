# RUST_ISSUE_184: constructor/destructor range/selectionRange are the whole body span instead of the keyword name span

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/document_symbols.rs:264` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/document_symbols.rs:264 — constructor/destructor range/selectionRange are the whole body span instead of the keyword name span.
`constructor {name} { ... }`: outline/breadcrumb highlights the entire body, not the `constructor` keyword; MethodDef.name_span ignored. Destructor identical at :276. Confidence: high
