# RUST_ISSUE_202: `compiler-explorer-gui` ends with `@ls -lh $(EXPLORER_CDN_DIR)/` but `EXPLORER_CDN_DIR` is defined nowhere, expanding to `ls -lh /`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Build tooling & CI |
| **Location** | `Makefile:1154` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

Makefile:1154 — `compiler-explorer-gui` ends with `@ls -lh $(EXPLORER_CDN_DIR)/` but `EXPLORER_CDN_DIR` is defined nowhere, expanding to `ls -lh /`. Confidence: high
