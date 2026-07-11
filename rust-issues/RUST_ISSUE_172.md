# RUST_ISSUE_172: `setint`, `seti32`, and `setu32` all lower identically to a 64-bit `Ty::Int` + plain `Inst::Copy`; no 32-bit truncation, sign-, or zero-extension. A value overflowing 32 bits is silently kept at 64 bits, so `seti32`/`setu32` do not deliver their named width. Documented as a follow-on in ty.rs:33-35, so an acknowledged gap — but silent (no diagnostic) rather than an explicit "unsupported" error

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | eBPF pipeline |
| **Location** | `rust/bpf-tcl-ir/src/lower.rs:478` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/bpf-tcl-ir/src/lower.rs:478 — `setint`, `seti32`, and `setu32` all lower identically to a 64-bit `Ty::Int` + plain `Inst::Copy`; no 32-bit truncation, sign-, or zero-extension. A value overflowing 32 bits is silently kept at 64 bits, so `seti32`/`setu32` do not deliver their named width. Documented as a follow-on in ty.rs:33-35, so an acknowledged gap — but silent (no diagnostic) rather than an explicit "unsupported" error. Confidence: medium
