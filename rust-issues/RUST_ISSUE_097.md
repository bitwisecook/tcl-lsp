# RUST_ISSUE_097: `IntBinOp::Shr => RSH` emits LOGICAL right shift `BPF_RSH`; Tcl `>>` is arithmetic (sign-preserving) on negatives. Should be `BPF_ARSH` (0xc0), which codegen never emits

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | eBPF pipeline |
| **Location** | `rust/bpf-tcl-codegen/src/ebpf/emit.rs:301` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/bpf-tcl-codegen/src/ebpf/emit.rs:301 — `IntBinOp::Shr => RSH` emits LOGICAL right shift `BPF_RSH`; Tcl `>>` is arithmetic (sign-preserving) on negatives. Should be `BPF_ARSH` (0xc0), which codegen never emits.
`setint x {0 - 8}; setint y {$x >> 1}` → Tcl `-4`; emitted RSH64_REG (logical) gives `0x7FFFFFFFFFFFFFFC`. Aside: rbpf masks the shift count to 6 bits, so `$x >> 70` becomes `>> 6` rather than Tcl's `0`/`-1`. `IntBinOp::Shr => RSH,` Confidence: high
