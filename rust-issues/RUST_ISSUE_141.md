# RUST_ISSUE_141: the weekly sweep deletes every Actions cache whose ref is not `refs/heads/main`, wiping the active `rust` branch's rust-cache entries that the blocking rust-gate/rust-lsp-e2e workflows depend on

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `.github/workflows/cache-cleanup.yml:62-79` |
| **Status** | Open (fix staged; needs workflow-scope push) |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

.github/workflows/cache-cleanup.yml:62-79 — the weekly sweep deletes every Actions cache whose ref is not `refs/heads/main`, wiping the active `rust` branch's rust-cache entries that the blocking rust-gate/rust-lsp-e2e workflows depend on.
`--jq '.[] | select(.ref != "refs/heads/main") | .id'` — rust-gate.yml/rust-lsp-e2e.yml run on `push: branches: [rust]`, so their caches live on `refs/heads/rust` and get purged every Monday, forcing cold rebuilds. Confidence: high
