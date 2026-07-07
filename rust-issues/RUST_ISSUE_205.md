# RUST_ISSUE_205: the MCP launcher fetches `tcl-mcp-<triple>` from `releases/latest` with no SHA256SUMS/cosign verification and executes it; also `releases/latest` excludes pre-releases, so on the odd-minor (rust-line) channel the asset is never found and every cold cache falls through to a full `cargo build`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Build tooling & CI |
| **Location** | `scripts/tcl-mcp:43-53` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

scripts/tcl-mcp:43-53 — the MCP launcher fetches `tcl-mcp-<triple>` from `releases/latest` with no SHA256SUMS/cosign verification and executes it; also `releases/latest` excludes pre-releases, so on the odd-minor (rust-line) channel the asset is never found and every cold cache falls through to a full `cargo build`. Confidence: high
