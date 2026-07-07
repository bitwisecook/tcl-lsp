# RUST_ISSUE_149: `rebase_function_unit` never shifts `fu.cfg.inline_eval_spans` (span-carrying, contra the module doc's "every other lattice field is span-free" / "byte-identical to a freshly-built unit"), so a memoised, offset-shifted unit keeps stale absolute spans for inlined `eval {…}` bodies

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Compiler middle-end (CFG/SSA/SCCP/optimiser) |
| **Location** | `rust/tcl-compiler/src/lattice_rebase.rs:45-71` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/lattice_rebase.rs:45-71 — `rebase_function_unit` never shifts `fu.cfg.inline_eval_spans` (span-carrying, contra the module doc's "every other lattice field is span-free" / "byte-identical to a freshly-built unit"), so a memoised, offset-shifted unit keeps stale absolute spans for inlined `eval {…}` bodies.
A proc containing a folded `eval {…}` body that is cache-hit after lines are inserted above it yields inline-eval spans pointing at the old offsets for any consumer (error-region mapping / explorer views). Confidence: medium
