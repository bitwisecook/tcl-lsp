# RUST_ISSUE_061: the bytecode codegen emits opcodes the VM cannot execute, so several valid constructs compile then TRAP at runtime

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `VM` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

VM — the bytecode codegen emits opcodes the VM cannot execute, so several valid constructs compile then TRAP at runtime.
exec.rs:2224 is a catch-all `"opcode <X> not implemented in tcl-vm"`, no dispatch arm (main match exec.rs:814) for `EVAL_STK`, `BEGIN_CATCH4`, `END_CATCH`, `PUSH_RESULT`, `PUSH_RETURN_OPTS`, `SYNTAX`. Yet codegen emits them: value-position multi-command subst `[a; b]` → EVAL_STK (cmd_subst.rs:765-772); proc-context `dict for {k v}`/`dict map` straight-line body → BEGIN_CATCH4/END_CATCH/PUSH_RESULT/PUSH_RETURN_OPTS (control_flow.rs:236,269-271); `if {malformed-expr}` → SYNTAX (control_flow.rs:771). Spurious "opcode … not implemented" instead of the correct result. Not fuzzed. Confidence: medium
