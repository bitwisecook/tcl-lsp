# RUST_ISSUE_031: `bin_alu` lowers Tcl `/` and `%` to the UNSIGNED eBPF ops `BPF_DIV`/`BPF_MOD`, but the rest of the pipeline models signed 64-bit Tcl integers (signed compares JSLT…, neg64, sign-extending MOV64_IMM). Any negative operand yields a catastrophically wrong result, silently

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | eBPF pipeline |
| **Location** | `rust/bpf-tcl-codegen/src/ebpf/emit.rs:295-296` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/bpf-tcl-codegen/src/ebpf/emit.rs:295-296 — `bin_alu` lowers Tcl `/` and `%` to the UNSIGNED eBPF ops `BPF_DIV`/`BPF_MOD`, but the rest of the pipeline models signed 64-bit Tcl integers (signed compares JSLT…, neg64, sign-extending MOV64_IMM). Any negative operand yields a catastrophically wrong result, silently.
`when SOCKET_FILTER { setint x {0 - 8}; setint y {$x / 2}; accept {$y} }`: Tcl expects `-8/2 = -4`; emitted DIV64_REG runs `0xFFFFFFFFFFFFFFF8 / 2` unsigned = `9223372036854775804`. eBPF signed div is `off=1` (BPF_SDIV), never emitted. (Tcl also floors vs BPF truncate — a further negative-operand mismatch.) `IntBinOp::Div => DIV, IntBinOp::Mod => MOD,` Confidence: high
