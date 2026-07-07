# RUST_ISSUE_009: TclOO works in the runtime + WASM-eval path but is entirely absent from the bytecode VM

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `VM` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

VM — TclOO works in the runtime + WASM-eval path but is entirely absent from the bytecode VM.
runtime/rust/src/cmd_oo.rs registers `oo::define/objdefine/copy/method/…` unconditionally (runs under WASM via eval→runtime). rust/tcl-vm has no OO whatsoever (grep finds no oo::class/method/self/MethodDef). `oo::class create C {...}` runs under WASM and native but errors/traps in the VM. Confidence: high
