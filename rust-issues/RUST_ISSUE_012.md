# RUST_ISSUE_012: stray top-level statements are silently DROPPED when a `when` block is present (critical silent mishandle)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `eBPF` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

eBPF — stray top-level statements are silently DROPPED when a `when` block is present (critical silent mishandle).
`compile_module` lowers only `when` calls; the raw-DSL fallback runs only `if !saw_when`, nothing verifies every top-level statement was consumed (frontend.rs:69-94). `when XDP { pass }` followed by a top-level `drop`/`load16 x ctx 0`/`set y 5`/unknown → trailing statement silently discarded, no diagnostic, absent from emitted program. (In-handler out-of-subset constructs are otherwise explicit-reject.) Confidence: high
