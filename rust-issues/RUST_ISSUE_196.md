# RUST_ISSUE_196: `diag`/`lint`/`validate` (and `minimize`) accept the shared `-o/--output FILE` flag but silently ignore it, always printing to stdout

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/diag.rs:219` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/diag.rs:219 — `diag`/`lint`/`validate` (and `minimize`) accept the shared `-o/--output FILE` flag but silently ignore it, always printing to stdout.
`tcl lint --json -o report.json src/` leaves `report.json` uncreated and dumps JSON to stdout — these handlers use bare `println!` and never construct `OutputTarget` from `input.output`, unlike every other InputArgs verb. Confidence: high
