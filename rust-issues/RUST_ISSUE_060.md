# RUST_ISSUE_060: the workflow runs `uv sync --extra dev` (no pyproject.toml) and `make test-vm` (no such target; the Makefile target is `vm-test`), so it can never pass. [VERIFIED: vm-tests.yml has both; Makefile target is `vm-test` at line 1426, not `test-vm`; no ci-fast rule.]

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | Build tooling & CI |
| **Location** | `.github/workflows/vm-tests.yml:26,29` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

.github/workflows/vm-tests.yml:26,29 — the workflow runs `uv sync --extra dev` (no pyproject.toml) and `make test-vm` (no such target; the Makefile target is `vm-test`), so it can never pass. [VERIFIED: vm-tests.yml has both; Makefile target is `vm-test` at line 1426, not `test-vm`; no ci-fast rule.]
Dead Python-era workflow (workflow_dispatch-only). Confidence: high
