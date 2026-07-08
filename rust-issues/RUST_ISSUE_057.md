# RUST_ISSUE_057: `install_cli` downloads Python zipapps (`tcl-<ver>.pyz` / `f5-<ver>.pyz`) that this branch's release pipeline never publishes, so the released installer cannot install the CLIs

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Build tooling & CI |
| **Location** | `scripts/install/install.sh:1940` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

scripts/install/install.sh:1940 — `install_cli` downloads Python zipapps (`tcl-<ver>.pyz` / `f5-<ver>.pyz`) that this branch's release pipeline never publishes, so the released installer cannot install the CLIs.
`asset="${name}-${VER_NO_V}.pyz"` → `download` → `die`. ci.yml `publish-native-binaries` uploads only `tcl-<triple>` / `f5-query-<triple>` / `tcl-mcp-<triple>` and claims install.sh fetches a prebuilt binary; only the MCP path was migrated. `scripts/release/smoke_installer.sh` is consistent with the stale scheme, so post-tag smoke also fails. Confidence: high
