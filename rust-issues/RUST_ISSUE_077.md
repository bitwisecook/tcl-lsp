# RUST_ISSUE_077: `record_version_gate_sites` advances `i += 1` and never skips an option's value word, though its doc claims it mirrors `emit_w004_dialect_invalid_option` (which does skip via `value_word_count`). A value word starting with `-` is re-tested as an option use → duplicate/mis-anchored W136

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Analyser & diagnostics |
| **Location** | `rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs:118-157` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs:118-157 — `record_version_gate_sites` advances `i += 1` and never skips an option's value word, though its doc claims it mirrors `emit_w004_dialect_invalid_option` (which does skip via `value_word_count`). A value word starting with `-` is re-tested as an option use → duplicate/mis-anchored W136.
With `package require Tk 8.6`, `entry .e -placeholder -placeholder` (`-placeholder` value, min_version 8.7) emits **two** W136 instead of one; `entry .e -textvariable -placeholder` emits a spurious W136 on the value. Confidence: medium
