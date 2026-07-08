# RUST_ISSUE_183: `is_var_continuation` treats a lone `:` as a name char, pulling a single colon into the resolved variable

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/hover.rs:95` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/hover.rs:95 — `is_var_continuation` treats a lone `:` as a name char, pulling a single colon into the resolved variable.
`$host:$port`, hover on `$host` → name `"host:"` (Tcl substitutes only `host`), lookup misses. Only `::` is a real qualifier. Confidence: high
