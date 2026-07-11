# RUST_ISSUE_169: unguarded integer overflow the shared core guards with `saturating_*`: `lrepeat`'s `count as usize * values.len()` and `ledit`'s `hi = (last + 1)…`. `lrepeat 9223372036854775807 a` / `ledit l 0 9223372036854775807 X` panic (debug) or wrap (release) instead of Tcl's clamp / `max length exceeded`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | WASM codegen & Rust runtime |
| **Location** | `runtime/rust/src/cmd_list.rs:371,553` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

runtime/rust/src/cmd_list.rs:371,553 — unguarded integer overflow the shared core guards with `saturating_*`: `lrepeat`'s `count as usize * values.len()` and `ledit`'s `hi = (last + 1)…`. `lrepeat 9223372036854775807 a` / `ledit l 0 9223372036854775807 X` panic (debug) or wrap (release) instead of Tcl's clamp / `max length exceeded`. Confidence: medium
