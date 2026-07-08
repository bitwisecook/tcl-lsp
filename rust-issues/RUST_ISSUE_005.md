# RUST_ISSUE_005: `test-tclpkg-tcl` cd's into `tooling/tclpkg/tcl`, which does not exist on this branch, so the mandatory `make test-slow` gate can never pass. [VERIFIED: no tooling/ dir; git ls-files tooling → 0 matches; runner runs it in the parallel batch, exits non-zero, make aborts before `test-slow-stamp.sh write` → explains the missing committed .test-slow.stamp / G5.]

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | critical |
| **Subsystem** | Build tooling & CI |
| **Location** | `Makefile:35,343-345` |
| **Status** | Open |
| **Verification** | Verified firsthand by reviewer |

## Finding

Makefile:35,343-345 — `test-tclpkg-tcl` cd's into `tooling/tclpkg/tcl`, which does not exist on this branch, so the mandatory `make test-slow` gate can never pass. [VERIFIED: no tooling/ dir; git ls-files tooling → 0 matches; runner runs it in the parallel batch, exits non-zero, make aborts before `test-slow-stamp.sh write` → explains the missing committed .test-slow.stamp / G5.]
`TCLPKG_TCL_DIR := $(ROOT)tooling/tclpkg/tcl`; `cd $(TCLPKG_TCL_DIR) && for t in tests/*_test.tcl; ...`. `test-slow` runs `test-tclpkg-tcl` (Makefile:807) with no SKIP var → FAIL → stamp never written → `release`/`publish-*` (depend on verify-test-slow-stamp) fail, and ci.yml `test-slow-stamp` job fails any PR to main. Confidence: high
