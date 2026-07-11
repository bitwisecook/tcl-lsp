# RUST_ISSUE_092: `dict filter … script` uses a loose truthiness test instead of Tcl's boolean parse: `!matches!(s.trim(), "" | "0" | "false" | "no" | "off")`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_dict.rs:336,322` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

runtime/rust/src/cmd_dict.rs:336,322 — `dict filter … script` uses a loose truthiness test instead of Tcl's boolean parse: `!matches!(s.trim(), "" | "0" | "false" | "no" | "off")`.
`dict filter {a 0x0 b 5} script {k v} {expr {$v}}` keeps `a` (0x0 read as true); Tcl parses `0x0` as boolean false and drops it. A non-boolean body result is silently kept instead of raising `expected boolean value`. Confidence: high
