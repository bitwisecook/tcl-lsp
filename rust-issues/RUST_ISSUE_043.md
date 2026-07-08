# RUST_ISSUE_043: `::orch::reset` unconditionally forces `_tmm_select_mode` back to `"manual"`, so `configure_tests -tmm_select auto` (fakeCMP auto TMM selection) never applies inside `::orch::test` cases

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | high |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-irule-test/tcl/orchestrator.tcl:1017` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-irule-test/tcl/orchestrator.tcl:1017 — `::orch::reset` unconditionally forces `_tmm_select_mode` back to `"manual"`, so `configure_tests -tmm_select auto` (fakeCMP auto TMM selection) never applies inside `::orch::test` cases.
`::orch::test` runs `reset` before every body; `reset` contains `set _tmm_select_mode "manual"`; `run_http_request` then skips `_fakecmp_auto_select`. The crate's own `example_multi_tmm_test.tcl` scenario 4 depends on auto mode and cannot work. Confidence: high
