# RUST_ISSUE_007: ≥13 registry Tcl commands have no runtime handler and no not-required classification, so they error under the WASM/tree-walking path

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `registry↔runtime` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

registry↔runtime — ≥13 registry Tcl commands have no runtime handler and no not-required classification, so they error under the WASM/tree-walking path.
Absent from the 112 `register_builtin` names: `exec, exit, time, timerate, tailcall, lpop, lremove, zlib, pid, fileevent, fcopy, load, socket` — each has a registry spec. `exec/exit/socket/load` appear only in the safe-interp hide list. A miss routes to `unknown` → "invalid command name". Compute-only ones (tailcall, lpop, lremove, time, timerate) have no portability excuse. `exit 0` doesn't exit; `time {...}`/`tailcall f`/`lpop l` raise "invalid command name". Confidence: high
