# RUST_ISSUE_041: `split_inline_keys`/`split_compact_props` re-split quoted multi-word scalar values on whitespace and treat any inner word equal to a known property name as a new property, corrupting the model

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-bigip/src/parser/bespoke.rs:261` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-bigip/src/parser/bespoke.rs:261 — `split_inline_keys`/`split_compact_props` re-split quoted multi-word scalar values on whitespace and treat any inner word equal to a known property name as a new property, corrupting the model.
`ltm virtual vs { description "TLS enabled for clients" disabled ... }` → `enabled` is a known prop → description becomes `"TLS`, a phantom `enabled` prop is inserted, and the explicitly-disabled VS reports `state_flag = "enabled"`. Same path fabricates `pool`/`reject`/`source`/`mask` from description words on `ltm virtual`, `ltm virtual-address`, `net route`, `net self` (bespoke.rs:699, 854, 1380, 1406). No quote-awareness. Confidence: high
