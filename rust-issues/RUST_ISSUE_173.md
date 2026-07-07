# RUST_ISSUE_173: `Inst::Load` emits `ldx` off the packet pointer with NO `data_end` bounds check before the access; the kernel verifier would reject every packet-reading program. Safe under the actual rbpf execution path (runtime bounds-checks) and the ELF/kernel path is documented not-yet-wired (elf.rs:33-39, maps hard-rejected). Flagged for the kernel-target roadmap; not a silent-wrong-code defect under rbpf

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | eBPF pipeline |
| **Location** | `rust/bpf-tcl-codegen/src/ebpf/emit.rs:245-247` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/bpf-tcl-codegen/src/ebpf/emit.rs:245-247 — `Inst::Load` emits `ldx` off the packet pointer with NO `data_end` bounds check before the access; the kernel verifier would reject every packet-reading program. Safe under the actual rbpf execution path (runtime bounds-checks) and the ELF/kernel path is documented not-yet-wired (elf.rs:33-39, maps hard-rejected). Flagged for the kernel-target roadmap; not a silent-wrong-code defect under rbpf. Confidence: high
