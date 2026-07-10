# RUST_ISSUE_167: `string map -nocase` folds case ASCII-only (`region.eq_ignore_ascii_case(key)`), whereas `string equal/compare -nocase` and `toupper/tolower` use Unicode folding. `string map -nocase {ä X} "ÄBC"` → `ÄBC`; Tcl → `XBC`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_string.rs:1075` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

runtime/rust/src/cmd_string.rs:1075 — `string map -nocase` folds case ASCII-only (`region.eq_ignore_ascii_case(key)`), whereas `string equal/compare -nocase` and `toupper/tolower` use Unicode folding. `string map -nocase {ä X} "ÄBC"` → `ÄBC`; Tcl → `XBC`. Confidence: high
