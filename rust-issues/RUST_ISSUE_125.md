# RUST_ISSUE_125: `tcl pkg sync` ("Lock-driven install (alias for install --frozen)") installs nothing: it reads the lockfile, prints each package plus "synced from … (N packages)", and exits 0 without materialising, verifying, or even comparing against the manifest

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | CLIs & tools (tcl/mcp/pkg/sandbox) |
| **Location** | `rust/tcl-cli/src/commands/pkg.rs:802` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

rust/tcl-cli/src/commands/pkg.rs:802 — `tcl pkg sync` ("Lock-driven install (alias for install --frozen)") installs nothing: it reads the lockfile, prints each package plus "synced from … (N packages)", and exits 0 without materialising, verifying, or even comparing against the manifest.
On a fresh clone with `tclpkg.lock`, `tcl pkg sync` reports success while `lib/` stays empty (`run_sync` contains only `println!` calls; note `run_install --frozen` also skips all fetch/materialise via `offline = frozen || common.offline`). Confidence: medium
