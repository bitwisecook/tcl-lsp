# RUST_ISSUE_102: `schedule_diagnostics_impl` marks the slot dirty and *then* resolves `latest_inputs` across an `await`, so a running worker can drain the dirty flag with the stale toggles and the config change is silently never applied

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | LSP server & document sync |
| **Location** | `rust/tcl-lsp-server/src/lib.rs:4503-4516` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-lsp-server/src/lib.rs:4503-4516 — `schedule_diagnostics_impl` marks the slot dirty and *then* resolves `latest_inputs` across an `await`, so a running worker can drain the dirty flag with the stale toggles and the config change is silently never applied.
`reschedule_diagnostics` (didChangeConfiguration) sets `slot.dirty = true`, returns `(false, true)` for a running worker, and only stores the fresh `DiagInputs` after `self.diag_inputs(&uri, &dialect).await`; if the worker's drain lands in that window it consumes `dirty` with the old `latest_inputs` (e.g. `diagnostics_enabled` still true) — squiggles the user just disabled persist until the next keystroke. Confidence: medium
