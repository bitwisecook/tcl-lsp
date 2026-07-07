# RUST_ISSUE_144: both fingerprints hash tracked files only, on the rationale that untracked files "don't affect tests run from tracked sources"; cargo auto-discovers untracked `tests/*.rs`/`src/bin/*.rs`, so a green run whose pass depended on an untracked file yields a stamp certifying a tree that was never tested as-committed

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `scripts/worktree-fingerprint.sh:26-27 and scripts/test-slow-stamp.sh:69-83` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: medium) |

## Finding

scripts/worktree-fingerprint.sh:26-27 and scripts/test-slow-stamp.sh:69-83 — both fingerprints hash tracked files only, on the rationale that untracked files "don't affect tests run from tracked sources"; cargo auto-discovers untracked `tests/*.rs`/`src/bin/*.rs`, so a green run whose pass depended on an untracked file yields a stamp certifying a tree that was never tested as-committed. Confidence: medium
