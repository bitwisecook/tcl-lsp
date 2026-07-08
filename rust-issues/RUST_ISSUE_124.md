# RUST_ISSUE_124: `--left-source`/`--right-source` are unusable on their own: `run_diff` bails when the positional paths are absent, although they are `Option<PathBuf>` precisely for inline use

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/diff.rs:293` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/diff.rs:293 — `--left-source`/`--right-source` are unusable on their own: `run_diff` bails when the positional paths are absent, although they are `Option<PathBuf>` precisely for inline use.
`tcl diff --left-source 'set x 1' --right-source 'set x 2'` → `error: diff requires a left input` (exit 2). `let Some(left) = left else { anyhow::bail!("diff requires a left input"); };` then the inline source is only ever *appended* to the path's documents. Confidence: high
