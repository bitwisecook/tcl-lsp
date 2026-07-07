# RUST_ISSUE_118: origin-pool resource labels and references interpolate the raw iRule pool name into HCL identifiers, producing syntactically invalid Terraform for the very common `pool /Common/name` form

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | f5-query / report-gen / f5-xc |
| **Location** | `rust/f5-xc/src/terraform.rs:93,142-148` |
| **Status** | Open |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

rust/f5-xc/src/terraform.rs:93,142-148 — origin-pool resource labels and references interpolate the raw iRule pool name into HCL identifiers, producing syntactically invalid Terraform for the very common `pool /Common/name` form.
`pool /Common/web_pool` emits `resource "volterra_origin_pool" "/Common/web_pool"` and `name = volterra_origin_pool./Common/web_pool.name` — `terraform validate` rejects both (slashes/dots in identifiers). Quote: `"{p}      name      = volterra_origin_pool.{}.name", op.name`.
Confidence: high
