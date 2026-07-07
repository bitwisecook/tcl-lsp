# RUST_ISSUE_126: `tcl venv delete --force` ignores `--force` entirely, so the documented "Force deletion even if active" (and the library's own hint "use --force") can never work

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/venv.rs:111` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/venv.rs:111 — `tcl venv delete --force` ignores `--force` entirely, so the documented "Force deletion even if active" (and the library's own hint "use --force") can never work.
With `TCL_VENV` pointing at the venv, `tcl venv delete --force .venv` still fails: `fn run_delete(path: &Path, _force: bool, json: bool)` never passes force, and `delete_venv` unconditionally errors `"cannot delete the currently active venv"` with hint `"run 'deactivate' first or use --force"`. Confidence: high
