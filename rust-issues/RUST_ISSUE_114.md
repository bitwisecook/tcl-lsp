# RUST_ISSUE_114: the `class match` mock parses `<value> <operator> <datagroup>` positionally and cannot handle leading options / `--`

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | BIG-IP model & iRule-test |
| **Location** | `rust/tcl-irule-test/tcl/command_mocks.tcl:1341` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/tcl-irule-test/tcl/command_mocks.tcl:1341 — the `class match` mock parses `<value> <operator> <datagroup>` positionally and cannot handle leading options / `--`.
`class match -- [HTTP::host] equals hosts` → `dg_name="equals"` → raises `class "equals" not found`, which per-handler `catch` records as an error, silently skipping the rest of the event handler (incl. `pool` selection). Confidence: high
