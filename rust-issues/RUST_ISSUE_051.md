# RUST_ISSUE_051: the PostFetch hook's `TCLPKG_PKG_DIR` points at `lib/<name>` but packages are materialised at `lib/<name>-<version>`, so operator security scanners are handed a nonexistent directory

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/pkg.rs:349` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-cli/src/commands/pkg.rs:349 — the PostFetch hook's `TCLPKG_PKG_DIR` points at `lib/<name>` but packages are materialised at `lib/<name>-<version>`, so operator security scanners are handed a nonexistent directory.
A policy hook `command = ["scan", "${TCLPKG_PKG_DIR}"]` on `post-fetch` scans nothing (or errors and blocks every install): `.var("PKG_DIR", lib_dir.join(&name).to_string_lossy())` vs installer.rs:204 `let dest = lib_dir.join(format!("{name}-{version}"));`. Confidence: high
