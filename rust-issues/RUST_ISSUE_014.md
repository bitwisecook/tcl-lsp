# RUST_ISSUE_014: only two backend pairs are compared; the real WASM runtime, eBPF, and the tree-walking interpreter are never differentially validated

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Backend parity (WASM/VM/eBPF/registry) |
| **Location** | `fuzzer` |
| **Status** | Open — re-verified at the branch tip (2026-08-07) and promoted to **GitHub issue #1313**. Still exactly two pairs: `campaign.rs:100-101` (`tclsh` ↔ `tclvm`) and `wasm_diff.rs` (WASM control flow ↔ `tcl-vm`, both sides evaluating leaf commands on `tcl-vm`). `rust/tcl-fuzz/Cargo.toml` has no `bpf-*` or `runtime/rust` dependency. `harness.rs:114-142` still folds errors to a bool, so error-message text is never compared. |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

fuzzer — only two backend pairs are compared; the real WASM runtime, eBPF, and the tree-walking interpreter are never differentially validated.
Compared: {tclvm ↔ tclsh} (campaign.rs:100-104) and {eval-fallback-WASM-control-flow ↔ tcl-vm} (wasm_diff.rs:291 — self-consistency, commands run on tcl-vm on BOTH sides). Never compared: real linked-runtime WASM vs anything; eBPF vs anything (no `bpf` ref in tcl-fuzz); tcl-vm vs runtime/rust; runtime/rust vs tclsh; tclsh cross-version. Error-message text never compared (folded to bool). Confidence: high
