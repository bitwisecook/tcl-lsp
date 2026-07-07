# RUST_ISSUE_010: structured `if`/`while`/`for` silently swallow error/return/break completion codes from eval'd leaf commands

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `WASM backend` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

WASM backend — structured `if`/`while`/`for` silently swallow error/return/break completion codes from eval'd leaf commands.
`tcl_eval` discards completion codes (codegen_abi.rs:148 "Completion codes are discarded in this tier"); the structured walk emits body/leaf commands as emit_command→tcl_eval (structured.rs:128,136) and `return` as eval-then-`WasmOp::Return` dropping the code (:112-115). So `error` raised inside a compiled while/for/if body does not propagate (loop keeps going), and `return -code error`/`-level N` degrade to a plain return — diverging from VM/runtime/tclsh. Confidence: high
