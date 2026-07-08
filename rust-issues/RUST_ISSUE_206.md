# RUST_ISSUE_206: stale self-descriptions: report-pyz.yml still says "this file lives here (not under `.github/workflows/`)"; github-pages.yml cites `scripts/verify-explorer-wasm.mjs`, which doesn't exist (actual: `scripts/verify-wasm-externref.mjs`)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Build tooling & CI |
| **Location** | `.github/workflows/report-pyz.yml:16-21 and github-pages.yml:98` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

.github/workflows/report-pyz.yml:16-21 and github-pages.yml:98 — stale self-descriptions: report-pyz.yml still says "this file lives here (not under `.github/workflows/`)"; github-pages.yml cites `scripts/verify-explorer-wasm.mjs`, which doesn't exist (actual: `scripts/verify-wasm-externref.mjs`). Confidence: high
