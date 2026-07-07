# RUST_ISSUE_198: `tcl minify --symbol-map FILE` without `--compact`/`--aggressive` silently writes no map file (plain minify yields `map = None`), so later `unminify-error` fails on a missing file

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/transform.rs:217` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/transform.rs:217 — `tcl minify --symbol-map FILE` without `--compact`/`--aggressive` silently writes no map file (plain minify yields `map = None`), so later `unminify-error` fails on a missing file.
`if let (Some(path), Some(map)) = (symbol_map, map)` skips the write with no warning; the flag's help ("Write the symbol map…") documents no dependency on `--compact`. Confidence: high
