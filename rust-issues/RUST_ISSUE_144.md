# RUST_ISSUE_144: both fingerprints hash tracked files only, on the rationale that untracked files "don't affect tests run from tracked sources"; cargo auto-discovers untracked `tests/*.rs`/`src/bin/*.rs`, so a green run whose pass depended on an untracked file yields a stamp certifying a tree that was never tested as-committed

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `scripts/worktree-fingerprint.sh:26-27 and scripts/test-slow-stamp.sh:69-83` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

scripts/worktree-fingerprint.sh:26-27 and scripts/test-slow-stamp.sh:69-83 — both fingerprints hash tracked files only, on the rationale that untracked files "don't affect tests run from tracked sources"; cargo auto-discovers untracked `tests/*.rs`/`src/bin/*.rs`, so a green run whose pass depended on an untracked file yields a stamp certifying a tree that was never tested as-committed. Confidence: medium

## Resolution

Fixed by removing the whole stamp/fingerprint layer. `scripts/worktree-fingerprint.sh`
and `scripts/test-slow-stamp.sh` were deleted, along with the committed
`.test-slow.stamp` CI gate (the `test-slow-stamp` CI job, `make
verify-test-slow-stamp`, and the release/publish prerequisites) and the local
pre-push stamp gate. With no fingerprint certifying a tree, the tracked-only
under-coverage it described no longer exists; the local gates are enforced by
the agent rule in AGENTS.md instead.
