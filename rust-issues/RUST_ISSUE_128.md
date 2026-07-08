# RUST_ISSUE_128: pkg/venv verbs force ANSI escapes into non-JSON stdout unconditionally (`ui::use_colour(Some(!common.json))`), ignoring pipes and `NO_COLOR`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/pkg.rs:200` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/pkg.rs:200 — pkg/venv verbs force ANSI escapes into non-JSON stdout unconditionally (`ui::use_colour(Some(!common.json))`), ignoring pipes and `NO_COLOR`.
`tcl pkg install > log` / `NO_COLOR=1 tcl pkg verify` still emit `\x1b[32m✓…\x1b[0m` because `use_colour(Some(true))` short-circuits the TTY/`NO_COLOR` auto-detection (ui.rs:49-61); the same pattern is in `run_init`, `run_add`, `run_verify`, `run_vendor`, venv.rs `run_create`/`run_delete`/`run_update`. Confidence: high
