# RUST_ISSUE_129: `expand()` iterates bytes and does `out.push(bytes[i] as char)`, mojibake-corrupting any non-ASCII hook command, program path, or fs-read/fs-write entry

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-pkg/src/hooks.rs:219` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-pkg/src/hooks.rs:219 — `expand()` iterates bytes and does `out.push(bytes[i] as char)`, mojibake-corrupting any non-ASCII hook command, program path, or fs-read/fs-write entry.
A hook `command = ["/opt/prüfer", …]` becomes `/opt/prÃ¼fer` (each UTF-8 byte re-encoded as a Latin-1 char), so the spawn fails and the install aborts with a confusing path error. Applies to `program`, args, `fs_read`, `fs_write`. Confidence: high
