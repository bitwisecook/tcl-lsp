# RUST_ISSUE_137: the hook's minimum documented gate `make ci-fast` no longer exists (retired), so its remediation advice fails and the accepted `tmp/ci-fast.stamp` is never written by anything. [VERIFIED: grep '^ci-fast:' Makefile → 0.]

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `scripts/hooks/pre-push:9,49,78` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

scripts/hooks/pre-push:9,49,78 — the hook's minimum documented gate `make ci-fast` no longer exists (retired), so its remediation advice fails and the accepted `tmp/ci-fast.stamp` is never written by anything. [VERIFIED: grep '^ci-fast:' Makefile → 0.]
"make ci-fast Fast Python gate (~10s)"; users following the hook's own instructions get "No rule to make target 'ci-fast'". Confidence: high
