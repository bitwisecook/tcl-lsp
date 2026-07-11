# RUST_ISSUE_029: `BEGIN_CATCH4`/`END_CATCH`/`PUSH_RESULT`/`PUSH_RETURN_OPTS` have no arm in `tick()`; they fall to `other =>` "opcode … not implemented in tcl-vm". The inline `dict for`/`dict map`/`dict update`/`dict with` codegen emits `BEGIN_CATCH4` unconditionally for proc bodies (control_flow.rs:236), and the dict-iteration barrier driving it is produced by cfg_builder in BOTH lowerings (mod.rs:855, ungated by faithful_exceptions), unlike try/catch which lower_to_ir_for_bytecode neutralises into runtime barriers. A first-defined global proc whose body inline-compiles `dict for {k v} $d { incr n }` runs the eager is_proc=true body and hits the unimplemented opcode → error instead of iterating. Documented at cmd_control_e2e.rs:369. Note: builtins_e2e.rs:1332 asserts this path succeeds (couldn't run to reconcile)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Bytecode VM |
| **Location** | `rust/tcl-vm/src/exec.rs:2224` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-vm/src/exec.rs:2224 — `BEGIN_CATCH4`/`END_CATCH`/`PUSH_RESULT`/`PUSH_RETURN_OPTS` have no arm in `tick()`; they fall to `other =>` "opcode … not implemented in tcl-vm". The inline `dict for`/`dict map`/`dict update`/`dict with` codegen emits `BEGIN_CATCH4` unconditionally for proc bodies (control_flow.rs:236), and the dict-iteration barrier driving it is produced by cfg_builder in BOTH lowerings (mod.rs:855, ungated by faithful_exceptions), unlike try/catch which lower_to_ir_for_bytecode neutralises into runtime barriers. A first-defined global proc whose body inline-compiles `dict for {k v} $d { incr n }` runs the eager is_proc=true body and hits the unimplemented opcode → error instead of iterating. Documented at cmd_control_e2e.rs:369. Note: builtins_e2e.rs:1332 asserts this path succeeds (couldn't run to reconcile). Confidence: medium
