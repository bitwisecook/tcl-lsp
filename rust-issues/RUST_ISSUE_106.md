# RUST_ISSUE_106: `rename_params_in_list` renames raw word-boundary byte matches of a param name anywhere in the param-list region, including inside *other* params' default values

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP navigation / rename / formatting |
| **Location** | `rust/tcl-lsp-core/src/minify.rs:786-807` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/minify.rs:786-807 — `rename_params_in_list` renames raw word-boundary byte matches of a param name anywhere in the param-list region, including inside *other* params' default values.
`proc f {{x 1} {y x}} {…}` compacts `x`→`a` and also rewrites y's default value `x` → `a`, changing the default the proc receives. Confidence: high
