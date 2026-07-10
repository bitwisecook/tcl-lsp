# RUST_ISSUE_095: `dict incr` overflows silently via `wrapping_add`, both in the inline DICT_INCR_IMM op and the runtime command

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Bytecode VM |
| **Location** | `rust/tcl-vm/src/exec.rs:1942 (and rust/tcl-vm/src/cmd_dict.rs:325)` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-vm/src/exec.rs:1942 (and rust/tcl-vm/src/cmd_dict.rs:325) — `dict incr` overflows silently via `wrapping_add`, both in the inline DICT_INCR_IMM op and the runtime command.
`dict set d k 9223372036854775807; dict incr d k` yields `-9223372036854775808` where tclsh returns bignum `9223372036854775808`. The exact wrapping_add bug that plain `incr` was fixed for (exec.rs:2432 routes `incr` through `int_add`), leaving `dict incr` inconsistent with both tclsh and `incr`. `Ok(Value::int(base.wrapping_add(amount)))` [overlaps wasm-runtime cmd_dict.rs:492] Confidence: high
