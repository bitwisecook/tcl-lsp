# RUST_ISSUE_081: `assigns_variable_at: Some(0)` contradicts unset's own arg-role resolver (which skips `-nocomplain`/`--`) and the field's documented meaning

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Command registry |
| **Location** | `rust/tcl-registry/src/commands/tcl/unset_.rs:69` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-registry/src/commands/tcl/unset_.rs:69 — `assigns_variable_at: Some(0)` contradicts unset's own arg-role resolver (which skips `-nocomplain`/`--`) and the field's documented meaning.
Consumer tcl-compiler/src/side_effects.rs:733 checks `assigns_variable_at` *before* the `DESTROYS_VARIABLE` branch (:765), so for unset the destroy classification is unreachable, and `unset -nocomplain x` is recorded as a Variable *write* keyed `"-nocomplain"` — the real victim `x` (and every name after the first in `unset a b c`) is never invalidated. Confidence: high
