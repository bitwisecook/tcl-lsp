# RUST_ISSUE_094: a local `lrepeat` shadows the radix-aware `list_core::lrepeat`: decimal-only count (`lrepeat 0x3 a` → error; Tcl → `a a a`), and the negative-count error HARD-CODES `"-1"`: `lrepeat -3 a` → `bad count "-1"…` (Tcl reports `"-3"`)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_list.rs:360,368` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

runtime/rust/src/cmd_list.rs:360,368 — a local `lrepeat` shadows the radix-aware `list_core::lrepeat`: decimal-only count (`lrepeat 0x3 a` → error; Tcl → `a a a`), and the negative-count error HARD-CODES `"-1"`: `lrepeat -3 a` → `bad count "-1"…` (Tcl reports `"-3"`). Confidence: high
