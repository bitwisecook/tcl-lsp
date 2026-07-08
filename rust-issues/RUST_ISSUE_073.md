# RUST_ISSUE_073: `handle_uplevel` treats `uplevel 0` identically to `uplevel #0`, so it does NOT escape current-frame locals the body touches

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Compiler lowering / variable scoping |
| **Location** | `rust/tcl-compiler/src/var_escape/walker.rs:241 and var_escape/cfg_propagation/walker.rs:231` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/var_escape/walker.rs:241 and var_escape/cfg_propagation/walker.rs:231 — `handle_uplevel` treats `uplevel 0` identically to `uplevel #0`, so it does NOT escape current-frame locals the body touches.
`uplevel 0` runs the script in the *current* frame (not global `#0`), so `set x …` inside it name-writes the proc's local `x` like `eval` (which `handle_eval` correctly walks). Because `x` is never tagged `Frame`, `safe_for_frame_elision()` returns true for `proc p {} { set x 1; uplevel 0 {set x 2}; return $x }` whose frame the interpreted body needs. `if first != "#0" && first != "0"` wrongly whitelists `"0"`. Consumer is the WASM emitter (other consumers shielded by `has_fallback`). Confidence: medium
