# RUST_ISSUE_159: `instance_method` is documented "breadth-first" but `queue.pop()` on a `Vec` is LIFO (depth-first, reversed siblings). Currently inert (every `superclasses` slice is empty) — latent only

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/registry.rs:398` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-registry/src/registry.rs:398 — `instance_method` is documented "breadth-first" but `queue.pop()` on a `Vec` is LIFO (depth-first, reversed siblings). Currently inert (every `superclasses` slice is empty) — latent only. Confidence: high
