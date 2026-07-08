# RUST_ISSUE_153: when a grouped rewrite member is dropped for overlap, surviving siblings have their group cleared but are kept (not dropped), so a group can be applied partially despite the documented "all-or-nothing" intent

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Optimiser passes (inlining/taint/expr-simplify) |
| **Location** | `rust/tcl-compiler/src/optimiser/helpers/select.rs:85` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/optimiser/helpers/select.rs:85 — when a grouped rewrite member is dropped for overlap, surviving siblings have their group cleared but are kept (not dropped), so a group can be applied partially despite the documented "all-or-nothing" intent.
For an O104/O130/O119 fold whose replacement sits on the last statement, if that member is knocked out by an overlap while earlier deletions are already selected, the deletions survive ungrouped (e.g. `set s ""; append s foo; append s bar` losing the fold → `s` = `bar`). Hard to hit (needs overlap on the last chain statement) but clear-vs-drop is the wrong primitive for atomic groups. Confidence: medium
