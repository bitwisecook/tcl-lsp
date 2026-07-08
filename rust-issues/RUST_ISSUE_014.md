# RUST_ISSUE_014: only two backend pairs are compared; the real WASM runtime, eBPF, and the tree-walking interpreter are never differentially validated

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `fuzzer` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

fuzzer — only two backend pairs are compared; the real WASM runtime, eBPF, and the tree-walking interpreter are never differentially validated.
Compared: {tclvm ↔ tclsh} (campaign.rs:100-104) and {eval-fallback-WASM-control-flow ↔ tcl-vm} (wasm_diff.rs:291 — self-consistency, commands run on tcl-vm on BOTH sides). Never compared: real linked-runtime WASM vs anything; eBPF vs anything (no `bpf` ref in tcl-fuzz); tcl-vm vs runtime/rust; runtime/rust vs tclsh; tclsh cross-version. Error-message text never compared (folded to bool). Confidence: high
