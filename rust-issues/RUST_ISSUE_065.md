# RUST_ISSUE_065: the runtime's `lset`/`ledit` use a radix-less local index parser, diverging from `lindex`/`lrange` and from the VM

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `list` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

list — the runtime's `lset`/`ledit` use a radix-less local index parser, diverging from `lindex`/`lrange` and from the VM.
lindex/lrange share tcl-cmd-core::index::resolve (radix-aware); runtime lset/ledit call a local `index_spec` that is ASCII-decimal only (cmd_list.rs:84,95,435,546). The VM routes lset/ledit through the shared parser (command.rs:957). `lset x 0x1 v` works in the VM, errors in the runtime. [Overlaps wasm-runtime report.] Confidence: high
