# RUST_ISSUE_123: `tcl venv create --force <dir>` recursively deletes any existing directory before any venv-marker check, unlike `delete_venv` which refuses non-venvs "to avoid data loss"

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/venv.rs:74` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/venv.rs:74 — `tcl venv create --force <dir>` recursively deletes any existing directory before any venv-marker check, unlike `delete_venv` which refuses non-venvs "to avoid data loss".
`tcl venv create --force .` (or a typo'd path to a real project dir) wipes it: `if force && path.exists() { let _ = std::fs::remove_dir_all(path); }` — no `tclvenv.cfg` check, while venv.rs (lib) `delete_venv` explicitly guards `missing tclvenv.cfg — refusing to delete`. Confidence: high
