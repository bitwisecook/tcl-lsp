# RUST_ISSUE_023: tcl-registry/src/taint.rs:157 (`is_taint_source`), :246 (`is_sanitiser`), and compiler taint.rs:1233 (`classify_irules_sink`), :1267 (`classify_network_interp_sinks`). Real Tcl accepts unique-prefix subcommands, and the same file's `transform_colour` (taint.rs:354) already uses `resolve_subcommand`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Analyser & diagnostics |
| **Location** | `taint subcommand matching is exact, not prefix-abbreviation aware` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

taint subcommand matching is exact, not prefix-abbreviation aware — tcl-registry/src/taint.rs:157 (`is_taint_source`), :246 (`is_sanitiser`), and compiler taint.rs:1233 (`classify_irules_sink`), :1267 (`classify_network_interp_sinks`). Real Tcl accepts unique-prefix subcommands, and the same file's `transform_colour` (taint.rs:354) already uses `resolve_subcommand`.
False positive: `puts [string le $tainted]` — `string le` is a valid abbreviation of the `string length` sanitiser but `is_sanitiser("string",["le"])` misses it → spurious T101 where `[string length …]` is correctly suppressed. False negative: `HTTP::cookie ins $c $v` dodges IRULE3002 via `matches!(sub, Some("insert"|"replace"))`; `chan g`/`encoding convertf` dodge source classification; `interp ev` dodges T105. Confidence: high
