# RUST_ISSUE_145: the accepted surface is a tiny non-Tcl DSL; identical Tcl source diverges by acceptance vs the other two backends (rejections are explicit, by design). The coverage boundary: `set x 5`, `expr {...}`, `if` that compile on VM/WASM are rejected by eBPF

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `eBPF` |
| **Status** | Closed — by design, no change intended. Re-checked at the branch tip (2026-08-07): `lower.rs:583-600` still rejects `set` / `incr` / a bare `expr` / `return` with a specific `OutOfSubset` diagnostic naming the typed replacement (`setint` / `seti32` / `setbuf`, `accept` / `drop`). That is the documented coverage boundary of a backend whose programs must be a single bounded run to a verdict, and every rejection is explicit — there is no silent divergence to fix. Recorded here so the boundary is not re-filed as a defect. |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

eBPF — the accepted surface is a tiny non-Tcl DSL; identical Tcl source diverges by acceptance vs the other two backends (rejections are explicit, by design). The coverage boundary: `set x 5`, `expr {...}`, `if` that compile on VM/WASM are rejected by eBPF. Confidence: high
