# RUST_ISSUE_204: binaryen installs unverified on aarch64 (`expected_sha=""`), and wasi-sdk checksum verification is silently skipped whenever the SHA256SUMS fetch fails — both undermine the hook's own pinned-checksum supply-chain policy. ensure-test-deps.sh:737-748 has the same wasi-sdk soft-skip

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Build tooling & CI |
| **Location** | `.claude/hooks/session-start.sh:211-221,265-273` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

.claude/hooks/session-start.sh:211-221,265-273 — binaryen installs unverified on aarch64 (`expected_sha=""`), and wasi-sdk checksum verification is silently skipped whenever the SHA256SUMS fetch fails — both undermine the hook's own pinned-checksum supply-chain policy. ensure-test-deps.sh:737-748 has the same wasi-sdk soft-skip. Confidence: high
