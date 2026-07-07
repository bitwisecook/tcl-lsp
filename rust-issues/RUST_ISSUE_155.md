# RUST_ISSUE_155: `has_super_cycle(start)` returns true for *any* cycle reachable from `start`, even when `start` isn't in it; `tcloo_linearise` then errors and `build_class_hierarchy` backfills that class's MRO to `[self]`, so `method_target` returns None and W308 fires on genuinely-inherited methods

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Analyser & diagnostics |
| **Location** | `rust/tcl-compiler/src/analyser/mro.rs:147-169` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/analyser/mro.rs:147-169 — `has_super_cycle(start)` returns true for *any* cycle reachable from `start`, even when `start` isn't in it; `tcloo_linearise` then errors and `build_class_hierarchy` backfills that class's MRO to `[self]`, so `method_target` returns None and W308 fires on genuinely-inherited methods.
`A{superclass B}`/`B{superclass A}` (real cycle) plus `C{superclass A Base}` with `Base` defining `foo` → `$c foo` falsely flagged W308, error names `C` not `A`/`B`. Only triggers on already-cyclic (runtime-invalid) source. Confidence: medium
