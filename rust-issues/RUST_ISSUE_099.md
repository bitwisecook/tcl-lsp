# RUST_ISSUE_099: tower-lsp-server 0.23 runs notification handlers concurrently (`buffer_unordered(4)`), and `did_change`'s `or_insert_with(|| DocumentState::new(String::new(), …))` lets a reordered change resurrect a closed document or apply a ranged edit to an empty phantom before `did_open` commits

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:4867-4871` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-lsp-server/src/lib.rs:4867-4871 — tower-lsp-server 0.23 runs notification handlers concurrently (`buffer_unordered(4)`), and `did_change`'s `or_insert_with(|| DocumentState::new(String::new(), …))` lets a reordered change resurrect a closed document or apply a ranged edit to an empty phantom before `did_open` commits.
`didOpen`/`didClose` and `didChange` take different lock sequences before `documents`, so under contention a later `didChange` can win the `documents` race: change-before-open splices against `""` then is silently overwritten at the wrong version; change-after-close re-creates the entry and keeps publishing diagnostics for a closed document. Confidence: medium
