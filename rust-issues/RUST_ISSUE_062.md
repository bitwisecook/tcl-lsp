# RUST_ISSUE_062: `expr` `/`,`%`,`>>` emit UNSIGNED opcodes while accepting negatives and using SIGNED comparisons → silent wrong results (no diagnostic)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `eBPF` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

eBPF — `expr` `/`,`%`,`>>` emit UNSIGNED opcodes while accepting negatives and using SIGNED comparisons → silent wrong results (no diagnostic).
`bin_alu` emits DIV=0x30/MOD=0x90/RSH=0x70 (emit.rs:295-301) — no BPF_SDIV/BPF_ARSH — but `cmp_jop` emits signed JSLT/JSLE/JSGT/JSGE. Negative operand in `$x / $y`, `$x % $y`, `$x >> $n` silently diverges from Tcl's signed floored div / arithmetic shift (which VM and runtime honor). [Same as eBPF report.] Confidence: high
