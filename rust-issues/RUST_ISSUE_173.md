# RUST_ISSUE_173: `Inst::Load` emits `ldx` off the packet pointer with NO `data_end` bounds check before the access; the kernel verifier would reject every packet-reading program. Safe under the actual rbpf execution path (runtime bounds-checks) and the ELF/kernel path is documented not-yet-wired (elf.rs:33-39, maps hard-rejected). Flagged for the kernel-target roadmap; not a silent-wrong-code defect under rbpf

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | eBPF pipeline |
| **Location** | `rust/bpf-tcl-codegen/src/ebpf/emit.rs:245-247` |
| **Status** | Fixed — re-checked at the branch tip (2026-08-07). `emit_load` (`ebpf/emit.rs:501-560`) now emits the verifier-safe shape: `r6` holds `data`, `r7` holds `data_end`, and every packet dereference is dominated by an `r2 = r6 + off + width; if r2 <= r7` bounds proof that takes the out-of-bounds verdict otherwise. `ebpf/verifier.rs:29-48` is a standing pass asserting the property, and `emit.rs:816` asserts "load must be dominated by a `data_end` bounds check". |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/bpf-tcl-codegen/src/ebpf/emit.rs:245-247 — `Inst::Load` emits `ldx` off the packet pointer with NO `data_end` bounds check before the access; the kernel verifier would reject every packet-reading program. Safe under the actual rbpf execution path (runtime bounds-checks) and the ELF/kernel path is documented not-yet-wired (elf.rs:33-39, maps hard-rejected). Flagged for the kernel-target roadmap; not a silent-wrong-code defect under rbpf. Confidence: high
