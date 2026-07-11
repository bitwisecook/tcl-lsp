# RUST_ISSUE_146: stale (codegen_abi.rs provides a working leak-tested eval surface). runtime-execution-gaps.md lists dictGet/dictSet/upvar/nsupvar/lsetFlat as "enum-only [not executed]" but exec.rs now dispatches all of them

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `doc/tracker drift: backend.rs:28-30 claims the runtime wasm export surface "is still a stub (capi.rs)"` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

doc/tracker drift: backend.rs:28-30 claims the runtime wasm export surface "is still a stub (capi.rs)" — stale (codegen_abi.rs provides a working leak-tested eval surface). runtime-execution-gaps.md lists dictGet/dictSet/upvar/nsupvar/lsetFlat as "enum-only [not executed]" but exec.rs now dispatches all of them. Confidence: high
