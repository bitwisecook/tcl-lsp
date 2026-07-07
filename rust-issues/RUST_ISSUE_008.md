# RUST_ISSUE_008: coroutines/`yield`/`yieldto` error on the wasm32 target though the native runtime implements them; the VM lacks them entirely

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `WASM backend` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

WASM backend — coroutines/`yield`/`yieldto` error on the wasm32 target though the native runtime implements them; the VM lacks them entirely.
cmd_coro.rs:445-473 cfg-gates the wasm32 impls to `set_error("coroutines are not supported in the single-threaded wasm build")` (native at :391-438). `yieldto` errors on BOTH targets (:412 "not yet implemented"). The bytecode VM has zero coroutine support. Across backends: native ✓, WASM target ✗ (runtime error), VM ✗ (missing). Confidence: high
