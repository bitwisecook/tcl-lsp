# RUST_ISSUE_145: the accepted surface is a tiny non-Tcl DSL; identical Tcl source diverges by acceptance vs the other two backends (rejections are explicit, by design). The coverage boundary: `set x 5`, `expr {...}`, `if` that compile on VM/WASM are rejected by eBPF

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `eBPF` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

eBPF — the accepted surface is a tiny non-Tcl DSL; identical Tcl source diverges by acceptance vs the other two backends (rejections are explicit, by design). The coverage boundary: `set x 5`, `expr {...}`, `if` that compile on VM/WASM are rejected by eBPF. Confidence: high
