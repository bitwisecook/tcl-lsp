# RUST_ISSUE_109: the inferred-type map is keyed by bare variable name, last-writer-wins across functions, bleeding one scope's type onto same-named vars elsewhere

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP display features |
| **Location** | `rust/tcl-lsp-core/src/inlay_hints.rs:234` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-lsp-core/src/inlay_hints.rs:234 — the inferred-type map is keyed by bare variable name, last-writer-wins across functions, bleeding one scope's type onto same-named vars elsewhere.
`proc a {} { set x 42 }` + `proc b {} { set x "hi" }`: both `x` collapse to one entry, so one definition shows a wrong `: int`/`: str` hint (iteration-order dependent). `type_map.insert(fu.ssa.var_name(*name).to_owned(), display);` Confidence: high
